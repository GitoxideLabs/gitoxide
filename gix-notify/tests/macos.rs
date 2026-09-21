#![cfg(target_os = "macos")]

use gix_notify::{Budget, Options, SynchronizeError, Watch, Watcher};
use std::time::{Duration, Instant};

#[test]
fn immediate_writes_across_roots_are_covered_by_native_fences() -> Result<(), Box<dyn std::error::Error>> {
    let first = tempfile::tempdir()?;
    let second = tempfile::tempdir()?;
    let first = first.path().canonicalize()?;
    let second = second.path().canonicalize()?;
    let mut watcher = Watcher::new(Options::default()).map_err(gix_error::Exn::into_error)?;
    watcher
        .replace([
            Watch {
                path: first.clone(),
                recursive: true,
            },
            Watch {
                path: second.clone(),
                recursive: true,
            },
        ])
        .map_err(gix_error::Exn::into_error)?;
    watcher.drain(Budget::default());

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
fn file_scope_filters_siblings_and_rejects_uncovered_marker_directories() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let directory = directory.path().canonicalize()?;
    let file = directory.join("watched");
    let sibling = directory.join("unwatched");
    std::fs::write(&file, "baseline")?;
    let mut watcher = Watcher::new(Options::default()).map_err(gix_error::Exn::into_error)?;
    watcher
        .replace([Watch {
            path: file.clone(),
            recursive: false,
        }])
        .map_err(gix_error::Exn::into_error)?;
    watcher.drain(Budget::default());
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
fn replacing_a_root_identity_invalidates_its_fence() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?.join("root");
    std::fs::create_dir(&root)?;
    let mut watcher = Watcher::new(Options::default()).map_err(gix_error::Exn::into_error)?;
    watcher
        .replace([Watch {
            path: root.clone(),
            recursive: true,
        }])
        .map_err(gix_error::Exn::into_error)?;
    let initial = watcher.drain(Budget::default());
    std::fs::rename(&root, directory.path().join("old-root"))?;
    std::fs::create_dir(&root)?;
    assert_eq!(
        watcher.synchronize_at(&root, Instant::now() + Duration::from_secs(5)),
        Err(SynchronizeError::CoverageLost),
        "a stream still watching the original object cannot certify a replacement at the same path"
    );
    let batch = watcher.drain(Budget::default());
    assert!(
        batch.generation > initial.generation && batch.loss.is_some(),
        "root replacement invalidates the consumer baseline"
    );
    Ok(())
}
