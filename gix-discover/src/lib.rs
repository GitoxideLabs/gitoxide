//! Find git repositories or search them upwards from a starting point, or determine if a directory looks like a git repository.
//!
//! Note that detection methods are educated guesses using the presence of files, without looking too much into the details.
#![deny(missing_docs, rust_2018_idioms)]
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
        #[error("The repository's config file doesn't exist or didn't have a 'bare' configuration or contained core.worktree without value")]
        Inconclusive,
        #[error("Could not obtain current directory for resolving the '.' repository path")]
        CurrentDir(#[from] std::io::Error),
    }
}

mod is;
pub use is::{bare as is_bare, git as is_git, submodule_git_dir as is_submodule_git_dir};

///
pub mod upwards;
pub use upwards::function::{discover as upwards, discover_opts as upwards_opts};

///
pub mod path;

///
pub mod parse;
