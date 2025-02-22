use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsStr,
    ops::Deref,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU16, Ordering},
        Arc,
    },
    time::SystemTime,
};

use crate::store::{handle, types, RefreshMode};

pub(crate) struct Snapshot {
    /// Indices ready for object lookup or contains checks, ordered usually by modification data, recent ones first.
    pub(crate) indices: Vec<handle::IndexLookup>,
    /// A set of loose objects dbs to search once packed objects weren't found.
    pub(crate) loose_dbs: Arc<Vec<crate::loose::Store>>,
    /// remember what this state represents and to compare to other states.
    pub(crate) marker: types::SlotIndexMarker,
}

mod error {
    use std::path::PathBuf;

    use gix_pack::multi_index::PackIndex;

    /// Returned by [`crate::at_opts()`]
    #[derive(thiserror::Error, Debug)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("The objects directory at '{0}' is not an accessible directory")]
        Inaccessible(PathBuf),
        #[error(transparent)]
        Io(#[from] std::io::Error),
        #[error(transparent)]
        Alternate(#[from] crate::alternate::Error),
        #[error("The slotmap turned out to be too small with {} entries, would need {} more", .current, .needed)]
        InsufficientSlots { current: usize, needed: usize },
        /// The problem here is that some logic assumes that more recent generations are higher than previous ones. If we would overflow,
        /// we would break that invariant which can lead to the wrong object from being returned. It would probably be super rare, but…
        /// let's not risk it.
        #[error(
            "Would have overflown amount of max possible generations of {}",
            super::Generation::MAX
        )]
        GenerationOverflow,
        #[error("Cannot numerically handle more than {limit} packs in a single multi-pack index, got {actual} in file {index_path:?}")]
        TooManyPacksInMultiIndex {
            actual: PackIndex,
            limit: PackIndex,
            index_path: PathBuf,
        },
    }
}

pub use error::Error;

use crate::store::types::{Generation, IndexAndPacks, MutableIndexAndPack, PackId, SlotMapIndex};

impl super::Store {
    /// Load all indices, refreshing from disk only if needed.
    pub(crate) fn load_all_indices(&self) -> Result<Snapshot, Error> {
        let mut snapshot = self.collect_snapshot();
        while let Some(new_snapshot) = self.load_one_index(RefreshMode::Never, snapshot.marker)? {
            snapshot = new_snapshot;
        }
        Ok(snapshot)
    }

    /// If `None` is returned, there is new indices and the caller should give up. This is a possibility even if it's allowed to refresh
    /// as here might be no change to pick up.
    pub(crate) fn load_one_index(
        &self,
        refresh_mode: RefreshMode,
        marker: types::SlotIndexMarker,
    ) -> Result<Option<Snapshot>, Error> {
        let index = self.index.load();
        if !index.is_initialized() {
            return self.consolidate_with_disk_state(true /* needs_init */, false /*load one new index*/);
        }

        if marker.generation != index.generation || marker.state_id != index.state_id() {
            // We have a more recent state already, provide it.
            Ok(Some(self.collect_snapshot()))
        } else {
            // always compare to the latest state
            // Nothing changed in the meantime, try to load another index…
            if self.load_next_index(index) {
                Ok(Some(self.collect_snapshot()))
            } else {
                // …and if that didn't yield anything new consider refreshing our disk state.
                match refresh_mode {
                    RefreshMode::Never => Ok(None),
                    RefreshMode::AfterAllIndicesLoaded => {
                        self.consolidate_with_disk_state(false /* needs init */, true /*load one new index*/)
                    }
                }
            }
        }
    }

