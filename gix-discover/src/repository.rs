use std::path::PathBuf;

use gix_error::{Result, ResultExt, message};
use gix_sec::Trust;

/// Determine the minimum ownership trust of `candidate`, `git_dir`, its common directory, and existing `work_dir`.
///
/// Relative paths are accessed from `current_dir`. `candidate` identifies the original path before resolving a
/// possible gitfile. Its optional, previously determined ownership trust can only be lowered by the other paths.
/// If its spelling exactly matches `git_dir` after joining with `current_dir`, ownership is checked only once.
/// No normalization or filesystem alias checks are performed to establish equality.
/// A missing `work_dir` is skipped, but missing `git_dir` or common directories are errors.
pub fn trust(
    git_dir: &std::path::Path,
    work_dir: Option<&std::path::Path>,
    current_dir: &std::path::Path,
    candidate: (&std::path::Path, Option<Trust>),
) -> Result<Trust> {
    trust_with(git_dir, work_dir, current_dir, candidate, Trust::from_path_ownership)
}

pub(crate) fn trust_with(
    git_dir: &std::path::Path,
    work_dir: Option<&std::path::Path>,
    current_dir: &std::path::Path,
    (candidate, candidate_trust): (&std::path::Path, Option<Trust>),
    mut trust_from_path: impl FnMut(&std::path::Path) -> std::io::Result<Trust>,
) -> Result<Trust> {
    let git_dir = current_dir.join(git_dir);
    let candidate = current_dir.join(candidate);
    let common_dir = crate::path::from_plain_file(&git_dir.join("commondir"))
        .transpose()
        .or_raise(|| message("Could not resolve the common repository directory for trust check"))?
        .map(|common_dir| git_dir.join(common_dir));
    let mut trust = match candidate_trust {
        Some(trust) => trust,
        None => trust_from_path(&candidate)
            .or_raise(|| message!("Could not determine trust level for path '{}'.", candidate.display()))?,
    };
    for (path, optional_worktree) in [
        (
            (git_dir.as_os_str() != candidate.as_os_str()).then_some(git_dir.as_path()),
            false,
        ),
        (common_dir.as_deref(), false),
        (work_dir, true),
    ] {
        let Some(path) = path else { continue };
        let path = current_dir.join(path);
        let ownership = match trust_from_path(&path) {
            // A private git directory remains usable after its checkout was removed.
            Err(err) if optional_worktree && err.kind() == std::io::ErrorKind::NotFound => continue,
            ownership => ownership,
        };
        trust = trust
            .min(ownership.or_raise(|| message!("Could not determine trust level for path '{}'.", path.display()))?);
    }
    Ok(trust)
}

#[cfg(test)]
mod trust_tests {
    use super::trust_with;
    use gix_sec::Trust;

    #[test]
    fn known_candidate_trust_skips_only_identical_git_directory_spellings() -> gix_testtools::TestResult {
        let cwd = gix_testtools::scripted_fixture_read_only("make_ownership_repos.sh")?;
        for name in ["worktree/.git", "bare"] {
            let git_dir = std::path::PathBuf::from(name);
            for candidate in [git_dir.clone(), cwd.join(&git_dir), git_dir.join(".")] {
                let mut checked_paths = Vec::new();
                let trust = trust_with(&git_dir, None, &cwd, (&candidate, Some(Trust::Reduced)), |path| {
                    checked_paths.push(path.to_path_buf());
                    Ok(Trust::Full)
                })?;
                let expected_paths = if cwd.join(&candidate).as_os_str() == cwd.join(&git_dir).as_os_str() {
                    Vec::new()
                } else {
                    vec![cwd.join(&git_dir)]
                };
                assert_eq!(
                    checked_paths, expected_paths,
                    "only an exact candidate spelling can avoid checking the git directory"
                );
                assert_eq!(trust, Trust::Reduced, "reuse must not upgrade the candidate's trust");
            }
        }
        Ok(())
    }

