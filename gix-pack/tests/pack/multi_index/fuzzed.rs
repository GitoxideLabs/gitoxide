use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    path::PathBuf,
};

#[test]
fn unsupported_headers_are_classified() {
    for (offset, value) in [(4, 2), (5, 255)] {
        let mut data = valid_multi_index_with_index_name(b"pack.idx");
        data[offset] = value;
        let err = gix_pack::multi_index::File::from_data(data, "unsupported.midx".into())
            .err()
            .expect("the header requests an unsupported version or hash kind");
        assert!(err.is_unsupported(), "another multi-index implementation is required");
        assert!(
            !err.is_validation() && !err.is_corrupted(),
            "unknown versions are not malformed input"
        );
    }
}

#[test]
fn artifact_inputs_can_be_opened_without_panicking() {
    for path in crate::fuzz_artifact_paths("multi_index_file") {
        _ = gix_pack::multi_index::File::from_data(std::fs::read(&path).expect("artifact is readable"), path);
    }
}

/// Reproducer for the fanout fuzz case: malformed multi-index files must not panic during lookup
/// if the fanout table advertises bounds that exceed the lookup chunk.
#[test]
fn malformed_fanout_is_reported_without_panicking() {
    let result = catch_unwind(AssertUnwindSafe(|| {
        match gix_pack::multi_index::File::from_data(
            malformed_multi_index_with_inconsistent_fanout(),
            PathBuf::from("fuzzed.midx"),
        ) {
            Ok(index) => {
                let _ = index.lookup(gix_hash::Kind::Sha1.null());
            }
            Err(err) => {
                let _ = err;
            }
        }
    }));

    assert!(result.is_ok(), "malformed multi-index files must not panic");
}

/// Pack counts must fit into the available name data before reserving a `Vec<PathBuf>`.
#[test]
fn impossible_pack_count_is_rejected_before_reserving() {
    let err = gix_pack::multi_index::File::from_data(
        multi_index_with_absurd_pack_count(),
        PathBuf::from("fuzzed-absurd-pack-count.midx"),
    )
    .err()
    .expect("an impossible pack count must be rejected before reserving");
    assert!(
        err.is_corrupted(),
        "a pack count which cannot fit in the available name data is corrupt input"
    );
    insta::assert_debug_snapshot!(err, "the error explains the structural failure", @"Pack count exceeds the available pack-name data");
}

#[test]
fn out_of_bounds_pack_indices_are_rejected_on_access() -> gix_testtools::TestResult {
    for pack_index in [1, u32::MAX] {
        let index = gix_pack::multi_index::File::from_data(
            multi_index_with_offset(b"a.idx", pack_index, 0, None),
            PathBuf::from("invalid-pack-index.midx"),
        )?;
        assert_eq!(
            index.lookup(gix_hash::Kind::Sha1.null()),
            Some(0),
            "loading and OID lookup stay lazy"
        );
        let err = index
            .pack_id_and_pack_offset_at_index(0)
            .expect_err("pack indices must refer to an existing pack name");
        assert!(err.is_corrupted(), "out-of-bounds pack indices are corrupt data");
        assert_eq!(
            err.to_string(),
            "Multi-index pack index is out of bounds",
            "the invalid pack index is rejected before accessing a pack"
        );
        let err = index
            .iter()
            .next()
            .expect("there is one object")
            .expect_err("iteration validates pack references");
        assert!(err.is_corrupted(), "iteration preserves errors");
    }
    Ok(())
}

#[test]
fn out_of_bounds_large_offsets_are_rejected_on_access() -> gix_testtools::TestResult {
    for ordinal in [1, 0x7fff_ffff] {
        let index = gix_pack::multi_index::File::from_data(
            multi_index_with_offset(b"a.idx", 0, (1 << 31) | ordinal, Some(&[1 << 32])),
            PathBuf::from("invalid-large-offset.midx"),
        )?;
        assert_eq!(
            index.lookup(gix_hash::Kind::Sha1.null()),
            Some(0),
            "loading and OID lookup stay lazy"
        );
        let err = index
            .pack_id_and_pack_offset_at_index(0)
            .expect_err("large-offset ordinals must stay inside the LOFF chunk, including its checksum boundary");
        assert!(err.is_corrupted(), "out-of-bounds large offsets are corrupt data");
        assert_eq!(
            err.to_string(),
            "Multi-index large-offset index is out of bounds",
            "the invalid large-offset ordinal is rejected before reading the LOFF table"
        );
        let err = index
            .iter()
            .next()
            .expect("there is one object")
            .expect_err("iteration validates LOFF references");
        assert!(err.is_corrupted(), "iteration preserves corruption errors");
    }
    Ok(())
}