    /// load a new index (if not yet loaded), and return true if one was indeed loaded (leading to a `state_id()` change) of the current index.
    /// Note that interacting with the slot-map is inherently racy and we have to deal with it, being conservative in what we even try to load
    /// as our index might already be out-of-date as we try to use it to learn what's next.
    fn load_next_index(&self, mut index: arc_swap::Guard<Arc<SlotMapIndex>>) -> bool {
        'retry_with_changed_index: loop {
            let previous_state_id = index.state_id();
            'retry_with_next_slot_index: loop {
                match index
                    .next_index_to_load
                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                        (current != index.slot_indices.len()).then_some(current + 1)
                    }) {
                    Ok(slot_map_index) => {
                        // This slot-map index is in bounds and was only given to us.
                        let _ongoing_operation = IncOnNewAndDecOnDrop::new(&index.num_indices_currently_being_loaded);
                        let slot = &self.files[index.slot_indices[slot_map_index]];
                        let _lock = slot.write.lock();
                        if slot.generation.load(Ordering::SeqCst) > index.generation {
                            // There is a disk consolidation in progress which just overwrote a slot that could be disposed with some other
                            // index, one we didn't intend to load.
                            // Continue with the next slot index in the hope there is something else we can do…
                            continue 'retry_with_next_slot_index;
                        }
                        let mut bundle = slot.files.load_full();
                        let bundle_mut = Arc::make_mut(&mut bundle);
                        if let Some(files) = bundle_mut.as_mut() {
                            // these are always expected to be set, unless somebody raced us. We handle this later by retrying.
                            let res = {
                                let res = files.load_index(self.object_hash);
                                slot.files.store(bundle);
                                index.loaded_indices.fetch_add(1, Ordering::SeqCst);
                                res
                            };
                            match res {
                                Ok(_) => {
                                    break 'retry_with_next_slot_index;
                                }
                                Err(_err) => {
                                    gix_features::trace::error!(err=?_err, "Failed to load index file - some objects may seem to not exist");
                                    continue 'retry_with_next_slot_index;
                                }
                            }
                        }
                    }
                    Err(_nothing_more_to_load) => {
                        // There can be contention as many threads start working at the same time and take all the
                        // slots to load indices for. Some threads might just be left-over and have to wait for something
                        // to change.
                        // TODO: potentially hot loop - could this be a condition variable?
                        // This is a timing-based fix for the case that the `num_indices_being_loaded` isn't yet incremented,
                        // and we might break out here without actually waiting for the loading operation. Then we'd fail to
                        // observe a change and the underlying handler would not have all the indices it needs at its disposal.
                        // Yielding means we will definitely loose enough time to observe the ongoing operation,
                        // or its effects.
                        std::thread::yield_now();
                        while index.num_indices_currently_being_loaded.load(Ordering::SeqCst) != 0 {
                            std::thread::yield_now();
                        }
                        break 'retry_with_next_slot_index;
                    }
                }
            }
            if previous_state_id == index.state_id() {
                let potentially_new_index = self.index.load();
                if Arc::as_ptr(&potentially_new_index) == Arc::as_ptr(&index) {
                    // There isn't a new index with which to retry the whole ordeal, so nothing could be done here.
                    return false;
                } else {
                    // the index changed, worth trying again
                    index = potentially_new_index;
                    continue 'retry_with_changed_index;
                }
            } else {
                // something inarguably changed, probably an index was loaded. 'probably' because we consider failed loads valid attempts,
                // even they don't change anything for the caller which would then do a round for nothing.
                return true;
            }
        }
    }

    /// refresh and possibly clear out our existing data structures, causing all pack ids to be invalidated.
    /// `load_new_index` is an optimization to at least provide one newly loaded pack after refreshing the slot map.
    pub(crate) fn consolidate_with_disk_state(
        &self,
        needs_init: bool,
        load_new_index: bool,
    ) -> Result<Option<Snapshot>, Error> {
        let index = self.index.load();
        let previous_index_state = Arc::as_ptr(&index) as usize;

        // IMPORTANT: get a lock after we recorded the previous state.
        let write = self.write.lock();
        let objects_directory = &self.path;

        // Now we know the index isn't going to change anymore, even though threads might still load indices in the meantime.
        let index = self.index.load();
        if previous_index_state != Arc::as_ptr(&index) as usize {
            // Someone else took the look before and changed the index. Return it without doing any additional work.
            return Ok(Some(self.collect_snapshot()));
        }

        let was_uninitialized = !index.is_initialized();

        // We might not be able to detect by pointer if the state changed, as this itself is racy. So we keep track of double-initialization
        // using a flag, which means that if `needs_init` was true we saw the index uninitialized once, but now that we are here it's
        // initialized meaning that somebody was faster, and we couldn't detect it by comparisons to the index.
        // If so, make sure we collect the snapshot instead of returning None in case nothing actually changed, which is likely with a
        // race like this.
        if !was_uninitialized && needs_init {
            return Ok(Some(self.collect_snapshot()));
        }
        self.num_disk_state_consolidation.fetch_add(1, Ordering::Relaxed);

        let db_paths: Vec<_> = std::iter::once(objects_directory.to_owned())
            .chain(crate::alternate::resolve(objects_directory.clone(), &self.current_dir)?)
            .collect();

        // turn db paths into loose object databases. Reuse what's there, but only if it is in the right order.
        let loose_dbs = if was_uninitialized
            || db_paths.len() != index.loose_dbs.len()
            || db_paths
                .iter()
                .zip(index.loose_dbs.iter().map(|ldb| &ldb.path))
                .any(|(lhs, rhs)| lhs != rhs)
        {
            Arc::new(
                db_paths
                    .iter()
                    .map(|path| crate::loose::Store::at(path, self.object_hash))
                    .collect::<Vec<_>>(),
            )
        } else {
            Arc::clone(&index.loose_dbs)
        };

        let indices_by_modification_time = Self::collect_indices_and_mtime_sorted_by_size(
            db_paths,
            index.slot_indices.len().into(),
            self.use_multi_pack_index.then_some(self.object_hash),
        )?;
        let mut idx_by_index_path: BTreeMap<_, _> = index
            .slot_indices
            .iter()
            .filter_map(|&idx| {
                let f = &self.files[idx];
                Option::as_ref(&f.files.load()).map(|f| (f.index_path().to_owned(), idx))
            })
            .collect();

        let mut new_slot_map_indices = Vec::new(); // these indices into the slot map still exist there/didn't change
        let mut index_paths_to_add = was_uninitialized
            .then(|| VecDeque::with_capacity(indices_by_modification_time.len()))
            .unwrap_or_default();

        // Figure out this number based on what we see while handling the existing indices
        let mut num_loaded_indices = 0;
        for (index_info, mtime) in indices_by_modification_time.into_iter().map(|(a, b, _)| (a, b)) {
            match idx_by_index_path.remove(index_info.path()) {
                Some(slot_idx) => {
                    let slot = &self.files[slot_idx];
                    let files_guard = slot.files.load();
                    let files =
                        Option::as_ref(&files_guard).expect("slot is set or we wouldn't know it points to this file");
                    if index_info.is_multi_index() && files.mtime() != mtime {
                        // we have a changed multi-pack index. We can't just change the existing slot as it may alter slot indices
                        // that are currently available. Instead, we have to move what's there into a new slot, along with the changes,
                        // and later free the slot or dispose of the index in the slot (like we do for removed/missing files).
                        index_paths_to_add.push_back((index_info, mtime, Some(slot_idx)));
                        // If the current slot is loaded, the soon-to-be copied multi-index path will be loaded as well.
                        if files.index_is_loaded() {
                            num_loaded_indices += 1;
                        }
                    } else {
                        // packs and indices are immutable, so no need to check modification times. Unchanged multi-pack indices also
                        // are handled like this just to be sure they are in the desired state. For these, the only way this could happen
                        // is if somebody deletes and then puts back
                        if Self::assure_slot_matches_index(&write, slot, index_info, mtime, index.generation) {
                            num_loaded_indices += 1;
                        }
                        new_slot_map_indices.push(slot_idx);
                    }
                }
                None => index_paths_to_add.push_back((index_info, mtime, None)),
            }
        }
        let needs_stable_indices = self.maintain_stable_indices(&write);

        let mut next_possibly_free_index = index
            .slot_indices
            .iter()
            .max()
            .map_or(0, |idx| (idx + 1) % self.files.len());
        let mut num_indices_checked = 0;
        let mut needs_generation_change = false;
        let mut slot_indices_to_remove: Vec<_> = idx_by_index_path.into_values().collect();
        while let Some((mut index_info, mtime, move_from_slot_idx)) = index_paths_to_add.pop_front() {
            'increment_slot_index: loop {
                if num_indices_checked == self.files.len() {
                    return Err(Error::InsufficientSlots {
                        current: self.files.len(),
                        needed: index_paths_to_add.len() + 1, /*the one currently popped off*/
                    });
                }
                // Don't allow duplicate indicates, we need a 1:1 mapping.
                if new_slot_map_indices.contains(&next_possibly_free_index) {
                    next_possibly_free_index = (next_possibly_free_index + 1) % self.files.len();
                    num_indices_checked += 1;
                    continue 'increment_slot_index;
                }
                let slot_index = next_possibly_free_index;
                let slot = &self.files[slot_index];
                next_possibly_free_index = (next_possibly_free_index + 1) % self.files.len();
                num_indices_checked += 1;
                match move_from_slot_idx {
                    Some(move_from_slot_idx) => {
                        debug_assert!(index_info.is_multi_index(), "only set for multi-pack indices");
                        if slot_index == move_from_slot_idx {
                            // don't try to move onto ourselves
                            continue 'increment_slot_index;
                        }
                        match Self::try_set_index_slot(
                            &write,
                            slot,
                            index_info,
                            mtime,
                            index.generation,
                            needs_stable_indices,
                        ) {
                            Ok(dest_was_empty) => {
                                slot_indices_to_remove.push(move_from_slot_idx);
                                new_slot_map_indices.push(slot_index);
                                // To avoid handling out the wrong pack (due to reassigned pack ids), declare this a new generation.
                                if !dest_was_empty {
                                    needs_generation_change = true;
                                }
                                break 'increment_slot_index;
                            }
                            Err(unused_index_info) => index_info = unused_index_info,
                        }
                    }
                    None => {
                        match Self::try_set_index_slot(
                            &write,
                            slot,
                            index_info,
                            mtime,
                            index.generation,
                            needs_stable_indices,
                        ) {
                            Ok(dest_was_empty) => {
                                new_slot_map_indices.push(slot_index);
                                if !dest_was_empty {
                                    needs_generation_change = true;
                                }
                                break 'increment_slot_index;
                            }
                            Err(unused_index_info) => index_info = unused_index_info,
                        }
                    }
                }
                // This isn't racy as it's only us who can change the Option::Some/None state of a slot.
            }
        }
        assert_eq!(
            index_paths_to_add.len(),
            0,
            "By this time we have assigned all new files to slots"
        );

        let generation = if needs_generation_change {
            index.generation.checked_add(1).ok_or(Error::GenerationOverflow)?
        } else {
            index.generation
        };
        let index_unchanged = index.slot_indices == new_slot_map_indices;
        if generation != index.generation {
            assert!(
                !index_unchanged,
                "if the generation changed, the slot index must have changed for sure"
            );
        }
        if !index_unchanged || loose_dbs != index.loose_dbs {
            let new_index = Arc::new(SlotMapIndex {
                slot_indices: new_slot_map_indices,
                loose_dbs,
                generation,
                // if there was a prior generation, some indices might already be loaded. But we deal with it by trying to load the next index then,
                // until we find one.
                next_index_to_load: index_unchanged
                    .then(|| Arc::clone(&index.next_index_to_load))
                    .unwrap_or_default(),
                loaded_indices: index_unchanged
                    .then(|| Arc::clone(&index.loaded_indices))
                    .unwrap_or_else(|| Arc::new(num_loaded_indices.into())),
                num_indices_currently_being_loaded: Default::default(),
            });
            self.index.store(new_index);
        }

        // deleted items - remove their slots AFTER we have set the new index if we may alter indices, otherwise we only declare them garbage.
        // removing slots may cause pack loading to fail, and they will then reload their indices.
        for slot in slot_indices_to_remove.into_iter().map(|idx| &self.files[idx]) {
            let _lock = slot.write.lock();
            let mut files = slot.files.load_full();
            let files_mut = Arc::make_mut(&mut files);
            if needs_stable_indices {
                if let Some(files) = files_mut.as_mut() {
                    files.trash();
                    // generation stays the same, as it's the same value still but scheduled for eventual removal.
                }
            } else {
                // set the generation before we actually change the value, otherwise readers of old generations could observe the new one.
                // We rather want them to turn around here and update their index, which, by that time, might actually already be available.
                // If not, they would fail unable to load a pack or index they need, but that's preferred over returning wrong objects.
                // Safety: can't race as we hold the lock, have to set the generation beforehand to help avoid others to observe the value.
                slot.generation.store(generation, Ordering::SeqCst);
                *files_mut = None;
            }
            slot.files.store(files);
        }

        let new_index = self.index.load();
        Ok(if index.state_id() == new_index.state_id() {
            // there was no change, and nothing was loaded in the meantime, reflect that in the return value to not get into loops.
            None
        } else {
            if load_new_index {
                self.load_next_index(new_index);
            }
            Some(self.collect_snapshot())
        })
    }

    pub(crate) fn collect_indices_and_mtime_sorted_by_size(
        db_paths: Vec<PathBuf>,
        initial_capacity: Option<usize>,
        multi_pack_index_object_hash: Option<gix_hash::Kind>,
    ) -> Result<Vec<(Either, SystemTime, u64)>, Error> {
        let mut indices_by_modification_time = Vec::with_capacity(initial_capacity.unwrap_or_default());
        for db_path in db_paths {
            let packs = db_path.join("pack");
            let entries = match std::fs::read_dir(packs) {
                Ok(e) => e,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
                Err(err) => return Err(err.into()),
            };
            let indices = entries
                .filter_map(Result::ok)
                .filter_map(|e| e.metadata().map(|md| (e.path(), md)).ok())
                .filter(|(_, md)| md.file_type().is_file())
                .filter(|(p, _)| {
                    let ext = p.extension();
                    (ext == Some(OsStr::new("idx")) && p.with_extension("pack").is_file())
                        || (multi_pack_index_object_hash.is_some() && ext.is_none() && is_multipack_index(p))
                })
                .map(|(p, md)| md.modified().map_err(Error::from).map(|mtime| (p, mtime, md.len())))
                .collect::<Result<Vec<_>, _>>()?;

            let multi_index_info = multi_pack_index_object_hash
                .and_then(|hash| {
                    indices.iter().find_map(|(p, a, b)| {
                        is_multipack_index(p)
                            .then(|| {
                                // we always open the multi-pack here to be able to remove indices
                                gix_pack::multi_index::File::at(p)
                                    .ok()
                                    .filter(|midx| midx.object_hash() == hash)
                                    .map(|midx| (midx, *a, *b))
                            })
                            .flatten()
                            .map(|t| {
                                if t.0.num_indices() > PackId::max_packs_in_multi_index() {
                                    Err(Error::TooManyPacksInMultiIndex {
                                        index_path: p.to_owned(),
                                        actual: t.0.num_indices(),
                                        limit: PackId::max_packs_in_multi_index(),
                                    })
                                } else {
                                    Ok(t)
                                }
                            })
                    })
                })
                .transpose()?;
            if let Some((multi_index, mtime, flen)) = multi_index_info {
                let index_names_in_multi_index: Vec<_> = multi_index.index_names().iter().map(AsRef::as_ref).collect();
                let mut indices_not_in_multi_index: Vec<(Either, _, _)> = indices
                    .into_iter()
                    .filter_map(|(path, a, b)| {
                        (path != multi_index.path()
                            && !index_names_in_multi_index
                                .contains(&Path::new(path.file_name().expect("file name present"))))
                        .then_some((Either::IndexPath(path), a, b))
                    })
                    .collect();
                indices_not_in_multi_index.insert(0, (Either::MultiIndexFile(Arc::new(multi_index)), mtime, flen));
                indices_by_modification_time.extend(indices_not_in_multi_index);
            } else {
                indices_by_modification_time.extend(
                    indices
                        .into_iter()
                        .filter_map(|(p, a, b)| (!is_multipack_index(&p)).then_some((Either::IndexPath(p), a, b))),
                );
            }
        }
        // Unlike libgit2, do not sort by modification date, but by size and put the biggest indices first. That way
        // the chance to hit an object should be higher. We leave it to the handle to sort by LRU.
        // Git itself doesn't change the order which may save time, but we want it to be stable which also helps some tests.
        // NOTE: this will work well for well-packed repos or those using geometric repacking, but force us to open a lot
        //       of files when dealing with new objects, as there is no notion of recency here as would be with unmaintained
        //       repositories. Different algorithms should be provided, like newest packs first, and possibly a mix of both
        //       with big packs first, then sorting by recency for smaller packs.
        //       We also want to implement `fetch.unpackLimit` to alleviate this issue a little.
        indices_by_modification_time.sort_by(|l, r| l.2.cmp(&r.2).reverse());
        Ok(indices_by_modification_time)
    }

    /// returns `Ok(dest_slot_was_empty)` if the copy could happen because dest-slot was actually free or disposable.
    #[allow(clippy::too_many_arguments)]
    fn try_set_index_slot(
        lock: &parking_lot::MutexGuard<'_, ()>,
        dest_slot: &MutableIndexAndPack,
        index_info: Either,
        mtime: SystemTime,
        current_generation: Generation,
        needs_stable_indices: bool,
    ) -> Result<bool, Either> {
        let (dest_slot_was_empty, generation) = match &**dest_slot.files.load() {
            Some(bundle) => {
                if bundle.index_path() == index_info.path() || (bundle.is_disposable() && needs_stable_indices) {
                    // it might be possible to see ourselves in case all slots are taken, but there are still a few more destination
                    // slots to look for.
                    return Err(index_info);
                }
                // Since we overwrite an existing slot, we have to increment the generation to prevent anyone from trying to use it while
                // before we are replacing it with a different value.
                // In detail:
                // We need to declare this to be the future to avoid anything in that slot to be returned to people who
                // last saw the old state. They will then try to get a new index which by that time, might be happening
                // in time so they get the latest one. If not, they will probably get into the same situation again until
                // it finally succeeds. Alternatively, the object will be reported unobtainable, but at least it won't return
                // some other object.
                (false, current_generation + 1)
            }
            None => {
                // For multi-pack indices:
                //   Do NOT copy the packs over, they need to be reopened to get the correct pack id matching the new slot map index.
                //   If we are allowed to delete the original, and nobody has the pack referenced, it is closed which is preferred.
                //   Thus we simply always start new with packs in multi-pack indices.
                //   In the worst case this could mean duplicate file handle usage though as the old and the new index can't share
                //   packs due to the intrinsic id.
                //   Note that the ID is used for cache access, too, so it must be unique. It must also be mappable from pack-id to slotmap id.
                (true, current_generation)
            }
        };
        Self::set_slot_to_index(lock, dest_slot, index_info, mtime, generation);
        Ok(dest_slot_was_empty)
    }

    fn set_slot_to_index(
        _lock: &parking_lot::MutexGuard<'_, ()>,
        slot: &MutableIndexAndPack,
        index_info: Either,
        mtime: SystemTime,
        generation: Generation,
    ) {
        let _lock = slot.write.lock();
        let mut files = slot.files.load_full();
        let files_mut = Arc::make_mut(&mut files);
        // set the generation before we actually change the value, otherwise readers of old generations could observe the new one.
        // We rather want them to turn around here and update their index, which, by that time, might actually already be available.
        // If not, they would fail unable to load a pack or index they need, but that's preferred over returning wrong objects.
        // Safety: can't race as we hold the lock, have to set the generation beforehand to help avoid others to observe the value.
        slot.generation.store(generation, Ordering::SeqCst);
        *files_mut = Some(index_info.into_index_and_packs(mtime));
        slot.files.store(files);
    }

    /// Returns true if the index was left in a loaded state.
    fn assure_slot_matches_index(
        _lock: &parking_lot::MutexGuard<'_, ()>,
        slot: &MutableIndexAndPack,
        index_info: Either,
        mtime: SystemTime,
        current_generation: Generation,
    ) -> bool {
        match Option::as_ref(&slot.files.load()) {
            Some(bundle) => {
                assert_eq!(
                    bundle.index_path(),
                    index_info.path(),
                    "Parallel writers cannot change the file the slot points to."
                );
                if bundle.is_disposable() {
                    // put it into the correct mode, it's now available for sure so should not be missing or garbage.
                    // The latter can happen if files are removed and put back for some reason, but we should definitely
                    // have them in a decent state now that we know/think they are there.
                    let _lock = slot.write.lock();
                    let mut files = slot.files.load_full();
                    let files_mut = Arc::make_mut(&mut files)
                        .as_mut()
                        .expect("BUG: cannot change from something to nothing, would be race");
                    files_mut.put_back();
                    debug_assert_eq!(
                        files_mut.mtime(),
                        mtime,
                        "BUG: we can only put back files that didn't obviously change"
                    );
                    // Safety: can't race as we hold the lock, must be set before replacing the data.
                    // NOTE that we don't change the generation as it's still the very same index we talk about, it doesn't change
                    // identity.
                    slot.generation.store(current_generation, Ordering::SeqCst);
                    slot.files.store(files);
                } else {
                    // it's already in the correct state, either loaded or unloaded.
                }
                bundle.index_is_loaded()
            }
            None => {
                unreachable!("BUG: a slot can never be deleted if we have it recorded in the index WHILE changing said index. There shouldn't be a race")
            }
        }
    }

    /// Stability means that indices returned by this API will remain valid.
    /// Without that constraint, we may unload unused packs and indices, and may rebuild the slotmap index.
    ///
    /// Note that this must be called with a lock to the relevant state held to assure these values don't change while
    /// we are working on said index.
    fn maintain_stable_indices(&self, _guard: &parking_lot::MutexGuard<'_, ()>) -> bool {
        self.num_handles_stable.load(Ordering::SeqCst) > 0
    }

    pub(crate) fn collect_snapshot(&self) -> Snapshot {
        // We don't observe changes-on-disk in our 'wait-for-load' loop.
        // That loop is meant to help assure the marker (which includes the amount of loaded indices) matches
        // the actual amount of indices we collect.
        let index = self.index.load();
        loop {
            if index.num_indices_currently_being_loaded.deref().load(Ordering::SeqCst) != 0 {
                std::thread::yield_now();
                continue;
            }
            let marker = index.marker();
            let indices = if index.is_initialized() {
                index
                    .slot_indices
                    .iter()
                    .map(|idx| (*idx, &self.files[*idx]))
                    .filter_map(|(id, file)| {
                        let lookup = match (**file.files.load()).as_ref()? {
                            types::IndexAndPacks::Index(bundle) => handle::SingleOrMultiIndex::Single {
                                index: bundle.index.loaded()?.clone(),
                                data: bundle.data.loaded().cloned(),
                            },
                            types::IndexAndPacks::MultiIndex(multi) => handle::SingleOrMultiIndex::Multi {
                                index: multi.multi_index.loaded()?.clone(),
                                data: multi.data.iter().map(|f| f.loaded().cloned()).collect(),
                            },
                        };
                        handle::IndexLookup { file: lookup, id }.into()
                    })
                    .collect()
            } else {
                Vec::new()
            };

            return Snapshot {
                indices,
                loose_dbs: Arc::clone(&index.loose_dbs),
                marker,
            };
        }
    }
}

