use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use gix_reftable::{
    record::{RefRecord, RefValue},
    stack::{Stack, StackOptions},
    writer::WriteOptions,
};

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new() -> Self {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("u-reftable-stack-{stamp}"));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

// Upstream mapping: test_reftable_stack__add_one + transaction_api
#[test]
fn add_one_transaction() {
    let tmp = TempDir::new();
    let mut stack = Stack::open(
        &tmp.path,
        StackOptions {
            disable_auto_compact: true,
            write_options: WriteOptions {
                block_size: 128,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();

    let mut tx = stack.transaction();
    tx.add_ref(RefRecord {
        refname: "refs/heads/main".into(),
        update_index: 0,
        value: RefValue::Val1(vec![1; 20]),
    });
    tx.commit().unwrap();

    assert_eq!(stack.tables().len(), 1);
    stack.fsck().unwrap();
}

// Upstream mapping: test_reftable_stack__auto_compaction
#[test]
fn auto_compaction() {
    let tmp = TempDir::new();
    let mut stack = Stack::open(
        &tmp.path,
        StackOptions {
            auto_compaction_factor: 2,
            write_options: WriteOptions {
                block_size: 96,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();

    for idx in 0..3u8 {
        let mut tx = stack.transaction();
        tx.add_ref(RefRecord {
            refname: format!("refs/heads/{idx}"),
            update_index: 0,
            value: RefValue::Val1(vec![idx; 20]),
        });
        tx.commit().unwrap();
    }

    assert!(stack.tables().len() <= 2);
}
