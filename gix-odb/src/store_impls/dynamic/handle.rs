use std::{
    cell::RefCell,
    ops::Deref,
    rc::Rc,
    sync::{Arc, atomic::Ordering},
};

use gix_features::threading::OwnShared;
use gix_hash::oid;

use crate::store::{RefreshMode, handle, types};

#[derive(Clone)]
pub(crate) enum SingleOrMultiIndex {
    Single {
        index: Arc<gix_pack::index::File>,
        data: Option<Arc<gix_pack::data::File>>,
    },
    Multi {
        index: Arc<gix_pack::multi_index::File>,
        data: Vec<Option<Arc<gix_pack::data::File>>>,
    },
}

/// A utility to allow looking up pack offsets for a particular pack
pub(crate) enum IntraPackLookup<'a> {
    Single(&'a gix_pack::index::File),
    /// the internal pack-id inside of a multi-index for which the lookup is supposed to be.
    /// Used to prevent ref-delta OIDs to, for some reason, point to a different pack.
    Multi {
        index: &'a gix_pack::multi_index::File,
        required_pack_index: gix_pack::multi_index::PackIndex,
    },
}

impl IntraPackLookup<'_> {
    pub(crate) fn matches(&self, files: &types::IndexAndPacks) -> bool {
        match (self, files) {
            (IntraPackLookup::Single(expected), types::IndexAndPacks::Index(bundle)) => bundle
                .index
                .loaded()
                .is_some_and(|actual| std::ptr::eq(actual.as_ref(), *expected)),
            (IntraPackLookup::Multi { index: expected, .. }, types::IndexAndPacks::MultiIndex(bundle)) => bundle
                .multi_index
                .loaded()
                .is_some_and(|actual| std::ptr::eq(actual.as_ref(), *expected)),
            _ => false,
        }
    }

    pub(crate) fn pack_offset_by_id(&self, id: &oid) -> Option<gix_pack::data::Offset> {
        match self {
            IntraPackLookup::Single(index) => index
                .lookup(id)
                .map(|entry_index| index.pack_offset_at_index(entry_index)),
            IntraPackLookup::Multi {
                index,
                required_pack_index,
            } => index.lookup(id).and_then(|entry_index| {
                let (pack_index, pack_offset) = index.pack_id_and_pack_offset_at_index(entry_index);
                (pack_index == *required_pack_index).then_some(pack_offset)
            }),
        }
    }
}

#[derive(Clone)]
pub struct IndexLookup {
    pub(crate) file: SingleOrMultiIndex,
    /// The resource node from which packs can be loaded without resolving a mutable catalog position.
    pub(crate) slot: Arc<types::MutableIndexAndPack>,
    /// The slot position, used only for diagnostics and deterministic test synchronization.
    pub(crate) slot_id: usize,
}

pub struct IndexForObjectInPack {
    /// The pack within a multi-pack index, or `None` for a standalone pack.
    pub(crate) pack_index: Option<gix_pack::multi_index::PackIndex>,
    /// The offset at which the object's entry can be found
    pub(crate) pack_offset: u64,
}

pub(crate) mod index_lookup {
    use std::{collections::HashSet, sync::Arc};

    use gix_hash::oid;

    use crate::store::{handle, handle::IntraPackLookup, types};

