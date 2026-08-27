#![no_main]

use std::hint::black_box;

use gix_reftable::{Limits, SnapshotOptions, Stack};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(root) = tempfile::tempdir() else {
        return;
    };
    let directory = root.path().join("reftable");
    if std::fs::create_dir(&directory).is_err() || std::fs::write(directory.join("tables.list"), data).is_err() {
        return;
    }

    let object_hash = if data.len() % 2 == 0 {
        gix_hash::Kind::Sha1
    } else {
        gix_hash::Kind::Sha256
    };
    let snapshot_options = SnapshotOptions {
        max_attempts: 2,
        max_list_size: 64 * 1024,
        max_total_table_size: 1024 * 1024,
        max_total_records: 64 * 1024,
    };
    let limits = Limits {
        max_file_size: 1024 * 1024,
        max_block_size: 1024 * 1024,
        max_value_size: 256 * 1024,
        max_records: 64 * 1024,
    };
    _ = black_box(Stack::open(directory, object_hash, snapshot_options, limits));
});
