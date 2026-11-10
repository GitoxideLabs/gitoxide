use std::{
    ffi::OsString,
    os::unix::{ffi::OsStringExt, fs::symlink},
    path::Path,
    time::{Duration, Instant},
};

use gix_error::TestResult;
use gix_notify::{Budget, Event, EventKind, Loss, Options, PathKind, SynchronizeError, Watch, Watcher};

fn watch(path: &Path, recursive: bool) -> TestResult<Watcher> {
    let mut watcher = Watcher::new(Options::default())?;
    watcher.replace([Watch {
        path: path.to_owned(),
        recursive,
    }])?;
    assert_eq!(
        watcher.drain(Budget::default()).loss,
        Some(Loss::WatchSetChanged),
        "installing coverage requires an initial baseline"
    );
    Ok(watcher)
}

fn synchronize(watcher: &mut Watcher, directory: &Path) -> TestResult<Vec<Event>> {
    let fence = watcher.synchronize_at(directory, Instant::now() + Duration::from_secs(5))?;
    let mut events = Vec::new();
    loop {
        let batch = watcher.drain(Budget {
            max_events: 1,
            max_bytes: usize::MAX,
        });
        assert_eq!(
            batch.generation, fence.generation,
            "the fence retains continuous coverage"
        );
        assert!(batch.loss.is_none(), "ordinary changes preserve coverage: {batch:?}");
        assert!(batch.error.is_none(), "native delivery succeeds: {batch:?}");
        events.extend(batch.events);
        if batch.sequence >= fence.sequence {
            break;
        }
        assert!(
            batch.more,
            "every publication preceding the fence is available to drain"
        );
    }
    Ok(events)
}

fn contains(events: &[Event], path: &Path) -> bool {
    events
        .iter()
        .any(|event| event.paths.iter().any(|candidate| candidate == path))
}

#[test]
fn missing_watch_roots_preserve_native_errors() -> TestResult {
    let directory = tempfile::tempdir()?;
    let missing = directory.path().canonicalize()?.join("missing");
    let mut watcher = Watcher::new(Options::default())?;
    let error = watcher
        .replace([Watch {
            path: missing,
            recursive: true,
        }])
        .expect_err("a missing root cannot be registered");
    assert!(
        error.is_not_found(),
        "the native missing-path classification remains visible"
    );
    assert!(!error.is_validation(), "an absent root is not malformed caller input");
    assert!(
        !error.is_conflict(),
        "initial lookup failure does not establish changed coverage"
    );
    assert!(
        !error.is_retryable(),
        "a lookup failure does not promise recovery by retrying"
    );
    assert_eq!(
        error
            .downcast_any_ref::<std::io::Error>()
            .expect("native cause retained")
            .raw_os_error(),
        Some(rustix::io::Errno::NOENT.raw_os_error()),
        "registration context preserves the original OS error"
    );
    Ok(())
}

#[test]
fn immediate_writes_across_roots_are_covered_by_fences() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    let first = root.join("first");
    let second = root.join("second");
    std::fs::create_dir(&first)?;
    std::fs::create_dir(&second)?;
    let mut watcher = watch(&first, true)?;
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
        "adding a root invalidates the baseline"
    );
    for iteration in 0..10 {
        let paths = [
            first.join(format!("first-{iteration}")),
            second.join(format!("second-{iteration}")),
        ];
        for path in &paths {
            std::fs::write(path, "changed")?;
        }
        let events = synchronize(&mut watcher, &first)?;
        for path in paths {
            assert!(
                contains(&events, &path),
                "an immediate write precedes the fence: {path:?}"
            );
        }
    }
    assert_eq!(
        watcher.synchronize_at(&first, Instant::now()),
        Err(SynchronizeError::TimedOut),
        "an expired deadline cannot return a successful fence"
    );
    Ok(())
}

