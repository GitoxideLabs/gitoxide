use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use crate::error::{ErrorExt, ResultExt, bail, message, validation};
use gix_discover::DOT_GIT_DIR;

use crate::{Error, Result};

fn io_error(source: std::io::Error, action: &str, path: &Path) -> Error {
    source.and_raise(message!("{action} at \"{}\"", path.display()))
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
    fn at(self, component: &str) -> Result<Self> {
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

fn write_file(data: &[u8], path: &Path) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .append(false)
        .open(path)
        .map_err(|err| io_error(err, "Could not open data", path))?;
    file.write_all(data)
        .map_err(|err| io_error(err, "Could not write data", path))
}

fn create_dir(p: &Path) -> Result<()> {
    fs::create_dir_all(p).map_err(|err| io_error(err, "Could not create directory", p))
}

/// Options for use in [`into()`];
#[derive(Copy, Clone, Default)]
pub struct Options {
    /// Control whether the destination directory must be empty when creating a repository with a worktree.
    ///
    /// - `None` (default): initialize like Git and allow a non-empty destination directory, as long as no `.git`
    ///   directory is present.
    /// - `Some(true)`: require an empty destination directory.
    /// - `Some(false)`: explicitly allow initialization into a non-empty destination directory (still requires that no
    ///   `.git` directory is present).
    ///
    /// For clones, checkout failure cleanup is based on whether the destination was already present and non-empty before
    /// initialization began, not on this option alone. In particular, if the destination was empty or had to be created,
    /// cleanup may remove the entire destination, including the created `.git` directory. Preservation of the destination
    /// for inspection or manual cleanup is only guaranteed when the destination was non-empty before the clone started.
    ///
    /// Bare repositories always require an empty destination, regardless of this option.
    pub destination_must_be_empty: Option<bool>,
    /// If set, use these filesystem capabilities to populate the respective git-config fields.
    /// If `None`, the directory will be probed.
    pub fs_capabilities: Option<gix_fs::Capabilities>,
    /// If set to `Some(Sha256)`, write `extensions.objectFormat=sha256`.
    /// Otherwise, create a repository without an explicit object-format extension,
    /// which is interpreted as legacy SHA-1.
    /// In builds without SHA-1 support, `None` means SHA-256.
    ///
    /// If `None` (the default), [`ThreadSafeRepository::init_opts()`][crate::ThreadSafeRepository::init_opts()]
    /// will apply `init.defaultObjectFormat` or `GIT_DEFAULT_HASH` if set.
    pub object_hash: Option<gix_hash::Kind>,
}

fn default_object_hash() -> Option<gix_hash::Kind> {
    #[cfg(feature = "sha1")]
    {
        None
    }
    #[cfg(all(not(feature = "sha1"), feature = "sha256"))]
    {
        Some(gix_hash::Kind::Sha256)
    }
    #[cfg(all(not(feature = "sha1"), not(feature = "sha256")))]
    {
        unreachable!("hash support features are validated by gix-hash")
    }
}

/// Create a new `.git` repository of `kind` within the possibly non-existing `directory`
/// and return its path.
/// This uses the built-in templates; [`ThreadSafeRepository::init_opts()`][crate::ThreadSafeRepository::init_opts()]
/// also supports configured template directories.
/// Note that this is a simple template-based initialization routine which should be accompanied with additional corrections
/// to respect git configuration, which is accomplished by [its callers][crate::ThreadSafeRepository::init_opts()]
/// that return a [Repository][crate::Repository].
/// Rejected non-empty destinations or existing `.git` directories include the display-formatted path as `input` bytes
/// in
/// [metadata](gix_error::Error::metadata()).
pub fn into(directory: impl Into<PathBuf>, kind: Kind, options: Options) -> Result<gix_discover::repository::Path> {
    into_with_capabilities(directory, kind, options, None).map(|(path, _, _)| path)
}

pub(crate) fn into_with_capabilities(
    directory: impl Into<PathBuf>,
    kind: Kind,
    Options {
        fs_capabilities,
        destination_must_be_empty,
        object_hash,
    }: Options,
    template: Option<&Path>,
) -> Result<(gix_discover::repository::Path, gix_fs::Capabilities, bool)> {
    let mut dot_git = directory.into();
    let bare = matches!(kind, Kind::Bare);
    let object_hash = object_hash.or_else(default_object_hash);

    if bare || destination_must_be_empty.unwrap_or(false) {
        let num_entries_in_dot_git = fs::read_dir(&dot_git)
            .or_else(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    fs::create_dir_all(&dot_git).and_then(|_| fs::read_dir(&dot_git))
                } else {
                    Err(err)
                }
            })
            .map_err(|err| io_error(err, "Could not open data", &dot_git))?
            .count();
        if num_entries_in_dot_git != 0 {
            bail!(
                validation("Refusing to initialize the non-empty directory as")
                    .with_input(dot_git.display().to_string().into_bytes())
            );
        }
    }

    if !bare {
        dot_git.push(DOT_GIT_DIR);

        if dot_git.is_dir() {
            bail!(
                validation("Refusing to initialize an existing directory")
                    .with_input(dot_git.display().to_string().into_bytes())
            );
        }
    }
    create_dir(&dot_git)?;

    // Keep required storage directories from being redirected by template symlinks.
    for path in ["objects/info", "objects/pack", "refs/heads", "refs/tags"] {
        create_dir(&dot_git.join(path))?;
    }
    let mut config = if let Some(template) = template {
        copy_template(template, &dot_git, object_hash)?
    } else {
        copy_builtin_template(&mut dot_git)?;
        gix_config::File::default()
    };
    let head_path = dot_git.join("HEAD");
    let created_head = match head_path.symlink_metadata() {
        Ok(_) => false,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            write_file(TPL_HEAD, &head_path)?;
            true
        }
        Err(err) => return Err(io_error(err, "Could not inspect HEAD", &head_path)),
    };

    let caps = fs_capabilities.unwrap_or_else(|| gix_fs::Capabilities::probe(&dot_git));
    config.set_raw_value("core.filemode", bool(caps.executable_bit))?;
    config.set_raw_value("core.bare", bool(bare))?;
    if config.string_implicit("core.logallrefupdates").is_none() {
        config.set_raw_value("core.logallrefupdates", bool(!bare))?;
    }
    if !caps.symlink {
        config.set_raw_value("core.symlinks", bool(false))?;
    }
    config.set_raw_value("core.ignorecase", bool(caps.ignore_case))?;
    config.set_raw_value("core.precomposeunicode", bool(caps.precompose_unicode))?;
    match object_hash {
        #[cfg(feature = "sha256")]
        Some(gix_hash::Kind::Sha256) => {
            config.set_raw_value("core.repositoryformatversion", "1")?;
            config.set_raw_value("extensions.objectformat", "sha256")?;
        }
        _ => {
            if let Ok(mut values) = config.raw_values_mut("extensions.objectformat") {
                values.delete_all();
            }
            if config.integer("core.repositoryformatversion")? != Some(1) {
                config.set_raw_value("core.repositoryformatversion", "0")?;
            }
        }
    }
    let config_path = dot_git.join("config");
    let mut config_file =
        gix_lock::File::acquire_to_update_resource(&config_path, gix_lock::acquire::Fail::Immediately, None, 0)?;
    config_file
        .write_all(&config.to_bstring())
        .map_err(|err| io_error(err, "Could not write data", &config_path))?;
    config_file.commit().or_error()?;

    Ok((
        gix_discover::repository::Path::from_dot_git_dir(
            dot_git,
            if bare {
                gix_discover::repository::Kind::PossiblyBare
            } else {
                gix_discover::repository::Kind::WorkTree { linked_git_dir: None }
            },
            &gix_fs::current_dir(caps.precompose_unicode)
                .or_raise(|| message("Could not obtain the current directory"))?,
        )
        .expect("by now the `dot_git` dir is valid as we have accessed it"),
        caps,
        created_head,
    ))
}

