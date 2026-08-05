use gix_features::parallel;

#[test]
fn in_parallel_with_mut_slice_in_chunks() {
    let num_items = 33;
    let mut input: Vec<_> = std::iter::repeat_n(1, num_items).collect();
    let counts = parallel::in_parallel_with_slice(
        &mut input,
        None,
        |_| 0usize,
        |item, acc, _threads_left, _should_interrupt| {
            *acc += *item;
            *item += 1;
            Ok::<_, ()>(())
        },
        || Some(std::time::Duration::from_millis(10)),
        std::convert::identity,
    )
    .expect("successful computation");
    let expected = std::iter::repeat_n(1, num_items).sum::<usize>();
    assert_eq!(counts.iter().sum::<usize>(), expected);
    assert_eq!(input.iter().sum::<usize>(), expected * 2, "we increment each entry");
}

#[cfg(feature = "parallel")]
mod threaded {
    use std::sync::{
        Barrier,
        atomic::{AtomicBool, Ordering},
    };

    use super::parallel;

    #[test]
    fn periodic_duration_does_not_have_to_fit_in_an_instant() {
        let watcher_and_worker = &Barrier::new(2);
        parallel::in_parallel_with_slice(
            &mut [()],
            Some(1),
            |_| (),
            |_item, _state, _threads_left, _should_interrupt| {
                watcher_and_worker.wait();
                Ok::<_, ()>(())
            },
            || {
                watcher_and_worker.wait();
                Some(std::time::Duration::MAX)
            },
            std::convert::identity,
        )
        .expect("an effectively infinite polling interval can still be interrupted");
    }

    #[test]
    fn the_error_that_caused_the_stop_is_returned_even_if_other_threads_fail_on_interrupt() {
        // Regression test for the race behind issue #2701: a thread that observes the stop
        // signal and bails out with a reactive error (like `Interrupted`) must not mask the
        // error of the thread that caused the stop, no matter the join order.
        let both_have_items = &Barrier::new(2);
        let mut input = [(); 2];
        let err = parallel::in_parallel_with_slice(
            &mut input,
            Some(2),
            |thread_id| thread_id,
            |_item, thread_id, _threads_left, should_interrupt| {
                both_have_items.wait();
                if *thread_id == 0 {
                    while !should_interrupt.load(Ordering::Relaxed) {
                        std::thread::yield_now();
                    }
                    Err("reaction to the stop signal")
                } else {
                    Err("the cause of the stop")
                }
            },
            || Some(std::time::Duration::from_millis(10)),
            |thread_id| thread_id,
        )
        .expect_err("failing consumers lead to an error");
        assert_eq!(
            err, "the cause of the stop",
            "the causal error wins over errors of threads that merely reacted to the stop, \
             even though they are joined first"
        );
    }

    #[test]
    fn the_error_of_a_consumer_that_set_the_stop_flag_itself_is_still_the_cause() {
        // Consumers receive the stop flag and may set it themselves right before failing,
        // e.g. to stop nested or peer work immediately. Their error is still the cause of
        // the stop and must win over reactive errors of threads that are joined earlier.
        let thread0_has_item = &AtomicBool::new(false);
        let mut input = [(); 2];
        let err = parallel::in_parallel_with_slice(
            &mut input,
            Some(2),
            |thread_id| thread_id,
            |_item, thread_id, threads_left, should_interrupt| {
                if *thread_id == 0 {
                    thread0_has_item.store(true, Ordering::Relaxed);
                    while !should_interrupt.load(Ordering::Relaxed) {
                        std::thread::yield_now();
                    }
                    // Wait for the causing thread to have failed for good, observable by it
                    // handing back its slot in `threads_left`, to assure this reactive error
                    // strictly follows the causal one.
                    while threads_left.load(Ordering::SeqCst) < 1 {
                        std::thread::yield_now();
                    }
                    Err("reaction to the stop signal")
                } else {
                    while !thread0_has_item.load(Ordering::Relaxed) {
                        std::thread::yield_now();
                    }
                    should_interrupt.store(true, Ordering::Relaxed);
                    Err("the cause of the stop")
                }
            },
            || Some(std::time::Duration::from_millis(10)),
            |thread_id| thread_id,
        )
        .expect_err("failing consumers lead to an error");
        assert_eq!(
            err, "the cause of the stop",
            "causality is not determined by the stop flag, which may be set by consumers themselves"
        );
    }
}