#[test]
fn file_scope_filters_siblings_and_accepts_its_native_parent_anchor() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    let file = root.join("watched");
    let sibling = root.join("unwatched");
    std::fs::write(&file, "baseline")?;
    let mut watcher = watch(&file, false)?;
    let outside = tempfile::tempdir()?;
    assert_eq!(
        watcher.synchronize_at(outside.path(), Instant::now() + Duration::from_secs(5)),
        Err(SynchronizeError::Unsupported),
        "an unrelated directory cannot fence the native stream"
    );
    assert_eq!(
        std::fs::read_dir(outside.path())?.count(),
        0,
        "unsupported anchors stay untouched"
    );
    std::fs::write(&sibling, "unrelated")?;
    std::fs::write(&file, "changed")?;
    let events = synchronize(&mut watcher, &root)?;
    assert!(contains(&events, &file), "file-scoped writes are delivered");
    assert!(
        !contains(&events, &sibling),
        "native parent coverage does not leak sibling events"
    );
    std::fs::rename(&sibling, &file)?;
    assert!(
        contains(&synchronize(&mut watcher, &root)?, &file),
        "replacing a watched file invalidates its name"
    );
    std::fs::write(&file, "replacement changed")?;
    assert!(
        contains(&synchronize(&mut watcher, &root)?, &file),
        "file watches continue observing replacement inodes"
    );
    Ok(())
}

#[test]
fn nonrecursive_watches_exclude_nested_changes_and_marker_directories() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    let child = root.join("child");
    std::fs::create_dir(&child)?;
    let mut watcher = watch(&root, false)?;
    assert_eq!(
        watcher.synchronize_at(&child, Instant::now() + Duration::from_secs(5)),
        Err(SynchronizeError::Unsupported),
        "a nonrecursive root does not observe marker files in child directories"
    );
    assert_eq!(
        std::fs::read_dir(&child)?.count(),
        0,
        "unsupported anchors stay untouched"
    );
    let nested = child.join("nested");
    let direct = root.join("direct");
    std::fs::write(&nested, "unwatched")?;
    std::fs::write(&direct, "watched")?;
    let events = synchronize(&mut watcher, &root)?;
    assert!(contains(&events, &direct), "direct children are covered");
    assert!(
        !contains(&events, &nested),
        "nonrecursive coverage excludes grandchildren"
    );
    Ok(())
}

#[test]
fn recursive_watches_cover_existing_new_and_moved_in_subtrees() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    let watched = root.join("watched");
    let existing = watched.join("existing/nested");
    let incoming = root.join("incoming");
    std::fs::create_dir_all(&existing)?;
    std::fs::create_dir_all(incoming.join("nested"))?;
    let mut watcher = watch(&watched, true)?;
    let existing_file = existing.join("changed");
    std::fs::write(&existing_file, "existing")?;
    assert!(
        contains(&synchronize(&mut watcher, &watched)?, &existing_file),
        "initial descendants are watched"
    );

    let created = watched.join("created");
    std::fs::create_dir_all(created.join("nested"))?;
    std::fs::write(created.join("nested/early"), "before registration may finish")?;
    let moved = watched.join("moved");
    std::fs::rename(&incoming, &moved)?;
    let events = synchronize(&mut watcher, &watched)?;
    for path in [&created, &moved] {
        assert!(
            events
                .iter()
                .any(|event| event.paths.contains(path) && event.path_kind == PathKind::Directory),
            "a directory invalidation covers contents populated before descendant watches: {path:?}"
        );
    }
    for subtree in [created, moved] {
        let path = subtree.join("nested/later");
        std::fs::write(&path, "after registration")?;
        assert!(
            contains(&synchronize(&mut watcher, &watched)?, &path),
            "new descendants remain covered: {path:?}"
        );
    }
    Ok(())
}

#[test]
fn recursive_watches_do_not_follow_symlinks_and_preserve_byte_paths() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    let watched = root.join("watched");
    let outside = root.join("outside");
    std::fs::create_dir(&watched)?;
    std::fs::create_dir(&outside)?;
    symlink(&outside, watched.join("link"))?;
    symlink(&watched, watched.join("loop"))?;
    let alias = root.join("alias");
    symlink(&watched, &alias)?;
    let mut watcher = watch(&alias, true)?;
    let outside_file = outside.join("unwatched");
    let changed = watched.join(OsString::from_vec(b"changed-\xff".to_vec()));
    std::fs::write(&outside_file, "outside")?;
    std::fs::write(&changed, "inside")?;
    let events = synchronize(&mut watcher, &watched)?;
    assert!(contains(&events, &changed), "inotify names preserve non-UTF-8 bytes");
    assert!(
        !contains(&events, &outside_file),
        "symlink targets stay outside recursive coverage"
    );
    assert!(
        !contains(&events, &watched.join("link/unwatched")),
        "symlink aliases are not recursively watched"
    );
    Ok(())
}

