use gix_reftable::{
    basics::{decode_varint, encode_varint},
    constants,
    record::{decode_key, encode_key, IndexRecord, LogRecord, LogValue, ObjRecord, Record, RefRecord, RefValue},
};

fn hash(seed: u8, len: usize) -> Vec<u8> {
    (0..len).map(|i| seed.wrapping_add(i as u8)).collect()
}

// Upstream mapping: test_reftable_record__varint_roundtrip
#[test]
fn varint_roundtrip() {
    let mut buf = [0u8; 10];
    for value in [0, 1, 27, 127, 128, 257, 4096, u64::MAX] {
        let n = encode_varint(value, &mut buf);
        let (decoded, consumed) = decode_varint(&buf[..n]).expect("decode");
        assert_eq!(consumed, n);
        assert_eq!(decoded, value);
    }
}

// Upstream mapping: test_reftable_record__key_roundtrip
#[test]
fn key_roundtrip() {
    let (encoded, _restart) = encode_key(b"refs/heads/master", b"refs/tags/v1", 6).expect("encode");
    let mut key = b"refs/heads/master".to_vec();
    let (_n, extra) = decode_key(&mut key, &encoded).expect("decode");
    assert_eq!(extra, 6);
    assert_eq!(key, b"refs/tags/v1");
}

// Upstream mapping: test_reftable_record__ref_record_roundtrip
#[test]
fn ref_record_roundtrip() {
    let rec = Record::Ref(RefRecord {
        refname: "refs/heads/main".into(),
        update_index: 42,
        value: RefValue::Val2 {
            value: hash(1, 20),
            target_value: hash(2, 20),
        },
    });
    let payload = rec.encode(20).expect("encode");
    let out = Record::decode(constants::BLOCK_TYPE_REF, &rec.key(), rec.val_type(), &payload, 20).expect("decode");
    assert_eq!(rec, out);
}

// Upstream mapping: test_reftable_record__log_record_roundtrip
#[test]
fn log_record_roundtrip() {
    let rec = Record::Log(LogRecord {
        refname: "refs/heads/main".into(),
        update_index: 9,
        value: LogValue::Update {
            old_hash: hash(1, 20),
            new_hash: hash(2, 20),
            name: "n".into(),
            email: "e@x".into(),
            time: 123,
            tz_offset: 100,
            message: "m".into(),
        },
    });
    let payload = rec.encode(20).expect("encode");
    let out = Record::decode(constants::BLOCK_TYPE_LOG, &rec.key(), rec.val_type(), &payload, 20).expect("decode");
    assert_eq!(rec, out);
}

// Upstream mapping: test_reftable_record__obj_record_roundtrip + index_record_roundtrip
#[test]
fn obj_and_index_roundtrip() {
    let obj = Record::Obj(ObjRecord {
        hash_prefix: vec![1, 2, 3, 4],
        offsets: vec![1, 5, 9],
    });
    let obj_out = Record::decode(
        constants::BLOCK_TYPE_OBJ,
        &obj.key(),
        obj.val_type(),
        &obj.encode(20).unwrap(),
        20,
    )
    .expect("obj decode");
    assert_eq!(obj, obj_out);

    let idx = Record::Index(IndexRecord {
        last_key: b"refs/heads/main".to_vec(),
        offset: 77,
    });
    let idx_out = Record::decode(
        constants::BLOCK_TYPE_INDEX,
        &idx.key(),
        idx.val_type(),
        &idx.encode(20).unwrap(),
        20,
    )
    .expect("index decode");
    assert_eq!(idx, idx_out);
}
