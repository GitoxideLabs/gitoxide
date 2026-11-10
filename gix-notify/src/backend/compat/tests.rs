use super::*;
use notify::event::{AccessKind, AccessMode, ModifyKind, RenameMode};

#[cfg(any(
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
#[test]
fn kqueue_topology_changes_require_registration_before_a_new_baseline() {
    use gix_parallel::{Mutable, OwnShared, lock};

    for kind in [
        Kind::Create(notify::event::CreateKind::File),
        Kind::Remove(notify::event::RemoveKind::Any),
        Kind::Modify(ModifyKind::Name(RenameMode::Any)),
    ] {
        let state = OwnShared::new(Mutable::new(queue::State::new(crate::Options::default())));
        let sender = queue::Sender::new(state.clone(), 0);
        receive(&sender, Ok(notify::Event::new(kind).add_path("/watched/file".into())));
        let batch = lock(&state).drain(crate::Budget::default());
        assert_eq!(
            batch.loss,
            Some(Loss::Rescan),
            "incomplete topology cannot certify incremental paths"
        );
        assert!(
            batch
                .error
                .expect("topology changes require coverage recovery")
                .is_conflict(),
            "the caller must re-register before scanning"
        );
        assert!(
            lock(&state).needs_restart,
            "unchanged logical watches must still restore native coverage"
        );
    }
}

#[test]
fn access_filter_preserves_close_write() {
    assert!(
        translate(notify::Event::new(Kind::Access(AccessKind::Read))).is_none(),
        "reads do not dirty paths"
    );
    let event = translate(notify::Event::new(Kind::Access(AccessKind::Close(AccessMode::Write))));
    assert_eq!(
        event.map(|event| event.kind),
        Some(EventKind::Modify),
        "closing a writable file can change its contents"
    );
}

#[test]
fn rescan_precedes_access_filtering_and_missing_paths_invalidate() {
    use gix_parallel::{Mutable, OwnShared, lock};

    let state = OwnShared::new(Mutable::new(queue::State::new(crate::Options::default())));
    let sender = queue::Sender::new(state.clone(), 0);
    receive(
        &sender,
        Ok(notify::Event::new(Kind::Access(AccessKind::Read)).set_flag(notify::event::Flag::Rescan)),
    );
    let batch = lock(&state).drain(crate::Budget::default());
    assert_eq!(
        batch.loss,
        Some(Loss::Rescan),
        "access filtering must not hide native loss flags"
    );
    assert!(batch.events.is_empty(), "read-only events are still filtered");
    assert!(
        batch.error.is_none(),
        "rescan alone is not a classified backend failure"
    );
    receive(&sender, Ok(notify::Event::new(Kind::Any)));
    assert_eq!(
        lock(&state).drain(crate::Budget::default()).loss,
        Some(Loss::Rescan),
        "changes without a path invalidate every watched root"
    );
}

#[test]
fn backend_failure_classifies_lost_coverage_without_reinterpreting_the_cause() -> gix_error::TestResult {
    use gix_parallel::{Mutable, OwnShared, lock};
    use std::io;

    let state = OwnShared::new(Mutable::new(queue::State::new(crate::Options::default())));
    let sender = queue::Sender::new(state.clone(), 0);
    let path = std::path::PathBuf::from("watched");
    let native = io::Error::from_raw_os_error(12345);
    let native_kind = native.kind();
    for cause in [
        notify::Error::generic("opaque backend failure"),
        notify::Error::io(native),
    ] {
        receive(&sender, Err(cause.add_path(path.clone())));
        let batch = lock(&state).drain(crate::Budget::default());
        assert_eq!(
            batch.loss,
            Some(Loss::BackendError),
            "backend errors require re-registration"
        );
        let error = batch.error.expect("the original backend failure is retained");
        assert!(
            error.is_conflict(),
            "coverage must be re-established before relying on notifications"
        );
        assert!(
            !error.is_retryable(),
            "an opaque failure does not establish retryability"
        );
        assert!(!error.is_validation(), "the backend did not reject caller input");
        assert!(!error.is_corrupted(), "an opaque failure does not establish corruption");
        assert!(
            !error.is_resource_exhausted(),
            "an opaque failure does not establish exhaustion"
        );
        let cause = error
            .downcast_any_ref::<notify::Error>()
            .expect("notify's concrete cause is preserved");
        assert_eq!(cause.paths, [path.clone()], "native diagnostic paths are retained");
        match &cause.kind {
            notify::ErrorKind::Generic(message) => {
                assert_eq!(message, "opaque backend failure", "the opaque diagnostic is unchanged")
            }
            notify::ErrorKind::Io(native) => {
                assert_eq!(
                    native.raw_os_error(),
                    Some(12345),
                    "the original OS error is preserved inside notify's payload"
                );
                assert_eq!(native.kind(), native_kind, "the original I/O kind is unchanged");
            }
            other => panic!("the native error kind was changed: {other:?}"),
        }
    }
    Ok(())
}

#[test]
fn rename_retains_both_endpoints_without_requiring_them_to_exist() {
    let from = std::path::PathBuf::from("/old");
    let to = std::path::PathBuf::from("/new");
    let event = translate(
        notify::Event::new(Kind::Modify(ModifyKind::Name(RenameMode::Both)))
            .add_path(from.clone())
            .add_path(to.clone()),
    );
    assert_eq!(
        event,
        Some(Event {
            paths: vec![from, to],
            kind: EventKind::Rename,
            path_kind: PathKind::Any,
        }),
        "both sides need invalidation, and no stat call can reliably determine the old kind"
    );
}
