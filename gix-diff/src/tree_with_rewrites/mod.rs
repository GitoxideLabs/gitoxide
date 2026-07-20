use crate::{Rewrites, tree::recorder::Location};

mod change;
pub use change::{Change, ChangeRef};

/// The error returned by [`tree_with_rewrites()`](super::tree_with_rewrites()).
#[derive(Debug)]
#[allow(missing_docs)]
pub enum Error {
    Diff(crate::tree::Error),
    ForEach(Box<dyn std::error::Error + Send + Sync + 'static>),
    RenameTracking(crate::rewrites::tracker::emit::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Diff(err) => std::fmt::Display::fmt(err, f),
            Error::ForEach(_) => f.write_str("The user-provided callback failed"),
            Error::RenameTracking(_) => f.write_str("Failure during rename tracking"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Diff(err) => err.source(),
            Error::ForEach(err) => Some(&**err),
            Error::RenameTracking(err) => Some(err),
        }
    }
}

impl From<crate::tree::Error> for Error {
    fn from(err: crate::tree::Error) -> Self {
        Error::Diff(err)
    }
}

impl From<crate::rewrites::tracker::emit::Error> for Error {
    fn from(err: crate::rewrites::tracker::emit::Error) -> Self {
        Error::RenameTracking(err)
    }
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
