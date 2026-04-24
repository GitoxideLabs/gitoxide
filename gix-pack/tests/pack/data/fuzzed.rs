use std::{
    io::Write,
    panic::{catch_unwind, AssertUnwindSafe},
    path::PathBuf,
};

use gix_pack::data;

#[test]
fn artifact_inputs_can_be_opened_without_panicking() {
    for path in crate::pack::fuzz_artifact_paths("data_file") {
        _ = data::File::from_data(
            std::fs::read(&path).expect("artifact is readable"),
            path,
            gix_hash::Kind::Sha1,
        );
    }
}

/// Reproducer for the truncated ref-delta metadata fuzz case: malformed entry data must not panic
/// while attempting to read the base object id.
#[test]
fn truncated_ref_delta_metadata_is_reported_without_panicking() {
    let result = catch_unwind(|| data::Entry::from_bytes(&[0x70], 0, gix_hash::Kind::Sha1.len_in_bytes()));

    assert!(
        result
            .expect("truncated ref-delta metadata should not panic during entry decoding")
            .is_err(),
        "truncated ref-delta metadata should be rejected as corrupt input"
    );
}

/// Reproducer for the oversized pack header fuzz case: malformed entry headers with too many
/// continuation bytes must not panic while decoding the first object entry.
#[test]
fn oversized_pack_entry_header_is_reported_without_panicking() {
    let data = [
        b'P', b'A', b'C', b'K', 0, 0, 0, 2, 0, 0, 0, 1, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
        0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
    ];
    let file = data::File::from_data(
        data.as_slice(),
        PathBuf::from("fuzzed-header.pack"),
        gix_hash::Kind::Sha1,
    )
    .expect("pack header is syntactically valid");
    let result = catch_unwind(AssertUnwindSafe(|| file.entry(12)));

    assert!(
        result
            .expect("forged pack entry headers should not panic during entry decoding")
            .is_err(),
        "forged pack entry headers should be rejected as corrupt pack data"
    );
}

/// Reproducer for the large-allocation fuzz case: attacker-controlled object sizes must not cause
/// `decode_entry()` to attempt multi-gigabyte allocations.
#[test]
fn oversized_declared_object_size_is_reported_without_panicking() {
    let mut bytes = data::header::encode(data::Version::V2, 1).to_vec();
    data::entry::Header::Blob
        .write_to((i32::MAX as u64) + 1, &mut bytes)
        .expect("header write succeeds");
    bytes.extend_from_slice(&[0x78, 0x9c, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01]);
    bytes.extend_from_slice(&[0; 20]);

    let file = data::File::from_data(bytes.as_slice(), PathBuf::from("fuzzed-oom.pack"), gix_hash::Kind::Sha1)
        .expect("pack header is syntactically valid")
        .with_alloc_limit_bytes(Some(4_000_000));
    let entry = file.entry(12).expect("entry metadata is parseable");

    let result = catch_unwind(AssertUnwindSafe(|| {
        file.decode_entry(
            entry,
            &mut Vec::new(),
            &mut Default::default(),
            &|_, _| None,
            &mut gix_pack::cache::Never,
        )
    }));

    assert!(
        result
            .expect("oversized declared object sizes should not abort while decoding")
            .is_err(),
        "oversized declared object sizes should be rejected"
    );
}

/// Reproducer for the allocation-limit fuzz case: with a configured cap, attacker-controlled
/// object sizes must fail with `OutOfMemory` instead of attempting the allocation or panicking.
#[test]
fn declared_object_size_over_alloc_limit_bytes_is_reported_as_out_of_memory() {
    fn deflate(bytes: &[u8]) -> Vec<u8> {
        let mut out = gix_features::zlib::stream::deflate::Write::new(Vec::new());
        out.write_all(bytes).expect("writing to deflater succeeds");
        out.flush().expect("flushing deflater succeeds");
        out.into_inner()
    }

    let object = [0u8; 65];
    let mut bytes = data::header::encode(data::Version::V2, 1).to_vec();
    data::entry::Header::Blob
        .write_to(object.len() as u64, &mut bytes)
        .expect("header write succeeds");
    bytes.extend_from_slice(&deflate(&object));
    bytes.extend_from_slice(&[0; 20]);

    let file = data::File::from_data(
        bytes.as_slice(),
        PathBuf::from("fuzzed-alloc-limit.pack"),
        gix_hash::Kind::Sha1,
    )
    .expect("pack header is syntactically valid")
    .with_alloc_limit_bytes(Some(64));
    let entry = file.entry(12).expect("entry metadata is parseable");

    let result = catch_unwind(AssertUnwindSafe(|| {
        file.decode_entry(
            entry,
            &mut Vec::new(),
            &mut Default::default(),
            &|_, _| None,
            &mut gix_pack::cache::Never,
        )
    }));

    assert!(
        matches!(
            result.expect("configured allocation limits must not cause panics"),
            Err(gix_pack::data::decode::Error::OutOfMemory)
        ),
        "pack-controlled allocations larger than the configured limit must be rejected"
    );
}

