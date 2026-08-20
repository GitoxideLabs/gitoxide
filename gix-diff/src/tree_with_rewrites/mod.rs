use crate::{Rewrites, tree::recorder::Location};

mod change;
pub use change::{Change, ChangeRef};

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