    #[test]
    fn resolved_paths_constrain_trust_without_upgrading_the_candidate() -> gix_testtools::TestResult {
        let root = gix_testtools::scripted_fixture_read_only("make_linked_ownership_repo.sh")?;
        let cwd = std::env::current_dir()?;
        let candidate = root.join("linked/.git");
        let git_dir = root.join("main/.git/worktrees/linked");
        let common_dir = root.join("main/.git");
        let worktree = root.join("linked");
        for foreign_path in [&git_dir, &common_dir, &worktree] {
            let foreign_path = foreign_path.canonicalize()?;
            let trust = trust_with(
                &git_dir,
                Some(&worktree),
                &cwd,
                (&candidate, Some(Trust::Full)),
                |path| {
                    Ok(if path.canonicalize()? == foreign_path {
                        Trust::Reduced
                    } else {
                        Trust::Full
                    })
                },
            )?;
            assert_eq!(trust, Trust::Reduced, "every resolved repository path constrains trust");
        }
        assert_eq!(
            trust_with(
                &git_dir,
                Some(&worktree),
                &cwd,
                (&candidate, Some(Trust::Reduced)),
                |_| Ok(Trust::Full)
            )?,
            Trust::Reduced,
            "a trusted git directory must not upgrade an untrusted gitfile"
        );
        let missing_worktree = root.join("missing-checkout");
        assert!(
            super::trust(&git_dir, Some(&missing_worktree), &cwd, (&candidate, Some(Trust::Full))).is_ok(),
            "a missing checkout must not prevent opening the repository"
        );
        let common_dir = common_dir.canonicalize()?;
        assert!(
            trust_with(
                &git_dir,
                Some(&missing_worktree),
                &cwd,
                (&candidate, Some(Trust::Full)),
                |path| {
                    if path.canonicalize()? == common_dir {
                        Err(std::io::ErrorKind::NotFound.into())
                    } else {
                        Trust::from_path_ownership(path)
                    }
                }
            )
            .is_err(),
            "only a missing checkout may be skipped; repository paths remain mandatory"
        );
        Ok(())
    }
}

/// A repository path which either points to a work tree or the `.git` repository itself.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Path {
    /// The currently checked out linked worktree along with its connected and existing git directory, or the worktree checkout of a
    /// submodule.
    LinkedWorkTree {
        /// The base of the work tree.
        work_dir: PathBuf,
        /// The worktree-private git dir, located within the main git directory which holds most of the information.
        git_dir: PathBuf,
    },
    /// The currently checked out or nascent work tree of a git repository
    WorkTree(PathBuf),
    /// The git repository itself, typically bare and without known worktree.
    /// It could also be non-bare with a worktree configured using git configuration, or no worktree at all despite
    /// not being bare (due to mis-configuration for example).
    ///
    /// Note that it might still have linked work-trees which can be accessed later, bare or not, or it might be a
    /// submodule git directory in the `.git/modules/**/<name>` directory of the parent repository.
    Repository(PathBuf),
}

mod path {
    use std::path::PathBuf;

    use crate::{
        DOT_GIT_DIR,
        path::without_dot_git_dir,
        repository::{Kind, Path},
    };

    impl AsRef<std::path::Path> for Path {
        fn as_ref(&self) -> &std::path::Path {
            match self {
                Path::WorkTree(path)
                | Path::Repository(path)
                | Path::LinkedWorkTree {
                    work_dir: _,
                    git_dir: path,
                } => path,
            }
        }
    }

