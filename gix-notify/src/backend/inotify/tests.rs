use super::*;
use crate::{Budget, Options};
use gix_error::TestResult;
use gix_parallel::{Mutable, lock};
use std::ffi::OsStr;

fn setup() -> TestResult<(tempfile::TempDir, Stream, OwnShared<Mutable<queue::State>>)> {
    let directory = tempfile::tempdir()?;
    let state = OwnShared::new(Mutable::new(queue::State::new(Options::default())));
    let sender = OwnShared::new(queue::Sender::new(state.clone(), 0));
    let stream = Stream::new(
        &[Watch {
            path: directory.path().canonicalize()?,
            recursive: true,
        }],
        sender,
    )?;
    Ok((directory, stream, state))
}

fn event<'a>(stream: &Stream, name: Option<&'a str>, mask: EventMask) -> inotify::Event<&'a OsStr> {
    inotify::Event {
        wd: stream
            .paths
            .values()
            .next()
            .expect("setup installed a root watch")
            .clone(),
        mask,
        cookie: 0,
        name: name.map(OsStr::new),
    }
}

#[test]
fn stopped_workers_report_conflicts_and_preserve_channel_causes() -> TestResult {
    for disconnected in [false, true] {
        let (commands, receive) = mpsc::sync_channel(1);
        drop(receive);
        let mut backend = Backend {
            commands: disconnected.then_some(commands),
            wake: OwnShared::new(eventfd(0, EventfdFlags::NONBLOCK | EventfdFlags::CLOEXEC)?),
            worker: None,
        };
        let error = backend
            .replace(&[])
            .expect_err("a stopped worker cannot replace watches");
        assert!(
            error.is_conflict(),
            "the backend must be recreated before replacing watches"
        );
        assert!(
            !error.is_retryable(),
            "retrying the same stopped worker cannot recover it"
        );
        if disconnected {
            assert!(
                error.downcast_any_ref::<mpsc::SendError<Command>>().is_some(),
                "the original channel disconnection remains available"
            );
        }
    }
    Ok(())
}

#[test]
fn directory_aliases_are_unsupported() -> TestResult {
    let (directory, mut stream, _state) = setup()?;
    let before = directory.path().join("before");
    let after = directory.path().join("after");
    std::fs::create_dir(&before)?;
    stream.add_tree(&before, false)?;
    // Moving the watched inode produces a second path for the same native descriptor without a bind mount.
    std::fs::rename(&before, &after)?;
    let error = stream
        .add_tree(&after, false)
        .expect_err("one descriptor cannot map to two paths");
    assert!(
        error.is_unsupported(),
        "directory aliases need an unsupported descriptor mapping"
    );
    assert!(!error.is_validation(), "both directory paths are valid");
    Ok(())
}

#[test]
fn kernel_overflow_after_marker_requires_reinstallation_even_after_draining() -> TestResult {
    let (directory, mut stream, state) = setup()?;
    let marker = directory.path().join("marker");
    let events = [
        event(&stream, Some("marker"), EventMask::CREATE),
        event(&stream, None, EventMask::Q_OVERFLOW),
    ];
    assert!(
        stream.publish_batch(events, Some(&marker)),
        "the marker was present in the read"
    );
    let batch = lock(&state).drain(Budget::default());
    assert_eq!(
        batch.loss,
        Some(Loss::Rescan),
        "kernel overflow can lose watch topology"
    );
    let error = batch.error.as_ref().expect("recovery needs a diagnostic");
    assert!(
        error.is_conflict(),
        "kernel overflow invalidates recursive watch coverage"
    );
    assert!(
        !error.is_retryable(),
        "recovery requires reinstallation, not a blind retry"
    );
    assert!(
        !error.is_resource_exhausted(),
        "kernel event loss is not an application allocation limit or allocation failure"
    );
    assert!(stream.failed, "the old directory map cannot continue delivering paths");
    assert_eq!(
        stream.sender.fence(batch.generation),
        Err(SynchronizeError::CoverageLost),
        "acknowledging the loss does not repair missing native watches"
    );
    Ok(())
}

#[test]
fn fences_include_paths_after_the_marker_and_reject_old_markers() -> TestResult {
    let (directory, mut stream, state) = setup()?;
    let marker = directory.path().join("marker");
    let events = [
        event(&stream, Some("before"), EventMask::MODIFY),
        event(&stream, Some("marker"), EventMask::CREATE),
        event(&stream, Some("after"), EventMask::CREATE | EventMask::ISDIR),
    ];
    std::fs::create_dir(directory.path().join("after"))?;
    assert!(
        stream.publish_batch(events, Some(&marker)),
        "the active marker is recognized"
    );
    let fence = stream.sender.fence(0)?;
    let batch = lock(&state).drain(Budget::default());
    assert_eq!(batch.sequence, fence.sequence, "the entire read precedes the fence");
    assert_eq!(batch.events.len(), 2, "only the active marker is filtered");
    assert_eq!(
        batch.events[1].paths,
        [directory.path().join("after")],
        "later paths are included"
    );
    assert_eq!(
        batch.events[1].path_kind,
        PathKind::Directory,
        "directory invalidations cover descendants"
    );
    let events = [event(&stream, Some("old-marker"), EventMask::CREATE)];
    assert!(
        !stream.publish_batch(events, Some(&marker)),
        "an earlier timed-out marker cannot satisfy a new request"
    );
    Ok(())
}

#[test]
fn unknown_descriptors_and_removed_watches_invalidate_coverage() -> TestResult {
    for unknown in [false, true] {
        let (directory, mut stream, state) = setup()?;
        let mut notification = event(&stream, None, EventMask::IGNORED);
        if unknown {
            let child = directory.path().join("unmapped");
            std::fs::create_dir(&child)?;
            notification.wd = stream.native.watches().add(child, WatchMask::CREATE)?;
            notification.mask = EventMask::CREATE;
        }
        stream.publish_batch([notification], None);
        let batch = lock(&state).drain(Budget::default());
        assert_eq!(
            batch.loss,
            Some(Loss::Rescan),
            "unusable descriptor mappings lose coverage"
        );
        let error = batch.error.as_ref().expect("lost coverage has a diagnostic");
        assert!(
            error.is_conflict(),
            "the current watch topology cannot certify coverage"
        );
        assert!(
            !error.is_corrupted(),
            "unmapped descriptors do not prove malformed native data"
        );
        assert_eq!(
            stream.sender.fence(batch.generation),
            Err(SynchronizeError::CoverageLost),
            "loss stays latched until native watches are rebuilt"
        );
    }
    Ok(())
}
