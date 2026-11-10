use std::{
    ffi::OsString,
    os::windows::{ffi::OsStringExt, fs::OpenOptionsExt},
    path::Path,
    sync::mpsc,
    time::{Duration, Instant},
};

use gix_error::{ErrorExt, TestResult};
use gix_notify::{Budget, Event, EventKind, Loss, Options, SynchronizeError, Watch, Watcher};
use gix_parallel::{Mutable, OwnShared, lock};
use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS;

fn watch(path: &Path, recursive: bool) -> TestResult<(Watcher, mpsc::Receiver<()>)> {
    watch_with_options(path, recursive, Options::default())
}

fn watch_with_options(path: &Path, recursive: bool, options: Options) -> TestResult<(Watcher, mpsc::Receiver<()>)> {
    let (send, receive) = mpsc::sync_channel(1);
    let mut watcher = Watcher::new(options)?;
    watcher.set_waker(move || {
        let _ = send.try_send(());
    });
    watcher.replace([Watch {
        path: path.to_path_buf(),
        recursive,
    }])?;
    let initial = watcher.drain(Budget::default());
    assert_eq!(
        initial.loss,
        Some(Loss::WatchSetChanged),
        "registration requires a fresh baseline"
    );
    Ok((watcher, receive))
}

fn observe(watcher: &mut Watcher, receive: &mpsc::Receiver<()>, complete: impl Fn(&[Event]) -> bool) -> Vec<Event> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut events = Vec::new();
    loop {
        let batch = watcher.drain(Budget::default());
        assert!(batch.error.is_none(), "native delivery failed: {:?}", batch.error);
        assert!(batch.loss.is_none(), "native coverage was lost: {:?}", batch.loss);
        events.extend(batch.events);
        if complete(&events) {
            return events;
        }
        assert!(
            Instant::now() < deadline,
            "native watcher did not deliver the expected paths within five seconds: {events:?}"
        );
        if !batch.more {
            let _ = receive.recv_timeout(deadline.saturating_duration_since(Instant::now()));
        }
    }
}

#[test]
fn recursive_delivery_preserves_unicode_and_rename_endpoints() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    let nested = root.join("nested");
    std::fs::create_dir(&nested)?;
    let (mut watcher, receive) = watch(&root, true)?;

    let original = nested.join("日本語-🦀");
    let renamed = nested.join("Änderung-🦀");
    std::fs::write(&original, "created immediately after registration")?;
    observe(&mut watcher, &receive, |events| {
        events
            .iter()
            .any(|event| event.kind == EventKind::Create && event.paths.contains(&original))
    });
    std::fs::rename(&original, &renamed)?;
    observe(&mut watcher, &receive, |events| {
        [&original, &renamed].into_iter().all(|path| {
            events
                .iter()
                .any(|event| event.kind == EventKind::Rename && event.paths.contains(path))
        })
    });
    std::fs::remove_file(&renamed)?;
    observe(&mut watcher, &receive, |events| {
        events
            .iter()
            .any(|event| event.kind == EventKind::Remove && event.paths.contains(&renamed))
    });
    Ok(())
}

#[test]
fn nonrecursive_coverage_excludes_descendants() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    let nested = root.join("nested");
    std::fs::create_dir(&nested)?;
    let (mut watcher, receive) = watch(&root, false)?;

    let excluded = nested.join("unwatched");
    let included = root.join("watched");
    std::fs::write(&excluded, "outside the requested scope")?;
    std::fs::write(&included, "inside the requested scope")?;
    let events = observe(&mut watcher, &receive, |events| {
        events.iter().any(|event| event.paths.contains(&included))
    });
    assert!(
        events.iter().all(|event| !event.paths.contains(&excluded)),
        "nonrecursive coverage must not leak nested file events: {events:?}"
    );
    Ok(())
}

