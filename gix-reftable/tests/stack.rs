use std::{
    error::Error,
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use bstr::BString;
use gix_hash::{Kind, ObjectId};
use gix_reftable::{
    Cleanup, CompactOptions, Limits, LockOptions, LogRecord, LogValue, RefRecord, RefValue, SnapshotOptions, Stack,
    StackError,
};

fn oid(byte: u8) -> ObjectId {
    ObjectId::from([byte; 20])
}

fn direct(name: &str, update_index: u64, byte: u8) -> RefRecord {
    RefRecord {
        name: BString::from(name),
        update_index,
        value: RefValue::Direct(oid(byte)),
    }
}

fn lock_options() -> LockOptions {
    LockOptions {
        timeout: Duration::ZERO,
    }
}

#[test]
fn additions_create_stable_merged_snapshots() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let stack = Stack::create(
        temp.path().join("reftable"),
        Kind::Sha1,
        SnapshotOptions::default(),
        Limits::default(),
    )?;
    let empty = stack.snapshot()?;
    assert_eq!(empty.refs().len(), 0, "a newly created stack has no references");

    let first = stack.begin_addition(lock_options())?;
    assert_eq!(
        first.next_update_index(),
        1,
        "the first stack addition uses update index one"
    );
    let first_snapshot = first.commit(
        &[direct("refs/heads/main", 1, 1), direct("refs/heads/topic", 1, 2)],
        &[LogRecord {
            ref_name: BString::from("refs/heads/main"),
            update_index: 1,
            value: LogValue::Update {
                old_id: oid(0),
                new_id: oid(1),
                name: BString::from("Stack Test"),
                email: BString::from("stack@example.com"),
                time: 100,
                tz_offset: 0,
                message: BString::from("create"),
            },
        }],
    )?;
    assert_eq!(
        first_snapshot.members().len(),
        1,
        "the first publication creates one immutable member"
    );

    let second = stack.begin_addition(lock_options())?;
    assert_eq!(
        second.next_update_index(),
        2,
        "the next addition advances beyond the published update range"
    );
    let latest = second.commit(
        &[
            RefRecord {
                name: BString::from("refs/heads/main"),
                update_index: 2,
                value: RefValue::Deletion,
            },
            direct("refs/heads/topic", 2, 3),
        ],
        &[],
    )?;

    assert!(
        latest.find_ref(b"refs/heads/main").is_none(),
        "a newer tombstone hides the old value"
    );
    assert_eq!(
        latest.find_ref(b"refs/heads/topic").expect("topic remains").value,
        RefValue::Direct(oid(3)),
        "the newest direct value wins across stack members"
    );
    assert_eq!(
        latest.logs_for(b"refs/heads/main").len(),
        1,
        "a reference tombstone does not remove its historical log"
    );
    assert!(
        first_snapshot.find_ref(b"refs/heads/main").is_some(),
        "an already-open snapshot remains stable"
    );

    let remove_log = stack.begin_addition(lock_options())?;
    assert_eq!(
        remove_log.next_update_index(),
        3,
        "log deletion uses the next authoritative update index"
    );
    let without_log = remove_log.commit(
        &[],
        &[
            LogRecord {
                ref_name: BString::from("refs/heads/main"),
                update_index: 3,
                value: LogValue::Placeholder,
            },
            LogRecord {
                ref_name: BString::from("refs/heads/main"),
                update_index: 1,
                value: LogValue::Deletion,
            },
        ],
    )?;
    assert!(
        without_log.logs_for(b"refs/heads/main").is_empty(),
        "a newer table can tombstone a historical log key"
    );
    assert!(
        without_log.reflog_exists(b"refs/heads/main"),
        "a placeholder retains the logical existence of the reflog"
    );
    assert!(
        matches!(
            without_log.log_records_for(b"refs/heads/main").as_slice(),
            [LogRecord {
                value: LogValue::Placeholder,
                ..
            }]
        ),
        "the placeholder is the only visible record after log deletion"
    );
    assert_eq!(
        without_log.members()[2].header.min_update_index,
        3,
        "the log-deletion member starts at its allocated update index"
    );
    assert_eq!(
        without_log.members()[2].header.max_update_index,
        3,
        "the log-deletion member ends at its allocated update index"
    );
    Ok(())
}