#[test]
fn out_of_bounds_entry_indices_are_validation_errors() -> gix_testtools::TestResult {
    let index = gix_pack::multi_index::File::from_data(
        valid_multi_index_with_index_name(b"a.idx"),
        PathBuf::from("valid.midx"),
    )?;
    for entry_index in [index.num_objects(), u32::MAX] {
        let err = index
            .pack_id_and_pack_offset_at_index(entry_index)
            .expect_err("caller-provided entry indices must stay in bounds");
        assert!(
            err.is_validation(),
            "an invalid caller index is not corrupt on-disk data"
        );
        assert!(!err.is_corrupted(), "the file itself remains valid");
    }
    Ok(())
}

#[test]
fn integrity_verification_rejects_corrupt_offset_references() -> gix_testtools::TestResult {
    for mut data in [
        multi_index_with_offset(b"a.idx", 1, 0, None),
        multi_index_with_offset(b"a.idx", 0, (1 << 31) | 1, Some(&[1 << 32])),
    ] {
        let checksum_start = data.len() - gix_hash::Kind::Sha1.len_in_bytes();
        let mut hasher = gix_hash::hasher(gix_hash::Kind::Sha1);
        hasher.update(&data[..checksum_start]);
        data[checksum_start..].copy_from_slice(hasher.try_finalize()?.as_slice());
        let index = gix_pack::multi_index::File::from_data(data, "invalid-reference.midx".into())?;
        let err = index
            .verify_integrity_fast(
                &mut gix_utils::progress::Discard,
                &std::sync::atomic::AtomicBool::new(false),
            )
            .expect_err("verification must inspect references even though loading does not");
        assert!(err.is_corrupted(), "verification retains corruption classification");
        assert!(
            err.to_string().contains("index is out of bounds"),
            "a valid checksum lets verification reach the corrupt reference before opening packs: {err}"
        );
    }
    Ok(())
}

#[test]
fn high_bit_offsets_follow_the_presence_of_the_large_offset_chunk() -> gix_testtools::TestResult {
    for (offset, large_offsets, expected) in [
        (1 << 31, Some(&[1 << 32][..]), 1 << 32),
        ((1 << 31) | 1, Some(&[1 << 32, 1 << 33][..]), 1 << 33),
        (42, Some(&[1 << 32][..]), 42),
        (1 << 31, None, 1 << 31),
        (u32::MAX, None, u64::from(u32::MAX)),
    ] {
        let index = gix_pack::multi_index::File::from_data(
            multi_index_with_offset(b"a.idx", 0, offset, large_offsets),
            PathBuf::from("valid-large-offset.midx"),
        )?;
        assert_eq!(
            index.pack_id_and_pack_offset_at_index(0)?,
            (0, expected),
            "both large-offset ordinals and literal 32-bit offsets remain supported"
        );
    }
    Ok(())
}

fn malformed_multi_index_with_inconsistent_fanout() -> Vec<u8> {
    const HEADER_LEN: usize = 12;
    const TOC_LEN: usize = 5 * 12;
    const PNAM_LEN: usize = 6;
    const FAN_LEN: usize = 256 * 4;
    const LOOKUP_LEN: usize = 20;
    const OFFSETS_LEN: usize = 8;
    const TRAILER_LEN: usize = 20;

    let pnam_start = HEADER_LEN + TOC_LEN;
    let fan_start = pnam_start + PNAM_LEN;
    let lookup_start = fan_start + FAN_LEN;
    let offsets_start = lookup_start + LOOKUP_LEN;
    let trailer_start = offsets_start + OFFSETS_LEN;

    let mut data = Vec::with_capacity(trailer_start + TRAILER_LEN);
    data.extend_from_slice(b"MIDX");
    data.push(1);
    data.push(gix_hash::Kind::Sha1 as u8);
    data.push(4);
    data.push(0);
    data.extend_from_slice(&1u32.to_be_bytes());

    push_chunk(&mut data, b"PNAM", pnam_start as u64);
    push_chunk(&mut data, b"OIDF", fan_start as u64);
    push_chunk(&mut data, b"OIDL", lookup_start as u64);
    push_chunk(&mut data, b"OOFF", offsets_start as u64);
    push_chunk(&mut data, b"\0\0\0\0", trailer_start as u64);

    data.extend_from_slice(b"a.idx\0");

    for fan_idx in 0..256 {
        let count = match fan_idx {
            0 => 2u32,
            _ => 1u32,
        };
        data.extend_from_slice(&count.to_be_bytes());
    }

    data.extend_from_slice(gix_hash::Kind::Sha1.null().as_slice());
    data.extend_from_slice(&0u32.to_be_bytes());
    data.extend_from_slice(&0u32.to_be_bytes());
    data.extend_from_slice(&[0; TRAILER_LEN]);
    debug_assert_eq!(data.len(), trailer_start + TRAILER_LEN);
    data
}

