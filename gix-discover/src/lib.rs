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
//! let (path, _trust) =
//!     gix_discover::upwards(&nested).map_err(gix_discover::upwards::Error::into_error)?;
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
    /// The error returned by [`crate::is_git()`].
    pub type Error = gix_error::Exn;
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
