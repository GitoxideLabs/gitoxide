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
    assert!(batch.error.is_some(), "recovery needs a diagnostic");
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
        assert_eq!(
            stream.sender.fence(batch.generation),
            Err(SynchronizeError::CoverageLost),
            "loss stays latched until native watches are rebuilt"
        );
    }
    Ok(())
}
