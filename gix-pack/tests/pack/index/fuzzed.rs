use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    path::PathBuf,
};

#[test]
fn artifact_inputs_can_be_opened_without_panicking() {
    for path in crate::pack::fuzz_artifact_paths("index_file") {
        _ = gix_pack::index::File::from_data(
            std::fs::read(&path).expect("artifact is readable"),
            path,
            gix_hash::Kind::Sha1,
        );
    }
}

/// Reproducer for the large-offset fuzz case: malformed V2 indices must not panic while
/// dereferencing a 64-bit pack offset that is missing from the file.
#[test]
fn truncated_large_offset_table_is_reported_without_panicking() {
    let result = catch_unwind(AssertUnwindSafe(|| {
        match gix_pack::index::File::from_data(
            malformed_v2_index_with_missing_large_offset_table(),
            PathBuf::from("fuzzed.idx"),
            gix_hash::Kind::Sha1,
        ) {
            Ok(index) => {
                let _ = index.iter().take(1).count();
            }
            Err(err) => {
                let _ = err;
            }
        }
    }));

    assert!(result.is_ok(), "malformed pack indices must not panic");
}

fn malformed_v2_index_with_missing_large_offset_table() -> Vec<u8> {
    let mut data = Vec::with_capacity(1064);
    data.extend_from_slice(b"\xfftOc");
    data.extend_from_slice(&2u32.to_be_bytes());
    for fan_idx in 0..256 {
        data.extend_from_slice(&u32::from(fan_idx == 255).to_be_bytes());
    }
    data.extend_from_slice(&[0; 20]);
    data.extend_from_slice(&0u32.to_be_bytes());
    data.extend_from_slice(&(1u32 << 31).to_be_bytes());
    data.extend_from_slice(&[0; 4]);
    debug_assert_eq!(data.len(), 1064);
    data
}

/// Reproducer for the out-of-range large-offset fuzz case: malformed V2 indices must not panic if
/// an offset entry points past the available 64-bit offset table.
#[test]
fn invalid_large_offset_index_is_reported_without_panicking() {
    let result = catch_unwind(AssertUnwindSafe(|| {
        match gix_pack::index::File::from_data(
            malformed_v2_index_with_out_of_range_large_offset_index(),
            PathBuf::from("fuzzed-large-offset.idx"),
            gix_hash::Kind::Sha1,
        ) {
            Ok(index) => {
                let _ = index.iter().take(1).count();
            }
            Err(err) => {
                let _ = err;
            }
        }
    }));

    assert!(result.is_ok(), "malformed pack indices must not panic");
}

fn malformed_v2_index_with_out_of_range_large_offset_index() -> Vec<u8> {
    let mut data = Vec::with_capacity(1108);
    data.extend_from_slice(b"\xfftOc");
    data.extend_from_slice(&2u32.to_be_bytes());
    for fan_idx in 0..256 {
        data.extend_from_slice(&u32::from(fan_idx == 255).to_be_bytes());
    }
    data.extend_from_slice(&[0; 20]);
    data.extend_from_slice(&0u32.to_be_bytes());
    data.extend_from_slice(&((1u32 << 31) | 0b10_1010).to_be_bytes());
    data.extend_from_slice(&0u64.to_be_bytes());
    data.extend_from_slice(&[0; 40]);
    debug_assert_eq!(data.len(), 1108);
    data
}

/// Reproducer for the truncated V1 index fuzz case: malformed indices whose fan-out table advertises
/// more objects than fit into the file must be rejected during initialization instead of panicking
/// later during lookup.
#[test]
fn truncated_v1_index_is_reported_without_panicking() {
    let result = catch_unwind(AssertUnwindSafe(|| {
        match gix_pack::index::File::from_data(
            malformed_v1_index_with_truncated_entry_table(),
            PathBuf::from("fuzzed-v1-truncated.idx"),
            gix_hash::Kind::Sha1,
        ) {
            Ok(index) => {
                let _ = index.lookup(gix_hash::Kind::Sha1.null());
            }
            Err(err) => {
                let _ = err;
            }
        }
    }));

    assert!(result.is_ok(), "malformed pack indices must not panic");
}

fn malformed_v1_index_with_truncated_entry_table() -> Vec<u8> {
    let advertised_objects = 67u32;
    let actual_objects = 26u32;
    let hash_len = gix_hash::Kind::Sha1.len_in_bytes();

    let mut data = Vec::with_capacity(256 * 4 + actual_objects as usize * (4 + hash_len) + hash_len * 2);
    for fan_idx in 0..256 {
        let count = if fan_idx == 255 { advertised_objects } else { 0 };
        data.extend_from_slice(&count.to_be_bytes());
    }
    for offset in 0..actual_objects {
        data.extend_from_slice(&(offset + 1).to_be_bytes());
        data.extend_from_slice(&[offset as u8; 20]);
    }
    data.extend_from_slice(&[0; 40]);
    data
}