#[test]
fn the_authoritative_list_lock_excludes_another_writer() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let stack = Stack::create(
        temp.path().join("reftable"),
        Kind::Sha1,
        SnapshotOptions::default(),
        Limits::default(),
    )?;
    let first = stack.begin_addition(lock_options())?;
    assert!(
        stack.begin_addition(lock_options()).is_err(),
        "the second writer cannot bypass tables.list.lock"
    );
    drop(first);
    assert!(
        stack.begin_addition(lock_options()).is_ok(),
        "dropping the session releases the lock"
    );
    Ok(())
}

#[test]
fn a_locked_snapshot_keeps_its_members_stable_until_release() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let stack = Stack::create(
        temp.path().join("reftable"),
        Kind::Sha1,
        SnapshotOptions::default(),
        Limits::default(),
    )?;
    stack
        .begin_addition(lock_options())?
        .commit(&[direct("refs/heads/main", 1, 1)], &[])?;
    stack
        .begin_addition(lock_options())?
        .commit(&[direct("refs/heads/main", 2, 2)], &[])?;

    let locked = stack.lock_snapshot(lock_options())?;
    let member_paths = locked
        .snapshot()
        .members()
        .iter()
        .map(|member| stack.directory().join(&member.file_name))
        .collect::<Vec<_>>();
    assert!(
        stack.compact(CompactOptions::default(), lock_options()).is_err(),
        "compaction cannot publish or remove members while the list lock is held"
    );
    assert!(
        member_paths.iter().all(|path| path.is_file()),
        "every member selected by the locked snapshot remains readable"
    );
    drop(locked);
    assert_eq!(
        stack
            .compact(CompactOptions::default(), lock_options())?
            .snapshot
            .members()
            .len(),
        1,
        "compaction proceeds after the protected inspection ends"
    );
    Ok(())
}

#[test]
fn compaction_never_waits_for_member_locks_while_holding_the_list_lock() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let stack = Stack::create(
        temp.path().join("reftable"),
        Kind::Sha1,
        SnapshotOptions::default(),
        Limits::default(),
    )?;
    stack
        .begin_addition(lock_options())?
        .commit(&[direct("refs/heads/main", 1, 1)], &[])?;
    stack
        .begin_addition(lock_options())?
        .commit(&[direct("refs/heads/main", 2, 2)], &[])?;
    let member_path = stack.directory().join(&stack.snapshot()?.members()[0].file_name);
    let _held_member =
        gix_lock::Marker::acquire_to_hold_resource(&member_path, gix_lock::acquire::Fail::Immediately, None)?;

    let error = stack
        .compact(
            CompactOptions::default(),
            LockOptions {
                timeout: Duration::from_secs(2),
            },
        )
        .expect_err("the held member prevents compaction");
    match error {
        StackError::Lock { path, source } => {
            assert_eq!(path, member_path, "the contended member is reported");
            assert!(
                matches!(
                    source,
                    gix_lock::acquire::Error::PermanentlyLocked {
                        mode: gix_lock::acquire::Fail::Immediately,
                        ..
                    }
                ),
                "member locks use Git's non-blocking acquisition protocol"
            );
        }
        other => panic!("member contention should return its lock error, got {other}"),
    }
    assert!(
        stack.begin_addition(lock_options()).is_ok(),
        "member contention releases the authoritative list lock immediately"
    );
    Ok(())
}

