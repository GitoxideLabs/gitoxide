use gix_notify::{Budget, Loss, Options, SynchronizeError, Watch, Watcher};
use std::time::{Duration, Instant};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn immediate_writes_across_roots_are_covered_by_native_fences() -> TestResult {
    let first = tempfile::tempdir()?;
    let second = tempfile::tempdir()?;
    let first = first.path().canonicalize()?;
    let second = second.path().canonicalize()?;
    let mut watcher = Watcher::new(Options::default())?;
    watcher.replace([
        Watch {
            path: first.clone(),
            recursive: true,
        },
        Watch {
            path: second.clone(),
            recursive: true,
        },
    ])?;
    assert_eq!(
        watcher.drain(Budget::default()).loss,
        Some(Loss::WatchSetChanged),
        "installing coverage requires an initial baseline"
    );

    for iteration in 0..10 {
        let paths = [
            first.join(format!("first-{iteration}")),
            second.join(format!("second-{iteration}")),
        ];
        for path in &paths {
            std::fs::write(path, "changed")?;
        }
        let fence = watcher.synchronize_at(&first, Instant::now() + Duration::from_secs(5))?;
        let mut observed = Vec::new();
        loop {
            let batch = watcher.drain(Budget {
                max_events: 1,
                max_bytes: usize::MAX,
            });
            assert_eq!(
                batch.generation, fence.generation,
                "the native cut retains continuous coverage"
            );
            observed.extend(batch.events.into_iter().flat_map(|event| event.paths));
            if batch.sequence >= fence.sequence {
                break;
            }
            assert!(
                batch.more,
                "every publication preceding a completed fence is available to drain"
            );
        }
        for path in paths {
            assert!(
                observed.contains(&path),
                "an immediate write must precede the completed native fence: {path:?}"
            );
        }
    }
    assert_eq!(
        watcher.synchronize_at(&first, Instant::now()),
        Err(SynchronizeError::TimedOut),
        "expired deadlines never block on a native flush"
    );
    Ok(())
}

#[test]
fn file_scope_filters_siblings_and_rejects_uncovered_marker_directories() -> TestResult {
    let directory = tempfile::tempdir()?;
    let directory = directory.path().canonicalize()?;
    let file = directory.join("watched");
    let sibling = directory.join("unwatched");
    std::fs::write(&file, "baseline")?;
    let mut watcher = Watcher::new(Options::default())?;
    watcher.replace([Watch {
        path: file.clone(),
        recursive: false,
    }])?;
    assert_eq!(
        watcher.drain(Budget::default()).loss,
        Some(Loss::WatchSetChanged),
        "installing file coverage requires an initial baseline"
    );
    let outside = tempfile::tempdir()?;
    assert_eq!(
        watcher.synchronize_at(outside.path(), Instant::now() + Duration::from_secs(5)),
        Err(SynchronizeError::Unsupported),
        "an unrelated directory cannot fence this native stream"
    );
    assert_eq!(
        std::fs::read_dir(outside.path())?.count(),
        0,
        "unsupported anchors are not modified"
    );
    std::fs::write(&sibling, "unrelated")?;
    std::fs::write(&file, "changed")?;
    let fence = watcher.synchronize_at(&directory, Instant::now() + Duration::from_secs(5))?;
    let batch = watcher.drain(Budget::default());
    assert!(
        batch.sequence >= fence.sequence,
        "all publications through the fence can be consumed"
    );
    assert!(
        batch.events.iter().any(|event| event.paths.contains(&file)),
        "file-scoped writes are delivered"
    );
    assert!(
        !batch.events.iter().any(|event| event.paths.contains(&sibling)),
        "native parent coverage does not leak sibling events"
    );
    Ok(())
}

#[test]
fn replacing_a_root_identity_invalidates_its_fence() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?.join("root");
    let cookies = directory.path().canonicalize()?.join("cookies");
    std::fs::create_dir(&root)?;
    std::fs::create_dir(&cookies)?;
    let mut watcher = Watcher::new(Options::default())?;
    watcher.replace([
        Watch {
            path: root.clone(),
            recursive: true,
        },
        Watch {
            path: cookies.clone(),
            recursive: true,
        },
    ])?;
    let initial = watcher.drain(Budget::default());
    std::fs::rename(&root, directory.path().join("old-root"))?;
    std::fs::create_dir(&root)?;
    assert_eq!(
        watcher.synchronize_at(&cookies, Instant::now() + Duration::from_secs(5)),
        Err(SynchronizeError::CoverageLost),
        "a healthy marker root cannot certify a stream whose other root was replaced"
    );
    let batch = watcher.drain(Budget::default());
    assert!(
        batch.generation > initial.generation && batch.loss.is_some(),
        "root replacement invalidates the consumer baseline"
    );
    Ok(())
}