/// Reproducer for the fuzz target OOM case: with the same allocation cap used by the fuzz harness,
/// malformed delta chains must be rejected without panicking or tripping allocator aborts.
#[test]
fn runaway_delta_allocation_is_rejected_with_fuzz_alloc_limit() {
    let bytes = [
        0x50, 0x41, 0x43, 0x4b, 0x00, 0x00, 0x00, 0x02, 0xf2, 0x00, 0x00, 0xdd, 0x09, 0x96, 0x98, 0xe7, 0x6c, 0x01,
        0x78, 0x9c, 0x7a, 0x73, 0x6a, 0xfd, 0xff, 0x9b, 0x9b, 0x9b, 0x9b, 0x9b, 0x00, 0x5b,
    ];
    let file = data::File::from_data(
        bytes.as_slice(),
        PathBuf::from("fuzzed-runaway-delta-allocation.pack"),
        gix_hash::Kind::Sha1,
    )
    .expect("pack header is syntactically valid")
    .with_alloc_limit_bytes(Some(64 * 1024 * 1024));
    let len = bytes.len() as u64;
    let mut offsets = vec![0, 12.min(len - 1), (len - 1) / 2];
    let mut first_eight = [0u8; 8];
    first_eight.copy_from_slice(&bytes[..8]);
    offsets.push(u64::from_le_bytes(first_eight) % len);
    offsets.sort_unstable();
    offsets.dedup();

    let mut saw_parseable_entry = false;
    for offset in offsets {
        let Ok(entry) = file.entry(offset) else {
            continue;
        };
        saw_parseable_entry = true;
        let result = catch_unwind(AssertUnwindSafe(|| {
            file.decode_entry(
                entry,
                &mut Vec::new(),
                &mut Default::default(),
                &|_, _| None,
                &mut gix_pack::cache::Never,
            )
        }));

        assert!(
            result
                .expect("fuzz-target allocation caps must prevent allocator aborts")
                .is_err(),
            "malformed delta chains should be rejected under the fuzz allocation cap"
        );
    }
    assert!(
        saw_parseable_entry,
        "the reproducer should exercise at least one parseable entry"
    );
}

/// Reproducer for the invalid ofs-delta base-distance fuzz case: a delta whose base distance points
/// before the beginning of the pack must be rejected without panicking.
#[test]
fn invalid_ofs_delta_base_distance_is_reported_without_panicking() {
    let mut bytes = data::header::encode(data::Version::V2, 1).to_vec();
    data::entry::Header::OfsDelta { base_distance: 13 }
        .write_to(1, &mut bytes)
        .expect("header write succeeds");
    bytes.extend_from_slice(&[0x78, 0x9c, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01]);
    bytes.extend_from_slice(&[0; 20]);

    let file = data::File::from_data(
        bytes.as_slice(),
        PathBuf::from("fuzzed-invalid-ofs-delta.pack"),
        gix_hash::Kind::Sha1,
    )
    .expect("pack header is syntactically valid")
    .with_alloc_limit_bytes(Some(8 * 1024 * 1024));
    let entry = file.entry(12).expect("entry metadata is parseable");

    let result = catch_unwind(AssertUnwindSafe(|| {
        file.decode_entry(
            entry,
            &mut Vec::new(),
            &mut Default::default(),
            &|_, _| None,
            &mut gix_pack::cache::Never,
        )
    }));

    assert!(
        result
            .expect("invalid ofs-delta base distances must not panic")
            .is_err(),
        "invalid ofs-delta base distances should be rejected as corrupt pack data"
    );
}

