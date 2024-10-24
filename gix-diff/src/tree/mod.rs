use crate::tree::visit::Relation;
use bstr::BStr;
use gix_hash::ObjectId;
use gix_object::bstr::BString;
use std::collections::VecDeque;

/// The error returned by [`tree()`](super::tree()).
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    Find(#[from] gix_object::find::existing_iter::Error),
    #[error("The delegate cancelled the operation")]
    Cancelled,
    #[error(transparent)]
    EntriesDecode(#[from] gix_object::decode::Error),
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