#[test]
fn publication_enforces_per_table_and_list_limits_before_visibility() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let limits = Limits {
        max_file_size: 1,
        ..Limits::default()
    };
    let stack = Stack::create(
        temp.path().join("table-limit"),
        Kind::Sha1,
        SnapshotOptions::default(),
        limits,
    )?;
    let error = stack
        .begin_addition(lock_options())?
        .commit(&[direct("refs/heads/main", 1, 1)], &[])
        .expect_err("a table above the configured read limit cannot be published");
    assert!(
        matches!(error, StackError::Table(_)),
        "the codec limit violation is returned before publication"
    );
    assert!(
        stack.snapshot()?.members().is_empty(),
        "a rejected table leaves the empty generation visible"
    );

    let list_stack = Stack::create(
        temp.path().join("list-limit"),
        Kind::Sha1,
        SnapshotOptions {
            max_list_size: 1,
            ..SnapshotOptions::default()
        },
        Limits::default(),
    )?;
    let error = list_stack
        .begin_addition(lock_options())?
        .commit(&[direct("refs/heads/main", 1, 1)], &[])
        .expect_err("a generation above the list limit cannot be published");
    assert!(
        matches!(&error, StackError::Limit(message) if *message == "tables.list size"),
        "the configured list limit rejects the generation"
    );
    assert!(
        list_stack.snapshot()?.members().is_empty(),
        "the rejected generation leaves no visible table"
    );
    assert!(
        std::fs::read_dir(list_stack.directory())?
            .filter_map(Result::ok)
            .all(|entry| !matches!(
                entry.path().extension().and_then(|extension| extension.to_str()),
                Some("ref" | "log" | "tmp")
            )),
        "pre-publication limit failures remove their staged artifact"
    );
    Ok(())
}

#[test]
fn stack_limits_bound_aggregate_records_and_table_bytes() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let record_stack = Stack::create(
        temp.path().join("record-limit"),
        Kind::Sha1,
        SnapshotOptions {
            max_total_records: 1,
            ..SnapshotOptions::default()
        },
        Limits::default(),
    )?;
    record_stack
        .begin_addition(lock_options())?
        .commit(&[direct("refs/heads/main", 1, 1)], &[])?;
    let error = record_stack
        .begin_addition(lock_options())?
        .commit(&[direct("refs/heads/topic", 2, 2)], &[])
        .expect_err("the second table would exceed the aggregate record budget");
    assert!(
        matches!(&error, StackError::Limit(message) if *message == "stack record count"),
        "publication enforces the aggregate record budget"
    );
    assert_eq!(
        record_stack.snapshot()?.members().len(),
        1,
        "aggregate validation runs before the new table becomes visible"
    );

    let byte_directory = temp.path().join("byte-limit");
    let unbounded = Stack::create(
        &byte_directory,
        Kind::Sha1,
        SnapshotOptions::default(),
        Limits::default(),
    )?;
    unbounded
        .begin_addition(lock_options())?
        .commit(&[direct("refs/heads/main", 1, 1)], &[])?;
    unbounded
        .begin_addition(lock_options())?
        .commit(&[direct("refs/heads/topic", 2, 2)], &[])?;
    let snapshot = unbounded.snapshot()?;
    let total_bytes = snapshot
        .members()
        .iter()
        .try_fold(0usize, |total, member| total.checked_add(member.file_size))
        .expect("two fixture tables fit in usize");
    let error = Stack::open(
        &byte_directory,
        Kind::Sha1,
        SnapshotOptions {
            max_total_table_size: total_bytes - 1,
            ..SnapshotOptions::default()
        },
        Limits::default(),
    )
    .expect_err("individually valid tables cannot bypass the aggregate byte budget");
    assert!(
        matches!(&error, StackError::Limit(message) if *message == "stack table bytes"),
        "opening an untrusted list enforces cumulative member bytes"
    );
    Ok(())
}