fn copy_builtin_template(dot_git: &mut PathBuf) -> Result<()> {
    {
        let mut cursor = NewDir(dot_git).at("info")?;
        write_file(TPL_INFO_EXCLUDE, PathCursor(cursor.as_mut()).at("exclude"))?;
    }

    {
        let mut cursor = NewDir(dot_git).at("hooks")?;
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

    write_file(TPL_DESCRIPTION, &dot_git.join("description"))
}

fn copy_template(template: &Path, destination: &Path, object_hash: Option<gix_hash::Kind>) -> Result<gix_config::File> {
    if template.as_os_str().is_empty() {
        return Ok(gix_config::File::default());
    }
    let entries = match fs::read_dir(template) {
        Ok(entries) => entries,
        Err(_err) => {
            gix_trace::warn!("Templates not found in '{}': {_err}", template.display());
            return Ok(gix_config::File::default());
        }
    };
    let config_path = template.join("config");
    let config = match fs::read(&config_path) {
        Ok(bytes) => gix_config::File::from_bytes_no_includes(
            &bytes,
            gix_config::file::Metadata::from(gix_config::Source::Local).at(destination.join("config")),
            Default::default(),
        )?,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => gix_config::File::default(),
        Err(err) => return Err(io_error(err, "Could not read template configuration", &config_path)),
    };
    if let Err(_err) = validate_template_format(&config, object_hash) {
        gix_trace::warn!("Not copying templates from '{}': {_err:#}", template.display());
        return Ok(gix_config::File::default());
    }
    copy_template_entries(entries, destination)?;
    Ok(config)
}

fn validate_template_format(config: &gix_config::File, _object_hash: Option<gix_hash::Kind>) -> Result<()> {
    use crate::{
        config::tree::{Core, Extensions},
        repository::FormatVersion,
    };

    let version = Core::REPOSITORY_FORMAT_VERSION
        .try_into_repository_format_version(config.integer(Core::REPOSITORY_FORMAT_VERSION))?;
    #[cfg(feature = "sha256")]
    if _object_hash == Some(gix_hash::Kind::Sha256) {
        version.unwrap_or_default().validate_upgrade_to_v1(config)?;
    }
    let Some(version) = version else {
        return Ok(());
    };
    for section in config.sections_by_name("extensions").into_iter().flatten() {
        for name in section.value_names() {
            let legacy = ["noop", "preciousobjects", "partialclone", "worktreeconfig"]
                .iter()
                .any(|known| name.eq_ignore_ascii_case(known));
            let v1 = ["noop-v1", "objectformat", "relativeworktrees", "refstorage"]
                .iter()
                .any(|known| name.eq_ignore_ascii_case(known));
            if (version == FormatVersion::V1 && (section.header().subsection_name().is_some() || !(legacy || v1)))
                || (version == FormatVersion::V0 && v1)
            {
                bail!(
                    message("Unsupported template repository extension")
                        .with("extension", name)
                        .unsupported()
                );
            }
        }
    }
    if let Some(format) = config.string(Extensions::OBJECT_FORMAT) {
        Extensions::OBJECT_FORMAT.try_into_object_format(format)?;
    }
    if let Some(storage) = config.string("extensions.refStorage")
        && storage != "files"
    {
        bail!(
            message("Unsupported template reference storage")
                .with_input(storage)
                .unsupported()
        );
    }
    Ok(())
}

fn copy_template_entries(entries: fs::ReadDir, destination: &Path) -> Result<()> {
    for entry in entries {
        let entry = entry.map_err(|err| io_error(err, "Could not list template", destination))?;
        if entry.file_name().as_encoded_bytes().starts_with(b".") {
            continue;
        }
        let source = entry.path();
        let target = destination.join(entry.file_name());
        let kind = entry
            .file_type()
            .map_err(|err| io_error(err, "Could not inspect template", &source))?;
        let existing = match target.symlink_metadata() {
            Ok(metadata) => Some(metadata),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
            Err(err) => return Err(io_error(err, "Could not inspect template destination", &target)),
        };
        if kind.is_dir() {
            if let Some(existing) = existing {
                if !existing.is_dir() {
                    bail!(
                        validation("Template directory conflicts with an existing entry")
                            .with_input(target.display().to_string())
                    );
                }
            } else {
                fs::create_dir(&target).map_err(|err| io_error(err, "Could not create template directory", &target))?;
            }
            copy_template_entries(
                fs::read_dir(&source).map_err(|err| io_error(err, "Could not read template directory", &source))?,
                &target,
            )?;
        } else if existing.is_none() {
            if kind.is_symlink() {
                let original =
                    fs::read_link(&source).map_err(|err| io_error(err, "Could not read template symlink", &source))?;
                gix_fs::symlink::create(&original, &target)
                    .map_err(|err| io_error(err, "Could not copy template symlink", &target))?;
            } else if kind.is_file() {
                fs::copy(&source, &target).map_err(|err| io_error(err, "Could not copy template file", &target))?;
            }
        }
    }
    Ok(())
}

fn bool(v: bool) -> &'static str {
    match v {
        true => "true",
        false => "false",
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn default_object_hash_matches_available_hash_support() {
        let object_hash = super::default_object_hash();
        #[cfg(feature = "sha1")]
        assert_eq!(
            object_hash, None,
            "SHA1-capable builds keep Git's implicit legacy object format"
        );
        #[cfg(all(not(feature = "sha1"), feature = "sha256"))]
        assert_eq!(
            object_hash,
            Some(gix_hash::Kind::Sha256),
            "SHA256-only builds must initialize repositories that can be reopened"
        );
    }
}
