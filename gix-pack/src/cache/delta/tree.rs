use super::{traverse, Error};
use crate::exact_vec;
/// An item stored within the [`Tree`] whose data is stored in a pack file, identified by
/// the offset of its first (`offset`) and last (`next_offset`) bytes.
///
/// It represents either a root entry, or one that relies on a base to be resolvable,
/// alongside associated `data` `T`.
pub struct Item<T> {
    /// The offset into the pack file at which the pack entry's data is located.
    pub offset: crate::data::Offset,
    /// The offset of the next item in the pack file.
    pub next_offset: crate::data::Offset,
    /// Data to store with each Item, effectively data associated with each entry in a pack.
    pub data: T,
    /// Indices into our Tree's `items`, one for each pack entry that depends on us.
    ///
    /// Limited to u32 as that's the maximum amount of objects in a pack.
    // SAFETY INVARIANT:
    //    - only one Item in a tree may have any given child index. `future_child_offsets`
    //      should also not contain any indices found in `children`.\
    //    - These indices should be in bounds for tree.child_items
    children: Vec<u32>,
}

impl<T> Item<T> {
    /// Get the children
    // (we don't want to expose mutable access)
    pub fn children(&self) -> &[u32] {
        &self.children
    }
}

/// Identify what kind of node we have last seen
enum NodeKind {
    Root,
    Child,
}

/// A tree that allows one-time iteration over all nodes and their children, consuming it in the process,
/// while being shareable among threads without a lock.
/// It does this by making the guarantee that iteration only happens once.
pub struct Tree<T> {
    /// The root nodes, i.e. base objects
    // SAFETY invariant: see Item.children
    root_items: Vec<Item<T>>,
    /// The child nodes, i.e. those that rely a base object, like ref and ofs delta objects
    // SAFETY invariant: see Item.children
    child_items: Vec<Item<T>>,
    /// The last encountered node was either a root or a child.
    last_seen: Option<NodeKind>,
    /// Future child offsets, associating their offset into the pack with their index in the items array.
    /// (parent_offset, child_index)
    // SAFETY invariant:
    //    - None of these child indices should already have parents
    //      i.e. future_child_offsets[i].1 should never be also found
    //      in Item.children. Indices should be found here at most once.
    //    - These indices should be in bounds for tree.child_items.
    future_child_offsets: Vec<(crate::data::Offset, usize)>,
}

impl<T> Tree<T> {
    /// Instantiate a empty tree capable of storing `num_objects` amounts of items.
    pub fn with_capacity(num_objects: usize) -> Result<Self, Error> {
        Ok(Tree {
            root_items: exact_vec(num_objects / 2),
            child_items: exact_vec(num_objects / 2),
            last_seen: None,
            future_child_offsets: Vec::new(),
        })
    }

    pub(super) fn num_items(&self) -> usize {
        self.root_items.len() + self.child_items.len()
    }

    /// Returns self's root and child items.
    ///
    /// You can rely on them following the same `children` invariants as they did in the tree
    pub(super) fn take_root_and_child(self) -> (Vec<Item<T>>, Vec<Item<T>>) {
        (self.root_items, self.child_items)
    }

    pub(super) fn assert_is_incrementing_and_update_next_offset(
        &mut self,
        offset: crate::data::Offset,
    ) -> Result<(), Error> {
        let items = match &self.last_seen {
            Some(NodeKind::Root) => &mut self.root_items,
            Some(NodeKind::Child) => &mut self.child_items,
            None => return Ok(()),
        };
        let item = &mut items.last_mut().expect("last seen won't lie");
        if offset <= item.offset {
            return Err(Error::InvariantIncreasingPackOffset {
                last_pack_offset: item.offset,
                pack_offset: offset,
            });
        }
        item.next_offset = offset;
        Ok(())
    }

    pub(super) fn set_pack_entries_end_and_resolve_ref_offsets(
        &mut self,
        pack_entries_end: crate::data::Offset,
    ) -> Result<(), traverse::Error> {
        if !self.future_child_offsets.is_empty() {
            for (parent_offset, child_index) in self.future_child_offsets.drain(..) {
                // SAFETY invariants upheld:
                //  - We are draining from future_child_offsets and adding to children, keeping things the same.
                //  - We can rely on the `future_child_offsets` invariant to be sure that `children` is
                //    not getting any indices that are already in use in `children` elsewhere
                //  - The indices are in bounds for child_items since they were in bounds for future_child_offsets,
                //    we can carry over the invariant.
                if let Ok(i) = self.child_items.binary_search_by_key(&parent_offset, |i| i.offset) {
                    self.child_items[i].children.push(child_index as u32);
                } else if let Ok(i) = self.root_items.binary_search_by_key(&parent_offset, |i| i.offset) {
                    self.root_items[i].children.push(child_index as u32);
                } else {
                    return Err(traverse::Error::OutOfPackRefDelta {
                        base_pack_offset: parent_offset,
                    });
                }
            }
        }

        self.assert_is_incrementing_and_update_next_offset(pack_entries_end)
            .expect("BUG: pack now is smaller than all previously seen entries");
        Ok(())
    }

