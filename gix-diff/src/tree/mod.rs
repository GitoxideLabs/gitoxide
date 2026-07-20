use std::collections::VecDeque;

use bstr::BStr;
use gix_hash::ObjectId;
use gix_object::bstr::BString;

use crate::tree::visit::Relation;

/// The error returned by [`tree()`](super::tree()).
// TODO(review): these implementations hand-preserve `#[error(transparent)]` semantics for `Find`
//                and `EntriesDecode`: `Display` passes the formatter through and `source()` forwards
//                to the inner error's source, exactly like the `thiserror`-generated code did. The
//                same pattern is used across the other error types of this crate.
#[derive(Debug)]
#[allow(missing_docs)]
pub enum Error {
    Find(gix_object::find::existing_iter::Error),
    Cancelled,
    EntriesDecode(gix_object::decode::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Find(err) => std::fmt::Display::fmt(err, f),
            Error::Cancelled => f.write_str("The delegate cancelled the operation"),
            Error::EntriesDecode(err) => std::fmt::Display::fmt(err, f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Find(err) => err.source(),
            Error::Cancelled => None,
            Error::EntriesDecode(err) => err.source(),
        }
    }
}

impl From<gix_object::find::existing_iter::Error> for Error {
    fn from(err: gix_object::find::existing_iter::Error) -> Self {
        Error::Find(err)
    }
}

impl From<gix_object::decode::Error> for Error {
    fn from(err: gix_object::decode::Error) -> Self {
        Error::EntriesDecode(err)
    }
}

/// A trait to allow responding to a traversal designed to figure out the [changes](visit::Change)
/// to turn tree A into tree B.
pub trait Visit {
    /// Sets the full path in front of the queue so future calls to push and pop components affect it instead.
    fn pop_front_tracked_path_and_set_current(&mut self);
    /// Append a `component` to the end of a path, which may be empty.
    fn push_back_tracked_path_component(&mut self, component: &BStr);
    /// Append a `component` to the end of a path, which may be empty.
    fn push_path_component(&mut self, component: &BStr);
    /// Removes the last component from the path, which may leave it empty.
    fn pop_path_component(&mut self);
    /// Record a `change` and return an instruction whether to continue or not.
    ///
    /// The implementation may use the current path to lean where in the tree the change is located.
    fn visit(&mut self, change: visit::Change) -> visit::Action;
}

/// The state required to run [tree-diffs](super::tree()).
#[derive(Default, Clone)]
pub struct State {
    /// A buffer for object data.
    pub buf1: Vec<u8>,
    /// Another buffer for object data.
    pub buf2: Vec<u8>,
    trees: VecDeque<TreeInfoTuple>,
    change_id: visit::ChangeId,
}

type TreeInfoTuple = (Option<ObjectId>, Option<ObjectId>, Option<Relation>);

impl State {
    fn clear(&mut self) {
        self.trees.clear();
        self.buf1.clear();
        self.buf2.clear();
        self.change_id = 0;
    }
}

pub(super) mod function;

///
pub mod visit;

/// A [Visit] implementation to record every observed change and keep track of the changed paths.
#[derive(Clone, Debug)]
pub struct Recorder {
    path_deque: VecDeque<BString>,
    path: BString,
    location: Option<recorder::Location>,
    /// The observed changes.
    pub records: Vec<recorder::Change>,
}

/// Useful for use as delegate implementing [`Visit`] to keep track of all seen changes. Useful for debugging or printing primarily.
pub mod recorder;