#[test]
fn disabled_or_long_periodic_root_checks_still_validate_fences() -> gix_error::TestResult {
    for root_check_interval in [None, Some(Duration::MAX)] {
        let directory = tempfile::tempdir()?;
        let root = directory.path().canonicalize()?.join("root");
        std::fs::create_dir(&root)?;
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        let mut watcher = Watcher::new(Options {
            root_check_interval,
            ..Options::default()
        })?;
        watcher.set_waker(move || {
            let _ = send.try_send(());
        });
        let watch = Watch {
            path: root.clone(),
            recursive: true,
        };
        watcher.replace([watch.clone()])?;
        let initial = watcher.drain(Budget::default());
        assert!(initial.loss.is_some(), "registration invalidates the baseline");
        std::fs::rename(&root, directory.path().join("old-root"))?;
        std::fs::create_dir(&root)?;

        // Observe past the default check interval. Native invalidations are still allowed, but
        // disabling or deferring periodic checks must not produce an idle identity-check failure.
        let deadline = Instant::now() + Duration::from_millis(1200);
        while Instant::now() < deadline {
            let _ = receive.recv_timeout(deadline.saturating_duration_since(Instant::now()));
            let batch = watcher.drain(Budget::default());
            assert_ne!(
                batch.loss,
                Some(Loss::BackendError),
                "disabled or deferred periodic checks must not diagnose idle root replacement: {batch:?}"
            );
            assert!(batch.error.is_none(), "native delivery must remain healthy: {batch:?}");
        }
        assert_eq!(
            watcher.synchronize_at(&root, Instant::now() + Duration::from_secs(5)),
            Err(SynchronizeError::CoverageLost),
            "disabling or deferring periodic checks must not allow a fence over a replaced root"
        );
        let failed = watcher.drain(Budget::default());
        assert_eq!(failed.loss, Some(Loss::BackendError), "the fence checks root identity");
        assert!(
            failed
                .error
                .as_ref()
                .expect("root replacement includes a diagnostic")
                .is_conflict(),
            "the public diagnostic classifies changed coverage as a conflict"
        );

        watcher.replace([watch])?;
        let recovered = watcher.drain(Budget::default());
        assert_eq!(
            recovered.loss,
            Some(Loss::WatchSetChanged),
            "retrying restores coverage"
        );
        let fence = watcher.synchronize_at(&root, Instant::now() + Duration::from_secs(5))?;
        assert_eq!(
            fence.generation, recovered.generation,
            "recovered coverage can be fenced without waiting for a periodic root check"
        );
        let batch = watcher.drain(Budget::default());
        assert!(
            batch.generation == fence.generation
                && batch.sequence >= fence.sequence
                && batch.loss.is_none()
                && batch.error.is_none(),
            "the recovered fence must be drainable without further loss: {fence:?}, {batch:?}"
        );
    }
    Ok(())
}

#[test]
fn child_directory_renames_keep_native_coverage() -> gix_error::TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    let original = root.join("original");
    let renamed = root.join("renamed");
    std::fs::create_dir(&original)?;
    let mut watcher = Watcher::new(Options::default())?;
    watcher.replace([Watch {
        path: root.clone(),
        recursive: true,
    }])?;
    let initial = watcher.drain(Budget::default());

    std::fs::rename(&original, &renamed)?;
    let changed = renamed.join("changed");
    std::fs::write(&changed, "inside the renamed child")?;
    let fence = watcher.synchronize_at(&root, Instant::now() + Duration::from_secs(5))?;
    let batch = watcher.drain(Budget::default());
    assert_eq!(
        batch.generation, initial.generation,
        "a child rename preserves root identity"
    );
    assert!(
        batch.loss.is_none() && batch.error.is_none(),
        "child renames retain coverage: {batch:?}"
    );
    assert!(
        batch.sequence >= fence.sequence,
        "the complete callback precedes the fence"
    );
    for path in [&original, &renamed] {
        assert!(
            batch
                .events
                .iter()
                .any(|event| event.kind == gix_notify::EventKind::Rename && event.paths.contains(path)),
            "both child rename endpoints must be invalidated: {batch:?}"
        );
    }
    assert!(
        batch.events.iter().any(|event| event.paths.contains(&changed)),
        "writes inside the renamed child remain covered: {batch:?}"
    );
    Ok(())
}

