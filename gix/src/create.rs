use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use gix_config::parse::section;
use gix_discover::DOT_GIT_DIR;

/// The error used in [`into()`].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Could not obtain the current directory")]
    CurrentDir(#[from] std::io::Error),
    #[error("Could not open data at '{}'", .path.display())]
    IoOpen { source: std::io::Error, path: PathBuf },
    #[error("Could not write data at '{}'", .path.display())]
    IoWrite { source: std::io::Error, path: PathBuf },
    #[error("Refusing to initialize the existing '{}' directory", .path.display())]
    DirectoryExists { path: PathBuf },
    #[error("Refusing to initialize the non-empty directory as '{}'", .path.display())]
    DirectoryNotEmpty { path: PathBuf },
    #[error("Could not create directory at '{}'", .path.display())]
    CreateDirectory { source: std::io::Error, path: PathBuf },
}

/// The kind of repository to create.
#[derive(Debug, Copy, Clone)]
pub enum Kind {
    /// An empty repository with a `.git` folder, setup to contain files in its worktree.
    WithWorktree,
    /// A bare repository without a worktree.
    Bare,
}

const TPL_INFO_EXCLUDE: &[u8] = include_bytes!("assets/init/info/exclude");
const TPL_HOOKS_APPLYPATCH_MSG: &[u8] = include_bytes!("assets/init/hooks/applypatch-msg.sample");
const TPL_HOOKS_COMMIT_MSG: &[u8] = include_bytes!("assets/init/hooks/commit-msg.sample");
const TPL_HOOKS_FSMONITOR_WATCHMAN: &[u8] = include_bytes!("assets/init/hooks/fsmonitor-watchman.sample");
const TPL_HOOKS_POST_UPDATE: &[u8] = include_bytes!("assets/init/hooks/post-update.sample");
const TPL_HOOKS_PRE_APPLYPATCH: &[u8] = include_bytes!("assets/init/hooks/pre-applypatch.sample");
const TPL_HOOKS_PRE_COMMIT: &[u8] = include_bytes!("assets/init/hooks/pre-commit.sample");
const TPL_HOOKS_PRE_MERGE_COMMIT: &[u8] = include_bytes!("assets/init/hooks/pre-merge-commit.sample");
const TPL_HOOKS_PRE_PUSH: &[u8] = include_bytes!("assets/init/hooks/pre-push.sample");
const TPL_HOOKS_PRE_REBASE: &[u8] = include_bytes!("assets/init/hooks/pre-rebase.sample");
const TPL_HOOKS_PREPARE_COMMIT_MSG: &[u8] = include_bytes!("assets/init/hooks/prepare-commit-msg.sample");
const TPL_HOOKS_DOCS_URL: &[u8] = include_bytes!("assets/init/hooks/docs.url");
const TPL_DESCRIPTION: &[u8] = include_bytes!("assets/init/description");
const TPL_HEAD: &[u8] = include_bytes!("assets/init/HEAD");

struct PathCursor<'a>(&'a mut PathBuf);

struct NewDir<'a>(&'a mut PathBuf);

impl PathCursor<'_> {
    fn at(&mut self, component: &str) -> &Path {
        self.0.push(component);
        self.0.as_path()
    }
}

impl NewDir<'_> {
    fn at(self, component: &str) -> Result<Self, Error> {
        self.0.push(component);
        create_dir(self.0)?;
        Ok(self)
    }
    fn as_mut(&mut self) -> &mut PathBuf {
        self.0
    }
}

impl Drop for NewDir<'_> {
    fn drop(&mut self) {
        self.0.pop();
    }
}

impl Drop for PathCursor<'_> {
    fn drop(&mut self) {
        self.0.pop();
    }
}

fn write_file(data: &[u8], path: &Path) -> Result<(), Error> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .append(false)
        .open(path)
        .map_err(|e| Error::IoOpen {
            source: e,
            path: path.to_owned(),
        })?;
    file.write_all(data).map_err(|e| Error::IoWrite {
        source: e,
        path: path.to_owned(),
    })
}

fn create_dir(p: &Path) -> Result<(), Error> {
    fs::create_dir_all(p).map_err(|e| Error::CreateDirectory {
        source: e,
        path: p.to_owned(),
    })
}

/// Options for use in [`into()`];
#[derive(Copy, Clone, Default)]
pub struct Options {
    /// If true, and the kind of repository to create has a worktree, then the destination directory must be empty.
    ///
    /// By default repos with worktree can be initialized into a non-empty repository as long as there is no `.git` directory.
    pub destination_must_be_empty: bool,
    /// If set, use these filesystem capabilities to populate the respective git-config fields.
    /// If `None`, the directory will be probed.
    pub fs_capabilities: Option<gix_fs::Capabilities>,
}

