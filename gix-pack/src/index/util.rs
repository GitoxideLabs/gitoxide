use std::time::Instant;

use gix_features::progress::{self, Progress};

use crate::exact_vec;

pub(crate) fn index_entries_sorted_by_offset_ascending(
    idx: &crate::index::File<impl crate::FileData>,
    progress: &mut dyn Progress,
) -> Vec<crate::index::Entry> {
    progress.init(Some(idx.num_objects as usize), progress::count("entries"));
    let start = Instant::now();

    let mut v = exact_vec(idx.num_objects as usize);
    for entry in idx.iter() {
        v.push(entry);
        progress.inc();
    }
    v.sort_by_key(|e| e.pack_offset);

    progress.show_throughput(start);
    v
}
