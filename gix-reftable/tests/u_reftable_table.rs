use gix_reftable::{
    constants,
    record::{RefRecord, RefValue},
    writer::{WriteOptions, Writer},
};

fn table_with_refs() -> gix_reftable::table::Table {
    let mut writer = Writer::new(WriteOptions {
        block_size: 96,
        ..Default::default()
    });
    writer.set_limits(1, 10).unwrap();
    for i in 0..16u8 {
        writer
            .add_ref(RefRecord {
                refname: format!("refs/heads/{i:02}"),
                update_index: 2,
                value: RefValue::Val1(vec![i; 20]),
            })
            .unwrap();
    }
    writer.finish_into_table("t").unwrap()
}

// Upstream mapping: test_reftable_table__seek_once
#[test]
fn seek_once() {
    let table = table_with_refs();
    let mut iter = table.iter(constants::BLOCK_TYPE_REF).unwrap();
    iter.seek_key(b"refs/heads/08").unwrap();
    let rec = iter.next_record().unwrap().unwrap();
    assert_eq!(rec.key(), b"refs/heads/08");
}

// Upstream mapping: test_reftable_table__reseek
#[test]
fn reseek() {
    let table = table_with_refs();
    let mut iter = table.iter(constants::BLOCK_TYPE_REF).unwrap();
    iter.seek_key(b"refs/heads/10").unwrap();
    let rec = iter.next_record().unwrap().unwrap();
    assert_eq!(rec.key(), b"refs/heads/10");

    iter.seek_key(b"refs/heads/03").unwrap();
    let rec = iter.next_record().unwrap().unwrap();
    assert_eq!(rec.key(), b"refs/heads/03");
}

// Upstream mapping: test_reftable_table__block_iterator
#[test]
fn block_iterator_progresses() {
    let table = table_with_refs();
    let mut iter = table.iter(constants::BLOCK_TYPE_REF).unwrap();
    let mut count = 0;
    while iter.next_record().unwrap().is_some() {
        count += 1;
    }
    assert_eq!(count, 16);
}
