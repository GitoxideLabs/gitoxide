use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::mpsc,
    time::{Duration, Instant},
};

use gix_error::{Class, ErrorExt, TestResult, message};
use gix_notify::{Budget, Loss, Options, SynchronizeError, Watch, Watcher};

#[test]
fn empty_watch_set_and_synchronization_contract() -> TestResult {
    let mut watcher = Watcher::new(Options::default())?;
    watcher.replace([])?;
    let batch = watcher.drain(Budget::default());
    assert_eq!(batch.generation, 0, "an identical empty watch set is a no-op");
    assert!(batch.loss.is_none(), "no coverage has been lost");
    assert_eq!(
        watcher.synchronize(Instant::now() + Duration::from_secs(1)),
        Err(SynchronizeError::Unsupported),
        "an empty queue is not a native delivery barrier"
    );
    Ok(())
}

#[test]
fn invalid_watch_configuration_is_classified_as_validation() -> TestResult {
    for options in [
        Options {
            max_events: 0,
            ..Options::default()
        },
        Options {
            max_bytes: 0,
            ..Options::default()
        },
    ] {
        let error = Watcher::new(options).err().expect("zero queue limits are invalid");
        assert!(
            error.is_validation(),
            "invalid limits are caller input errors: {error:?}"
        );
    }
    let mut watcher = Watcher::new(Options::default())?;
    let error = watcher
        .replace([Watch {
            path: PathBuf::from("relative"),
            recursive: true,
        }])
        .expect_err("watch roots must be absolute");
    assert!(error.is_validation(), "relative roots are invalid input: {error:?}");
    assert_eq!(
        watcher.drain(Budget::default()).generation,
        0,
        "invalid input must not change existing coverage"
    );
    Ok(())
}

#[test]
fn synchronization_errors_retain_their_classification_and_concrete_type() {
    for (failure, class, can_retry) in [
        (SynchronizeError::Unsupported, Class::Unsupported, false),
        (SynchronizeError::TimedOut, Class::Retryable, true),
        (SynchronizeError::Busy, Class::Retryable, true),
        (SynchronizeError::CoverageLost, Class::Conflict, false),
        (SynchronizeError::Stopped, Class::Conflict, false),
    ] {
        let error = failure.and_raise(message("could not synchronize notifications"));
        assert_eq!(error.dominant_class(), Some(class), "classification survives context");
        assert_eq!(
            error.can_retry(),
            can_retry,
            "lost coverage requires recovery, not simply retrying synchronization"
        );
        assert_eq!(
            error.downcast_any_ref::<SynchronizeError>(),
            Some(&failure),
            "classification must not erase the concrete recovery contract"
        );
    }
}

#[test]
fn native_delivery_and_unchanged_registration() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().canonicalize()?;
    let changed = root.join("changed");
    let (send, receive) = mpsc::sync_channel(1);
    let mut watcher = Watcher::new(Options::default())?;
    watcher.set_waker(move || {
        let _ = send.try_send(());
    });
    let watch = Watch {
        path: dir.path().to_path_buf(),
        recursive: true,
    };
    watcher.replace([watch.clone()])?;
    let initial = watcher.drain(Budget::default());
    assert_eq!(
        initial.loss,
        Some(Loss::WatchSetChanged),
        "registration requires a baseline scan"
    );
    watcher.replace([watch.clone()])?;
    let repeated = watcher.drain(Budget::default());
    assert_eq!(
        repeated.generation, initial.generation,
        "unchanged coverage must not restart native streams"
    );
    assert!(
        repeated.loss.is_none(),
        "a healthy unchanged set creates no coverage gap"
    );
    watcher.replace([
        watch.clone(),
        Watch {
            path: watch.path.join("changed"),
            recursive: false,
        },
    ])?;
    assert_eq!(
        watcher.drain(Budget::default()).generation,
        repeated.generation,
        "a child of a recursive root needs no independent native registration"
    );

    let extra = tempfile::tempdir()?;
    watcher.replace([
        watch.clone(),
        Watch {
            path: extra.path().canonicalize()?,
            recursive: false,
        },
    ])?;
    assert_eq!(
        watcher.drain(Budget::default()).loss,
        Some(Loss::WatchSetChanged),
        "adding coverage invalidates the baseline without disabling the existing registration"
    );
    watcher.replace([watch])?;
    let reduced = watcher.drain(Budget::default());
    assert_eq!(
        reduced.loss,
        Some(Loss::WatchSetChanged),
        "removing the extra root invalidates the baseline"
    );
    assert!(
        reduced.error.is_none(),
        "reducing coverage must not fail: {:?}",
        reduced.error
    );

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut attempt = 0u32;
    loop {
        std::fs::write(&changed, attempt.to_string())?;
        attempt += 1;
        let _ = receive.recv_timeout(Duration::from_millis(25));
        let batch = watcher.drain(Budget::default());
        assert!(batch.error.is_none(), "native delivery failed: {:?}", batch.error);
        if batch.loss == Some(Loss::Rescan) || batch.events.iter().any(|event| event.paths.contains(&changed)) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "native watcher did not report a changing file within five seconds"
        );
    }
    watcher.replace([])?;
    assert_eq!(
        watcher.drain(Budget::default()).loss,
        Some(Loss::WatchSetChanged),
        "removing coverage requires invalidation"
    );
    Ok(())
}

