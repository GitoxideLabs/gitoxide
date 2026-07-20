//! Find git repositories or search them upwards from a starting point, or determine if a directory looks like a git repository.
//!
//! Note that detection methods are educated guesses using the presence of files, without looking too much into the details.
//!
//! ## Examples
//!
//! ```
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let dir = tempfile::tempdir()?;
//! # let git_dir = dir.path().join(".git");
//! # std::fs::create_dir_all(git_dir.join("objects"))?;
//! # std::fs::create_dir_all(git_dir.join("refs").join("heads"))?;
//! # std::fs::write(git_dir.join("HEAD"), b"ref: refs/heads/main\n")?;
//! # std::fs::write(
//! #     git_dir.join("refs").join("heads").join("main"),
//! #     b"1111111111111111111111111111111111111111\n",
//! # )?;
//! # let nested = dir.path().join("src").join("module");
//! # std::fs::create_dir_all(&nested)?;
//! let (path, _trust) = gix_discover::upwards(&nested)?;
//! let (repository_dir, worktree_dir) = path.into_repository_and_work_tree_directories();
//!
//! assert_eq!(repository_dir, git_dir);
//! assert_eq!(worktree_dir, Some(dir.path().to_path_buf()));
//! assert!(gix_discover::is_git(&repository_dir).is_ok());
//! # Ok(()) }
//! ```
#![deny(missing_docs)]
#![forbid(unsafe_code)]

/// The name of the `.git` directory.
pub const DOT_GIT_DIR: &str = ".git";

/// The name of the `modules` sub-directory within a `.git` directory for keeping submodule checkouts.
pub const MODULES: &str = "modules";

///
pub mod repository;

///
pub mod is_git {
    use std::path::PathBuf;

    /// The error returned by [`crate::is_git()`].
    // TODO(review): this implementation hand-preserves `#[error(transparent)]` semantics for
    //                `GitFile`: `Display` passes the formatter through and `source()` forwards to
    //                the inner error's source, exactly like the `thiserror`-generated code did.
    //                The same pattern is used in `path::from_gitdir_file::Error`.
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        FindHeadRef(gix_ref::file::find::existing::Error),
        MissingHead,
        MisplacedHead { name: bstr::BString },
        MissingObjectsDirectory { missing: PathBuf },
        MissingCommonDir { missing: PathBuf, source: std::io::Error },
        MissingRefsDirectory { missing: PathBuf },
        GitFile(crate::path::from_gitdir_file::Error),
        Metadata { source: std::io::Error, path: PathBuf },
        Inconclusive,
        CurrentDir(std::io::Error),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::FindHeadRef(_) => f.write_str("Could not find a valid HEAD reference"),
                Error::MissingHead => f.write_str("Missing HEAD at '.git/HEAD'"),
                Error::MisplacedHead { name } => write!(f, "Expected HEAD at '.git/HEAD', got '.git/{name}'"),
                Error::MissingObjectsDirectory { missing } => {
                    write!(f, "Expected an objects directory at '{}'", missing.display())
                }
                Error::MissingCommonDir { missing, .. } => write!(
                    f,
                    "The worktree's private repo's commondir file at '{}' or it could not be read",
                    missing.display()
                ),
                Error::MissingRefsDirectory { missing } => {
                    write!(f, "Expected a refs directory at '{}'", missing.display())
                }
                Error::GitFile(err) => std::fmt::Display::fmt(err, f),
                Error::Metadata { path, .. } => {
                    write!(f, "Could not retrieve metadata of \"{}\"", path.display())
                }
                Error::Inconclusive => f.write_str(
                    "The repository's config file doesn't exist or didn't have a 'bare' configuration or contained core.worktree without value",
                ),
                Error::CurrentDir(_) => {
                    f.write_str("Could not obtain current directory for resolving the '.' repository path")
                }
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::FindHeadRef(err) => Some(err),
                Error::MissingCommonDir { source, .. } => Some(source),
                Error::GitFile(err) => err.source(),
                Error::Metadata { source, .. } => Some(source),
                Error::CurrentDir(err) => Some(err),
                Error::MissingHead
                | Error::MisplacedHead { .. }
                | Error::MissingObjectsDirectory { .. }
                | Error::MissingRefsDirectory { .. }
                | Error::Inconclusive => None,
            }
        }
    }

    impl From<gix_ref::file::find::existing::Error> for Error {
        fn from(err: gix_ref::file::find::existing::Error) -> Self {
            Error::FindHeadRef(err)
        }
    }

    impl From<crate::path::from_gitdir_file::Error> for Error {
        fn from(err: crate::path::from_gitdir_file::Error) -> Self {
            Error::GitFile(err)
        }
    }

    impl From<std::io::Error> for Error {
        fn from(err: std::io::Error) -> Self {
            Error::CurrentDir(err)
        }
    }
}

mod is;
#[expect(
    deprecated,
    reason = "this re-export preserves compatibility with the deprecated API"
)]
pub use is::submodule_git_dir as is_submodule_git_dir;
pub use is::{bare as is_bare, git as is_git};

///
pub mod upwards;
pub use upwards::function::{discover as upwards, discover_opts as upwards_opts};

///
pub mod path;

///
pub mod parse;