#[test]
fn file_coverage_filters_siblings_and_survives_replacement() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    let file = root.join("watched");
    let sibling = root.join("unwatched");
    std::fs::write(&file, "baseline")?;
    let (mut watcher, receive) = watch(&file, false)?;

    std::fs::write(&sibling, "unrelated")?;
    std::fs::write(&file, "modified")?;
    let events = observe(&mut watcher, &receive, |events| {
        events
            .iter()
            .any(|event| event.kind == EventKind::Modify && event.paths.contains(&file))
    });
    assert!(
        events.iter().all(|event| !event.paths.contains(&sibling)),
        "watching a file through its parent must not leak unrelated sibling events: {events:?}"
    );

    let replacement = root.join("replacement");
    std::fs::write(&replacement, "replacement contents")?;
    std::fs::rename(&replacement, &file)?;
    observe(&mut watcher, &receive, |events| {
        events
            .iter()
            .any(|event| event.kind == EventKind::Rename && event.paths.contains(&file))
    });
    std::fs::write(&file, "modified replacement")?;
    observe(&mut watcher, &receive, |events| {
        events
            .iter()
            .any(|event| event.kind == EventKind::Modify && event.paths.contains(&file))
    });
    Ok(())
}

#[test]
fn file_coverage_survives_case_only_renames() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    let original = root.join("Watched");
    let renamed = root.join("WATCHED");
    std::fs::write(&original, "baseline")?;
    let (mut watcher, receive) = watch(&original, false)?;

    std::fs::rename(&original, &renamed)?;
    observe(&mut watcher, &receive, |events| {
        events
            .iter()
            .any(|event| event.kind == EventKind::Rename && event.paths.contains(&renamed))
    });
    std::fs::write(&renamed, "modified after case-only rename")?;
    observe(&mut watcher, &receive, |events| {
        events
            .iter()
            .any(|event| event.kind == EventKind::Modify && event.paths.contains(&renamed))
    });
    Ok(())
}

#[test]
fn paths_preserve_unpaired_utf16_in_directories_and_filenames() -> TestResult {
    let directory = tempfile::tempdir()?;
    // NTFS names are UTF-16 code units and need not contain valid Unicode scalar values.
    let root = directory
        .path()
        .canonicalize()?
        .join(OsString::from_wide(&[u16::from(b'd'), 0xd800]));
    std::fs::create_dir(&root)?;
    let (mut watcher, receive) = watch(&root, false)?;
    let file = root.join(OsString::from_wide(&[u16::from(b'f'), 0xdc00]));
    std::fs::write(&file, "unpaired UTF-16")?;
    observe(&mut watcher, &receive, |events| {
        events
            .iter()
            .any(|event| event.kind == EventKind::Create && event.paths.contains(&file))
    });
    watcher.replace([Watch {
        path: file.clone(),
        recursive: false,
    }])?;
    assert_eq!(
        watcher.drain(Budget::default()).loss,
        Some(Loss::WatchSetChanged),
        "switching to file coverage invalidates the baseline"
    );
    std::fs::write(&file, "watched directly")?;
    observe(&mut watcher, &receive, |events| {
        events
            .iter()
            .any(|event| event.kind == EventKind::Modify && event.paths.contains(&file))
    });
    Ok(())
}

#[test]
fn multiple_file_watches_share_parent_coverage_without_leaking_siblings() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    let files = [root.join("first"), root.join("second")];
    for file in &files {
        std::fs::write(file, "baseline")?;
    }
    let (mut watcher, receive) = watch(&files[0], false)?;
    watcher.replace(files.iter().map(|path| Watch {
        path: path.clone(),
        recursive: false,
    }))?;
    assert_eq!(
        watcher.drain(Budget::default()).loss,
        Some(Loss::WatchSetChanged),
        "adding another file invalidates the baseline"
    );

    let excluded = root.join("unwatched");
    std::fs::write(&excluded, "unrelated")?;
    for file in &files {
        std::fs::write(file, "modified")?;
    }
    let events = observe(&mut watcher, &receive, |events| {
        files.iter().all(|file| {
            events
                .iter()
                .any(|event| event.kind == EventKind::Modify && event.paths.contains(file))
        })
    });
    assert!(
        events.iter().all(|event| !event.paths.contains(&excluded)),
        "combining file interests must not broaden coverage to the complete parent: {events:?}"
    );
    Ok(())
}

