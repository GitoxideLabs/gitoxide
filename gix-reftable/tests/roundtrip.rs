use bstr::BString;
use gix_hash::{Kind, ObjectId};
use gix_reftable::{Limits, LogRecord, LogValue, RefRecord, RefValue, Table, Version, WriteOptions, Writer};

fn oid(byte: u8, kind: Kind) -> ObjectId {
    let bytes = vec![byte; kind.len_in_bytes()];
    ObjectId::try_from(bytes.as_slice()).expect("the digest has the selected hash length")
}

fn sample_records(kind: Kind) -> (Vec<RefRecord>, Vec<LogRecord>) {
    let refs = vec![
        RefRecord {
            name: BString::from("HEAD"),
            update_index: 7,
            value: RefValue::Symbolic(BString::from("refs/heads/main")),
        },
        RefRecord {
            name: BString::from("refs/heads/deleted"),
            update_index: 5,
            value: RefValue::Deletion,
        },
        RefRecord {
            name: BString::from("refs/heads/main"),
            update_index: 6,
            value: RefValue::Direct(oid(1, kind)),
        },
        RefRecord {
            name: BString::from("refs/tags/v1"),
            update_index: 7,
            value: RefValue::Peeled {
                target: oid(2, kind),
                peeled: oid(3, kind),
            },
        },
    ];
    let logs = vec![
        LogRecord {
            ref_name: BString::from("refs/heads/empty"),
            update_index: 5,
            value: LogValue::Placeholder,
        },
        LogRecord {
            ref_name: BString::from("refs/heads/main"),
            update_index: 7,
            value: LogValue::Update {
                old_id: oid(0, kind),
                new_id: oid(1, kind),
                name: BString::from("A U Thor"),
                email: BString::from("author@example.com"),
                time: 1_700_000_000,
                tz_offset: -300,
                message: BString::from("create main"),
            },
        },
        LogRecord {
            ref_name: BString::from("refs/heads/main"),
            update_index: 6,
            value: LogValue::Deletion,
        },
    ];
    (refs, logs)
}

fn roundtrip(version: Version, kind: Kind) {
    let (refs, logs) = sample_records(kind);
    let options = WriteOptions {
        version,
        object_hash: kind,
        block_size: 128,
        restart_interval: 2,
        align_blocks: true,
        include_object_index: true,
        update_index_range: None,
    };
    let bytes = Writer::new(options)
        .write(&refs, &logs)
        .expect("valid records can be encoded");
    assert_eq!(
        bytes,
        Writer::new(options)
            .write(&refs, &logs)
            .expect("encoding is repeatable"),
        "writing the same records is deterministic"
    );

    let table = Table::from_bytes(&bytes, Limits::default()).expect("the written table can be read");
    assert_eq!(
        table.header().version,
        version,
        "the selected format version is encoded"
    );
    assert_eq!(table.header().object_hash, kind, "the selected object hash is encoded");
    assert_eq!(
        table.refs().collect::<Vec<_>>(),
        refs.iter().collect::<Vec<_>>(),
        "all reference records round-trip in sorted order"
    );
    assert_eq!(
        table.logs().collect::<Vec<_>>(),
        logs.iter().collect::<Vec<_>>(),
        "all reflog records round-trip in sorted order"
    );
    assert_eq!(
        table.ref_views().collect::<Vec<_>>(),
        refs.iter().map(RefRecord::to_ref).collect::<Vec<_>>(),
        "borrowed reference views retain every decoded field"
    );
    assert_eq!(
        table.log_views().collect::<Vec<_>>(),
        logs.iter().map(LogRecord::to_ref).collect::<Vec<_>>(),
        "borrowed reflog views retain every decoded field"
    );
    assert_eq!(
        table.find_ref(b"refs/tags/v1").expect("the tag is present"),
        &refs[3],
        "exact lookup finds the encoded tag record"
    );
    assert_eq!(
        table.logs_for(b"refs/heads/main").collect::<Vec<_>>(),
        logs[1..].iter().collect::<Vec<_>>(),
        "name lookup returns every log for the selected reference"
    );
    assert_eq!(
        table.refs_with_prefix(b"refs/heads/").collect::<Vec<_>>(),
        refs[1..3].iter().collect::<Vec<_>>(),
        "indexed prefix lookup starts at the first matching reference and stops at the range boundary"
    );
    assert_eq!(
        table.refs_for_object(oid(1, kind).as_ref()).collect::<Vec<_>>(),
        vec![&refs[2]],
        "object lookup includes direct references"
    );
    assert_eq!(
        table.refs_for_object(oid(3, kind).as_ref()).collect::<Vec<_>>(),
        vec![&refs[3]],
        "object lookup includes peeled references"
    );
}

