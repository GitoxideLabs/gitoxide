use std::collections::VecDeque;

use gix_hash::ObjectId;

/// The error is part of the item returned by the [`breadthfirst()`](crate::tree::breadthfirst())  and
///[`depthfirst()`](crate::tree::depthfirst()) functions.
pub type Error = gix_error::Exn;

/// The state used and potentially shared by multiple tree traversals.
#[derive(Default, Clone)]
pub struct State {
    next: VecDeque<ObjectId>,
    buf: Vec<u8>,
}

impl State {
    fn clear(&mut self) {
        self.next.clear();
        self.buf.clear();
    }
}

pub(super) mod function {
    use std::borrow::BorrowMut;

    use gix_error::{CorruptionError, ErrorExt, ResultExt, message};
    use gix_object::{FindExt, TreeRefIter};

    use super::{Error, State};
    use crate::tree::Visit;

    /// Start a breadth-first iteration over the `root` trees entries.
    ///
    /// Note that non-trees will be listed first, so the natural order of entries within a tree is lost.
    ///
    /// * `root`
    ///   * the tree to iterate in a nested fashion.
    /// * `state` - all state used for the iteration. If multiple iterations are performed, allocations can be minimized by reusing
    ///   this state.
    /// * `find` - a way to lookup new object data during traversal by their `ObjectId`, writing their data into buffer and returning
    ///   an iterator over entries if the object is present and is a tree. Caching should be implemented within this function
    ///   as needed. The return value is `Option<TreeIter>` which degenerates all error information. Not finding a commit should also
    ///   be considered an error as all objects in the tree DAG should be present in the database. Hence lookup errors should
    ///   be escalated into a more specific error if encountered by the caller.
    /// * `delegate` - A way to observe entries and control the iteration while allowing the optimizer to let you pay only for what you use.
    pub fn breadthfirst<StateMut, Find, V>(
        root: TreeRefIter<'_>,
        mut state: StateMut,
        objects: Find,
        delegate: &mut V,
    ) -> Result<(), Error>
    where
        Find: gix_object::Find,
        StateMut: BorrowMut<State>,
        V: Visit,
    {
        let state = state.borrow_mut();
        state.clear();
        let mut tree = root;
        loop {
            for entry in tree {
                let entry =
                    entry.or_raise_erased(|| CorruptionError::new("A tree could not be decoded during traversal"))?;
                if entry.mode.is_tree() {
                    delegate.push_path_component(entry.filename);
                    let action = delegate.visit_tree(&entry);
                    match action {
                        std::ops::ControlFlow::Continue(false) => {}
                        std::ops::ControlFlow::Continue(true) => {
                            delegate.pop_path_component();
                            delegate.push_back_tracked_path_component(entry.filename);
                            state.next.push_back(entry.oid.to_owned());
                        }
                        std::ops::ControlFlow::Break(()) => {
                            return Err(message("The delegate cancelled the operation").raise_erased());
                        }
                    }
                } else {
                    delegate.push_path_component(entry.filename);
                    if delegate.visit_nontree(&entry).is_break() {
                        return Err(message("The delegate cancelled the operation").raise_erased());
                    }
                }
                delegate.pop_path_component();
            }
            match state.next.pop_front() {
                Some(oid) => {
                    delegate.pop_front_tracked_path_and_set_current();
                    tree = objects.find_tree_iter(&oid, &mut state.buf)?;
                }
                None => break Ok(()),
            }
        }
    }
}
