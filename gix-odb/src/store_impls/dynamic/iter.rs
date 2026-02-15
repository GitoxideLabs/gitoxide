use std::{ops::Deref, option::Option::None, vec::IntoIter};

use gix_hash::ObjectId;

use crate::{
    loose,
    store::{handle, handle::SingleOrMultiIndex, types::PackId},
    store_impls::dynamic,
};

struct EntryForOrdering {
    pack_offset: u64,
    entry_index: u32,
    pack_index: u16,
}

enum State {
    Pack {
        index_iter: IntoIter<handle::IndexLookup>,
        index: handle::IndexLookup,
        ordered_entries: Option<Vec<EntryForOrdering>>,
        entry_index: u32,
        num_objects: u32,
    },
    Loose {
        iter: loose::Iter,
        index: usize,
    },
    Depleted,
}

/// Define the order in which objects are returned.
#[derive(Default, Debug, Copy, Clone)]
pub enum Ordering {
    /// Traverse packs first as sorted by their index files in lexicographical order (sorted by object id), then traverse loose objects
    /// as sorted by their names as well.
    ///
    /// This mode uses no memory as it's the natural ordering of objects, and is best to obtain all object ids as quickly as possible,
    /// while noting that these may contain duplicates. However, it's very costly to obtain object information or decode them with this
    /// scheme as cache-hits are unlikely with it and memory maps are less efficient when loading them in random order.
    #[default]
    PackLexicographicalThenLooseLexicographical,
    /// Traverse packs first yielding object ids sorted by their position in the pack, with those at the beginning of the pack file coming first.
    /// Then follow loose objects sorted by their names.
    ///
    /// This mode allocates and as to pre-sort objects by their offsets, delaying the start of the iteration once per pack while keeping
    /// memory allocated once per pack. This price is usually worth paying once querying object information is planned as pack caches
    /// are more efficiently used that way.
    PackAscendingOffsetThenLooseLexicographical,
}

/// An iterator over all, _possibly duplicate_, objects of an object store, which by default uses no extra memory but yields an
/// order that is costly to traverse when querying object information or decoding them.
///
/// Use [`with_ordering()`][AllObjects::with_ordering()] to choose a performance trade-off.
pub struct AllObjects {
    state: State,
    num_objects: usize,
    loose_dbs: dynamic::load_index::NonEmptyLooseDbs,
    order: Ordering,
}

/// Builder
impl AllObjects {
    /// Set the ordering of the objects returned, trading off memory and latency for object query performance.
    pub fn with_ordering(mut self, order: Ordering) -> Self {
        self.order = order;
        self
    }
}

impl AllObjects {
    /// Create a new iterator from a dynamic store, which will be forced to load all indices eagerly and in the current thread.
    pub fn new(db: &dynamic::Store) -> Result<Self, crate::store::load_index::Error> {
        let snapshot = db.load_all_indices()?;

        let packed_objects = snapshot
            .indices
            .iter()
            .fold(0usize, |dbc, index| dbc.saturating_add(index.num_objects() as usize));
        let loose_dbs = snapshot
            .non_empty_loose_dbs()
            .expect("initialized snapshots always have at least one loose object db");
        let mut index_iter = snapshot.indices.into_iter();
        let order = Default::default();
        let state = match index_iter.next() {
            Some(index) => {
                let num_objects = index.num_objects();
                State::Pack {
                    index_iter,
                    ordered_entries: maybe_sort_entries(&index, order),
                    index,
                    entry_index: 0,
                    num_objects,
                }
            }
            None => {
                let index = 0;
                State::Loose {
                    iter: loose_dbs.first().iter(),
                    index,
                }
            }
        };
        Ok(AllObjects {
            state,
            loose_dbs,
            num_objects: packed_objects,
            order,
        })
    }
}

fn maybe_sort_entries(index: &handle::IndexLookup, order: Ordering) -> Option<Vec<EntryForOrdering>> {
    let mut order: Vec<_> = match order {
        Ordering::PackLexicographicalThenLooseLexicographical => return None,
        Ordering::PackAscendingOffsetThenLooseLexicographical => match &index.file {
            // We know that we cannot have more than u32 entry indices per pack.
            SingleOrMultiIndex::Single { index, .. } => index
                .iter()
                .enumerate()
                .map(|(idx, e)| EntryForOrdering {
                    pack_offset: e.pack_offset,
                    entry_index: idx as u32,
                    pack_index: 0,
                })
                .collect(),
            SingleOrMultiIndex::Multi { index, .. } => index
                .iter()
                .enumerate()
                .map(|(idx, e)| EntryForOrdering {
                    pack_offset: e.pack_offset,
                    entry_index: idx as u32,
                    pack_index: {
                        debug_assert!(
                            e.pack_index < PackId::max_packs_in_multi_index(),
                            "this shows the relation between u16 and pack_index (u32) and why this is OK"
                        );
                        e.pack_index as u16
                    },
                })
                .collect(),
        },
    };
    order.sort_by(|a, b| {
        a.pack_index
            .cmp(&b.pack_index)
            .then_with(|| a.pack_offset.cmp(&b.pack_offset))
    });
    Some(order)
}

impl Iterator for AllObjects {
    type Item = Result<ObjectId, loose::iter::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.state {
            State::Depleted => None,
            State::Pack {
                index_iter,
                ordered_entries,
                index,
                entry_index,
                num_objects,
            } => {
                if *entry_index < *num_objects {
                    let oid = match ordered_entries {
                        Some(entries) => index.oid_at_index(entries[*entry_index as usize].entry_index),
                        None => index.oid_at_index(*entry_index),
                    }
                    .to_owned();
                    *entry_index += 1;
                    Some(Ok(oid))
                } else {
                    match index_iter.next() {
                        Some(new_index) => {
                            *ordered_entries = maybe_sort_entries(&new_index, self.order);
                            *index = new_index;
                            *entry_index = 0;
                            *num_objects = index.num_objects();
                        }
                        None => {
                            let index = 0;
                            self.state = State::Loose {
                                iter: self.loose_dbs.first().iter(),
                                index,
                            }
                        }
                    }
                    self.next()
                }
            }
            State::Loose { iter, index } => match iter.next() {
                Some(id) => Some(id),
                None => {
                    *index += 1;
                    match self.loose_dbs.get(*index).map(loose::Store::iter) {
                        Some(new_iter) => {
                            *iter = new_iter;
                            self.next()
                        }
                        None => {
                            self.state = State::Depleted;
                            None
                        }
                    }
                }
            },
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.num_objects, None)
    }
}

impl<S> super::Handle<S>
where
    S: Deref<Target = super::Store> + Clone,
{
    /// Return an iterator over all, _possibly duplicate_, objects, first the ones in all packs of all linked databases (via alternates),
    /// followed by all loose objects.
    pub fn iter(&self) -> Result<AllObjects, dynamic::load_index::Error> {
        AllObjects::new(self.store_ref())
    }
}

impl dynamic::Store {
    /// Like [`Handle::iter()`][super::Handle::iter()], but accessible directly on the store.
    pub fn iter(&self) -> Result<AllObjects, dynamic::load_index::Error> {
        AllObjects::new(self)
    }
}