    pub(crate) struct Outcome<'a> {
        pub object_index: handle::IndexForObjectInPack,
        pub index_file: IntraPackLookup<'a>,
        pub pack: &'a mut Option<Arc<gix_pack::data::File>>,
        pub slot: &'a types::MutableIndexAndPack,
        pub slot_id: usize,
    }

    impl handle::IndexLookup {
        /// Return an iterator over the entries of the loaded pack identified by `pack_id`.
        pub(crate) fn iter(
            &self,
            pack_id: gix_pack::data::Id,
        ) -> Option<Box<dyn Iterator<Item = gix_pack::index::Entry> + '_>> {
            match &self.file {
                handle::SingleOrMultiIndex::Single { index, data } => {
                    (data.as_ref()?.id == pack_id).then(|| index.iter())
                }
                handle::SingleOrMultiIndex::Multi { index, data } => {
                    let pack_index = data
                        .iter()
                        .position(|pack| pack.as_ref().is_some_and(|pack| pack.id == pack_id))?
                        as gix_pack::multi_index::PackIndex;
                    Some(Box::new(index.iter().filter_map(move |e| {
                        (e.pack_index == pack_index).then_some(gix_pack::index::Entry {
                            oid: e.oid,
                            pack_offset: e.pack_offset,
                            crc32: None,
                        })
                    })))
                }
            }
        }

        pub(crate) fn pack(&mut self, pack_id: gix_pack::data::Id) -> Option<&'_ Arc<gix_pack::data::File>> {
            match &mut self.file {
                handle::SingleOrMultiIndex::Single { data, .. } => data.as_ref().filter(|pack| pack.id == pack_id),
                handle::SingleOrMultiIndex::Multi { data, .. } => {
                    data.iter().filter_map(Option::as_ref).find(|pack| pack.id == pack_id)
                }
            }
        }

        pub(crate) fn contains_pack(&self, pack_id: gix_pack::data::Id) -> bool {
            match &self.file {
                handle::SingleOrMultiIndex::Single { data, .. } => data.as_ref().is_some_and(|pack| pack.id == pack_id),
                handle::SingleOrMultiIndex::Multi { data, .. } => {
                    data.iter().filter_map(Option::as_ref).any(|pack| pack.id == pack_id)
                }
            }
        }

        pub(crate) fn set_pack(
            &mut self,
            pack_index: Option<gix_pack::multi_index::PackIndex>,
            pack: Arc<gix_pack::data::File>,
        ) {
            match (&mut self.file, pack_index) {
                (handle::SingleOrMultiIndex::Single { data, .. }, None) => *data = Some(pack),
                (handle::SingleOrMultiIndex::Multi { data, .. }, Some(pack_index)) => {
                    data[pack_index as usize] = Some(pack);
                }
                _ => debug_assert!(false, "index and pack selector originate from the same lookup"),
            }
        }

        /// Return true if the given object id exists in this index
        pub(crate) fn contains(&self, object_id: &oid) -> bool {
            match &self.file {
                handle::SingleOrMultiIndex::Single { index, .. } => index.lookup(object_id).is_some(),
                handle::SingleOrMultiIndex::Multi { index, .. } => index.lookup(object_id).is_some(),
            }
        }

        /// Return true if the given object id exists in this index
        pub(crate) fn oid_at_index(&self, entry_index: u32) -> &gix_hash::oid {
            match &self.file {
                handle::SingleOrMultiIndex::Single { index, .. } => index.oid_at_index(entry_index),
                handle::SingleOrMultiIndex::Multi { index, .. } => index.oid_at_index(entry_index),
            }
        }

        /// Return the amount of objects contained in the index, essentially the number of object ids.
        pub(crate) fn num_objects(&self) -> u32 {
            match &self.file {
                handle::SingleOrMultiIndex::Single { index, .. } => index.num_objects(),
                handle::SingleOrMultiIndex::Multi { index, .. } => index.num_objects(),
            }
        }

        /// Call `lookup_prefix(…)` on either index or multi-index, and transform matches into an object id.
        pub(crate) fn lookup_prefix(
            &self,
            prefix: gix_hash::Prefix,
            candidates: Option<&mut HashSet<gix_hash::ObjectId>>,
        ) -> Option<crate::store::prefix::lookup::Outcome> {
            let mut candidate_entries = candidates.as_ref().map(|_| 0..0);
            let res = match &self.file {
                handle::SingleOrMultiIndex::Single { index, .. } => {
                    index.lookup_prefix(prefix, candidate_entries.as_mut())
                }
                handle::SingleOrMultiIndex::Multi { index, .. } => {
                    index.lookup_prefix(prefix, candidate_entries.as_mut())
                }
            }?;

            if let Some((candidates, entries)) = candidates.zip(candidate_entries) {
                candidates.extend(entries.map(|entry| self.oid_at_index(entry).to_owned()));
            }
            Some(res.map(|entry_index| self.oid_at_index(entry_index).to_owned()))
        }

        /// See if the oid is contained in this index, and return its full id for lookup possibly alongside its data file if already
        /// loaded.
        /// Also return the index itself as it's needed to resolve intra-pack ref-delta objects. They are a possibility even though
        /// they won't be used in practice as it's more efficient to store their offsets.
        /// If it is not loaded, ask it to be loaded and put it into the returned mutable option for safe-keeping.
        pub(crate) fn lookup(&mut self, object_id: &oid) -> Option<Outcome<'_>> {
            let slot = &*self.slot;
            let slot_id = self.slot_id;
            match &mut self.file {
                handle::SingleOrMultiIndex::Single { index, data } => index.lookup(object_id).map(move |idx| Outcome {
                    object_index: handle::IndexForObjectInPack {
                        pack_index: None,
                        pack_offset: index.pack_offset_at_index(idx),
                    },
                    index_file: IntraPackLookup::Single(index),
                    pack: data,
                    slot,
                    slot_id,
                }),
                handle::SingleOrMultiIndex::Multi { index, data } => index.lookup(object_id).map(move |idx| {
                    let (pack_index, pack_offset) = index.pack_id_and_pack_offset_at_index(idx);
                    Outcome {
                        object_index: handle::IndexForObjectInPack {
                            pack_index: Some(pack_index),
                            pack_offset,
                        },
                        index_file: IntraPackLookup::Multi {
                            index,
                            required_pack_index: pack_index,
                        },
                        pack: &mut data[pack_index as usize],
                        slot,
                        slot_id,
                    }
                }),
            }
        }
    }
}

