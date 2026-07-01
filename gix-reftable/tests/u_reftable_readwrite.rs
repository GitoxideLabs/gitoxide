use gix_reftable::{
    error::Error,
    record::{LogRecord, LogValue, RefRecord, RefValue},
    writer::{WriteOptions, Writer},
};

// Upstream mapping: test_reftable_readwrite__write_empty_table
#[test]
fn write_empty_table() {
    let mut writer = Writer::new(WriteOptions::default());
    writer.set_limits(1, 1).unwrap();
    let bytes = writer.finish().unwrap();
    assert!(!bytes.is_empty());
}

// Upstream mapping: test_reftable_readwrite__log_write_limits
#[test]
fn log_write_limits() {
    let mut writer = Writer::new(WriteOptions::default());
    writer.set_limits(1, 1).unwrap();

    let err = writer
        .add_log(LogRecord {
            refname: "refs/heads/main".into(),
            update_index: 3,
            value: LogValue::Deletion,
        })
        .expect_err("out of range");
    assert!(matches!(err, Error::Api(_)));
}

// Upstream mapping: test_reftable_readwrite__table_read_write_sequential
#[test]
fn table_read_write_sequential() {
    let mut writer = Writer::new(WriteOptions {
        block_size: 128,
        ..Default::default()
    });
    writer.set_limits(1, 10).unwrap();

    for idx in 0..20u8 {
        writer
            .add_ref(RefRecord {
                refname: format!("refs/heads/{idx:02}"),
                update_index: 2,
                value: RefValue::Val1(vec![idx; 20]),
            })
            .unwrap();
    }

    let table = writer.finish_into_table("seq").unwrap();
    let mut iter = table.iter(gix_reftable::constants::BLOCK_TYPE_REF).unwrap();
    let mut count = 0;
    while iter.next_record().unwrap().is_some() {
        count += 1;
    }
    assert_eq!(count, 20);
}
