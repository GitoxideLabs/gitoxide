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
        let mut unreachable_indices = 0;
        let mut unreachable_packs = 0;

        let index = self.index.load();
        let slots = self.files.load();
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
            match slot.files.load().as_ref() {
                None => {
                    unused_slots += 1;
                }
                Some(bundle) => {
                    if bundle.is_disposable() {
                        unreachable_indices += 1;
                        unreachable_packs += match bundle {
                            IndexAndPacks::Index(single) => usize::from(single.data.is_loaded()),
                            IndexAndPacks::MultiIndex(multi) => {
                                multi.data.iter().map(|p| usize::from(p.is_loaded())).sum()
                            }
                        }
                    }
                }
            }
        }

        types::Metrics {
            num_handles: self.num_handles_unstable.load(Ordering::Relaxed)
                + self.num_handles_stable.load(Ordering::Relaxed),
            num_refreshes: self.num_disk_state_consolidation.load(Ordering::Relaxed),
            open_reachable_packs: open_packs,
            open_reachable_indices: open_indices,
            known_reachable_indices: known_indices,
            known_packs,
            unused_slots,
            loose_dbs: index.loose_dbs.len(),
            unreachable_indices,
            unreachable_packs,
        }
    }
}