#[test]
fn nonrecursive_directory_and_nested_file_keep_independent_scopes() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    let nested = root.join("nested");
    std::fs::create_dir(&nested)?;
    let file = nested.join("watched");
    std::fs::write(&file, "baseline")?;
    let (mut watcher, receive) = watch(&root, false)?;
    watcher.replace([
        Watch {
            path: root.clone(),
            recursive: false,
        },
        Watch {
            path: file.clone(),
            recursive: false,
        },
    ])?;
    assert_eq!(
        watcher.drain(Budget::default()).loss,
        Some(Loss::WatchSetChanged),
        "adding nested file coverage invalidates the baseline"
    );

    let excluded = nested.join("unwatched");
    let direct = root.join("direct-child");
    std::fs::write(&excluded, "outside both requested scopes")?;
    std::fs::write(&direct, "inside the directory scope")?;
    std::fs::write(&file, "inside the file scope")?;
    let events = observe(&mut watcher, &receive, |events| {
        [&direct, &file]
            .into_iter()
            .all(|path| events.iter().any(|event| event.paths.contains(path)))
    });
    assert!(
        events.iter().all(|event| !event.paths.contains(&excluded)),
        "a nested file interest does not make the outer directory recursive: {events:?}"
    );
    Ok(())
}

#[test]
fn replacing_a_directory_identity_invalidates_coverage_and_can_be_retried() -> TestResult {
    replacing_directory_identity(Options::default().root_check_interval)
}

#[test]
fn custom_interval_detects_idle_directory_identity_replacement() -> TestResult {
    replacing_directory_identity(Some(Duration::from_millis(25)))
}

#[test]
fn delivery_checks_root_identity_without_periodic_checks() -> TestResult {
    replacing_directory_identity(None)
}

fn replacing_directory_identity(root_check_interval: Option<Duration>) -> TestResult {
    let directory = tempfile::tempdir()?;
    let parent = directory.path().canonicalize()?;
    let root = parent.join("watched");
    std::fs::create_dir(&root)?;
    let (mut watcher, receive) = watch_with_options(
        &root,
        true,
        Options {
            root_check_interval,
            ..Options::default()
        },
    )?;
    let initial = watcher.drain(Budget::default());
    let old_root = parent.join("old-root");
    std::fs::rename(&root, &old_root)?;
    std::fs::create_dir(&root)?;
    let stale_path = root.join("probe");
    if root_check_interval.is_none() {
        // Force a delivery on the still-open directory handle. Windows may also report root loss
        // before this write; disabling periodic checks does not suppress native loss signals.
        std::fs::write(
            old_root.join("probe"),
            "the original handle must not publish a stale path",
        )?;
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let batch = watcher.drain(Budget::default());
        assert!(
            batch.events.iter().all(|event| !event.paths.contains(&stale_path)),
            "delivery must check root identity before publishing paths under the replacement directory"
        );
        if batch.loss.is_some() {
            let error = batch.error.expect("lost native coverage retains its failure");
            assert!(error.is_conflict(), "lost coverage requires re-registration");
            assert!(
                error.downcast_any_ref::<std::io::Error>().is_some(),
                "the original native I/O failure is retained"
            );
            assert!(
                !error.is_retryable(),
                "re-registration is required instead of retrying the failed native operation"
            );
            assert!(
                batch.generation > initial.generation,
                "a replacement directory needs a fresh baseline"
            );
            break;
        }
        assert!(
            Instant::now() < deadline,
            "watching the original directory object must not leave its replacement apparently covered"
        );
        let _ = receive.recv_timeout(deadline.saturating_duration_since(Instant::now()));
    }
    watcher.replace([Watch {
        path: root.clone(),
        recursive: true,
    }])?;
    assert_eq!(
        watcher.drain(Budget::default()).loss,
        Some(Loss::WatchSetChanged),
        "retrying the same desired path establishes new native coverage"
    );
    let changed = root.join("changed");
    std::fs::write(&changed, "inside the replacement directory")?;
    observe(&mut watcher, &receive, |events| {
        events.iter().any(|event| event.paths.contains(&changed))
    });
    Ok(())
}