#[test]
fn failed_registration_is_retried_for_the_same_set() -> TestResult {
    let dir = tempfile::tempdir()?;
    let missing = dir.path().canonicalize()?.join("missing");
    let watch = Watch {
        path: missing.clone(),
        recursive: true,
    };
    let mut watcher = Watcher::new(Options::default())?;
    let error = watcher
        .replace([watch.clone()])
        .expect_err("missing roots require a registration error");
    assert!(
        error.is_not_found(),
        "the native missing-path classification survives context"
    );
    assert!(!error.is_validation(), "a missing resource is not malformed input");
    assert_eq!(
        error
            .downcast_any_ref::<std::io::Error>()
            .expect("the native cause is retained")
            .kind(),
        std::io::ErrorKind::NotFound,
        "classification must preserve the original I/O error"
    );
    let failed = watcher.drain(Budget::default());
    assert!(
        failed.loss.is_some(),
        "a failed registration cannot leave apparently healthy coverage"
    );
    std::fs::create_dir(&missing)?;
    watcher.replace([watch])?;
    let recovered = watcher.drain(Budget::default());
    assert!(
        recovered.generation > failed.generation,
        "retrying the same desired set establishes a fresh generation"
    );
    Ok(())
}

#[test]
fn root_check_interval_must_be_nonzero_or_disabled() -> TestResult {
    assert_eq!(
        Options::default().root_check_interval,
        Some(Duration::from_secs(1)),
        "periodic root checks retain their one-second default"
    );
    for root_check_interval in [None, Some(Duration::from_millis(25)), Some(Duration::MAX)] {
        Watcher::new(Options {
            root_check_interval,
            ..Options::default()
        })?;
    }
    let error = Watcher::new(Options {
        root_check_interval: Some(Duration::ZERO),
        ..Options::default()
    })
    .err()
    .expect("a zero interval must not create a busy-looping backend; use None to disable checks");
    assert!(error.is_validation(), "a zero interval is invalid caller input");
    Ok(())
}

#[test]
fn application_reconciles_one_shot_changes_and_recovers_from_loss() -> TestResult {
    exercise_application(Options::default())
}

#[test]
fn application_reconciles_without_periodic_root_checks() -> TestResult {
    exercise_application(Options {
        root_check_interval: None,
        ..Options::default()
    })
}

fn exercise_application(options: Options) -> TestResult {
    for options in [
        options,
        // Even one event exceeds this limit, regardless of native batching or coalescing.
        Options {
            max_bytes: 1,
            ..options
        },
    ] {
        let directory = tempfile::tempdir()?;
        let root = directory.path().canonicalize()?.join("watched");
        fs::create_dir_all(root.join("before/nested"))?;
        fs::write(root.join("before/nested/file"), "baseline")?;
        for name in ["edited", "replaced", "deleted"] {
            fs::write(root.join(name), "baseline")?;
        }
        let mut application = Application::new(root.clone(), options)?;
        assert_converges(&mut application, "initial baseline")?;
        assert!(
            !application.losses.is_empty(),
            "registration requires a baseline even if newer loss supersedes WatchSetChanged"
        );

        let created = root.join("created");
        fs::write(&created, "created once")?;
        assert_converges(&mut application, "file creation")?;
        fs::write(root.join("edited"), "edited once")?;
        assert_converges(&mut application, "content change")?;

        // Prepare outside coverage so only the replacement can invalidate the cached file.
        let replacement = directory.path().join("replacement");
        fs::write(&replacement, "atomically replaced")?;
        fs::rename(&replacement, root.join("replaced"))?;
        assert_converges(&mut application, "atomic file replacement")?;

        let renamed = root.join("renamed");
        fs::rename(&created, &renamed)?;
        assert_converges(&mut application, "file rename")?;
        fs::remove_file(root.join("deleted"))?;
        assert_converges(&mut application, "file deletion")?;

        fs::create_dir_all(root.join("new/nested"))?;
        fs::write(
            root.join("new/nested/early"),
            "written before descendant watches may exist",
        )?;
        assert_converges(&mut application, "new populated subtree")?;

        let incoming = directory.path().join("incoming");
        fs::create_dir_all(incoming.join("nested"))?;
        fs::write(incoming.join("nested/file"), "populated outside coverage")?;
        fs::rename(&incoming, root.join("imported"))?;
        assert_converges(&mut application, "subtree moved into coverage")?;

        fs::rename(root.join("before"), root.join("after"))?;
        assert_converges(&mut application, "subtree rename")?;
        fs::write(root.join("after/nested/file"), "edited after moving")?;
        assert_converges(&mut application, "continued coverage of renamed descendants")?;
        fs::remove_dir_all(root.join("after"))?;
        assert_converges(&mut application, "subtree deletion")?;

        if options.max_bytes == 1 {
            assert!(
                application.losses.contains(&Loss::Overflow),
                "the bounded queue must exercise full-scan recovery, not just incremental refreshes"
            );
        }
    }
    Ok(())
}

