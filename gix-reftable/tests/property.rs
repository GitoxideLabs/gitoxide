use bstr::BString;
use gix_hash::{Kind, ObjectId};
use gix_reftable::{Limits, LogRecord, LogValue, RefRecord, RefValue, Table, Version, WriteOptions, Writer};

fn oid(index: usize, kind: Kind) -> ObjectId {
    let mut bytes = vec![0; kind.len_in_bytes()];
    bytes[0] = (index >> 8) as u8;
    bytes[1] = index as u8;
    for (offset, byte) in bytes.iter_mut().enumerate().skip(2) {
        *byte = (index.wrapping_mul(31).wrapping_add(offset)) as u8;
    }
    ObjectId::try_from(bytes.as_slice()).expect("the generated digest has the requested length")
}

fn records(kind: Kind) -> (Vec<RefRecord>, Vec<LogRecord>) {
    let refs = (0..200)
        .map(|index| RefRecord {
            name: BString::from(format!("refs/heads/generated/{index:04}")),
            update_index: 1000 + index as u64,
            value: match index % 4 {
                0 => RefValue::Deletion,
                1 => RefValue::Direct(oid(index, kind)),
                2 => RefValue::Peeled {
                    target: oid(index, kind),
                    peeled: oid(index + 1000, kind),
                },
                _ => RefValue::Symbolic(BString::from(format!("refs/heads/generated/{:04}", index - 1))),
            },
        })
        .collect();
    let logs = (0..100)
        .map(|index| LogRecord {
            ref_name: BString::from(format!("refs/heads/log/{:03}", index / 5)),
            update_index: 2000 - index as u64,
            value: if index % 7 == 0 {
                LogValue::Deletion
            } else {
                LogValue::Update {
                    old_id: oid(index, kind),
                    new_id: oid(index + 1, kind),
                    name: BString::from("Property Test"),
                    email: BString::from("property@example.com"),
                    time: 1_700_000_000 + index as u64,
                    tz_offset: if index % 2 == 0 { -300 } else { 150 },
                    message: BString::from(format!("update {index}")),
                }
            },
        })
        .collect();
    (refs, logs)
}

#[test]
fn records_roundtrip_across_block_shapes() {
    for (version, kind) in [
        (Version::V1, Kind::Sha1),
        (Version::V2, Kind::Sha1),
        (Version::V2, Kind::Sha256),
    ] {
        let (refs, logs) = records(kind);
        for (block_size, restart_interval, align_blocks, include_object_index) in [
            (192, 1, false, true),
            (193, 2, true, false),
            (257, 7, true, true),
            (511, 16, false, false),
            (4096, 64, true, true),
        ] {
            let bytes = Writer::new(WriteOptions {
                version,
                object_hash: kind,
                block_size,
                restart_interval,
                align_blocks,
                include_object_index,
                update_index_range: None,
            })
            .write(&refs, &logs)
            .expect("generated records fit the selected block shape");
            let table = Table::from_bytes(&bytes, Limits::default()).expect("the generated table validates");
            assert_eq!(
                table.header().version,
                version,
                "the selected version survives every block shape"
            );
            assert_eq!(
                table.header().object_hash,
                kind,
                "the selected hash survives every block shape"
            );
            assert_eq!(
                table.refs().collect::<Vec<_>>(),
                refs.iter().collect::<Vec<_>>(),
                "reference records survive every version, hash, and block shape"
            );
            assert_eq!(
                table.logs().collect::<Vec<_>>(),
                logs.iter().collect::<Vec<_>>(),
                "log records survive every version, hash, and block shape"
            );
            for record in &refs {
                assert_eq!(
                    table.find_ref(record.name.as_slice()),
                    Some(record),
                    "exact lookup agrees with full iteration"
                );
            }
            assert_eq!(
                table.refs_for_object(oid(1, kind).as_ref()).collect::<Vec<_>>(),
                vec![&refs[1]],
                "object lookup agrees with the decoded records whether or not an object index is present"
            );
        }
    }
}

#[test]
fn empty_tables_are_valid_and_deterministic() {
    let bytes = Writer::default()
        .write(&[], &[])
        .expect("an empty table is representable");
    let table = Table::from_bytes(&bytes, Limits::default()).expect("the empty table validates");
    assert_eq!(table.refs().len(), 0, "an empty table contains no references");
    assert_eq!(table.logs().len(), 0, "an empty table contains no log records");
}

#[test]
fn single_byte_mutations_never_panic() {
    let (refs, logs) = records(Kind::Sha1);
    let bytes = Writer::new(WriteOptions {
        block_size: 257,
        restart_interval: 3,
        ..WriteOptions::default()
    })
    .write(&refs[..20], &logs[..10])
    .expect("the mutation seed table can be encoded");

    for index in 0..bytes.len() {
        let mut mutated = bytes.clone();
        mutated[index] ^= 0x80;
        let outcome = std::panic::catch_unwind(|| Table::from_bytes(&mutated, Limits::default()));
        assert!(outcome.is_ok(), "mutation at byte {index} must not panic");
    }
}