#[test]
fn replacement_stops_old_coverage_and_starts_new_coverage_before_returning() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    let old_root = root.join("old");
    let new_root = root.join("new");
    std::fs::create_dir(&old_root)?;
    std::fs::create_dir(&new_root)?;
    let (mut watcher, receive) = watch(&old_root, true)?;
    watcher.replace([Watch {
        path: new_root.clone(),
        recursive: true,
    }])?;
    assert_eq!(
        watcher.drain(Budget::default()).loss,
        Some(Loss::WatchSetChanged),
        "replacing coverage invalidates the previous baseline"
    );

    let excluded = old_root.join("unwatched");
    let included = new_root.join("watched");
    std::fs::write(&excluded, "after replacement")?;
    std::fs::write(&included, "immediately after replacement")?;
    let events = observe(&mut watcher, &receive, |events| {
        events.iter().any(|event| event.paths.contains(&included))
    });
    assert!(
        events.iter().all(|event| !event.paths.contains(&excluded)),
        "a removed registration must not publish events into the new generation: {events:?}"
    );
    Ok(())
}

#[test]
fn dropping_idle_watches_cancels_pending_native_io() -> TestResult {
    for root_check_interval in [Options::default().root_check_interval, None, Some(Duration::MAX)] {
        let directory = tempfile::tempdir()?;
        let root = directory.path().canonicalize()?;
        let (watcher, _) = watch_with_options(
            &root,
            true,
            Options {
                root_check_interval,
                ..Options::default()
            },
        )?;
        let (send, receive) = mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || {
            drop(watcher);
            let _ = send.send(());
        });
        receive
            .recv_timeout(Duration::from_secs(5))
            .expect("dropping an idle watcher must cancel native I/O regardless of the root check interval");
        worker.join().expect("watcher shutdown must not panic");
        std::fs::remove_dir(&root)?;
    }
    Ok(())
}

#[test]
fn partial_registration_failure_closes_armed_roots_and_can_be_retried() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    let first = root.join("a-first");
    let second = root.join("z-second");
    std::fs::create_dir(&first)?;
    std::fs::create_dir(&second)?;
    let (mut watcher, receive) = watch(&first, true)?;

    // An exclusive read detects leaked native directory handles without timing a missing event.
    let exclusive_directory = |path: &Path| {
        std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(path)
    };
    assert!(
        exclusive_directory(&first).is_err(),
        "the live native watch prevents an exclusive directory read"
    );
    // Path resolution still succeeds, so sorted registration arms the first directory before
    // opening the second directory fails its sharing check.
    let blocked = exclusive_directory(&second)?;
    second.canonicalize()?;
    let watches = [
        Watch {
            path: first.clone(),
            recursive: true,
        },
        Watch {
            path: second.clone(),
            recursive: true,
        },
    ];
    let error = watcher
        .replace(watches.clone())
        .expect_err("a sharing violation must fail the complete watch set");
    let native = error
        .downcast_any_ref::<std::io::Error>()
        .expect("the native registration failure is retained");
    assert_eq!(
        native.raw_os_error(),
        Some(windows_sys::Win32::Foundation::ERROR_SHARING_VIOLATION as i32),
        "the native sharing violation code is preserved"
    );
    assert_eq!(
        native.kind(),
        std::io::Error::from_raw_os_error(windows_sys::Win32::Foundation::ERROR_SHARING_VIOLATION as i32).kind(),
        "the native registration error kind is unchanged"
    );
    assert!(
        !error.is_validation(),
        "an OS registration failure is not invalid caller input"
    );
    assert!(!error.is_retryable(), "the backend does not guess native retryability");
    assert!(
        watcher.drain(Budget::default()).loss.is_some(),
        "failed replacement invalidates the previous coverage"
    );
    drop(exclusive_directory(&first)?);
    drop(blocked);

    watcher.replace(watches)?;
    assert_eq!(
        watcher.drain(Budget::default()).loss,
        Some(Loss::WatchSetChanged),
        "the same desired set can be retried after releasing the conflicting handle"
    );
    let changed = [first.join("changed"), second.join("changed")];
    for path in &changed {
        std::fs::write(path, "after recovery")?;
    }
    observe(&mut watcher, &receive, |events| {
        changed
            .iter()
            .all(|path| events.iter().any(|event| event.paths.contains(path)))
    });
    Ok(())
}

