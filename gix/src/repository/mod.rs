//!
#![allow(clippy::empty_docs)]

/// The kind of Git repository, focussing on the repository data itself, i.e. what's in `.git`.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Kind {
    /// An ordinary Git repository.
    Common,
    /// A submodule worktree, whose `git` repository lives in `.git/modules/**/<name>` of the parent repository.
    ///
    /// Note that 'old-form' submodules (with a nested `.git` directory) are represented as [`Kind::Common`].
    Submodule,
    /// A worktree, whose `git` repository lives in `.git/worktrees/**/<name>` of the parent repository.
    LinkedWorkTree,
}

#[cfg(any(feature = "attributes", feature = "excludes"))]
pub mod attributes;
///
#[cfg(feature = "blame")]
mod blame;
/// Local branch operations.
pub mod branch;
mod cache;
#[cfg(feature = "worktree-mutation")]
mod checkout;
mod config;

///
#[cfg(feature = "blob-diff")]
mod diff;
///
#[cfg(feature = "dirwalk")]
mod dirwalk;
///
#[cfg(feature = "attributes")]
pub mod filter;
///
pub mod freelist;
mod graph;
pub(crate) mod identity;
mod impls;
#[cfg(feature = "index")]
mod index;
pub(crate) mod init;
mod location;
#[cfg(feature = "mailmap")]
mod mailmap;
///
#[cfg(feature = "merge")]
mod merge;
#[cfg(feature = "notes")]
mod note;
mod object;
#[cfg(feature = "attributes")]
mod pathspec;
mod reference;
mod remote;
mod revision;
mod shallow;
mod state;
#[cfg(feature = "attributes")]
mod submodule;
mod thread_safe;
pub(crate) mod worktree;

///
#[cfg(feature = "blame")]
pub mod blame_file {
    /// Options to be passed to [Repository::blame_file()](crate::Repository::blame_file()).
    #[derive(Default, Debug, Clone)]
    pub struct Options {
        /// The algorithm to use for diffing. If `None`, `diff.algorithm` will be used.
        pub diff_algorithm: Option<gix_diff::blob::Algorithm>,
        /// The ranges to blame in the file.
        pub ranges: gix_blame::BlameRanges,
        /// Don't consider commits before the given date.
        pub since: Option<gix_date::Time>,
        /// Determine if rename tracking should be performed, and how.
        pub rewrites: Option<gix_diff::Rewrites>,
    }
}
