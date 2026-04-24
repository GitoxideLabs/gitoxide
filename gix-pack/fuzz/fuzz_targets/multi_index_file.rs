#![no_main]

use anyhow::Result;
use gix_features::progress;
use gix_hash::Prefix;
use gix_pack::multi_index;
use gix_pack_fuzz::{empty_candidates, interrupt_flag, virtual_path};
use libfuzzer_sys::fuzz_target;
use std::hint::black_box;

const ALLOC_LIMIT_BYTES: usize = 64 * 1024 * 1024;

fn fuzz(input: &[u8]) -> Result<()> {
    let index = match multi_index::File::from_data(input, virtual_path(".midx"), Some(ALLOC_LIMIT_BYTES)) {
        Ok(index) => index,
        Err(err) => {
            _ = black_box(err);
            return Ok(());
        }
    };

    _ = black_box(index.version());
    _ = black_box(index.path());
    _ = black_box(index.num_indices());
    _ = black_box(index.num_objects());
    _ = black_box(index.object_hash());
    _ = black_box(index.index_names());
    _ = black_box(index.checksum());
    _ = black_box(index.verify_checksum(&mut progress::Discard, &interrupt_flag()));
    _ = black_box(index.iter().take(8).count());

    if index.num_objects() > 0 {
        let first = index.oid_at_index(0).to_owned();
        _ = black_box(index.pack_id_and_pack_offset_at_index(0));
        _ = black_box(index.lookup(&first));

        if let Ok(prefix) = Prefix::new(first.as_ref(), 7) {
            let mut candidates = empty_candidates();
            _ = black_box(index.lookup_prefix(prefix, Some(&mut candidates)));
            _ = black_box(candidates);
        }
    }

    Ok(())
}

fuzz_target!(|input: &[u8]| {
    _ = black_box(fuzz(input));
});
