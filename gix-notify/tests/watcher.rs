use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

use gix_notify::{Budget, Loss, Options, SynchronizeError, Watch, Watcher};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

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
    watcher.drain(Budget::default());

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut attempt = 0u32;
    loop {
        std::fs::write(&changed, attempt.to_string())?;
        attempt += 1;
        let _ = receive.recv_timeout(Duration::from_millis(25));
        let batch = watcher.drain(Budget::default());
        assert!(batch.errors.is_empty(), "native delivery failed: {:?}", batch.errors);
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
    assert!(
        watcher.replace([watch.clone()]).is_err(),
        "missing roots require a registration error"
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