/// Reproducer for the out-of-bounds data-offset fuzz case: malformed entry headers can produce an
/// entry whose data offset points past the available pack data, which must be rejected without
/// panicking during decompression.
#[test]
fn out_of_bounds_entry_data_offset_is_reported_without_panicking() {
    let data = [
        0x50, 0x41, 0x43, 0x4b, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x09, 0x97, 0x0e, 0x78, 0x9c, 0x95, 0x8b,
        0x51, 0x0a, 0x83, 0x30, 0x10, 0x44, 0xff, 0x3d, 0x45, 0xa0, 0x9f, 0xa5, 0xb2, 0x89, 0x9b, 0x44, 0xa1, 0x94,
        0x5e, 0xa2, 0xff, 0x6e, 0xe2, 0xc6, 0x0a, 0xd5, 0x48, 0x5c, 0xef, 0xdf, 0xf4, 0x08, 0x9d, 0x81, 0x07, 0x03,
        0x6f, 0xa4, 0x30, 0xab, 0x08, 0x88, 0xce, 0x76, 0x64, 0x7c, 0xb0, 0x11, 0x27, 0xb6, 0xc1, 0x38, 0xe3, 0x23,
        0x83, 0x4d, 0x5c, 0x6b, 0x89, 0x19, 0x23, 0x63, 0x6a, 0x76, 0x2a, 0xbc, 0x89, 0xc2, 0xc1, 0x5b, 0x5b, 0x5b,
        0x5b, 0x5b, 0x5b, 0x5b, 0x5b, 0x5b, 0x5b, 0x5b, 0x5b, 0x5b, 0x5b, 0x5b, 0x5b, 0x5b, 0x20, 0x74, 0x04, 0xb2,
        0xcc, 0x61, 0xcf, 0x52, 0x32, 0x5c, 0x73, 0xb7, 0x70, 0x02, 0xc9, 0x4d, 0x7e, 0x21, 0x51, 0x3d, 0x65, 0xbf,
        0xed, 0x17, 0x64, 0x0b, 0xa2, 0x98, 0x0a, 0x8f, 0x1e, 0xbb, 0x5e, 0x87, 0x35, 0xdb, 0xc3, 0x19, 0xcc, 0x1e,
        0x23, 0x2f, 0xd2, 0x9f, 0xd1, 0xb3, 0x8c, 0x39, 0xcf, 0x5e, 0xf4, 0x73, 0x66, 0x4f, 0x38, 0x38, 0x57, 0xd4,
        0x30, 0x56, 0x0b, 0x5e, 0x41, 0x0f, 0x17, 0xf0, 0xb5, 0xb2, 0xdb, 0xd1, 0xf1, 0x4d, 0xc3, 0xfe, 0xdf, 0x78,
        0x0b, 0xe7, 0xc1, 0x84, 0x5a, 0x76, 0xda, 0x22, 0x16, 0x0c, 0x84, 0xae, 0xe0, 0xe3, 0x18, 0x94, 0x11, 0xd1,
        0x05, 0x0c, 0x9f, 0x41, 0x56, 0x80, 0x4a, 0xdd, 0x35, 0x26, 0xef, 0xe0, 0x6b, 0xf7, 0x61, 0x6f, 0xef, 0x81,
        0x00, 0x2c, 0x54, 0x8b, 0xe4, 0xcf, 0x65, 0x9a, 0xeb, 0x44,
    ];
    let file = data::File::from_data(
        data.as_slice(),
        PathBuf::from("fuzzed-out-of-bounds-data-offset.pack"),
        gix_hash::Kind::Sha1,
    )
    .expect("pack header is syntactically valid")
    .with_alloc_limit_bytes(Some(8 * 1024 * 1024));
    let len = data.len() as u64;
    let mut offsets = vec![0, 12.min(len - 1), (len - 1) / 2];
    let mut first_eight = [0u8; 8];
    first_eight.copy_from_slice(&data[..8]);
    offsets.push(u64::from_le_bytes(first_eight) % len);
    offsets.sort_unstable();
    offsets.dedup();

    let mut saw_parseable_entry = false;
    for offset in offsets {
        let Ok(entry) = file.entry(offset) else {
            continue;
        };
        saw_parseable_entry = true;

        let result = catch_unwind(AssertUnwindSafe(|| {
            file.decode_entry(
                entry,
                &mut Vec::new(),
                &mut Default::default(),
                &|_, _| None,
                &mut gix_pack::cache::Never,
            )
        }));

        let outcome = result.expect("out-of-bounds entry data offsets must not panic");
        if outcome.is_err() {
            return;
        }
    }

    assert!(
        saw_parseable_entry,
        "at least one fuzz-selected offset should parse into an entry"
    );
    panic!("out-of-bounds entry data offsets should be rejected as corrupt pack data");
}