#[test]
fn version_one_sha1_roundtrip() {
    roundtrip(Version::V1, Kind::Sha1);
}

#[test]
fn version_two_sha1_roundtrip() {
    roundtrip(Version::V2, Kind::Sha1);
}

#[test]
fn version_two_sha256_roundtrip() {
    roundtrip(Version::V2, Kind::Sha256);
}

#[test]
fn rejects_corruption_truncation_and_limits() {
    let (refs, logs) = sample_records(Kind::Sha1);
    let bytes = Writer::default()
        .write(&refs, &logs)
        .expect("valid records can be encoded");

    for len in 0..bytes.len() {
        assert!(
            Table::from_bytes(&bytes[..len], Limits::default()).is_err(),
            "a table truncated to {len} bytes is rejected"
        );
    }

    let mut corrupted = bytes.clone();
    let last = corrupted.len() - 1;
    corrupted[last] ^= 1;
    assert!(
        Table::from_bytes(&corrupted, Limits::default()).is_err(),
        "a checksum mismatch rejects a corrupted table"
    );

    let limits = Limits {
        max_file_size: bytes.len() - 1,
        ..Limits::default()
    };
    assert!(
        Table::from_bytes(&bytes, limits).is_err(),
        "the configured file-size limit is enforced"
    );
}

#[test]
fn object_index_is_optional_when_sha256_ids_share_31_bytes() {
    let mut first = [7; 32];
    let mut second = first;
    first[31] = 1;
    second[31] = 2;
    let refs = vec![
        RefRecord {
            name: BString::from("refs/heads/first"),
            update_index: 3,
            value: RefValue::Direct(ObjectId::from(first)),
        },
        RefRecord {
            name: BString::from("refs/heads/second"),
            update_index: 3,
            value: RefValue::Direct(ObjectId::from(second)),
        },
    ];
    let bytes = Writer::new(WriteOptions {
        version: Version::V2,
        object_hash: Kind::Sha256,
        update_index_range: Some((1, 3)),
        ..WriteOptions::default()
    })
    .write(&refs, &[])
    .expect("a table remains valid when its object map cannot abbreviate safely");
    let table = Table::from_bytes(&bytes, Limits::default()).expect("the table without an object map validates");
    assert_eq!(
        table.header().min_update_index,
        1,
        "the explicit minimum update index is retained without an object index"
    );
    assert_eq!(
        table.refs().collect::<Vec<_>>(),
        refs.iter().collect::<Vec<_>>(),
        "SHA-256 references round-trip when no safe object prefix exists"
    );
    assert_eq!(
        table
            .refs_for_object(ObjectId::from(first).as_ref())
            .collect::<Vec<_>>(),
        vec![&refs[0]],
        "object lookup falls back to a full scan when the object map is omitted"
    );
}

#[test]
fn object_lookup_with_a_mismatched_hash_kind_is_empty_even_for_long_index_keys() {
    let first = [7; 32];
    let mut second = first;
    second[20] = 8;
    let refs = vec![
        RefRecord {
            name: BString::from("refs/heads/first"),
            update_index: 1,
            value: RefValue::Direct(ObjectId::from(first)),
        },
        RefRecord {
            name: BString::from("refs/heads/second"),
            update_index: 1,
            value: RefValue::Direct(ObjectId::from(second)),
        },
    ];
    let bytes = Writer::new(WriteOptions {
        version: Version::V2,
        object_hash: Kind::Sha256,
        ..WriteOptions::default()
    })
    .write(&refs, &[])
    .expect("the SHA-256 table can encode a long object-index abbreviation");
    let footer_start = bytes.len() - 72;
    let packed_object_position = u64::from_be_bytes(
        bytes[footer_start + 36..footer_start + 44]
            .try_into()
            .expect("the v2 footer contains the packed object position"),
    );
    assert!(
        packed_object_position & 31 > 20,
        "the regression requires an object-index key longer than a SHA-1 object ID"
    );

    let table = Table::from_bytes(&bytes, Limits::default()).expect("the SHA-256 table validates");
    assert_eq!(
        table.refs_for_object(ObjectId::from([7; 20]).as_ref()).count(),
        0,
        "a SHA-1 query cannot match a SHA-256 table and must not be sliced to its index-key width"
    );
}