#[test]
fn idle_root_loss_is_reported_once_and_the_same_watch_set_can_recover() -> gix_error::TestResult {
    let interval = Duration::from_millis(50);
    for operation in ["delete", "rename", "replace", "rename-parent"] {
        let directory = tempfile::tempdir()?;
        let directory = directory.path().canonicalize()?;
        let parent = directory.join("parent");
        let root = parent.join("root");
        std::fs::create_dir_all(&root)?;
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        let mut watcher = Watcher::new(Options {
            root_check_interval: Some(interval),
            ..Options::default()
        })?;
        watcher.set_waker(move || {
            let _ = send.try_send(());
        });
        let watch = Watch {
            path: root.clone(),
            recursive: true,
        };
        watcher.replace([watch.clone()])?;
        let initial = watcher.drain(Budget::default());
        match operation {
            "delete" => std::fs::remove_dir(&root)?,
            "rename" => std::fs::rename(&root, directory.join("old-root"))?,
            "replace" => {
                std::fs::rename(&root, directory.join("old-root"))?;
                std::fs::create_dir(&root)?;
            }
            "rename-parent" => std::fs::rename(&parent, directory.join("old-parent"))?,
            _ => unreachable!("all root changes are covered"),
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        let generation = loop {
            let batch = watcher.drain(Budget::default());
            if batch.loss == Some(Loss::BackendError) {
                let error = batch
                    .error
                    .as_ref()
                    .expect("root loss includes an identity-check diagnostic");
                assert!(
                    error.is_conflict() || error.is_not_found(),
                    "root loss distinguishes changed identities from missing paths: {error:?}"
                );
                assert!(
                    batch.generation > initial.generation,
                    "root loss invalidates the baseline"
                );
                break batch.generation;
            }
            assert!(
                Instant::now() < deadline,
                "{operation} must report lost root coverage without a fence: {batch:?}"
            );
            let _ = receive.recv_timeout(deadline.saturating_duration_since(Instant::now()));
        };

        assert_eq!(
            watcher.synchronize_at(&root, Instant::now() + Duration::from_secs(5)),
            Err(SynchronizeError::CoverageLost),
            "a failed stream cannot certify delivery after its loss was drained"
        );

        // Leave the failed registration idle across multiple health checks, draining every wake.
        while receive.try_recv().is_ok() {}
        let deadline = Instant::now() + interval * 3;
        let mut wakeups = 0;
        while Instant::now() < deadline {
            if receive
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .is_ok()
            {
                wakeups += 1;
            }
            let batch = watcher.drain(Budget::default());
            assert!(
                batch.loss.is_none() && batch.error.is_none(),
                "persistent root loss is reported once: {batch:?}"
            );
            assert_eq!(
                batch.generation, generation,
                "an unrepaired root does not repeatedly invalidate coverage"
            );
        }
        assert!(
            wakeups <= 1,
            "persistent root loss must not repeatedly wake the consumer"
        );

        std::fs::create_dir_all(&root)?;
        watcher.replace([watch])?;
        assert_eq!(
            watcher.drain(Budget::default()).loss,
            Some(Loss::WatchSetChanged),
            "the identical desired watch set must install fresh native coverage"
        );
        let changed = root.join("after-recovery");
        std::fs::write(&changed, "inside the recovered root")?;
        let fence = watcher.synchronize_at(&root, Instant::now() + Duration::from_secs(5))?;
        let batch = watcher.drain(Budget::default());
        assert_eq!(batch.generation, fence.generation, "recovered coverage can be fenced");
        assert!(
            batch.sequence >= fence.sequence,
            "recovery publishes writes before its fence"
        );
        assert!(
            batch.events.iter().any(|event| event.paths.contains(&changed)),
            "recovery resumes ordinary delivery: {batch:?}"
        );

        let (finished, wait) = std::sync::mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || {
            drop(watcher);
            let _ = finished.send(());
        });
        wait.recv_timeout(Duration::from_secs(5))
            .expect("an idle watcher must shut down without a filesystem event");
        worker.join().expect("watcher shutdown must not panic");
    }
    Ok(())
}
