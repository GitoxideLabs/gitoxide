use std::{
    path::Path,
    sync::{Arc, atomic::Ordering},
};

use crate::store::{handle, types};

impl super::Store {
    /// If Ok(None) is returned, the pack-id was stale and referred to an unloaded pack or a pack which couldn't be
    /// loaded as its file didn't exist on disk anymore.
    /// If the oid is known, just load indices again to continue
    /// (objects rarely ever removed so should be present, maybe in another pack though),
    /// and redo the entire lookup for a valid pack id whose pack can probably be loaded next time.
    pub(crate) fn load_pack(
        &self,
        id: types::PackId,
        marker: types::SlotIndexMarker,
    ) -> std::io::Result<Option<Arc<gix_pack::data::File>>> {
        let index = self.index.load();
        if index.generation != marker.generation {
            return Ok(None);
        }
        fn load_pack(
            path: &Path,
            id: types::PackId,
            object_hash: gix_hash::Kind,
            alloc_limit_bytes: Option<usize>,
        ) -> std::io::Result<Arc<gix_pack::data::File>> {
            gix_pack::data::File::at(path, object_hash)
                .map(|pack| pack.with_alloc_limit_bytes(alloc_limit_bytes))
                .map(|mut pack| {
                    pack.id = id.to_intrinsic_pack_id();
                    Arc::new(pack)
                })
                .map_err(|err| match err {
                    gix_pack::data::header::decode::Error::Io { source, .. } => source,
                    other => std::io::Error::other(other),
                })
        }

        let slot = &self.files[id.index];
        // pin the current state before loading in the generation. That way we won't risk seeing the wrong value later.
        let slot_files = &**slot.files.load();
        if slot.generation.load(Ordering::SeqCst) > marker.generation {
            // There is a disk consolidation in progress which just overwrote a slot that could be disposed with some other
            // pack, one we didn't intend to load.
            // Hope that when the caller returns/retries the new index is set so they can fetch it and retry.
            return Ok(None);
        }
        match id.multipack_index {
            None => {
                match slot_files {
                    Some(types::IndexAndPacks::Index(bundle)) => {
                        match bundle.data.loaded() {
                            Some(pack) => Ok(Some(pack.clone())),
                            None => {
                                let _lock = slot.write.lock();
                                let mut files = slot.files.load_full();
                                let files_mut = Arc::make_mut(&mut files);
                                let pack = match files_mut {
                                    Some(types::IndexAndPacks::Index(bundle)) => {
                                        bundle.data.load_with_recovery(|path| {
                                            load_pack(path, id, self.object_hash, self.alloc_limit_bytes)
                                        })?
                                    }
                                    Some(types::IndexAndPacks::MultiIndex(_)) => {
                                        // something changed between us getting the lock, trigger a complete index refresh.
                                        None
                                    }
                                    None => {
                                        unreachable!(
                                            "BUG: must set this handle to be stable to avoid slots to be cleared/changed"
                                        )
                                    }
                                };
                                slot.files.store(files);
                                Ok(pack)
                            }
                        }
                    }
                    // This can also happen if they use an old index into our new and refreshed data which might have a multi-index
                    // here.
                    Some(types::IndexAndPacks::MultiIndex(_)) => Ok(None),
                    None => {
                        unreachable!("BUG: must set this handle to be stable to avoid slots to be cleared/changed")
                    }
                }
            }
            Some(pack_index) => {
                match slot_files {
                    Some(types::IndexAndPacks::MultiIndex(bundle)) => {
                        match bundle.data.get(pack_index as usize) {
                            None => Ok(None), // somewhat unexpected, data must be stale
                            Some(on_disk_pack) => match on_disk_pack.loaded() {
                                Some(pack) => Ok(Some(pack.clone())),
                                None => {
                                    let _lock = slot.write.lock();
                                    let mut files = slot.files.load_full();
                                    let files_mut = Arc::make_mut(&mut files);
                                    let pack = match files_mut {
                                        Some(types::IndexAndPacks::Index(_)) => {
                                            // something changed between us getting the lock, trigger a complete index refresh.
                                            None
                                        }
                                        Some(types::IndexAndPacks::MultiIndex(bundle)) => bundle
                                            .data
                                            .get_mut(pack_index as usize)
                                            .expect("BUG: must set this handle to be stable")
                                            .load_with_recovery(|path| {
                                                load_pack(path, id, self.object_hash, self.alloc_limit_bytes)
                                            })?,
                                        None => {
                                            unreachable!(
                                                "BUG: must set this handle to be stable to avoid slots to be cleared/changed"
                                            )
                                        }
                                    };
                                    slot.files.store(files);
                                    Ok(pack)
                                }
                            },
                        }
                    }
                    // This can also happen if they use an old index into our new and refreshed data which might have a multi-index
                    // here.
                    Some(types::IndexAndPacks::Index(_)) => Ok(None),
                    None => {
                        unreachable!("BUG: must set this handle to be stable to avoid slots to be cleared/changed")
                    }
                }
            }
        }
    }

    /// Similar to `.load_pack()`, but for entire indices, bypassing the index entirely and going solely by marker and id.
    /// Returns `None` if the index wasn't available anymore or could otherwise not be loaded, which can be considered a bug
    /// as we should always keep needed indices available.
    pub(crate) fn index_by_id(&self, id: types::PackId, marker: types::SlotIndexMarker) -> Option<handle::IndexLookup> {
        let slot = self.files.get(id.index)?;
        // Pin this value before we check the generation to avoid seeing something newer later.
        let slot_files = &**slot.files.load();
        if slot.generation.load(Ordering::SeqCst) > marker.generation {
            // This means somebody just overwrote our trashed slot with a new (or about to be stored) index, which means the slot isn't
            // what we need it to be.
            // This shouldn't as we won't overwrite slots while handles need stable indices.
            return None;
        }
        let lookup = match (slot_files).as_ref()? {
            types::IndexAndPacks::Index(bundle) => handle::SingleOrMultiIndex::Single {
                index: bundle.index.loaded()?.clone(),
                data: bundle.data.loaded().cloned(),
            },
            types::IndexAndPacks::MultiIndex(multi) => handle::SingleOrMultiIndex::Multi {
                index: multi.multi_index.loaded()?.clone(),
                data: multi.data.iter().map(|f| f.loaded().cloned()).collect(),
            },
        };
        handle::IndexLookup {
            id: id.index,
            file: lookup,
        }
        .into()
    }
}
