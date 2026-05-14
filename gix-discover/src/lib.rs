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
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Could not find a valid HEAD reference")]
        FindHeadRef(#[from] gix_ref::file::find::existing::Error),
        #[error("Missing HEAD at '.git/HEAD'")]
        MissingHead,
        #[error("Expected HEAD at '.git/HEAD', got '.git/{}'", .name)]
        MisplacedHead { name: bstr::BString },
        #[error("Expected an objects directory at '{}'", .missing.display())]
        MissingObjectsDirectory { missing: PathBuf },
        #[error("The worktree's private repo's commondir file at '{}' or it could not be read", .missing.display())]
        MissingCommonDir { missing: PathBuf, source: std::io::Error },
        #[error("Expected a refs directory at '{}'", .missing.display())]
        MissingRefsDirectory { missing: PathBuf },
        #[error(transparent)]
        GitFile(#[from] crate::path::from_gitdir_file::Error),
        #[error("Could not retrieve metadata of \"{path}\"")]
        Metadata { source: std::io::Error, path: PathBuf },
        #[error(
            "The repository's config file doesn't exist or didn't have a 'bare' configuration or contained core.worktree without value"
        )]
        Inconclusive,
        #[error("Could not obtain current directory for resolving the '.' repository path")]
        CurrentDir(#[from] std::io::Error),
    }
}

mod is;
#[allow(deprecated)]
pub use is::submodule_git_dir as is_submodule_git_dir;
pub use is::{bare as is_bare, git as is_git};

///
pub mod upwards;
pub use upwards::function::{discover as upwards, discover_opts as upwards_opts};

///
pub mod path;

///
pub mod parse;