/// A file-content index driven only by notifications, with no periodic scan fallback.
struct Application {
    watcher: Watcher,
    watch: Watch,
    receive: mpsc::Receiver<()>,
    files: BTreeMap<PathBuf, Vec<u8>>,
    losses: Vec<Loss>,
}

impl Application {
    fn new(root: PathBuf, options: Options) -> TestResult<Self> {
        let (send, receive) = mpsc::sync_channel(1);
        let mut watcher = Watcher::new(options)?;
        watcher.set_waker(move || {
            let _ = send.try_send(());
        });
        let watch = Watch {
            path: root,
            recursive: true,
        };
        watcher.replace([watch.clone()])?;
        Ok(Self {
            watcher,
            watch,
            receive,
            files: BTreeMap::new(),
            losses: Vec::new(),
        })
    }

    /// Return whether the event loop should keep draining without waiting for another wake.
    fn poll(&mut self) -> TestResult<bool> {
        let batch = self.watcher.drain(Budget {
            max_events: 1,
            ..Budget::default()
        });
        if let Some(loss) = batch.loss {
            self.losses.push(loss);
            // Healthy registrations are unchanged; failed coverage is restored before scanning.
            self.watcher.replace([self.watch.clone()])?;
            self.files.clear();
            read_files(&self.watch.path, &mut self.files)?;
            // Retrying registration may itself have queued WatchSetChanged. Consume it too.
            return Ok(true);
        }
        assert!(batch.error.is_none(), "native delivery failed without loss: {batch:?}");
        for event in batch.events {
            // Kinds are hints, not operations to replay. Refresh every endpoint and invalidate
            // cached descendants even when the path no longer exists or has changed its type.
            for path in event.paths {
                if path.starts_with(&self.watch.path) {
                    self.files.retain(|cached, _| !cached.starts_with(&path));
                    read_files(&path, &mut self.files)?;
                }
            }
        }
        Ok(batch.more)
    }
}

fn read_files(path: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) -> TestResult {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
            ) =>
        {
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };
    if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            read_files(&entry?.path(), files)?;
        }
    } else if metadata.is_file() {
        files.insert(path.to_path_buf(), fs::read(path)?);
    }
    Ok(())
}

fn assert_converges(application: &mut Application, phase: &str) -> TestResult {
    // This full scan is only the test oracle: the application never receives it. Each mutation
    // happens once, before polling; the wait loop never writes or refreshes the cache itself.
    let mut expected = BTreeMap::new();
    read_files(&application.watch.path, &mut expected)?;
    assert_ne!(
        application.files, expected,
        "{phase} must make the index stale before notifications are processed"
    );
    // Also allow compatibility backends using notify's default 30-second polling interval.
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let more = application.poll()?;
        if application.files == expected && !more {
            return Ok(());
        }
        assert!(
            Instant::now() < deadline,
            "{phase}: the index did not converge; losses: {:?}, cached: {:?}, expected: {expected:?}",
            application.losses,
            application.files
        );
        if !more {
            // An empty queue is not a delivery barrier. Wait for a wake, not a fixed sleep.
            assert!(
                application
                    .receive
                    .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                    .is_ok(),
                "{phase}: no wake before the deadline; losses: {:?}, cached: {:?}, expected: {expected:?}",
                application.losses,
                application.files
            );
        }
    }
}
