use std::sync::atomic::Ordering;

use crate::store::{types, types::IndexAndPacks};

impl super::Store {
    /// Return metrics collected in a racy fashion, giving an idea of what's currently going on in the store.
    ///
    /// Use this to decide whether a new instance should be created to get a chance at dropping all open handles.
    pub fn metrics(&self) -> types::Metrics {
        let mut open_packs = 0;
        let mut open_indices = 0;
        let mut known_packs = 0;
        let mut known_indices = 0;
        let mut unused_slots = 0;

        let catalog = self.catalog.load();
        let index = &catalog.index;
        let slots = &catalog.slots;
        for f in index.slot_indices.iter().map(|idx| &slots[*idx]) {
            match &**f.files.load() {
                Some(IndexAndPacks::Index(bundle)) => {
                    if bundle.index.is_loaded() {
                        open_indices += 1;
                    }
                    known_indices += 1;
                    if bundle.data.is_loaded() {
                        open_packs += 1;
                    }
                    known_packs += 1;
                }
                Some(IndexAndPacks::MultiIndex(multi)) => {
                    if multi.multi_index.is_loaded() {
                        open_indices += 1;
                    }
                    known_indices += 1;
                    for pack in &multi.data {
                        if pack.is_loaded() {
                            open_packs += 1;
                        }
                        known_packs += 1;
                    }
                }
                None => {}
            }
        }

        for slot in slots.iter() {
            if slot.files.load().is_none() {
                unused_slots += 1;
            }
        }

        types::Metrics {
            num_handles: self.num_handles.load(Ordering::Relaxed),
            num_refreshes: self.num_disk_state_consolidation.load(Ordering::Relaxed),
            open_reachable_packs: open_packs,
            open_reachable_indices: open_indices,
            known_reachable_indices: known_indices,
            known_packs,
            unused_slots,
            loose_dbs: index.loose_dbs.len(),
            unreachable_indices: 0,
            unreachable_packs: 0,
        }
    }
}
