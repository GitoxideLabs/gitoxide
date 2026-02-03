use std::{borrow::Cow, ffi::OsStr, path::Path};

use crate::path::RepositoryKind;
use crate::DOT_GIT_DIR;

/// Returns true if the given `git_dir` seems to be a bare repository.
///
/// Please note that repositories without an index generally _look_ bare, even though they might also be uninitialized.
pub fn bare(git_dir_candidate: &Path) -> bool {
    !(git_dir_candidate.join("index").exists() || (git_dir_candidate.file_name() == Some(OsStr::new(DOT_GIT_DIR))))
}

/// Returns true if `git_dir` is located within a `.git/modules` directory, indicating it's a submodule clone.
#[deprecated = "use path::repository_kind() instead"]
pub fn submodule_git_dir(git_dir: &Path) -> bool {
    crate::path::repository_kind(git_dir).is_some_and(|kind| matches!(kind, RepositoryKind::Submodule))
}

/// What constitutes a valid git repository, returning the guessed repository kind
/// purely based on the presence of files. Note that the git-config ultimately decides what's bare.
///
/// Returns the `Kind` of git directory that was passed, possibly alongside the supporting private worktree git dir.
///
/// Note that `.git` files are followed to a valid git directory, which then requires…
///
///   * …a valid head
///   * …an objects directory
///   * …a refs directory
///
pub fn git(git_dir: &Path) -> Result<crate::repository::Kind, crate::is_git::Error> {
    let git_dir_metadata = git_dir.metadata().map_err(|err| crate::is_git::Error::Metadata {
        source: err,
        path: git_dir.into(),
    })?;
    // precompose-unicode can't be known here, so we just default it to false, hoping it won't matter.
    let cwd = gix_fs::current_dir(false)?;
    git_with_metadata(git_dir, git_dir_metadata, &cwd)
}

pub(crate) fn git_with_metadata(
    git_dir: &Path,
    git_dir_metadata: std::fs::Metadata,
    cwd: &Path,
) -> Result<crate::repository::Kind, crate::is_git::Error> {
    #[derive(Eq, PartialEq)]
    enum Kind {
        MaybeRepo,
        Submodule,
        LinkedWorkTreeDir,
        WorkTreeGitDir { work_dir: std::path::PathBuf },
    }

    let dot_git = if git_dir_metadata.is_file() {
        let private_git_dir = crate::path::from_gitdir_file(git_dir)?;
        Cow::Owned(private_git_dir)
    } else {
        Cow::Borrowed(git_dir)
    };

    {
        // Fast-path: avoid doing the complete search if HEAD is already not there.
        if !dot_git.join("HEAD").exists() {
            return Err(crate::is_git::Error::MissingHead);
        }
        // We expect to be able to parse any ref-hash, so we shouldn't have to know the repos hash here.
        // With ref-table, the hash is probably stored as part of the ref-db itself, so we can handle it from there.
        // In other words, it's important not to fail on detached heads here because we guessed the hash kind wrongly.
        let refs = gix_ref::file::Store::at(dot_git.as_ref().into(), Default::default());
        match refs.find_loose("HEAD") {
            Ok(head) => {
                if head.name.as_bstr() != "HEAD" {
                    return Err(crate::is_git::Error::MisplacedHead {
                        name: head.name.into_inner(),
                    });
                }
            }
            Err(gix_ref::file::find::existing::Error::Find(gix_ref::file::find::Error::ReferenceCreation {
                source: _,
                relative_path,
            })) if relative_path == Path::new("HEAD") => {
                // It's fine as long as the reference is found is `HEAD`.
            }
            Err(err) => {
                return Err(err.into());
            }
        }
    }

    let (common_dir, kind) = if git_dir_metadata.is_file() {
        let common_dir = dot_git.join("commondir");
        match crate::path::from_plain_file(&common_dir) {
            Some(Err(err)) => {
                return Err(crate::is_git::Error::MissingCommonDir {
                    missing: common_dir,
                    source: err,
                })
            }
            Some(Ok(common_dir)) => {
                let common_dir = dot_git.join(common_dir);
                (Cow::Owned(common_dir), Kind::LinkedWorkTreeDir)
            }
            None => (dot_git.clone(), Kind::Submodule),
        }
    } else {
        let common_dir = dot_git.join("commondir");
        let worktree_and_common_dir = crate::path::from_plain_file(&common_dir)
            .and_then(Result::ok)
            .and_then(|cd| {
                crate::path::from_plain_file(&dot_git.join("gitdir"))
                    .and_then(Result::ok)
                    .map(|worktree_gitfile| (crate::path::without_dot_git_dir(worktree_gitfile), cd))
            });
        match worktree_and_common_dir {
            Some((work_dir, common_dir)) => {
                let common_dir = dot_git.join(common_dir);
                (Cow::Owned(common_dir), Kind::WorkTreeGitDir { work_dir })
            }
            None => (dot_git.clone(), Kind::MaybeRepo),
        }
    };

    {
        let objects_path = common_dir.join("objects");
        if !objects_path.is_dir() {
            return Err(crate::is_git::Error::MissingObjectsDirectory { missing: objects_path });
        }
    }
    {
        let refs_path = common_dir.join("refs");
        if !refs_path.is_dir() {
            return Err(crate::is_git::Error::MissingRefsDirectory { missing: refs_path });
        }
    }
    Ok(match kind {
        Kind::LinkedWorkTreeDir => crate::repository::Kind::WorkTree {
            linked_git_dir: Some(dot_git.into_owned()),
        },
        Kind::WorkTreeGitDir { work_dir } => crate::repository::Kind::WorkTreeGitDir { work_dir },
        Kind::Submodule => crate::repository::Kind::Submodule {
            git_dir: dot_git.into_owned(),
        },
        Kind::MaybeRepo => {
            let conformed_git_dir = if git_dir == Path::new(".") {
                gix_path::realpath_opts(git_dir, cwd, gix_path::realpath::MAX_SYMLINKS)
                    .map(Cow::Owned)
                    .unwrap_or(Cow::Borrowed(git_dir))
            } else {
                gix_path::normalize(git_dir.into(), cwd).unwrap_or(Cow::Borrowed(git_dir))
            };
            if bare(conformed_git_dir.as_ref()) || conformed_git_dir.extension() == Some(OsStr::new("git")) {
                crate::repository::Kind::PossiblyBare
            } else if crate::path::repository_kind(conformed_git_dir.as_ref())
                .is_some_and(|kind| matches!(kind, RepositoryKind::Submodule))
            {
                crate::repository::Kind::SubmoduleGitDir
            } else if conformed_git_dir.file_name() == Some(OsStr::new(DOT_GIT_DIR)) {
                crate::repository::Kind::WorkTree { linked_git_dir: None }
            } else {
                crate::repository::Kind::PossiblyBare
            }
        }
    })
}
