#![no_main]

use std::hint::black_box;

use gix_reftable::{Limits, Table};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let limits = Limits {
        max_file_size: 1024 * 1024,
        max_block_size: 1024 * 1024,
        max_value_size: 256 * 1024,
        max_records: 64 * 1024,
    };
    if let Ok(table) = Table::from_bytes(data, limits) {
        _ = black_box(table.header());
        _ = black_box(table.refs().count());
        _ = black_box(table.logs().count());
    }
});