pub(crate) enum Mode {
    DeletedPacksAreInaccessible,
    /// This mode signals that we should not unload packs even after they went missing.
    KeepDeletedPacksAvailable,
}

/// Handle registration
impl super::Store {
    pub(crate) fn register_handle(&self) -> Mode {
        self.num_handles.fetch_add(1, Ordering::Relaxed);
        Mode::DeletedPacksAreInaccessible
    }
    pub(crate) fn remove_handle(&self, _mode: Mode) {
        self.num_handles.fetch_sub(1, Ordering::Relaxed);
    }
    pub(crate) fn upgrade_handle(&self, _mode: Mode) -> Mode {
        Mode::KeepDeletedPacksAvailable
    }
}

/// Handle creation
impl super::Store {
    /// The amount of times a ref-delta base can be followed when multi-indices are involved.
    pub const INITIAL_MAX_RECURSION_DEPTH: usize = 32;

    /// Create a new cache filled with a handle to this store, if this store is supporting shared ownership.
    ///
    /// Note that the actual type of `OwnShared` depends on the `parallel` feature toggle of the `gix-features` crate.
    pub fn to_cache(self: &OwnShared<Self>) -> crate::Cache<super::Handle<OwnShared<super::Store>>> {
        self.to_handle().into()
    }

    /// Create a new cache filled with a handle to this store if this store is held in an `Arc`.
    pub fn to_cache_arc(self: &Arc<Self>) -> crate::Cache<super::Handle<Arc<super::Store>>> {
        self.to_handle_arc().into()
    }

    /// Create a new database handle to this store if this store is supporting shared ownership.
    ///
    /// See also, [`to_cache()`][super::Store::to_cache()] which is probably more useful.
    pub fn to_handle(self: &OwnShared<Self>) -> super::Handle<OwnShared<super::Store>> {
        let token = self.register_handle();
        super::Handle {
            store: self.clone(),
            refresh: RefreshMode::default(),
            ignore_replacements: false,
            loose_compression: self.loose_compression,
            token: Some(token),
            inflate: RefCell::new(Default::default()),
            snapshot: RefCell::new(self.collect_snapshot()),
            retained_indices: Default::default(),
            max_recursion_depth: Self::INITIAL_MAX_RECURSION_DEPTH,
            packed_object_count: Default::default(),
        }
    }

    /// Create a new database handle to this store if this store is held in an `Arc`.
    ///
    /// This method is useful in applications that know they will use threads.
    pub fn to_handle_arc(self: &Arc<Self>) -> super::Handle<Arc<super::Store>> {
        let token = self.register_handle();
        super::Handle {
            store: self.clone(),
            refresh: Default::default(),
            ignore_replacements: false,
            loose_compression: self.loose_compression,
            token: Some(token),
            inflate: RefCell::new(Default::default()),
            snapshot: RefCell::new(self.collect_snapshot()),
            retained_indices: Default::default(),
            max_recursion_depth: Self::INITIAL_MAX_RECURSION_DEPTH,
            packed_object_count: Default::default(),
        }
    }

    /// Transform the only instance into an `Arc<Self>` or panic if this is not the only Rc handle
    /// to the contained store.
    ///
    /// This is meant to be used when the `gix_features::threading::OwnShared` refers to an `Rc` as it was compiled without the
    /// `parallel` feature toggle.
    pub fn into_shared_arc(self: OwnShared<Self>) -> Arc<Self> {
        match OwnShared::try_unwrap(self) {
            Ok(this) => Arc::new(this),
            Err(_) => panic!("BUG: Must be called when there is only one owner for this RC"),
        }
    }
}