#[test]
fn full_compaction_preserves_the_view_and_removes_old_members() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let stack = Stack::create(
        temp.path().join("reftable"),
        Kind::Sha1,
        SnapshotOptions::default(),
        Limits::default(),
    )?;
    stack
        .begin_addition(lock_options())?
        .commit(&[direct("refs/heads/main", 1, 1)], &[])?;
    stack.begin_addition(lock_options())?.commit(
        &[direct("refs/heads/main", 2, 2), direct("refs/heads/topic", 2, 3)],
        &[],
    )?;
    let before = stack.snapshot()?;
    let old_names = before
        .members()
        .iter()
        .map(|member| member.file_name.clone())
        .collect::<Vec<_>>();

    let outcome = stack.compact(CompactOptions::default(), lock_options())?;
    assert_eq!(
        outcome.snapshot.members().len(),
        1,
        "full compaction publishes one replacement member"
    );
    assert_eq!(
        outcome.snapshot.refs().collect::<Vec<_>>(),
        before.refs().collect::<Vec<_>>(),
        "compaction preserves the merged reference view"
    );
    assert_eq!(
        outcome.removed.len(),
        old_names.len(),
        "every superseded member is removed after publication"
    );
    assert!(
        outcome.retained.is_empty(),
        "no superseded member remains locked in this uncontended compaction"
    );
    assert_eq!(
        outcome.snapshot.members()[0].header.min_update_index,
        1,
        "the compacted member preserves the earliest update index"
    );
    assert_eq!(
        outcome.snapshot.members()[0].header.max_update_index,
        2,
        "the compacted member preserves the latest update index"
    );

    let verification = stack.verify()?;
    assert_eq!(verification.tables, 1, "verification observes the compacted member");
    assert_eq!(verification.references, 2, "verification observes both live references");
    Ok(())
}

#[test]
fn cleanup_removes_only_identifiable_staged_and_safe_unlisted_tables() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let stack = Stack::create(
        temp.path().join("reftable"),
        Kind::Sha1,
        SnapshotOptions::default(),
        Limits::default(),
    )?;
    stack
        .begin_addition(lock_options())?
        .commit(&[direct("refs/heads/main", 1, 1)], &[])?;
    let orphan = stack.directory().join("0x000000000001-0x000000000001-orphan.ref");
    std::fs::copy(
        stack.directory().join(&stack.snapshot()?.members()[0].file_name),
        &orphan,
    )?;
    let future = stack.directory().join("0x000000000002-0x000000000002-future.ref");
    std::fs::copy(
        stack.directory().join(&stack.snapshot()?.members()[0].file_name),
        &future,
    )?;
    let staged = stack
        .directory()
        .join(".0x0000000000000002-0x0000000000000002-0000000000000001.ref.tmp");
    std::fs::write(&staged, b"partial")?;
    let generic_temp = stack.directory().join("in-progress.tmp");
    std::fs::write(&generic_temp, b"partial")?;
    let guessed_temp = stack
        .directory()
        .join(".0x000000000002-0x000000000002-not-a-nonce.ref.tmp");
    std::fs::write(&guessed_temp, b"partial")?;

    let Cleanup {
        removed,
        retained,
        failures,
    } = stack.cleanup_abandoned(lock_options())?;
    assert_eq!(
        removed,
        vec![staged, orphan],
        "cleanup removes an exact generated staging name and an old-enough unlisted table"
    );
    assert!(
        retained.is_empty(),
        "an uncontended abandoned table does not need to be retained"
    );
    assert!(
        failures.is_empty(),
        "successful cleanup has no hidden filesystem errors"
    );
    assert!(
        future.exists(),
        "an unlisted complete table newer than the stack maximum is preserved"
    );
    assert!(
        generic_temp.exists() && guessed_temp.exists(),
        "cleanup does not guess whether arbitrary temporary files belong to the stack"
    );
    assert_eq!(
        stack.snapshot()?.refs().len(),
        1,
        "cleanup leaves the authoritative listed table readable"
    );
    Ok(())
}