/// Reproducer for the malformed delta-rescue fuzz case: degenerate delta metadata can force the
/// internal instruction buffer relocation path, which must not panic while moving bytes around.
#[test]
fn malformed_delta_instruction_relocation_is_reported_without_panicking() {
    let data = [
        0x50, 0x41, 0x43, 0x4b, 0x00, 0x00, 0x00, 0x02, 0x29, 0x00, 0x00, 0x09, 0x97, 0x21, 0x21, 0x21, 0x21, 0x21,
        0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21,
        0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21,
        0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21,
        0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21,
        0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x21, 0x51, 0x00, 0x00, 0x00, 0x00, 0xe8, 0x03, 0x45, 0xcf,
    ];
    let file = data::File::from_data(
        data.as_slice(),
        PathBuf::from("fuzzed-delta-relocation.pack"),
        gix_hash::Kind::Sha1,
    )
    .expect("pack header is syntactically valid")
    .with_alloc_limit_bytes(Some(8 * 1024 * 1024));

    let len = data.len() as u64;
    let mut offsets = vec![0, 12.min(len - 1), (len - 1) / 2];
    let mut first_eight = [0u8; 8];
    first_eight.copy_from_slice(&data[..8]);
    offsets.push(u64::from_le_bytes(first_eight) % len);
    offsets.sort_unstable();
    offsets.dedup();

    let mut saw_parseable_entry = false;
    for offset in offsets {
        let Ok(entry) = file.entry(offset) else {
            continue;
        };
        saw_parseable_entry = true;

        let result = catch_unwind(AssertUnwindSafe(|| {
            file.decode_entry(
                entry,
                &mut Vec::new(),
                &mut Default::default(),
                &|_, _| None,
                &mut gix_pack::cache::Never,
            )
        }));

        let outcome = result.expect("malformed delta instruction relocation must not panic");
        if outcome.is_err() {
            return;
        }
    }

    assert!(
        saw_parseable_entry,
        "at least one fuzz-selected offset should parse into an entry"
    );
    panic!("malformed delta instruction relocation should be rejected as corrupt pack data");
}

/// Reproducer for the runaway delta-chain fuzz case: malformed packs must not be able to grow
/// delta bookkeeping without bound before decode eventually fails.
#[test]
fn runaway_delta_chain_is_reported_without_panicking() {
    let data = [
        0x50, 0x41, 0x43, 0x4b, 0x00, 0x00, 0x00, 0x02, 0x50, 0x41, 0x43, 0x4b, 0x00, 0x00, 0x00, 0x02, 0x00, 0x84,
        0x00, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1e, 0x91,
        0x0e, 0x78, 0x9c, 0x9d, 0xcb, 0x4b, 0x04, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x4f, 0xd0, 0x82, 0x61, 0x15, 0xed, 0x00, 0x01, 0x05, 0x32, 0xe9,
        0xea, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x20, 0x36, 0x00, 0x03, 0x29, 0x00, 0x00, 0x01, 0x6c, 0x00,
        0xcc, 0x00, 0x00, 0xc1,
    ];
    let file = data::File::from_data(
        data.as_slice(),
        PathBuf::from("fuzzed-runaway-delta-chain.pack"),
        gix_hash::Kind::Sha1,
    )
    .expect("pack header is syntactically valid")
    .with_alloc_limit_bytes(Some(8 * 1024 * 1024));

    let len = data.len() as u64;
    let mut offsets = vec![0, 12.min(len - 1), (len - 1) / 2];
    let mut first_eight = [0u8; 8];
    first_eight.copy_from_slice(&data[..8]);
    offsets.push(u64::from_le_bytes(first_eight) % len);
    offsets.sort_unstable();
    offsets.dedup();

    for offset in offsets {
        let result = catch_unwind(AssertUnwindSafe(|| match file.entry(offset) {
            Ok(entry) => file
                .decode_entry(
                    entry,
                    &mut Vec::new(),
                    &mut Default::default(),
                    &|_, _| None,
                    &mut gix_pack::cache::Never,
                )
                .map(|_| ())
                .map_err(|_| ()),
            Err(_) => Err(()),
        }));

        let outcome = result.expect("runaway delta chains must not panic or OOM the process");
        if outcome.is_err() {
            return;
        }
    }

    panic!("runaway delta chains should be rejected as corrupt pack data");
}