// Outside of this method we will never assign new slot indices.
fn is_multipack_index(path: &Path) -> bool {
    path.file_name() == Some(OsStr::new("multi-pack-index"))
}

struct IncOnNewAndDecOnDrop<'a>(&'a AtomicU16);
impl<'a> IncOnNewAndDecOnDrop<'a> {
    pub fn new(v: &'a AtomicU16) -> Self {
        v.fetch_add(1, Ordering::SeqCst);
        Self(v)
    }
}
impl Drop for IncOnNewAndDecOnDrop<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

pub(crate) enum Either {
    IndexPath(PathBuf),
    MultiIndexFile(Arc<gix_pack::multi_index::File>),
}

impl Either {
    fn path(&self) -> &Path {
        match self {
            Either::IndexPath(p) => p,
            Either::MultiIndexFile(f) => f.path(),
        }
    }

    fn into_index_and_packs(self, mtime: SystemTime) -> IndexAndPacks {
        match self {
            Either::IndexPath(path) => IndexAndPacks::new_single(path, mtime),
            Either::MultiIndexFile(file) => IndexAndPacks::new_multi_from_open_file(file, mtime),
        }
    }

    fn is_multi_index(&self) -> bool {
        matches!(self, Either::MultiIndexFile(_))
    }
}

impl Eq for Either {}

impl PartialEq<Self> for Either {
    fn eq(&self, other: &Self) -> bool {
        self.path().eq(other.path())
    }
}

impl PartialOrd<Self> for Either {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.path().cmp(other.path()))
    }
}

impl Ord for Either {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.path().cmp(other.path())
    }
}