#[test]
fn native_buffer_overflow_requests_rescan_and_keeps_delivering() -> TestResult {
    struct Resume(mpsc::SyncSender<()>);
    impl Drop for Resume {
        fn drop(&mut self) {
            // Unblock the worker before Watcher::drop joins it, including on assertion or I/O failure.
            let _ = self.0.try_send(());
        }
    }

    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    let (wake, receive) = mpsc::sync_channel(1);
    let (entered, stalled) = mpsc::sync_channel(1);
    let gate = OwnShared::new(Mutable::new(None::<mpsc::Receiver<()>>));
    let mut watcher = Watcher::new(Options {
        max_events: 8192,
        max_bytes: 16 * 1024 * 1024,
        ..Options::default()
    })?;
    let worker_gate = gate.clone();
    watcher.set_waker(move || {
        let pause = lock(&worker_gate).take();
        if let Some(resume) = pause {
            let _ = entered.try_send(());
            let _ = resume.recv_timeout(Duration::from_secs(30));
        }
        let _ = wake.try_send(());
    });
    let watch = Watch {
        path: root.clone(),
        recursive: true,
    };
    watcher.replace([watch.clone()])?;
    assert_eq!(
        watcher.drain(Budget::default()).loss,
        Some(Loss::WatchSetChanged),
        "installing coverage requires an initial baseline"
    );
    let (resume, wait) = mpsc::sync_channel(1);
    let resume = Resume(resume);
    *lock(&gate) = Some(wait);
    std::fs::write(root.join("trigger"), "stall after the native read has been rearmed")?;
    stalled
        .recv_timeout(Duration::from_secs(5))
        .expect("a native publication must enter the armed waker");

    // One completion can fill the pending read; subsequent records overflow Windows' fixed
    // 64 KiB per-handle buffer while the worker cannot rearm it. The application queue is larger.
    let suffix = "x".repeat(200);
    for index in 0..1000 {
        std::fs::write(root.join(format!("{index:04}-{suffix}")), "overflow")?;
    }
    resume.0.send(())?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let generation = loop {
        let batch = watcher.drain(Budget::default());
        assert!(
            batch.error.is_none(),
            "native overflow is recoverable: {:?}",
            batch.error
        );
        if let Some(loss) = batch.loss {
            assert_eq!(loss, Loss::Rescan, "native buffer loss requires a complete scan");
            break batch.generation;
        }
        assert!(
            Instant::now() < deadline,
            "native buffer overflow must not be silently ignored"
        );
        if !batch.more {
            let _ = receive.recv_timeout(deadline.saturating_duration_since(Instant::now()));
        }
    };
    watcher.replace([watch])?;
    let repeated = watcher.drain(Budget::default());
    assert_eq!(
        repeated.generation, generation,
        "overflow retains healthy native coverage without a restart"
    );
    assert!(repeated.loss.is_none(), "repeating the healthy watch set is a no-op");
    let changed = root.join("after-overflow");
    std::fs::write(&changed, "delivery continues after requesting a new baseline")?;
    observe(&mut watcher, &receive, |events| {
        events.iter().any(|event| event.paths.contains(&changed))
    });
    Ok(())
}

#[test]
fn unsupported_fences_do_not_create_marker_files() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    let (mut watcher, _) = watch(&root, true)?;
    let error = watcher
        .synchronize_at(&root, Instant::now() + Duration::from_secs(5))
        .expect_err("Windows cannot certify delivery across independent native directory streams");
    assert_eq!(
        error,
        SynchronizeError::Unsupported,
        "the typed recovery contract is unchanged"
    );
    assert!(
        error.raise().is_unsupported(),
        "the shared error classifies this known unsupported capability"
    );
    assert_eq!(
        std::fs::read_dir(&root)?.count(),
        0,
        "unsupported synchronization must not leave filesystem markers"
    );
    Ok(())
}