#[test]
fn file_renames_and_removal_invalidate_paths() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    let before = root.join("before");
    let after = root.join("after");
    std::fs::write(&before, "baseline")?;
    let mut watcher = watch(&root, true)?;
    std::fs::rename(&before, &after)?;
    let events = synchronize(&mut watcher, &root)?;
    for path in [&before, &after] {
        assert!(
            events
                .iter()
                .any(|event| event.kind == EventKind::Rename && event.paths.contains(path)),
            "both rename endpoints must be invalidated, whether paired or separate: {path:?}"
        );
    }
    std::fs::remove_file(&after)?;
    let events = synchronize(&mut watcher, &root)?;
    assert!(
        events
            .iter()
            .any(|event| event.kind == EventKind::Remove && event.paths.contains(&after)),
        "removed names are delivered even though they cannot be resolved"
    );
    Ok(())
}

#[test]
fn renamed_subtree_never_silently_keeps_stale_paths() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    let before = root.join("before");
    let after = root.join("after");
    std::fs::create_dir_all(before.join("nested"))?;
    let mut watcher = watch(&root, true)?;
    std::fs::rename(&before, &after)?;
    match watcher.synchronize_at(&root, Instant::now() + Duration::from_secs(5)) {
        Ok(_) => {}
        Err(SynchronizeError::CoverageLost) => {
            assert!(
                watcher.drain(Budget::default()).loss.is_some(),
                "lost subtree coverage requires a baseline"
            );
            watcher.replace([Watch {
                path: root.clone(),
                recursive: true,
            }])?;
        }
        Err(error) => return Err(error.into()),
    }
    let batch = watcher.drain(Budget::default());
    assert!(batch.error.is_none(), "subtree recovery must not fail: {batch:?}");
    let changed = after.join("nested/changed");
    std::fs::write(&changed, "new path")?;
    let events = synchronize(&mut watcher, &root)?;
    assert!(
        contains(&events, &changed),
        "renamed descendants are covered after any required restart"
    );
    assert!(
        !contains(&events, &before.join("nested/changed")),
        "the obsolete prefix cannot be used for later writes"
    );
    Ok(())
}

#[test]
fn replacing_a_root_identity_invalidates_its_fence() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?.join("root");
    std::fs::create_dir(&root)?;
    let mut watcher = watch(&root, true)?;
    let generation = watcher.drain(Budget::default()).generation;
    std::fs::rename(&root, directory.path().join("old-root"))?;
    std::fs::create_dir(&root)?;
    assert_eq!(
        watcher.synchronize_at(&root, Instant::now() + Duration::from_secs(5)),
        Err(SynchronizeError::CoverageLost),
        "a watch of the original inode cannot fence its replacement"
    );
    let batch = watcher.drain(Budget::default());
    assert!(
        batch.generation > generation && batch.loss.is_some(),
        "root replacement invalidates the baseline"
    );
    assert!(
        batch
            .error
            .as_ref()
            .expect("root replacement has a diagnostic")
            .is_conflict(),
        "replacing watched inodes requires re-registration, not an unconditional retry"
    );
    watcher.replace([Watch {
        path: root.clone(),
        recursive: true,
    }])?;
    assert_eq!(
        watcher.drain(Budget::default()).loss,
        Some(Loss::WatchSetChanged),
        "registering the replacement root requires a new baseline"
    );
    let changed = root.join("changed");
    std::fs::write(&changed, "replacement")?;
    assert!(
        contains(&synchronize(&mut watcher, &root)?, &changed),
        "re-registration recovers the replacement root"
    );
    Ok(())
}

#[test]
fn moving_a_subtree_outside_latches_loss_until_registration_is_retried() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    let watched = root.join("watched");
    let before = watched.join("before");
    let outside = root.join("outside");
    std::fs::create_dir_all(before.join("nested"))?;
    let mut watcher = watch(&watched, true)?;
    std::fs::rename(&before, &outside)?;
    assert_eq!(
        watcher.synchronize_at(&watched, Instant::now() + Duration::from_secs(5)),
        Err(SynchronizeError::CoverageLost),
        "moving watched inodes outside the tree invalidates recursive coverage"
    );
    let batch = watcher.drain(Budget::default());
    assert!(batch.loss.is_some(), "coverage loss reaches the consumer");
    assert!(
        batch
            .error
            .as_ref()
            .expect("subtree move has a diagnostic")
            .is_conflict(),
        "moved watches conflict with the registered topology"
    );
    assert_eq!(
        watcher.synchronize_at(&watched, Instant::now() + Duration::from_secs(5)),
        Err(SynchronizeError::CoverageLost),
        "draining loss does not repair watches still attached to moved inodes"
    );
    std::fs::write(outside.join("nested/before-retry"), "outside coverage")?;
    watcher.replace([Watch {
        path: watched.clone(),
        recursive: true,
    }])?;
    assert_eq!(
        watcher.drain(Budget::default()).loss,
        Some(Loss::WatchSetChanged),
        "re-registering after a subtree move requires a new baseline"
    );
    std::fs::write(outside.join("nested/after-retry"), "outside coverage")?;
    let changed = watched.join("changed");
    std::fs::write(&changed, "inside coverage")?;
    let events = synchronize(&mut watcher, &watched)?;
    assert!(
        contains(&events, &changed),
        "re-registration restores delivery inside the root"
    );
    assert!(
        events
            .iter()
            .flat_map(|event| &event.paths)
            .all(|path| !path.starts_with(&before) && !path.starts_with(&outside)),
        "the replacement generation never delivers stale paths from moved-out watches: {events:?}"
    );
    Ok(())
}

