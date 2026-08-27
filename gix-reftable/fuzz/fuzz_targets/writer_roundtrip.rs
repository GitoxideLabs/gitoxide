#![no_main]

use bstr::BString;
use gix_hash::{Kind, ObjectId};
use gix_reftable::{Limits, LogRecord, LogValue, RefRecord, RefValue, Table, Version, WriteOptions, Writer};
use libfuzzer_sys::fuzz_target;

fn oid(seed: &[u8], kind: Kind) -> ObjectId {
    let mut bytes = vec![0; kind.len_in_bytes()];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = seed.get(index % seed.len()).copied().unwrap_or(index as u8) ^ index as u8;
    }
    ObjectId::try_from(bytes.as_slice()).expect("the generated object ID has the selected hash length")
}

fuzz_target!(|data: &[u8]| {
    let Some((&controls, payload)) = data.split_first() else {
        return;
    };
    let kind = if controls & 1 == 0 { Kind::Sha1 } else { Kind::Sha256 };
    let version = if kind == Kind::Sha1 && controls & 2 == 0 {
        Version::V1
    } else {
        Version::V2
    };
    let record_count = payload.len().div_ceil(8).min(128);
    let mut refs = Vec::with_capacity(record_count);
    let mut logs = Vec::with_capacity(record_count / 3);
    for (index, chunk) in payload.chunks(8).take(record_count).enumerate() {
        let update_index = index as u64 + 1;
        let name = BString::from(format!("refs/heads/fuzz/{index:04}"));
        let value = match chunk.first().copied().unwrap_or_default() & 3 {
            0 => RefValue::Deletion,
            1 => RefValue::Direct(oid(chunk, kind)),
            2 => RefValue::Peeled {
                target: oid(chunk, kind),
                peeled: oid(&[chunk.first().copied().unwrap_or_default().wrapping_add(1)], kind),
            },
            _ => RefValue::Symbolic(BString::from(format!("refs/heads/fuzz/{:04}", index / 2))),
        };
        refs.push(RefRecord {
            name: name.clone(),
            update_index,
            value,
        });
        if index % 3 == 0 {
            logs.push(LogRecord {
                ref_name: name,
                update_index,
                value: if controls & 4 == 0 {
                    LogValue::Deletion
                } else {
                    LogValue::Update {
                        old_id: kind.null(),
                        new_id: oid(chunk, kind),
                        name: BString::from("Fuzz Writer"),
                        email: BString::from("fuzz@example.invalid"),
                        time: update_index,
                        tz_offset: i16::from(chunk.get(1).copied().unwrap_or_default()) - 128,
                        message: BString::from(format!("update {index}")),
                    }
                },
            });
        }
    }
    logs.sort_by(|left, right| {
        left.ref_name
            .cmp(&right.ref_name)
            .then_with(|| right.update_index.cmp(&left.update_index))
    });
    let options = WriteOptions {
        version,
        object_hash: kind,
        block_size: 128 + u32::from(controls) * 16,
        restart_interval: u16::from(controls >> 4).max(1),
        align_blocks: controls & 8 != 0,
        include_object_index: controls & 16 != 0,
        update_index_range: None,
    };
    let bytes = Writer::new(options)
        .write(&refs, &logs)
        .expect("the generated records and writer options are valid");
    let table = Table::from_bytes(&bytes, Limits::default())
        .expect("every table accepted by the writer must be accepted by the reader");
    assert_eq!(
        table.header().version,
        version,
        "the writer-selected table version survives parsing"
    );
    assert_eq!(
        table.header().object_hash,
        kind,
        "the writer-selected object hash survives parsing"
    );
    assert_eq!(
        table.refs().collect::<Vec<_>>(),
        refs.iter().collect::<Vec<_>>(),
        "all written reference records survive parsing"
    );
    assert_eq!(
        table.logs().collect::<Vec<_>>(),
        logs.iter().collect::<Vec<_>>(),
        "all written log records survive parsing"
    );
    for record in &refs {
        assert_eq!(
            table.find_ref(record.name.as_slice()),
            Some(record),
            "exact lookup agrees with the written reference record"
        );
    }
});
