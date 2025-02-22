use std::path::PathBuf;

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
        path::without_dot_git_dir,
        repository::{Kind, Path},
        DOT_GIT_DIR,
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
