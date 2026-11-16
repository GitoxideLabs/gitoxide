use crate::{Rewrites, tree::recorder::Location};

mod change;
mod directories;
pub use crate::rewrites::tracker::Context;
pub use change::{Change, ChangeRef};

/// A removed directory's destinations inferred from finalized file renames.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryRename {
    /// The removed directory, as a repository-relative path.
    pub source: bstr::BString,
    /// Destinations tied for the largest number of renamed children, in path order.
    /// A single destination establishes a rename; an empty path denotes the repository root.
    /// Multiple destinations leave the directory's location ambiguous. This is never empty.
    pub destinations: nonempty::NonEmpty<bstr::BString>,
}

/// File-rewrite statistics and directory inference produced by a tree diff.
#[derive(Clone, Debug, PartialEq)]
pub struct Outcome {
    /// Statistics for the file rename and copy search.
    pub rewrites: crate::rewrites::Outcome,
    /// Directory destinations, including root destinations and unresolved ties that cannot
    /// be expressed as ordinary tree changes. Paths are complete regardless of `Options::location`.
    pub directory_renames: Vec<DirectoryRename>,
}

/// Returned by the [`tree_with_rewrites()`](super::tree_with_rewrites()) function to control flow.
///
/// Use [`std::ops::ControlFlow::Continue`] to continue the traversal of changes.
/// Use [`std::ops::ControlFlow::Break`] to stop the traversal of changes and stop calling the function that returned it.
pub type Action = std::ops::ControlFlow<()>;

/// Options for use in [`tree_with_rewrites()`](super::tree_with_rewrites()).
#[derive(Default, Clone, Debug)]
pub struct Options {
    /// Determine how locations of changes, i.e. their repository-relative path, should be tracked.
    /// If `None`, locations will always be empty.
    /// Rewrite tracking uses full paths internally regardless of this setting.
    pub location: Option<Location>,
    /// If not `None`, rename tracking will be performed accordingly.
    pub rewrites: Option<Rewrites>,
}

pub(super) mod function;
