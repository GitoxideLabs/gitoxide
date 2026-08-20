/// Returned when using various methods on a [`Tree`]
pub type Error = gix_error::CorruptionError;
///
/// A tree that allows one-time iteration over all nodes and their children, consuming it in the process,
/// while being shareable among threads without a lock.
/// It does this by making the guarantee that iteration only happens once.
pub struct Tree<T> {
    /// The root nodes, i.e. base objects
    // SAFETY invariant: see Item.children
    root_items: Vec<tree::Item<T>>,
    /// The child nodes, i.e. those that rely a base object, like ref and ofs delta objects
    // SAFETY invariant: see Item.children
    child_items: Vec<tree::Item<T>>,
    /// The last encountered node was either a root or a child.
    last_seen: Option<tree::NodeKind>,
    /// Future child offsets, associating their offset into the pack with their index in the items array.
    /// (parent_offset, child_index)
    // SAFETY invariant:
    //    - None of these child indices should already have parents
    //      i.e. future_child_offsets[i].1 should never be also found
    //      in Item.children. Indices should be found here at most once.
    //    - These indices should be in bounds for tree.child_items.
    future_child_offsets: Vec<(crate::data::Offset, usize)>,
    /// Child indices waiting for an in-pack object with the given id to be resolved.
    ref_child_indices: tree::RefDeltaChildren,
}

///
pub mod traverse;

///
pub mod from_offsets;

/// Types associated with [Tree].
// kept in separate module to encapsulate unsafety (it has field invariants)
pub mod tree;