fn multi_index_with_absurd_pack_count() -> Vec<u8> {
    const HEADER_LEN: usize = 12;
    const TOC_LEN: usize = 5 * 12;
    const PNAM_LEN: usize = 1;
    const FAN_LEN: usize = 256 * 4;
    const LOOKUP_LEN: usize = 20;
    const OFFSETS_LEN: usize = 8;
    const TRAILER_LEN: usize = 20;

    let pnam_start = HEADER_LEN + TOC_LEN;
    let fan_start = pnam_start + PNAM_LEN;
    let lookup_start = fan_start + FAN_LEN;
    let offsets_start = lookup_start + LOOKUP_LEN;
    let trailer_start = offsets_start + OFFSETS_LEN;

    let mut data = Vec::with_capacity(trailer_start + TRAILER_LEN);
    data.extend_from_slice(b"MIDX");
    data.push(1);
    data.push(gix_hash::Kind::Sha1 as u8);
    data.push(4);
    data.push(0);
    data.extend_from_slice(&u32::MAX.to_be_bytes());

    push_chunk(&mut data, b"PNAM", pnam_start as u64);
    push_chunk(&mut data, b"OIDF", fan_start as u64);
    push_chunk(&mut data, b"OIDL", lookup_start as u64);
    push_chunk(&mut data, b"OOFF", offsets_start as u64);
    push_chunk(&mut data, b"\0\0\0\0", trailer_start as u64);

    data.push(0);

    for fan_idx in 0..256 {
        let count = if fan_idx == 255 { 1u32 } else { 0u32 };
        data.extend_from_slice(&count.to_be_bytes());
    }
    data.extend_from_slice(gix_hash::Kind::Sha1.null().as_slice());
    data.extend_from_slice(&0u32.to_be_bytes());
    data.extend_from_slice(&0u32.to_be_bytes());
    data.extend_from_slice(&[0; TRAILER_LEN]);
    debug_assert_eq!(data.len(), trailer_start + TRAILER_LEN);
    data
}

fn valid_multi_index_with_index_name(index_name: &[u8]) -> Vec<u8> {
    multi_index_with_offset(index_name, 0, 0, None)
}

/// Build a one-object index with an optional LOFF chunk, so offset validation can distinguish
/// literal 32-bit offsets from ordinals without relying on large packs or filesystem fixtures.
fn multi_index_with_offset(index_name: &[u8], pack_index: u32, offset: u32, large_offsets: Option<&[u64]>) -> Vec<u8> {
    const HEADER_LEN: usize = 12;
    const FAN_LEN: usize = 256 * 4;
    const LOOKUP_LEN: usize = 20;
    const OFFSETS_LEN: usize = 8;
    const TRAILER_LEN: usize = 20;

    let pnam_len = index_name.len() + 1;
    let num_chunks = 4 + u8::from(large_offsets.is_some());
    let pnam_start = HEADER_LEN + (usize::from(num_chunks) + 1) * 12;
    let fan_start = pnam_start + pnam_len;
    let lookup_start = fan_start + FAN_LEN;
    let offsets_start = lookup_start + LOOKUP_LEN;
    let large_offsets_start = offsets_start + OFFSETS_LEN;
    let trailer_start = large_offsets_start + large_offsets.map_or(0, std::mem::size_of_val);

    let mut data = Vec::with_capacity(trailer_start + TRAILER_LEN);
    data.extend_from_slice(b"MIDX");
    data.push(1);
    data.push(gix_hash::Kind::Sha1 as u8);
    data.push(num_chunks);
    data.push(0);
    data.extend_from_slice(&1u32.to_be_bytes());

    push_chunk(&mut data, b"PNAM", pnam_start as u64);
    push_chunk(&mut data, b"OIDF", fan_start as u64);
    push_chunk(&mut data, b"OIDL", lookup_start as u64);
    push_chunk(&mut data, b"OOFF", offsets_start as u64);
    if large_offsets.is_some() {
        push_chunk(&mut data, b"LOFF", large_offsets_start as u64);
    }
    push_chunk(&mut data, b"\0\0\0\0", trailer_start as u64);

    data.extend_from_slice(index_name);
    data.push(0);

    for _ in 0..256 {
        data.extend_from_slice(&1u32.to_be_bytes());
    }

    data.extend_from_slice(gix_hash::Kind::Sha1.null().as_slice());
    data.extend_from_slice(&pack_index.to_be_bytes());
    data.extend_from_slice(&offset.to_be_bytes());
    for offset in large_offsets.into_iter().flatten() {
        data.extend_from_slice(&offset.to_be_bytes());
    }
    data.extend_from_slice(&[0; TRAILER_LEN]);
    debug_assert_eq!(data.len(), trailer_start + TRAILER_LEN);
    data
}

fn push_chunk(data: &mut Vec<u8>, id: &[u8; 4], offset: u64) {
    data.extend_from_slice(id);
    data.extend_from_slice(&offset.to_be_bytes());
}
