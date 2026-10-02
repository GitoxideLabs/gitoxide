use super::*;
use notify::event::{AccessKind, AccessMode, ModifyKind, RenameMode};

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
    use gix_features::threading::{Mutable, OwnShared, lock};

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
    receive(&sender, Ok(notify::Event::new(Kind::Any)));
    assert_eq!(
        lock(&state).drain(crate::Budget::default()).loss,
        Some(Loss::Rescan),
        "changes without a path invalidate every watched root"
    );
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
