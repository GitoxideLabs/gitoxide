#![no_main]

use anyhow::Result;
use gix_features::{progress, zlib};
use gix_pack::{cache, data};
use gix_pack_fuzz::{interrupt_flag, virtual_path};
use libfuzzer_sys::fuzz_target;
use std::hint::black_box;

const ALLOC_LIMIT_BYTES: usize = 64 * 1024 * 1024;

fn fuzz(input: &[u8]) -> Result<()> {
    let pack = match data::File::from_data(input, virtual_path(".pack"), gix_hash::Kind::Sha1) {
        Ok(pack) => pack,
        Err(err) => {
            _ = black_box(err);
            return Ok(());
        }
    }
    .with_alloc_limit_bytes(Some(ALLOC_LIMIT_BYTES));

    _ = black_box(pack.version());
    _ = black_box(pack.path());
    _ = black_box(pack.num_objects());
    _ = black_box(pack.object_hash());
    _ = black_box(pack.checksum());
    _ = black_box(pack.verify_checksum(&mut progress::Discard, &interrupt_flag()));

    for offset in interesting_offsets(input) {
        let entry = match pack.entry(offset) {
            Ok(entry) => entry,
            Err(err) => {
                _ = black_box(err);
                continue;
            }
        };

        let mut out = Vec::new();
        let mut inflate = zlib::Inflate::default();
        let mut cache = cache::Never;
        _ = black_box(pack.decode_entry(entry, &mut out, &mut inflate, &|_, _| None, &mut cache));
    }

    Ok(())
}

fn interesting_offsets(input: &[u8]) -> Vec<u64> {
    let len = input.len() as u64;
    if len == 0 {
        return Vec::new();
    }

    let mut offsets = vec![0, 12.min(len - 1), (len - 1) / 2];
    if input.len() >= 8 {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&input[..8]);
        offsets.push(u64::from_le_bytes(bytes) % len);
    }
    offsets.sort_unstable();
    offsets.dedup();
    offsets
}

fuzz_target!(|input: &[u8]| {
    _ = black_box(fuzz(input));
});