#[test]
fn rejects_reserved_object_abbreviation_widths() {
    let refs = vec![RefRecord {
        name: BString::from("refs/heads/main"),
        update_index: 1,
        value: RefValue::Direct(oid(7, Kind::Sha256)),
    }];
    let bytes = Writer::new(WriteOptions {
        version: Version::V2,
        object_hash: Kind::Sha256,
        ..WriteOptions::default()
    })
    .write(&refs, &[])
    .expect("the seed table contains an object section");
    let footer_start = bytes.len() - 72;
    let packed_range = footer_start + 36..footer_start + 44;
    let packed_object_position = u64::from_be_bytes(
        bytes[packed_range.clone()]
            .try_into()
            .expect("the v2 footer contains the packed object position"),
    );
    assert_ne!(
        packed_object_position >> 5,
        0,
        "the regression fixture contains an object section"
    );

    for invalid_width in [0, 1] {
        let mut corrupted = bytes.clone();
        let invalid = (packed_object_position & !31) | invalid_width;
        corrupted[packed_range.clone()].copy_from_slice(&invalid.to_be_bytes());
        let crc = gix_features::hash::crc32(&corrupted[footer_start..corrupted.len() - 4]);
        let crc_start = corrupted.len() - 4;
        corrupted[crc_start..].copy_from_slice(&crc.to_be_bytes());

        let error = Table::from_bytes(&corrupted, Limits::default())
            .expect_err("an object section cannot use a reserved abbreviation width");
        assert!(
            error
                .to_string()
                .contains("object abbreviation length is outside 2..=31"),
            "width {invalid_width} reports the object abbreviation invariant: {error}"
        );
    }
}

#[test]
fn zero_position_object_records_trigger_the_specified_full_scan() {
    let object_id = oid(9, Kind::Sha1);
    let refs = (0..400)
        .map(|index| RefRecord {
            name: BString::from(format!("refs/heads/{index:04}")),
            update_index: 1,
            value: RefValue::Direct(object_id),
        })
        .collect::<Vec<_>>();
    let bytes = Writer::new(WriteOptions {
        block_size: 128,
        restart_interval: 1,
        ..WriteOptions::default()
    })
    .write(&refs, &[])
    .expect("the object position list may use its scan-all sentinel when it cannot fit");
    let table = Table::from_bytes(&bytes, Limits::default()).expect("the scan-all object record is valid");
    assert_eq!(
        table.refs_for_object(object_id.as_ref()).count(),
        refs.len(),
        "a zero-position object record scans every reference for exact object matches"
    );
}

#[test]
fn rejects_object_records_that_do_not_identify_objects_in_their_ref_blocks() {
    let object_id = oid(0xab, Kind::Sha1);
    let refs = vec![RefRecord {
        name: BString::from("refs/heads/main"),
        update_index: 1,
        value: RefValue::Direct(object_id),
    }];
    let mut bytes = Writer::default()
        .write(&refs, &[])
        .expect("the seed table contains an object map");
    let footer_start = bytes.len() - 68;
    let packed_object_position = u64::from_be_bytes(
        bytes[footer_start + 32..footer_start + 40]
            .try_into()
            .expect("the v1 footer contains the packed object position"),
    );
    let object_position =
        usize::try_from(packed_object_position >> 5).expect("the generated object position fits in memory");
    assert_eq!(bytes[object_position], b'o', "the footer identifies the object block");
    let first_object_key = object_position + 6;
    bytes[first_object_key] ^= 1;

    let error = Table::from_bytes(&bytes, Limits::default())
        .expect_err("an object abbreviation that does not match its referenced block is rejected");
    assert!(
        error
            .to_string()
            .contains("object records do not exactly map object IDs to containing ref blocks"),
        "the corruption error identifies the invalid object mapping: {error}"
    );
}