impl<S> super::Handle<S>
where
    S: Deref<Target = super::Store> + Clone,
{
    pub(crate) fn index_ctx(&self, marker: types::SlotIndexMarker) -> super::IndexCtx {
        super::IndexCtx {
            refresh_mode: self.refresh,
            marker,
            loose_compression: self.loose_compression,
        }
    }

    /// Call once if pack locations are stored and later used for lookup, retaining their indices and packs even if they disappear.
    /// This must be called if there is a chance that git maintenance is happening while a pack is created.
    pub fn prevent_pack_unload(&mut self) {
        let was_unstable = matches!(self.token.as_ref(), Some(handle::Mode::DeletedPacksAreInaccessible));
        self.token = self.token.take().map(|token| self.store.upgrade_handle(token));
        if was_unstable {
            *self.snapshot.get_mut() = self.store.collect_snapshot();
            self.clear_cache();
        }
    }

    /// Return a shared reference to the contained store.
    pub fn store_ref(&self) -> &S::Target {
        &self.store
    }

    /// Return an owned store with shared ownership.
    pub fn store(&self) -> S {
        self.store.clone()
    }

    /// Set the handle to never cause ODB refreshes if an object could not be found.
    ///
    /// The latter is the default, as typically all objects referenced in a git repository are contained in the local clone.
    /// More recently, however, this doesn't always have to be the case due to sparse checkouts and other ways to only have a
    /// limited amount of objects available locally.
    pub fn refresh_never(&mut self) {
        self.refresh = RefreshMode::Never;
    }

    /// Return the current refresh mode.
    pub fn refresh_mode(&mut self) -> RefreshMode {
        self.refresh
    }
}

impl<S> Drop for super::Handle<S>
where
    S: Deref<Target = super::Store> + Clone,
{
    fn drop(&mut self) {
        if let Some(token) = self.token.take() {
            self.store.remove_handle(token);
        }
    }
}

impl TryFrom<&super::Store> for super::Store {
    type Error = std::io::Error;

    fn try_from(s: &super::Store) -> Result<Self, Self::Error> {
        let catalog = s.catalog.load();
        super::Store::at_opts(
            s.path().into(),
            s.object_hash,
            &mut s.replacements(),
            crate::store::init::Options {
                slots: crate::store::init::Slots::Limit(catalog.slots.len().try_into().expect("BUG: too many slots")),
                use_multi_pack_index: false,
                alloc_limit_bytes: s.alloc_limit_bytes,
                current_dir: s.current_dir.clone().into(),
                loose_compression: s.loose_compression,
                #[cfg(feature = "test-support")]
                debug: s.debug.clone(),
            },
        )
    }
}

impl super::Handle<Rc<super::Store>> {
    /// Convert a ref counted store into one that is ref-counted and thread-safe, by creating a new Store.
    pub fn into_arc(self) -> std::io::Result<super::Handle<Arc<super::Store>>> {
        let loose_compression = self.loose_compression;
        let store = Arc::new(super::Store::try_from(self.store_ref())?);
        let mut cache = store.to_handle_arc();
        cache.refresh = self.refresh;
        cache.max_recursion_depth = self.max_recursion_depth;
        cache.loose_compression = loose_compression;
        Ok(cache)
    }
}

impl super::Handle<Arc<super::Store>> {
    /// Convert a ref counted store into one that is ref-counted and thread-safe, by creating a new Store
    pub fn into_arc(self) -> std::io::Result<super::Handle<Arc<super::Store>>> {
        Ok(self)
    }
}

impl<S> Clone for super::Handle<S>
where
    S: Deref<Target = super::Store> + Clone,
{
    fn clone(&self) -> Self {
        super::Handle {
            store: self.store.clone(),
            refresh: self.refresh,
            ignore_replacements: self.ignore_replacements,
            loose_compression: self.loose_compression,
            token: {
                let token = self.store.register_handle();
                match self.token.as_ref().expect("token is always set here ") {
                    handle::Mode::DeletedPacksAreInaccessible => token,
                    handle::Mode::KeepDeletedPacksAvailable => self.store.upgrade_handle(token),
                }
                .into()
            },
            inflate: RefCell::new(Default::default()),
            snapshot: RefCell::new(self.store.collect_snapshot()),
            retained_indices: RefCell::new(self.retained_indices.borrow().clone()),
            max_recursion_depth: self.max_recursion_depth,
            packed_object_count: Default::default(),
        }
    }
}
