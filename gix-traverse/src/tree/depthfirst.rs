pub use super::breadthfirst::Error;

/// The state used and potentially shared by multiple tree traversals, reusing memory.
#[derive(Default, Clone)]
pub struct State {
    freelist: Vec<Vec<u8>>,
}

impl State {
    /// Pop one empty buffer from the free-list.
    pub fn pop_buf(&mut self) -> Vec<u8> {
        match self.freelist.pop() {
            None => Vec::new(),
            Some(mut buf) => {
                buf.clear();
                buf
            }
        }
    }

    /// Make `buf` available for re-use with [`Self::pop_buf()`].
    pub fn push_buf(&mut self, buf: Vec<u8>) {
        self.freelist.push(buf);
    }
}

pub(super) mod function {
    use std::borrow::BorrowMut;

    use gix_hash::ObjectId;
    use gix_object::{FindExt, TreeRefIter};

    use super::{Error, State};
    use crate::tree::Visit;

    /// A depth-first traversal of the `root` tree, that preserves the natural order of a tree while immediately descending
    /// into sub-trees.
    ///
    /// `state` can be passed to re-use memory during multiple invocations.
    pub fn depthfirst<StateMut, Find, V>(
        root: ObjectId,
        mut state: StateMut,
        objects: Find,
        delegate: &mut V,
    ) -> Result<(), Error>
    where
        Find: gix_object::Find,
        StateMut: BorrowMut<State>,
        V: Visit,
    {
        enum Machine {
            GetTree(ObjectId),
            Iterate {
                tree_buf: Vec<u8>,
                byte_offset_to_next_entry: usize,
            },
        }

        let state = state.borrow_mut();
        let mut stack = vec![Machine::GetTree(root)];
        'outer: while let Some(item) = stack.pop() {
            match item {
                Machine::GetTree(id) => {
                    let mut buf = state.pop_buf();
                    objects.find_tree_iter(&id, &mut buf)?;
                    stack.push(Machine::Iterate {
                        tree_buf: buf,
                        byte_offset_to_next_entry: 0,
                    });
                }
                Machine::Iterate {
                    tree_buf: buf,
                    byte_offset_to_next_entry,
                } => {
                    let mut iter = TreeRefIter::from_bytes(&buf[byte_offset_to_next_entry..]);
                    delegate.pop_back_tracked_path_and_set_current();
                    while let Some(entry) = iter.next() {
                        let entry = entry?;
                        if entry.mode.is_tree() {
                            delegate.push_path_component(entry.filename);
                            let res = delegate.visit_tree(&entry);
                            delegate.pop_path_component();
                            match res {
                                std::ops::ControlFlow::Continue(true) => {}
                                std::ops::ControlFlow::Break(()) => break 'outer,
                                std::ops::ControlFlow::Continue(false) => continue,
                            }

                            delegate.push_back_tracked_path_component("".into());
                            delegate.push_back_tracked_path_component(entry.filename);
                            let recurse_tree = Machine::GetTree(entry.oid.to_owned());
                            let continue_at_next_entry = Machine::Iterate {
                                byte_offset_to_next_entry: iter.offset_to_next_entry(&buf),
                                tree_buf: buf,
                            };
                            stack.push(continue_at_next_entry);
                            stack.push(recurse_tree);
                            continue 'outer;
                        } else {
                            delegate.push_path_component(entry.filename);
                            if let std::ops::ControlFlow::Break(()) = delegate.visit_nontree(&entry) {
                                break 'outer;
                            }
                            delegate.pop_path_component();
                        }
                    }
                    state.push_buf(buf);
                }
            }
        }
        Ok(())
    }
}