/// Reproducer for the overlong delta-header fuzz case: malformed varints in delta headers must be
/// rejected as corrupt input instead of overflowing the shift width while decoding sizes.
#[test]
fn overlong_delta_header_size_is_reported_without_panicking() {
    let data = [
        0x50, 0x41, 0x43, 0x4b, 0x00, 0x00, 0x00, 0x02, 0x29, 0x00, 0x00, 0x10, 0x68, 0xaf, 0x4f, 0x00, 0x00, 0x00,
        0x00, 0x32, 0x18, 0xa1, 0x71, 0x38, 0x95, 0x81, 0x48, 0x58, 0x49, 0x7f, 0x63, 0xe4, 0x4d, 0x6a, 0xe8, 0x85,
        0xdb, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0xaf, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
        0x80, 0x80, 0x80, 0x80, 0xdf, 0x79, 0x00, 0x17, 0x00, 0x80, 0x80, 0x00, 0x77, 0x77, 0x77, 0x77, 0x77, 0x77,
        0x77, 0x77, 0x77, 0x77, 0x77, 0x77, 0x77, 0x77, 0x77, 0x77, 0x77, 0x77, 0x77, 0x77,
    ];
    let file = data::File::from_data(
        data.as_slice(),
        PathBuf::from("fuzzed-overlong-delta-header.pack"),
        gix_hash::Kind::Sha1,
    )
    .expect("pack header is syntactically valid")
    .with_alloc_limit_bytes(Some(8 * 1024 * 1024));

    let len = data.len() as u64;
    let mut offsets = vec![0, 12.min(len - 1), (len - 1) / 2];
    let mut first_eight = [0u8; 8];
    first_eight.copy_from_slice(&data[..8]);
    offsets.push(u64::from_le_bytes(first_eight) % len);
    offsets.sort_unstable();
    offsets.dedup();

    let mut saw_parseable_entry = false;
    for offset in offsets {
        let Ok(entry) = file.entry(offset) else {
            continue;
        };
        saw_parseable_entry = true;

        let result = catch_unwind(AssertUnwindSafe(|| {
            file.decode_entry(
                entry,
                &mut Vec::new(),
                &mut Default::default(),
                &|_, _| None,
                &mut gix_pack::cache::Never,
            )
        }));

        let outcome = result.expect("overlong delta header sizes must not panic");
        if outcome.is_err() {
            return;
        }
    }

    assert!(
        saw_parseable_entry,
        "at least one fuzz-selected offset should parse into an entry"
    );
    panic!("overlong delta header sizes should be rejected as corrupt pack data");
}

/// Reproducer for the short delta-application fuzz case: malformed delta instructions can produce
/// fewer bytes than the advertised result size, which must be rejected without panicking.
#[test]
fn short_delta_application_is_reported_without_panicking() {
    fn deflate(bytes: &[u8]) -> Vec<u8> {
        let mut out = gix_features::zlib::stream::deflate::Write::new(Vec::new());
        out.write_all(bytes).expect("writing to deflater succeeds");
        out.flush().expect("flushing deflater succeeds");
        out.into_inner()
    }

    let base = deflate(b"a");
    let delta_payload = [1, 2, 1, b'x'];
    let delta = deflate(&delta_payload);

    let mut bytes = data::header::encode(data::Version::V2, 2).to_vec();
    data::entry::Header::Blob
        .write_to(1, &mut bytes)
        .expect("header write succeeds");
    bytes.extend_from_slice(&base);

    let delta_pack_offset = bytes.len() as u64;
    data::entry::Header::OfsDelta {
        base_distance: delta_pack_offset - 12,
    }
    .write_to(delta_payload.len() as u64, &mut bytes)
    .expect("header write succeeds");
    bytes.extend_from_slice(&delta);
    bytes.extend_from_slice(&[0; 20]);

    let file = data::File::from_data(
        bytes.as_slice(),
        PathBuf::from("fuzzed-short-delta-application.pack"),
        gix_hash::Kind::Sha1,
    )
    .expect("pack header is syntactically valid")
    .with_alloc_limit_bytes(Some(8 * 1024 * 1024));

    let result = catch_unwind(AssertUnwindSafe(|| {
        file.decode_entry(
            file.entry(delta_pack_offset).expect("delta entry is parseable"),
            &mut Vec::new(),
            &mut Default::default(),
            &|_, _| None,
            &mut gix_pack::cache::Never,
        )
    }));

    assert!(
        result
            .expect("short delta applications must not panic during decode")
            .is_err(),
        "short delta applications should be rejected as corrupt pack data"
    );
}
