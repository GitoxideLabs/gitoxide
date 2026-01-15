use crate::{tree::recorder::Location, Rewrites};

mod change;
pub use change::{Change, ChangeRef};

/// The error returned by [`tree_with_rewrites()`](super::tree_with_rewrites()).
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    Diff(#[from] crate::tree::Error),
    #[error("The user-provided callback failed")]
    ForEach(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
    #[error("Failure during rename tracking")]
    RenameTracking(#[from] crate::rewrites::tracker::emit::Error),
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
    pub location: Option<Location>,
    /// If not `None`, rename tracking will be performed accordingly.
    pub rewrites: Option<Rewrites>,
}

pub(super) mod function;