#[test]
fn concurrent_readers_observe_complete_update_and_compaction_generations() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let stack = Arc::new(Stack::create(
        temp.path().join("reftable"),
        Kind::Sha1,
        SnapshotOptions {
            max_attempts: 100,
            ..SnapshotOptions::default()
        },
        Limits::default(),
    )?);
    let done = Arc::new(AtomicBool::new(false));
    let reader_stack = Arc::clone(&stack);
    let reader_done = Arc::clone(&done);
    let reader = std::thread::spawn(move || -> Result<(), String> {
        while !reader_done.load(Ordering::Acquire) {
            let snapshot = reader_stack.snapshot().map_err(|err| err.to_string())?;
            if let Some(record) = snapshot.find_ref(b"refs/heads/main") {
                let max = snapshot
                    .members()
                    .last()
                    .map(|member| member.header.max_update_index)
                    .ok_or_else(|| "a visible ref requires a listed table".to_owned())?;
                if record.update_index > max {
                    return Err("a snapshot exposed a record newer than its listed generation".to_owned());
                }
            }
        }
        Ok(())
    });

    for expected in 1..=20 {
        let addition = stack.begin_addition(LockOptions {
            timeout: Duration::from_secs(2),
        })?;
        assert_eq!(
            addition.next_update_index(),
            expected,
            "serialized writers allocate consecutive update indices"
        );
        addition.commit(&[direct("refs/heads/main", expected, expected as u8)], &[])?;
        if expected % 5 == 0 {
            stack.compact(
                CompactOptions::default(),
                LockOptions {
                    timeout: Duration::from_secs(2),
                },
            )?;
        }
    }
    done.store(true, Ordering::Release);
    reader
        .join()
        .map_err(|_| io::Error::other("reader thread panicked"))?
        .map_err(io::Error::other)?;
    assert_eq!(
        stack
            .snapshot()?
            .find_ref(b"refs/heads/main")
            .expect("the final ref exists")
            .update_index,
        20,
        "the final snapshot exposes the last fully published generation"
    );
    Ok(())
}

#[test]
fn compaction_applies_reflog_expiry_after_the_keep_floor() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let stack = Stack::create(
        temp.path().join("reftable"),
        Kind::Sha1,
        SnapshotOptions::default(),
        Limits::default(),
    )?;
    for index in 1..=4 {
        let log = LogRecord {
            ref_name: BString::from("refs/heads/main"),
            update_index: index,
            value: LogValue::Update {
                old_id: oid(index as u8 - 1),
                new_id: oid(index as u8),
                name: BString::from("Stack Test"),
                email: BString::from("stack@example.com"),
                time: index * 10,
                tz_offset: 0,
                message: BString::from(format!("update {index}")),
            },
        };
        stack
            .begin_addition(lock_options())?
            .commit(&[direct("refs/heads/main", index, index as u8)], &[log])?;
    }
    let outcome = stack.compact(
        CompactOptions {
            expire_logs_before: Some(35),
            keep_latest_logs: 2,
        },
        lock_options(),
    )?;
    assert_eq!(
        outcome
            .snapshot
            .logs_for(b"refs/heads/main")
            .into_iter()
            .map(|record| record.update_index)
            .collect::<Vec<_>>(),
        vec![4, 3],
        "the two newest entries survive even when the second is older than the cutoff"
    );
    Ok(())
}

#[test]
fn reflog_expiry_rewrites_a_single_member_stack() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let stack = Stack::create(
        temp.path().join("reftable"),
        Kind::Sha1,
        SnapshotOptions::default(),
        Limits::default(),
    )?;
    let logs = (1..=3)
        .map(|index| LogRecord {
            ref_name: BString::from("refs/heads/main"),
            update_index: index,
            value: LogValue::Update {
                old_id: oid(index as u8 - 1),
                new_id: oid(index as u8),
                name: BString::from("Stack Test"),
                email: BString::from("stack@example.com"),
                time: index * 10,
                tz_offset: 0,
                message: BString::from(format!("update {index}")),
            },
        })
        .collect::<Vec<_>>();
    stack
        .begin_addition(lock_options())?
        .commit(&[direct("refs/heads/main", 3, 3)], &logs)?;
    assert_eq!(stack.snapshot()?.members().len(), 1, "precondition: one table");

    let outcome = stack.compact(
        CompactOptions {
            expire_logs_before: Some(25),
            keep_latest_logs: 1,
        },
        lock_options(),
    )?;
    assert_eq!(
        outcome.snapshot.members().len(),
        1,
        "single-member expiry rewrites to one replacement member"
    );
    assert_eq!(
        outcome
            .snapshot
            .logs_for(b"refs/heads/main")
            .into_iter()
            .map(|record| record.update_index)
            .collect::<Vec<_>>(),
        vec![3],
        "expiry is a semantic rewrite even when the stack is already physically compact"
    );
    Ok(())
}

