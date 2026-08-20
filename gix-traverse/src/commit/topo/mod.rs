//! Topological commit traversal, similar to `git log --topo-order`, which keeps track of graph state.

use bitflags::bitflags;

/// The errors that can occur during creation and iteration.
#[derive(thiserror::Error, Debug)]
#[expect(missing_docs)]
pub enum Error {
    #[error("Indegree information is missing")]
    MissingIndegreeUnexpected,
    #[error("Internal state (bitflags) not found")]
    MissingStateUnexpected,
    #[error(transparent)]
    ObjectDecode(#[from] gix_object::decode::Error),
    #[error(transparent)]
    Find(gix_error::Error),
}

impl From<gix_object::find::existing_iter::Error> for Error {
    fn from(err: gix_object::find::existing_iter::Error) -> Self {
        Error::Find(err.into_error())
    }
}

bitflags! {
    /// Set of flags to describe the state of a particular commit while iterating.
    // NOTE: The names correspond to the names of the flags in revision.h
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub(super) struct WalkFlags: u8 {
        /// Commit has been seen
        const Seen = 0b000001;
        /// Commit has been processed by the Explore walk
        const Explored = 0b000010;
        /// Commit has been processed by the Indegree walk
        const InDegree = 0b000100;
        /// Commit is deemed uninteresting for whatever reason
        const Uninteresting = 0b001000;
        /// Commit marks the end of a walk, like `foo` in `git rev-list foo..bar`
        const Bottom = 0b010000;
        /// Parents have been processed
        const Added = 0b100000;
    }
}

/// Sorting to use for the topological walk.
///
/// ### Sample History
///
/// The following history will be referred to for explaining how the sort order works, with the number denoting the commit timestamp
/// (*their X-alignment doesn't matter*).
///
/// ```text
/// ---1----2----4----7 <- second parent of 8
///     \              \
///      3----5----6----8---
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub enum Sorting {
    /// Show no parents before all of its children are shown, but otherwise show
    /// commits in the commit timestamp order.
    ///
    /// This is equivalent to `git rev-list --date-order`.
    #[default]
    DateOrder,
    /// Show no parents before all of its children are shown, and avoid
    /// showing commits on multiple lines of history intermixed.
    ///
    /// In the *sample history* the order would be `8, 6, 5, 3, 7, 4, 2, 1`.
    /// This is equivalent to `git rev-list --topo-order`.
    TopoOrder,
}

mod init;
pub use init::Builder;

pub(super) mod iter;
