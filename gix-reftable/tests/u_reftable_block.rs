use gix_reftable::{
    constants,
    record::{LogRecord, LogValue, RefRecord, RefValue},
    table::Table,
    writer::{WriteOptions, Writer},
};

fn sample_table() -> Table {
    let mut writer = Writer::new(WriteOptions {
        block_size: 128,
        ..Default::default()
    });
    writer.set_limits(1, 10).unwrap();
    writer
        .add_ref(RefRecord {
            refname: "HEAD".into(),
            update_index: 2,
            value: RefValue::Symref("refs/heads/main".into()),
        })
        .unwrap();
    writer
        .add_ref(RefRecord {
            refname: "refs/heads/main".into(),
            update_index: 2,
            value: RefValue::Val1(vec![1; 20]),
        })
        .unwrap();
    writer
        .add_log(LogRecord {
            refname: "refs/heads/main".into(),
            update_index: 2,
            value: LogValue::Update {
                old_hash: vec![0; 20],
                new_hash: vec![1; 20],
                name: "n".into(),
                email: "e@x".into(),
                time: 1,
                tz_offset: 0,
                message: "m".into(),
            },
        })
        .unwrap();
    writer.finish_into_table("sample").unwrap()
}

// Upstream mapping: test_reftable_block__read_write + iterator
#[test]
fn read_write_and_iterate_block() {
    let table = sample_table();
    let block = table
        .init_block(0, constants::BLOCK_TYPE_REF)
        .expect("init block")
        .expect("block present");
    let mut iter = gix_reftable::block::BlockIter::new(block);

    let mut count = 0;
    while iter.next_record().expect("next").is_some() {
        count += 1;
    }
    assert!(count >= 2);
}

// Upstream mapping: test_reftable_block__log_read_write
#[test]
fn log_block_is_readable() {
    let table = sample_table();
    let log_off = table.log_offsets.offset;
    let block = table
        .init_block(log_off, constants::BLOCK_TYPE_LOG)
        .expect("init block")
        .expect("log block");
    let mut iter = gix_reftable::block::BlockIter::new(block);
    assert!(iter.next_record().expect("next log").is_some());
}