    /// Add a new root node, one that only has children but is not a child itself, at the given pack `offset` and associate
    /// custom `data` with it.
    pub fn add_root(&mut self, offset: crate::data::Offset, data: T) -> Result<(), Error> {
        self.assert_is_incrementing_and_update_next_offset(offset)?;
        self.last_seen = NodeKind::Root.into();
        self.root_items.push(Item {
            offset,
            next_offset: 0,
            data,
            // SAFETY INVARIANT upheld: there are no children
            children: Default::default(),
        });
        Ok(())
    }

    /// Add a child of the item at `base_offset` which itself resides at pack `offset` and associate custom `data` with it.
    pub fn add_child(
        &mut self,
        base_offset: crate::data::Offset,
        offset: crate::data::Offset,
        data: T,
    ) -> Result<(), Error> {
        self.assert_is_incrementing_and_update_next_offset(offset)?;

        let next_child_index = self.child_items.len();
        // SAFETY INVARIANT upheld:
        // - This is one of two methods that modifies `children` and future_child_offsets. Out
        //   of the two, it is the only one that produces new indices in the system.
        // - This always pushes next_child_index to *either* `children` or `future_child_offsets`,
        //   maintaining the cross-field invariant there.
        // - This method will always push to child_items (at the end), incrementing
        //   future values of next_child_index. This means next_child_index is always
        //   unique for this method call.
        // - As the only method producing new indices, this is the only time
        //   next_child_index will be added to children/future_child_offsets, upholding the invariant.
        // - Since next_child_index will always be a valid index by the end of this method,
        //   this always produces valid in-bounds indices, upholding the bounds invariant.

        if let Ok(i) = self.child_items.binary_search_by_key(&base_offset, |i| i.offset) {
            self.child_items[i].children.push(next_child_index as u32);
        } else if let Ok(i) = self.root_items.binary_search_by_key(&base_offset, |i| i.offset) {
            self.root_items[i].children.push(next_child_index as u32);
        } else {
            self.future_child_offsets.push((base_offset, next_child_index));
        }

        self.last_seen = NodeKind::Child.into();
        self.child_items.push(Item {
            offset,
            next_offset: 0,
            data,
            // SAFETY INVARIANT upheld: there are no children
            children: Default::default(),
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    mod from_offsets_in_pack {
        use std::sync::atomic::AtomicBool;

        use crate as pack;

        const SMALL_PACK_INDEX: &str = "objects/pack/pack-a2bf8e71d8c18879e499335762dd95119d93d9f1.idx";
        const SMALL_PACK: &str = "objects/pack/pack-a2bf8e71d8c18879e499335762dd95119d93d9f1.pack";

        const INDEX_V1: &str = "objects/pack/pack-c0438c19fb16422b6bbcce24387b3264416d485b.idx";
        const PACK_FOR_INDEX_V1: &str = "objects/pack/pack-c0438c19fb16422b6bbcce24387b3264416d485b.pack";

        use gix_testtools::fixture_path;

        #[test]
        fn v1() -> Result<(), Box<dyn std::error::Error>> {
            tree(INDEX_V1, PACK_FOR_INDEX_V1)
        }

        #[test]
        fn v2() -> Result<(), Box<dyn std::error::Error>> {
            tree(SMALL_PACK_INDEX, SMALL_PACK)
        }

        fn tree(index_path: &str, pack_path: &str) -> Result<(), Box<dyn std::error::Error>> {
            let idx = pack::index::File::at(fixture_path(index_path), gix_hash::Kind::Sha1)?;
            crate::cache::delta::Tree::from_offsets_in_pack(
                &fixture_path(pack_path),
                idx.sorted_offsets().into_iter(),
                &|ofs| *ofs,
                &|id| idx.lookup(id).map(|index| idx.pack_offset_at_index(index)),
                &mut gix_features::progress::Discard,
                &AtomicBool::new(false),
                gix_hash::Kind::Sha1,
            )?;
            Ok(())
        }
    }

    mod size {
        use gix_testtools::size_ok;

        use super::super::Item;

        #[test]
        fn size_of_pack_tree_item() {
            let actual = std::mem::size_of::<[Item<()>; 7_500_000]>();
            let expected = 300_000_000;
            assert!(
                size_ok(actual, expected),
                "we don't want these to grow unnoticed: {actual} <~ {expected}"
            );
        }

        #[test]
        fn size_of_pack_verify_data_structure() {
            pub struct EntryWithDefault {
                _index_entry: crate::index::Entry,
                _kind: gix_object::Kind,
                _object_size: u64,
                _decompressed_size: u64,
                _compressed_size: u64,
                _header_size: u16,
                _level: u16,
            }

            let actual = std::mem::size_of::<[Item<EntryWithDefault>; 7_500_000]>();
            let sha1 = 840_000_000;
            let sha256_extra = 120_000_000;
            let expected = sha1 + sha256_extra;
            assert!(
                size_ok(actual, expected),
                "we don't want these to grow unnoticed: {actual} <~ {expected}"
            );
        }
    }
}
