use super::*;
use gix_features::threading::{Mutable, lock};

#[test]
fn ambiguous_native_flags_do_not_guess_a_file_kind() {
    assert_eq!(
        translate(fs::kFSEventStreamEventFlagItemIsDir | fs::kFSEventStreamEventFlagItemIsFile).1,
        PathKind::Any,
        "FSEvents can combine flags from several objects occupying one path"
    );
}

#[test]
fn native_batches_are_published_before_the_fence_and_keep_byte_paths() {
    let state = OwnShared::new(Mutable::new(queue::State::new(crate::Options::default())));
    let sender = OwnShared::new(queue::Sender::new(state.clone(), 0));
    let (reply, receive) = mpsc::sync_channel(1);
    let context = Context {
        pending: Mutable::new(Some(PendingFence {
            path: PathBuf::from("/watched/cookie"),
            generation: 0,
            reply,
        })),
        sender: sender.clone(),
        watches: BTreeMap::from([(PathBuf::from("/watched"), true)]),
    };
    let names = [
        CString::new(b"/watched/first".as_slice()).expect("no NUL"),
        CString::new(b"/watched/cookie".as_slice()).expect("no NUL"),
        CString::new(b"/watched/\xff".as_slice()).expect("no NUL"),
    ];
    let paths: Vec<_> = names.iter().map(|name| name.as_ptr()).collect();
    let flags = [fs::kFSEventStreamEventFlagItemModified; 3];
    callback(
        std::ptr::null_mut(),
        std::ptr::from_ref(&context).cast_mut().cast(),
        3,
        paths.as_ptr().cast_mut().cast(),
        flags.as_ptr(),
        std::ptr::null(),
    );
    let fence = receive
        .try_recv()
        .expect("the callback observed its marker")
        .expect("the callback retained continuous coverage");
    assert_eq!(fence.sequence, 2, "the complete native callback precedes a fence");
    let first = lock(&state).drain(crate::Budget {
        max_events: 1,
        max_bytes: usize::MAX,
    });
    assert!(
        first.sequence < fence.sequence,
        "one partial drain does not consume the whole native batch"
    );
    let second = lock(&state).drain(crate::Budget::default());
    assert_eq!(
        second.sequence, fence.sequence,
        "the final event consumes the fence boundary"
    );
    assert_eq!(
        second.events[0].paths[0].as_os_str().as_bytes(),
        b"/watched/\xff",
        "event paths are byte-preserving"
    );
}

#[test]
fn loss_after_a_marker_and_queue_overflow_both_abort_its_fence() {
    for case in 0..3 {
        let overflow = case == 0;
        let state = OwnShared::new(Mutable::new(queue::State::new(crate::Options {
            max_events: if overflow { 1 } else { 3 },
            max_bytes: usize::MAX,
        })));
        let (reply, receive) = mpsc::sync_channel(1);
        let context = Context {
            sender: OwnShared::new(queue::Sender::new(state.clone(), 0)),
            watches: BTreeMap::from([(PathBuf::from("/watched"), true)]),
            pending: Mutable::new(Some(PendingFence {
                path: PathBuf::from("/watched/cookie"),
                generation: 0,
                reply,
            })),
        };
        let names = ["/watched/cookie", "/watched/first", "/watched/last"];
        let names: Vec<_> = names
            .into_iter()
            .map(|name| CString::new(name).expect("no NUL"))
            .collect();
        let paths: Vec<_> = names.iter().map(|name| name.as_ptr()).collect();
        let flags = [
            if case == 2 {
                1 << 31
            } else {
                fs::kFSEventStreamEventFlagItemCreated
            },
            fs::kFSEventStreamEventFlagItemModified,
            if case != 1 {
                fs::kFSEventStreamEventFlagItemModified
            } else {
                1 << 31
            },
        ];
        callback(
            std::ptr::null_mut(),
            std::ptr::from_ref(&context).cast_mut().cast(),
            paths.len(),
            paths.as_ptr().cast_mut().cast(),
            flags.as_ptr(),
            std::ptr::null(),
        );
        assert_eq!(
            receive.try_recv().expect("loss completes the pending request"),
            Err(SynchronizeError::CoverageLost),
            "no marker can certify a partially published or unknown native batch"
        );
        assert_eq!(
            lock(&state).drain(crate::Budget::default()).loss,
            Some(if overflow { Loss::Overflow } else { Loss::Rescan }),
            "each distinct loss remains visible to consumers"
        );
    }
}

#[test]
fn late_marker_from_a_timed_out_request_cannot_complete_its_successor() {
    let state = OwnShared::new(Mutable::new(queue::State::new(crate::Options::default())));
    let (reply, receive) = mpsc::sync_channel(1);
    let context = Context {
        sender: OwnShared::new(queue::Sender::new(state, 0)),
        watches: BTreeMap::from([(PathBuf::from("/watched"), true)]),
        pending: Mutable::new(Some(PendingFence {
            path: PathBuf::from("/watched/new-cookie"),
            generation: 0,
            reply,
        })),
    };
    let name = CString::new("/watched/old-cookie").expect("no NUL");
    let path = name.as_ptr();
    let flags = fs::kFSEventStreamEventFlagItemCreated;
    callback(
        std::ptr::null_mut(),
        std::ptr::from_ref(&context).cast_mut().cast(),
        1,
        std::ptr::from_ref(&path).cast_mut().cast(),
        &flags,
        std::ptr::null(),
    );
    assert!(
        matches!(receive.try_recv(), Err(mpsc::TryRecvError::Empty)),
        "each fence requires its own unique marker"
    );
    assert!(lock(&context.pending).is_some(), "the newer request remains pending");
}

#[test]
fn loss_is_processed_even_for_filtered_native_paths() {
    let state = OwnShared::new(Mutable::new(queue::State::new(crate::Options::default())));
    let sender = OwnShared::new(queue::Sender::new(state.clone(), 0));
    let context = Context {
        pending: Mutable::new(None),
        sender: sender.clone(),
        watches: BTreeMap::from([(PathBuf::from("/watched"), false)]),
    };
    let name = CString::new("/unrelated").expect("no NUL");
    let path = name.as_ptr();
    let flags = fs::kFSEventStreamEventFlagMustScanSubDirs | fs::kFSEventStreamEventFlagHistoryDone;
    callback(
        std::ptr::null_mut(),
        std::ptr::from_ref(&context).cast_mut().cast(),
        1,
        std::ptr::from_ref(&path).cast_mut().cast(),
        &flags,
        std::ptr::null(),
    );
    assert_eq!(
        sender.fence(0),
        Err(SynchronizeError::CoverageLost),
        "loss invalidates every outstanding fence"
    );
    assert_eq!(
        lock(&state).drain(crate::Budget::default()).loss,
        Some(Loss::Rescan),
        "loss is not subject to scope filtering"
    );
}