    impl Path {
        /// Instantiate a new path from `dir` which is expected to be the `.git` directory, with `kind` indicating
        /// whether it's a bare repository or not, with `current_dir` being used to normalize relative paths
        /// as needed.
        ///
        /// `None` is returned if `dir` could not be resolved due to being relative and trying to reach outside of the filesystem root.
        pub fn from_dot_git_dir(dir: PathBuf, kind: Kind, current_dir: &std::path::Path) -> Option<Self> {
            let cwd = current_dir;
            let normalize_on_trailing_dot_dot = |dir: PathBuf| -> Option<PathBuf> {
                if !matches!(dir.components().next_back(), Some(std::path::Component::ParentDir)) {
                    dir
                } else {
                    gix_path::normalize(dir.into(), cwd)?.into_owned()
                }
                .into()
            };

            match kind {
                Kind::Submodule { git_dir } => Path::LinkedWorkTree {
                    git_dir: gix_path::normalize(git_dir.into(), cwd)?.into_owned(),
                    work_dir: without_dot_git_dir(normalize_on_trailing_dot_dot(dir)?),
                },
                Kind::SubmoduleGitDir => Path::Repository(dir),
                Kind::WorkTreeGitDir { work_dir } => Path::LinkedWorkTree { git_dir: dir, work_dir },
                Kind::WorkTree { linked_git_dir } => match linked_git_dir {
                    Some(git_dir) => Path::LinkedWorkTree {
                        git_dir,
                        work_dir: without_dot_git_dir(normalize_on_trailing_dot_dot(dir)?),
                    },
                    None => {
                        let mut dir = normalize_on_trailing_dot_dot(dir)?;
                        dir.pop(); // ".git" suffix
                        let work_dir = if dir.as_os_str().is_empty() {
                            PathBuf::from(".")
                        } else {
                            dir
                        };
                        Path::WorkTree(work_dir)
                    }
                },
                Kind::PossiblyBare => Path::Repository(dir),
            }
            .into()
        }
        /// Returns the [kind][Kind] of this repository path.
        pub fn kind(&self) -> Kind {
            match self {
                Path::LinkedWorkTree { work_dir: _, git_dir } => Kind::WorkTree {
                    linked_git_dir: Some(git_dir.to_owned()),
                },
                Path::WorkTree(_) => Kind::WorkTree { linked_git_dir: None },
                Path::Repository(_) => Kind::PossiblyBare,
            }
        }

        /// Consume and split this path into the location of the `.git` directory as well as an optional path to the work tree.
        pub fn into_repository_and_work_tree_directories(self) -> (PathBuf, Option<PathBuf>) {
            match self {
                Path::LinkedWorkTree { work_dir, git_dir } => (git_dir, Some(work_dir)),
                Path::WorkTree(working_tree) => (working_tree.join(DOT_GIT_DIR), Some(working_tree)),
                Path::Repository(repository) => (repository, None),
            }
        }
    }
}

/// The kind of repository path.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Kind {
    /// A bare repository does not have a work tree, that is files on disk beyond the `git` repository itself.
    ///
    /// Note that this is merely a guess at this point as we didn't read the configuration yet.
    ///
    /// Also note that due to optimizing for performance and *just* making an educated *guess in some situations*,
    /// we may consider a non-bare repository bare if it doesn't have an index yet due to be freshly initialized.
    /// The caller has to handle this, typically by reading the configuration.
    ///
    /// It could also be a directory which is non-bare by configuration, but is *not* named `.git`.
    /// Unusual, but it's possible that a worktree is configured via `core.worktree`.
    PossiblyBare,
    /// A `git` repository along with checked out files in a work tree.
    WorkTree {
        /// If set, this is the git dir associated with this _linked_ worktree.
        /// If `None`, the git_dir is the `.git` directory inside the _main_ worktree we represent.
        linked_git_dir: Option<PathBuf>,
    },
    /// A worktree's git directory in the common`.git` directory in `worktrees/<name>`.
    WorkTreeGitDir {
        /// Path to the worktree directory.
        work_dir: PathBuf,
    },
    /// The directory is a `.git` dir file of a submodule worktree.
    Submodule {
        /// The git repository itself that is referenced by the `.git` dir file, typically in the `.git/modules/**/<name>` directory of the parent
        /// repository.
        git_dir: PathBuf,
    },
    /// The git directory in the `.git/modules/**/<name>` directory tree of the parent repository
    SubmoduleGitDir,
}

impl Kind {
    /// Returns true if this is a bare repository, one without a work tree.
    pub fn is_bare(&self) -> bool {
        matches!(self, Kind::PossiblyBare)
    }
}