#[test]
fn deleting_and_recreating_a_subtree_requires_fresh_registration() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    let subtree = root.join("subtree/nested");
    std::fs::create_dir_all(&subtree)?;
    let mut watcher = watch(&root, true)?;
    std::fs::remove_dir_all(root.join("subtree"))?;
    assert_eq!(
        watcher.synchronize_at(&root, Instant::now() + Duration::from_secs(5)),
        Err(SynchronizeError::CoverageLost),
        "deleted native watches invalidate recursive coverage"
    );
    let batch = watcher.drain(Budget::default());
    assert!(
        batch.loss.is_some(),
        "deleted subtree coverage loss reaches the consumer"
    );
    assert!(
        batch
            .error
            .as_ref()
            .expect("subtree deletion has a diagnostic")
            .is_conflict(),
        "deleted watches require registration to be refreshed"
    );
    std::fs::create_dir_all(&subtree)?;
    assert_eq!(
        watcher.synchronize_at(&root, Instant::now() + Duration::from_secs(5)),
        Err(SynchronizeError::CoverageLost),
        "recreating directory names alone cannot restore coverage"
    );
    watcher.replace([Watch {
        path: root.clone(),
        recursive: true,
    }])?;
    assert_eq!(
        watcher.drain(Budget::default()).loss,
        Some(Loss::WatchSetChanged),
        "registering the recreated subtree requires a new baseline"
    );
    let changed = subtree.join("changed");
    std::fs::write(&changed, "new subtree")?;
    assert!(
        contains(&synchronize(&mut watcher, &root)?, &changed),
        "new subtree inodes are watched after recovery"
    );
    Ok(())
}

#[test]
fn bounded_queue_overflow_changes_generation_without_losing_native_coverage() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    let mut watcher = Watcher::new(Options {
        max_events: 8,
        max_bytes: usize::MAX,
        ..Options::default()
    })?;
    watcher.replace([Watch {
        path: root.clone(),
        recursive: true,
    }])?;
    let initial = watcher.drain(Budget::default());
    for index in 0..64 {
        std::fs::write(root.join(format!("changed-{index}")), "overflow")?;
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut overflow_seen = false;
    let generation = loop {
        let result = watcher.synchronize_at(&root, deadline);
        let batch = watcher.drain(Budget::default());
        overflow_seen |= batch.loss == Some(Loss::Overflow);
        assert!(
            batch.error.is_none(),
            "application queue overflow is not a native backend failure"
        );
        match result {
            Ok(fence) if batch.generation == fence.generation => {
                assert!(
                    batch.sequence >= fence.sequence,
                    "overflow explicitly invalidates dropped publications"
                );
                break fence.generation;
            }
            Ok(fence) => {
                assert!(
                    batch.generation > fence.generation,
                    "marker cleanup can overflow after a fence completes"
                );
                assert_eq!(batch.loss, Some(Loss::Overflow), "post-fence loss must remain visible");
            }
            Err(SynchronizeError::CoverageLost) => {
                assert!(
                    batch.generation > initial.generation,
                    "overflow invalidates a pending fence's generation"
                );
            }
            Err(error) => return Err(error.into()),
        }
    };
    assert!(
        overflow_seen && generation > initial.generation,
        "bounded overflow cannot hide the required baseline scan"
    );
    let changed = root.join("after-overflow");
    std::fs::write(&changed, "recovered")?;
    assert!(
        contains(&synchronize(&mut watcher, &root)?, &changed),
        "native coverage remains usable after draining overflow"
    );
    Ok(())
}