#[test]
fn a_log_only_first_block_shares_header_relative_offsets() {
    let logs = vec![LogRecord {
        ref_name: BString::from("refs/heads/empty"),
        update_index: 9,
        value: LogValue::Placeholder,
    }];
    let bytes = Writer::new(WriteOptions {
        update_index_range: Some((9, 9)),
        ..WriteOptions::default()
    })
    .write(&[], &logs)
    .expect("a log-only table can be written");
    assert_eq!(
        bytes[24], b'g',
        "the first log header immediately follows the v1 file header"
    );
    let table = Table::from_bytes(&bytes, Limits::default()).expect("the log-only table validates");
    assert_eq!(
        table.logs().collect::<Vec<_>>(),
        logs.iter().collect::<Vec<_>>(),
        "a log-only table round-trips its records"
    );
}

#[test]
fn explicit_update_ranges_bound_logs_above_but_allow_historical_logs_below() {
    let above = LogRecord {
        ref_name: BString::from("refs/heads/main"),
        update_index: 11,
        value: LogValue::Deletion,
    };
    let options = WriteOptions {
        update_index_range: Some((1, 10)),
        ..WriteOptions::default()
    };
    assert!(
        matches!(
            Writer::new(options).write(&[], std::slice::from_ref(&above)),
            Err(gix_reftable::Error::InvalidInput(_))
        ),
        "a log newer than the explicit table maximum is rejected"
    );

    let historical = LogRecord {
        ref_name: BString::from("refs/heads/main"),
        update_index: 0,
        value: LogValue::Deletion,
    };
    let bytes = Writer::new(options)
        .write(&[], std::slice::from_ref(&historical))
        .expect("historical logs may predate the table minimum");
    let table = Table::from_bytes(&bytes, Limits::default()).expect("the historical log table validates");
    assert_eq!(
        table.logs().collect::<Vec<_>>(),
        vec![&historical],
        "the lower-bound exception remains intact"
    );
}

#[test]
fn reader_rejects_logs_above_the_header_maximum() {
    let log = LogRecord {
        ref_name: BString::from("refs/heads/main"),
        update_index: 11,
        value: LogValue::Deletion,
    };
    let mut bytes = Writer::new(WriteOptions {
        update_index_range: Some((1, 11)),
        ..WriteOptions::default()
    })
    .write(&[], &[log])
    .expect("the seed table is valid before its header is corrupted");
    let footer_start = bytes.len() - 68;
    bytes[16..24].copy_from_slice(&10u64.to_be_bytes());
    bytes[footer_start + 16..footer_start + 24].copy_from_slice(&10u64.to_be_bytes());
    let crc = gix_features::hash::crc32(&bytes[footer_start..bytes.len() - 4]);
    let crc_start = bytes.len() - 4;
    bytes[crc_start..].copy_from_slice(&crc.to_be_bytes());

    let err = Table::from_bytes(&bytes, Limits::default())
        .expect_err("a log outside the advertised upper bound is malformed");
    assert!(
        err.to_string().contains("log update index exceeds the header maximum"),
        "the range violation is reported directly: {err}"
    );
}

#[test]
fn writer_rejects_log_updates_that_cannot_round_trip() {
    let trailing_newline = LogRecord {
        ref_name: BString::from("refs/heads/main"),
        update_index: 1,
        value: LogValue::Update {
            old_id: Kind::Sha1.null(),
            new_id: oid(1, Kind::Sha1),
            name: BString::from("A U Thor"),
            email: BString::from("author@example.com"),
            time: 1,
            tz_offset: 0,
            message: BString::from("message\n"),
        },
    };
    assert!(
        matches!(
            Writer::default().write(&[], &[trailing_newline]),
            Err(gix_reftable::Error::InvalidInput(_))
        ),
        "a trailing newline would be stripped by the reader and is rejected"
    );

    let all_zero_update = LogRecord {
        ref_name: BString::from("refs/heads/main"),
        update_index: 1,
        value: LogValue::Update {
            old_id: Kind::Sha1.null(),
            new_id: Kind::Sha1.null(),
            name: BString::new(Vec::new()),
            email: BString::new(Vec::new()),
            time: 0,
            tz_offset: 0,
            message: BString::new(Vec::new()),
        },
    };
    assert!(
        matches!(
            Writer::default().write(&[], &[all_zero_update]),
            Err(gix_reftable::Error::InvalidInput(_))
        ),
        "the placeholder encoding cannot be supplied through the Update variant"
    );
}
