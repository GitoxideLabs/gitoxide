use std::{
    path::Path,
    sync::{Arc, atomic::Ordering},
};

use crate::store::types;

impl super::Store {
    /// If Ok(None) is returned, the resource node was stale or its pack couldn't be loaded because it no longer existed.
    /// If the oid is known, just load indices again to continue
    /// (objects rarely ever removed so should be present, maybe in another pack though),
    /// and redo the entire lookup with a current resource node whose pack can probably be loaded next time.
    pub(crate) fn load_pack(
        &self,
        slot: &types::MutableIndexAndPack,
        _slot_id: usize,
        pack_index: Option<gix_pack::multi_index::PackIndex>,
        expected_index: &super::handle::IntraPackLookup<'_>,
        marker: types::SlotIndexMarker,
    ) -> std::io::Result<Option<Arc<gix_pack::data::File>>> {
        let catalog = self.catalog.load();
        let index = &catalog.index;
        if index.generation != marker.generation {
            return Ok(None);
        }
        fn load_pack(
            path: &Path,
            object_hash: gix_hash::Kind,
            alloc_limit_bytes: Option<usize>,
        ) -> std::io::Result<Arc<gix_pack::data::File>> {
            gix_pack::data::File::at(path, object_hash)
                .map(|pack| pack.with_alloc_limit_bytes(alloc_limit_bytes))
                .map(Arc::new)
                .map_err(|err| match err {
                    gix_pack::data::header::decode::Error::Io { source, .. } => source,
                    other => std::io::Error::other(other),
                })
        }

        // pin the current state before loading in the generation. That way we won't risk seeing the wrong value later.
        let slot_files = &**slot.files.load();
        if slot.generation.load(Ordering::SeqCst) > marker.generation {
            // There is a disk consolidation in progress which just overwrote a slot that could be disposed with some other
            // pack, one we didn't intend to load.
            // Hope that when the caller returns/retries the new index is set so they can fetch it and retry.
            return Ok(None);
        }
        if !slot_files.as_ref().is_some_and(|files| expected_index.matches(files)) {
            return Ok(None);
        }
        match pack_index {
            None => {
                match slot_files {
                    Some(types::IndexAndPacks::Index(bundle)) => {
                        match bundle.data.loaded() {
                            Some(pack) => Ok(Some(pack.clone())),
                            None => {
                                #[cfg(feature = "test-support")]
                                self.debug(crate::store::init::debug::Point::PackSlotLocking {
                                    slot: _slot_id,
                                    pack_index: None,
                                });
                                let _lock = slot.write.lock();
                                #[cfg(feature = "test-support")]
                                self.debug(crate::store::init::debug::Point::PackSlotLocked {
                                    slot: _slot_id,
                                    pack_index: None,
                                });
                                let mut files = slot.files.load_full();
                                let files_mut = Arc::make_mut(&mut files);
                                if !files_mut.as_ref().is_some_and(|files| expected_index.matches(files)) {
                                    return Ok(None);
                                }
                                let pack = match files_mut {
                                    Some(types::IndexAndPacks::Index(bundle)) => {
                                        bundle.data.load_with_recovery(|path| {
                                            let res = load_pack(path, self.object_hash, self.alloc_limit_bytes);
                                            #[cfg(feature = "test-support")]
                                            self.debug(crate::store::init::debug::Point::PackLoadCompleted {
                                                slot: _slot_id,
                                                pack_index: None,
                                                outcome: if res.is_ok() {
                                                    crate::store::init::debug::LoadOutcome::Success
                                                } else {
                                                    crate::store::init::debug::LoadOutcome::Failure
                                                },
                                            });
                                            res
                                        })
                                    }
                                    Some(types::IndexAndPacks::MultiIndex(_)) => {
                                        // something changed between us getting the lock, trigger a complete index refresh.
                                        Ok(None)
                                    }
                                    None => Ok(None),
                                };
                                slot.files.store(files);
                                pack
                            }
                        }
                    }
                    // This can also happen if they use an old index into our new and refreshed data which might have a multi-index
                    // here.
                    Some(types::IndexAndPacks::MultiIndex(_)) => Ok(None),
                    None => Ok(None),
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
                                    #[cfg(feature = "test-support")]
                                    self.debug(crate::store::init::debug::Point::PackSlotLocking {
                                        slot: _slot_id,
                                        pack_index: Some(pack_index),
                                    });
                                    let _lock = slot.write.lock();
                                    #[cfg(feature = "test-support")]
                                    self.debug(crate::store::init::debug::Point::PackSlotLocked {
                                        slot: _slot_id,
                                        pack_index: Some(pack_index),
                                    });
                                    let mut files = slot.files.load_full();
                                    let files_mut = Arc::make_mut(&mut files);
                                    if !files_mut.as_ref().is_some_and(|files| expected_index.matches(files)) {
                                        return Ok(None);
                                    }
                                    let pack = match files_mut {
                                        Some(types::IndexAndPacks::Index(_)) => {
                                            // something changed between us getting the lock, trigger a complete index refresh.
                                            Ok(None)
                                        }
                                        Some(types::IndexAndPacks::MultiIndex(bundle)) => bundle
                                            .data
                                            .get_mut(pack_index as usize)
                                            .expect("pack index came from this multi-pack index")
                                            .load_with_recovery(|path| {
                                                let res = load_pack(path, self.object_hash, self.alloc_limit_bytes);
                                                #[cfg(feature = "test-support")]
                                                self.debug(crate::store::init::debug::Point::PackLoadCompleted {
                                                    slot: _slot_id,
                                                    pack_index: Some(pack_index),
                                                    outcome: if res.is_ok() {
                                                        crate::store::init::debug::LoadOutcome::Success
                                                    } else {
                                                        crate::store::init::debug::LoadOutcome::Failure
                                                    },
                                                });
                                                res
                                            }),
                                        None => Ok(None),
                                    };
                                    slot.files.store(files);
                                    pack
                                }
                            },
                        }
                    }
                    // This can also happen if they use an old index into our new and refreshed data which might have a multi-index
                    // here.
                    Some(types::IndexAndPacks::Index(_)) => Ok(None),
                    None => Ok(None),
                }
            }
        }
    }
}
