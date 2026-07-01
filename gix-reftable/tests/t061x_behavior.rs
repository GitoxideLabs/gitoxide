use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use gix_reftable::{
    error::Error,
    record::{LogRecord, LogValue, RefRecord, RefValue},
    stack::{Stack, StackOptions},
    writer::{WriteOptions, Writer},
};

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new() -> Self {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("t061x-reftable-{stamp}"));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

// Selected parity from t0610: init creates structures.
#[test]
fn t0610_init_creates_basic_structures() {
    let tmp = TempDir::new();
    let _stack = Stack::open(&tmp.path, StackOptions::default()).unwrap();
    assert!(tmp.path.join("tables.list").is_file());
}

// Selected parity from t0610: corrupted tables list causes transaction/reload failure.
#[test]
fn t0610_corrupted_tables_list_fails_reload() {
    let tmp = TempDir::new();
    let mut stack = Stack::open(&tmp.path, StackOptions::default()).unwrap();

    let mut tx = stack.transaction();
    tx.add_ref(RefRecord {
        refname: "refs/heads/main".into(),
        update_index: 0,
        value: RefValue::Val1(vec![1; 20]),
    });
    tx.commit().unwrap();

    fs::write(tmp.path.join("tables.list"), "garbage\n").unwrap();
    assert!(stack.reload().is_err());
}

// Selected parity from t0613: default write options use 4096-byte block size.
#[test]
fn t0613_default_write_options() {
    let mut writer = Writer::new(WriteOptions::default());
    writer.set_limits(1, 1).unwrap();
    writer
        .add_ref(RefRecord {
            refname: "refs/heads/main".into(),
            update_index: 1,
            value: RefValue::Val1(vec![1; 20]),
        })
        .unwrap();

    let table = writer.finish_into_table("opts").unwrap();
    assert_eq!(table.block_size, 4096);
}

// Selected parity from t0613: tiny block size with large log entry errors out.
#[test]
fn t0613_small_block_size_fails_large_log() {
    let mut writer = Writer::new(WriteOptions {
        block_size: 64,
        ..Default::default()
    });
    writer.set_limits(1, 1).unwrap();

    let err = writer
        .add_log(LogRecord {
            refname: "refs/heads/main".into(),
            update_index: 1,
            value: LogValue::Update {
                old_hash: vec![0; 20],
                new_hash: vec![1; 20],
                name: "n".into(),
                email: "e@x".into(),
                time: 1,
                tz_offset: 0,
                message: "x".repeat(500),
            },
        })
        .and_then(|_| writer.finish().map(|_| ()))
        .expect_err("must fail");
    assert!(matches!(err, Error::Api(_)));
}

// Selected parity from t0614: fsck succeeds on healthy stack and fails on broken table names.
#[test]
fn t0614_fsck_behavior() {
    let tmp = TempDir::new();
    let mut stack = Stack::open(&tmp.path, StackOptions::default()).unwrap();

    let mut tx = stack.transaction();
    tx.add_ref(RefRecord {
        refname: "refs/heads/main".into(),
        update_index: 0,
        value: RefValue::Val1(vec![1; 20]),
    });
    tx.commit().unwrap();
    stack.fsck().unwrap();

    let current = stack.table_names()[0].clone();
    let broken = "broken.ref".to_string();
    fs::rename(tmp.path.join(&current), tmp.path.join(&broken)).unwrap();
    fs::write(tmp.path.join("tables.list"), format!("{broken}\n")).unwrap();

    let mut stack = Stack::open(&tmp.path, StackOptions::default()).unwrap();
    assert!(stack.reload().is_ok());
    assert!(stack.fsck().is_err());
}