#[test]
fn malformed_missing_and_out_of_order_lists_fail_closed() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let directory = temp.path().join("reftable");
    std::fs::create_dir(&directory)?;
    std::fs::write(directory.join("tables.list"), b"../outside.ref\n")?;
    assert!(
        Stack::open(&directory, Kind::Sha1, SnapshotOptions::default(), Limits::default()).is_err(),
        "a list entry cannot escape the stack directory"
    );

    std::fs::write(
        directory.join("tables.list"),
        b"0x000000000001-0x000000000001-missing.ref\n",
    )?;
    assert!(
        Stack::open(
            &directory,
            Kind::Sha1,
            SnapshotOptions {
                max_attempts: 2,
                ..SnapshotOptions::default()
            },
            Limits::default(),
        )
        .is_err(),
        "a list naming a missing member fails closed after bounded retries"
    );

    std::fs::remove_file(directory.join("tables.list"))?;
    let error = Stack::open(&directory, Kind::Sha1, SnapshotOptions::default(), Limits::default())
        .expect_err("opening an existing stack requires its authoritative list");
    assert!(
        matches!(
            error,
            StackError::Io { path, source }
                if path == directory.join("tables.list")
                    && source.kind() == std::io::ErrorKind::NotFound
        ),
        "a missing tables.list fails closed"
    );
    let stack = Stack::create(&directory, Kind::Sha1, SnapshotOptions::default(), Limits::default())?;
    assert!(
        directory.join("tables.list").is_file(),
        "explicit creation initializes the empty authoritative list"
    );
    stack
        .begin_addition(lock_options())?
        .commit(&[direct("refs/heads/main", 1, 1)], &[])?;
    stack
        .begin_addition(lock_options())?
        .commit(&[direct("refs/heads/main", 2, 2)], &[])?;
    let members = stack.snapshot()?.members().to_vec();
    std::fs::write(
        directory.join("tables.list"),
        format!("{}\n{}\n", members[1].file_name, members[0].file_name),
    )?;
    assert!(stack.snapshot().is_err(), "out-of-order update ranges are rejected");
    Ok(())
}

#[test]
fn sha256_stacks_use_version_two_and_survive_compaction() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let stack = Stack::create(
        temp.path().join("reftable"),
        Kind::Sha256,
        SnapshotOptions::default(),
        Limits::default(),
    )?;
    for index in 1..=2 {
        stack.begin_addition(lock_options())?.commit(
            &[RefRecord {
                name: BString::from("refs/heads/main"),
                update_index: index,
                value: RefValue::Direct(ObjectId::from([index as u8; 32])),
            }],
            &[],
        )?;
    }
    let compacted = stack.compact(CompactOptions::default(), lock_options())?;
    assert_eq!(
        compacted.snapshot.members().len(),
        1,
        "SHA-256 compaction publishes one replacement member"
    );
    assert_eq!(
        compacted.snapshot.members()[0].header.object_hash,
        Kind::Sha256,
        "the compacted member retains the SHA-256 object format"
    );
    assert_eq!(
        compacted.snapshot.members()[0].header.version,
        gix_reftable::Version::V2,
        "SHA-256 stack members use reftable version two"
    );
    assert_eq!(
        compacted
            .snapshot
            .find_ref(b"refs/heads/main")
            .expect("the latest SHA-256 ref remains")
            .value,
        RefValue::Direct(ObjectId::from([2; 32])),
        "the latest SHA-256 reference value survives compaction"
    );
    Ok(())
}
