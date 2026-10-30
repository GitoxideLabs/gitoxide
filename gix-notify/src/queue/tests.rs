use super::*;
use crate::{EventKind, PathKind};

fn event(path: &str) -> Event {
    Event {
        paths: vec![path.into()],
        kind: EventKind::Modify,
        path_kind: PathKind::Any,
    }
}

#[test]
fn overflow_is_latched_and_changes_generation() {
    let mut state = State::new(Options {
        max_events: 2,
        max_bytes: usize::MAX,
    });
    state.publish(vec![event("/one"), event("/two")]);
    state.publish(vec![event("/three")]);
    state.publish(vec![event("/after-loss")]);
    let batch = state.drain(Budget::default());
    assert_eq!(
        batch.loss,
        Some(Loss::Overflow),
        "new events must not hide lost notifications"
    );
    assert_eq!(batch.generation, 1, "loss starts a fresh coverage generation");
    assert_eq!(
        batch.events,
        vec![event("/after-loss")],
        "stale queued paths are discarded together"
    );
    assert_eq!(batch.sequence, 4, "loss occupies a publication boundary too");
    assert!(
        state.drain(Budget::default()).loss.is_none(),
        "loss is acknowledged by draining it"
    );
}

#[test]
fn byte_limit_rejects_whole_native_batch() {
    let first = event("/first");
    let second = event("/second");
    let mut state = State::new(Options {
        max_events: 10,
        max_bytes: event_bytes(&first) + event_bytes(&second) - 1,
    });
    state.publish(vec![first, second]);
    let batch = state.drain(Budget::default());
    assert!(
        batch.events.is_empty(),
        "a native publication cannot be partially retained"
    );
    assert_eq!(
        batch.loss,
        Some(Loss::Overflow),
        "path bytes are bounded independently of event count"
    );
}

#[test]
fn budget_makes_progress_and_does_not_advance_past_undrained_events() {
    let mut state = State::new(Options::default());
    state.publish(vec![event("/first"), event("/second")]);
    let budget = Budget {
        max_events: 256,
        max_bytes: 1,
    };
    let first = state.drain(budget);
    assert_eq!(
        first.events,
        vec![event("/first")],
        "one large event must not starve the queue"
    );
    assert_eq!(
        first.sequence, 1,
        "a future fence must not complete before undrained events"
    );
    assert!(
        first.more,
        "remaining work must be observable without waiting for another wake"
    );
    let second = state.drain(budget);
    assert_eq!(second.events, vec![event("/second")], "the next poll makes progress");
    assert_eq!(second.sequence, 2, "the complete publication has now been consumed");
    assert!(!second.more, "the queue is drained");
}

#[test]
fn zero_budget_still_reports_loss_and_only_latest_error() {
    let state = OwnShared::new(Mutable::new(State::new(Options::default())));
    let sender = Sender::new(state.clone(), 0);
    for _ in 0..100 {
        sender.publish(
            Vec::new(),
            Some(Loss::BackendError),
            Some(message("backend failed").raise()),
        );
    }
    sender.publish(vec![event("/pending")], None, None);
    let batch = lock(&state).drain(Budget {
        max_events: 0,
        max_bytes: 0,
    });
    assert_eq!(
        batch.loss,
        Some(Loss::BackendError),
        "loss is not subject to the event budget"
    );
    assert_eq!(batch.errors.len(), 1, "diagnostics cannot grow with an error flood");
    assert!(
        batch.events.is_empty() && batch.more,
        "zero budget retains queued paths"
    );
    assert_eq!(batch.generation, 100, "each coverage failure advances the generation");
}

#[test]
fn old_registrations_cannot_publish_or_report_disconnect() {
    let state = OwnShared::new(Mutable::new(State::new(Options::default())));
    let sender = Sender::new(state.clone(), 0);
    lock(&state).registration = 1;
    sender.publish(vec![event("/stale")], None, None);
    drop(sender);
    let batch = lock(&state).drain(Budget::default());
    assert!(
        batch.events.is_empty(),
        "late old callbacks cannot enter a new registration"
    );
    assert!(
        batch.loss.is_none() && batch.errors.is_empty(),
        "expected old shutdown is not coverage loss"
    );
}

#[test]
fn producer_disconnect_is_distinct_from_an_empty_queue() {
    let state = OwnShared::new(Mutable::new(State::new(Options::default())));
    drop(Sender::new(state.clone(), 0));
    let mut state = lock(&state);
    let batch = state.drain(Budget::default());
    assert_eq!(
        batch.loss,
        Some(Loss::BackendStopped),
        "dead producers must trigger recovery"
    );
    assert!(
        state.needs_restart,
        "replacing the same desired set must retry a dead backend"
    );
}

#[test]
fn waker_runs_without_the_queue_lock_and_panics_are_contained() {
    let state = OwnShared::new(Mutable::new(State::new(Options::default())));
    let weak = OwnShared::downgrade(&state);
    lock(&state).waker = Some(OwnShared::new(move || {
        if let Some(state) = weak.upgrade() {
            lock(&state).needs_restart = true;
        }
    }));
    wake(&state);
    assert!(
        lock(&state).needs_restart,
        "callbacks can take the queue lock without deadlocking"
    );
    lock(&state).waker = Some(OwnShared::new(|| panic!("test callback failure")));
    wake(&state);
    let mut state = lock(&state);
    assert!(
        state.waker.is_none(),
        "a panicking callback must not unwind through native code"
    );
    assert_eq!(
        state.drain(Budget::default()).errors.len(),
        1,
        "callback failure is observable"
    );
}