/// Create a new `.git` repository of `kind` within the possibly non-existing `directory`
/// and return its path.
/// Note that this is a simple template-based initialization routine which should be accompanied with additional corrections
/// to respect git configuration, which is accomplished by [its callers][crate::ThreadSafeRepository::init_opts()]
/// that return a [Repository][crate::Repository].
pub fn into(
    directory: impl Into<PathBuf>,
    kind: Kind,
    Options {
        fs_capabilities,
        destination_must_be_empty,
    }: Options,
) -> Result<gix_discover::repository::Path, Error> {
    let mut dot_git = directory.into();
    let bare = matches!(kind, Kind::Bare);

    if bare || destination_must_be_empty {
        let num_entries_in_dot_git = fs::read_dir(&dot_git)
            .or_else(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    fs::create_dir(&dot_git).and_then(|_| fs::read_dir(&dot_git))
                } else {
                    Err(err)
                }
            })
            .map_err(|err| Error::IoOpen {
                source: err,
                path: dot_git.clone(),
            })?
            .count();
        if num_entries_in_dot_git != 0 {
            return Err(Error::DirectoryNotEmpty { path: dot_git });
        }
    }

    if !bare {
        dot_git.push(DOT_GIT_DIR);

        if dot_git.is_dir() {
            return Err(Error::DirectoryExists { path: dot_git });
        }
    }
    create_dir(&dot_git)?;

    {
        let mut cursor = NewDir(&mut dot_git).at("info")?;
        write_file(TPL_INFO_EXCLUDE, PathCursor(cursor.as_mut()).at("exclude"))?;
    }

    {
        let mut cursor = NewDir(&mut dot_git).at("hooks")?;
        for (tpl, filename) in &[
            (TPL_HOOKS_DOCS_URL, "docs.url"),
            (TPL_HOOKS_PREPARE_COMMIT_MSG, "prepare-commit-msg.sample"),
            (TPL_HOOKS_PRE_REBASE, "pre-rebase.sample"),
            (TPL_HOOKS_PRE_PUSH, "pre-push.sample"),
            (TPL_HOOKS_PRE_COMMIT, "pre-commit.sample"),
            (TPL_HOOKS_PRE_MERGE_COMMIT, "pre-merge-commit.sample"),
            (TPL_HOOKS_PRE_APPLYPATCH, "pre-applypatch.sample"),
            (TPL_HOOKS_POST_UPDATE, "post-update.sample"),
            (TPL_HOOKS_FSMONITOR_WATCHMAN, "fsmonitor-watchman.sample"),
            (TPL_HOOKS_COMMIT_MSG, "commit-msg.sample"),
            (TPL_HOOKS_APPLYPATCH_MSG, "applypatch-msg.sample"),
        ] {
            write_file(tpl, PathCursor(cursor.as_mut()).at(filename))?;
        }
    }

    {
        let mut cursor = NewDir(&mut dot_git).at("objects")?;
        create_dir(PathCursor(cursor.as_mut()).at("info"))?;
        create_dir(PathCursor(cursor.as_mut()).at("pack"))?;
    }

    {
        let mut cursor = NewDir(&mut dot_git).at("refs")?;
        create_dir(PathCursor(cursor.as_mut()).at("heads"))?;
        create_dir(PathCursor(cursor.as_mut()).at("tags"))?;
    }

    for (tpl, filename) in &[(TPL_HEAD, "HEAD"), (TPL_DESCRIPTION, "description")] {
        write_file(tpl, PathCursor(&mut dot_git).at(filename))?;
    }

    let caps = {
        let mut config = gix_config::File::default();
        let caps = {
            let caps = fs_capabilities.unwrap_or_else(|| gix_fs::Capabilities::probe(&dot_git));
            let mut core = config.new_section("core", None).expect("valid section name");

            core.push(key("repositoryformatversion"), Some("0".into()));
            core.push(key("filemode"), Some(bool(caps.executable_bit).into()));
            core.push(key("bare"), Some(bool(bare).into()));
            core.push(key("logallrefupdates"), Some(bool(!bare).into()));
            core.push(key("symlinks"), Some(bool(caps.symlink).into()));
            core.push(key("ignorecase"), Some(bool(caps.ignore_case).into()));
            core.push(key("precomposeunicode"), Some(bool(caps.precompose_unicode).into()));
            caps
        };
        let mut cursor = PathCursor(&mut dot_git);
        let config_path = cursor.at("config");
        std::fs::write(config_path, config.to_bstring()).map_err(|err| Error::IoWrite {
            source: err,
            path: config_path.to_owned(),
        })?;
        caps
    };

    Ok(gix_discover::repository::Path::from_dot_git_dir(
        dot_git,
        if bare {
            gix_discover::repository::Kind::PossiblyBare
        } else {
            gix_discover::repository::Kind::WorkTree { linked_git_dir: None }
        },
        &gix_fs::current_dir(caps.precompose_unicode)?,
    )
    .expect("by now the `dot_git` dir is valid as we have accessed it"))
}

fn key(name: &'static str) -> section::ValueName<'static> {
    section::ValueName::try_from(name).expect("valid key name")
}

fn bool(v: bool) -> &'static str {
    match v {
        true => "true",
        false => "false",
    }
}
