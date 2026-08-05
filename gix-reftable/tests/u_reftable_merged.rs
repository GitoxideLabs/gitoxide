use gix_reftable::{
    merged::MergedTable,
    record::{RefRecord, RefValue},
    writer::{WriteOptions, Writer},
};

fn table_with_value(value: u8, update_index: u64) -> gix_reftable::table::Table {
    let mut writer = Writer::new(WriteOptions::default());
    writer.set_limits(update_index, update_index).unwrap();
    writer
        .add_ref(RefRecord {
            refname: "refs/heads/main".into(),
            update_index,
            value: RefValue::Val1(vec![value; 20]),
        })
        .unwrap();
    writer.finish_into_table("m").unwrap()
}

// Upstream mapping: test_reftable_merged__single_record + refs
#[test]
fn merged_prefers_newer_table_on_duplicate_keys() {
    let old = table_with_value(1, 1);
    let new = table_with_value(2, 2);
    let merged = MergedTable::new(vec![old, new]).unwrap();

    let mut iter = merged.ref_iter().unwrap();
    let rec = iter.next_record().unwrap().unwrap();
    let gix_reftable::record::Record::Ref(rec) = rec else {
        panic!("expected ref");
    };
    let RefValue::Val1(id) = rec.value else {
        panic!("expected val1");
    };
    assert_eq!(id, vec![2; 20]);
}

// Upstream mapping: test_reftable_merged__seek_multiple_times
#[test]
fn merged_seek_multiple_times() {
    let t = table_with_value(1, 1);
    let merged = MergedTable::new(vec![t.clone(), t]).unwrap();

    let mut iter = merged.ref_iter().unwrap();
    iter.seek_key(b"refs/heads/main").unwrap();
    assert!(iter.next_record().unwrap().is_some());
    iter.seek_key(b"refs/heads/main").unwrap();
    assert!(iter.next_record().unwrap().is_some());
}
