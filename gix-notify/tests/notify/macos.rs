use gix_notify::{Budget, Loss, Options, SynchronizeError, Watch, Watcher};
use std::time::{Duration, Instant};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn removing_nested_watches_preserves_queued_events_and_the_generation() -> gix_error::TestResult {
    let fixture = tempfile::tempdir()?;
    let root = fixture.path().canonicalize()?;
    let child = root.join("child");
    let sibling = root.join("sibling");
    let deep = child.join("deep");
    std::fs::create_dir_all(&deep)?;
    std::fs::create_dir(&sibling)?;
    let mut watcher = Watcher::new(Options::default())?;
    let alias = root.join("alias");
    std::os::unix::fs::symlink(&child, &alias)?;
    watcher.replace([&root, &child, &alias].map(|path| Watch {
        path: path.clone(),
        recursive: false,
    }))?;
    assert!(
        !watcher.try_remove_subtree(&child),
        "a second subscription through a symlink shares the canonical child watch"
    );
    watcher.replace(
        [(&root, false), (&child, false), (&deep, true), (&sibling, true)].map(|(path, recursive)| Watch {
            path: path.clone(),
            recursive,
        }),
    )?;
    let generation = watcher.drain(Budget::default()).generation;
    assert!(
        !watcher.try_remove_subtree(&root),
        "a request that includes the physical root must leave coverage intact"
    );
    let queued = deep.join("queued");
    std::fs::write(&queued, "before removal")?;
    watcher.synchronize_at(&root, Instant::now() + Duration::from_secs(5))?;
    assert!(
        watcher.try_remove_subtree(&child),
        "the shallow parent keeps the native stream alive"
    );
    let excluded = deep.join("later");
    let included = sibling.join("later");
    std::fs::write(&excluded, "retired subscription")?;
    std::fs::write(&included, "retained subscription")?;
    let fence = watcher.synchronize_at(&root, Instant::now() + Duration::from_secs(5))?;
    let batch = watcher.drain(Budget::default());
    assert_eq!(fence.generation, generation, "both fences span one native registration");
    assert_eq!(batch.loss, None, "retiring the nested watch preserves continuity");
    let paths: Vec<_> = batch.events.into_iter().flat_map(|event| event.paths).collect();
    assert!(
        paths.contains(&queued) && paths.contains(&included),
        "both old and new events are delivered"
    );
    assert!(
        !paths.contains(&excluded),
        "future descendants of the retired subscription are filtered"
    );
    Ok(())
}

#[test]
fn unicode_and_case_aliases_keep_native_coverage() -> TestResult {
    let directory = tempfile::tempdir()?;
    let directory = directory.path().canonicalize()?;
    let root = directory.join("A\u{308}bc");
    std::fs::create_dir(&root)?;
    let composed = directory.join("Äbc");
    let case_alias = directory.join("äBC");
    let cookie_dir = directory.join("cookies");
    std::fs::create_dir(&cookie_dir)?;
    for alias in [composed, case_alias] {
        if !alias.exists() {
            continue; // Case-sensitive volumes need not resolve case aliases.
        }
        let mut watcher = Watcher::new(Options::default())?;
        watcher.replace([
            Watch {
                path: alias,
                recursive: true,
            },
            Watch {
                path: cookie_dir.clone(),
                recursive: true,
            },
        ])?;
        assert_eq!(
            watcher.drain(Budget::default()).loss,
            Some(Loss::WatchSetChanged),
            "installing alias coverage requires an initial baseline"
        );
        let changed = root.join("changed");
        std::fs::write(&changed, "changed")?;
        let fence = watcher.synchronize_at(&cookie_dir, Instant::now() + Duration::from_secs(5))?;
        let batch = watcher.drain(Budget::default());
        assert_eq!(batch.generation, fence.generation, "native coverage remains continuous");
        assert!(
            batch.sequence >= fence.sequence,
            "the marker fences the complete native callback"
        );
        assert!(
            batch
                .events
                .iter()
                .flat_map(|event| &event.paths)
                .any(|path| path == &changed),
            "watching through a filesystem alias must deliver paths in their native spelling: {batch:?}"
        );
    }
    Ok(())
}

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
fn file_path_watches_follow_replacement_and_symlink_retargeting() -> gix_error::TestResult {
    let fixture = tempfile::tempdir()?;
    let scratch = fixture.path().canonicalize()?;
    let root = scratch.join("work");
    let link = root.join("ignore");
    let target = root.join("rules");
    let replacement = root.join("replacement");
    let other = root.join("other-rules");
    let nested = root.join("ignored/deep");
    std::fs::create_dir_all(&nested)?;
    std::fs::write(nested.join("unrelated"), "unrelated")?;

    let mut watcher = Watcher::new_for_files(Options {
        root_check_interval: Some(Duration::from_millis(50)),
        ..Default::default()
    })?;
    watcher.replace([Watch {
        path: link.clone(),
        recursive: false,
    }])?;
    assert_eq!(
        watcher.drain(Budget::default()).loss,
        Some(Loss::WatchSetChanged),
        "initial registration of a missing path needs a baseline"
    );
    let (wake, ready) = std::sync::mpsc::channel();
    watcher.set_waker(move || {
        let _ = wake.send(());
    });
    ready.try_recv()?;
    std::fs::remove_file(nested.join("unrelated"))?;
    assert!(
        ready.recv_timeout(Duration::from_millis(200)).is_err(),
        "a deletion below an unrelated directory does not wake a file watcher"
    );

    let mut wait_for_change = || -> gix_error::TestResult {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let batch = watcher.drain(Budget::default());
            assert!(
                batch.loss.is_none(),
                "rebinding a pathname preserves coverage: {batch:?}"
            );
            assert!(batch.error.is_none(), "file watching stays healthy: {:?}", batch.error);
            if !batch.events.is_empty() {
                assert!(
                    batch
                        .events
                        .iter()
                        .all(|event| event.paths.as_slice() == std::slice::from_ref(&link)),
                    "file watches report the subscribed name for changes to either inode"
                );
                return Ok(());
            }
            ready.recv_timeout(deadline.saturating_duration_since(Instant::now()))?;
        }
    };

    std::fs::write(&link, "file created")?;
    wait_for_change()?;
    std::fs::write(&replacement, "file replaced")?;
    std::fs::rename(&replacement, &link)?;
    wait_for_change()?;
    std::fs::write(&link, "replacement edited")?;
    wait_for_change()?;

    std::os::unix::fs::symlink(&target, &replacement)?;
    std::fs::rename(&replacement, &link)?;
    wait_for_change()?;
    std::fs::write(&target, "missing target created")?;
    wait_for_change()?;
    std::fs::write(&target, "target edited")?;
    wait_for_change()?;
    std::fs::write(&other, "retargeted")?;
    std::os::unix::fs::symlink(&other, &replacement)?;
    std::fs::rename(&replacement, &link)?;
    wait_for_change()?;
    std::fs::write(&other, "new target edited")?;
    wait_for_change()?;
    std::fs::remove_file(&link)?;
    wait_for_change()?;
    std::fs::write(&link, "name recreated")?;
    wait_for_change()?;

    std::fs::rename(&root, scratch.join("moved"))?;
    wait_for_change()?;
    std::fs::create_dir(&root)?;
    std::fs::write(&link, "ancestor recreated")?;
    wait_for_change()?;

    assert_eq!(
        watcher.synchronize_at(&root, Instant::now() + Duration::from_secs(1)),
        Err(SynchronizeError::Unsupported),
        "vnode events do not provide the FSEvents marker contract"
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
