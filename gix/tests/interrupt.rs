#[test]
fn iterator_errors_use_the_crate_result() -> gix_testtools::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let mut iter = gix::interrupt::Iter::new([1, 2].into_iter(), || {
        gix::Error::from_error(std::io::Error::from(std::io::ErrorKind::Interrupted))
    });
    let first: gix::Result<_> = iter.next().expect("the first item is available");
    assert_eq!(first?, 1, "items pass through before interruption");
    gix::interrupt::trigger();
    let interrupted: gix::Result<_> = iter.next().expect("interruption is reported once");
    let error = interrupted.expect_err("the interrupt stops iteration");
    assert_eq!(
        error
            .downcast_any_ref::<std::io::Error>()
            .expect("the concrete cause is retained")
            .kind(),
        std::io::ErrorKind::Interrupted,
        "the crate error retains the interruption cause"
    );
    assert!(
        error.can_retry(),
        "the concrete interruption remains classifiable for retry"
    );
    assert!(iter.next().is_none(), "iteration ends after the error");
    Ok(())
}

#[test]
fn iterator_errors_accept_exceptions_and_preserve_retry_classification() -> gix_testtools::Result {
    use gix::error::ErrorExt;

    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let mut iter = gix::interrupt::Iter::new([()].into_iter(), || {
        gix::error::retryable("interrupted by user").raise()
    });
    gix::interrupt::trigger();
    let error = iter
        .next()
        .expect("interruption is reported once")
        .expect_err("the interrupt stops iteration");
    assert!(
        error.is_retryable(),
        "converting the exception retains its explicit retryable classification"
    );
    assert!(
        error.can_retry(),
        "the classified interruption supports retry detection"
    );
    assert_eq!(
        error
            .downcast_any_ref::<gix::error::Message>()
            .expect("the exception retains its diagnostic message")
            .message,
        "interrupted by user",
        "conversion preserves the original diagnostic"
    );
    assert!(iter.next().is_none(), "iteration ends after the error");
    Ok(())
}

#[cfg(feature = "interrupt")]
mod needs_feature {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use signal_hook::consts::SIGTERM;

    #[test]
    fn multi_registration() -> gix_testtools::Result {
        static V1: AtomicUsize = AtomicUsize::new(0);
        static V2: AtomicBool = AtomicBool::new(false);

        // SAFETY: The closure doesn't use mutexes or memory allocation, so it should be safe to call from a signal handler.
        let reg1 = unsafe {
            gix::interrupt::init_handler(3, || {
                V1.fetch_add(1, Ordering::SeqCst);
            })
        }
        .expect("succeeds");
        assert!(!gix::interrupt::is_triggered());
        assert_eq!(V1.load(Ordering::Relaxed), 0);
        // SAFETY: The closure doesn't use mutexes or memory allocation, so it should be safe to call from a signal handler.
        let reg2 = unsafe { gix::interrupt::init_handler(2, || V2.store(true, Ordering::SeqCst)) }
            .expect("multi-initialization is OK");
        assert!(!V2.load(Ordering::Relaxed));

        signal_hook::low_level::raise(SIGTERM).expect("signal can be raised");
        assert!(gix::interrupt::is_triggered(), "this happens automatically");
        assert_eq!(V1.load(Ordering::Relaxed), 1, "the first trigger is invoked");
        assert!(!V2.load(Ordering::Relaxed), "the second trigger was ignored");

        reg1.deregister()?;
        signal_hook::low_level::raise(SIGTERM).expect("signal can be raised");
        assert_eq!(V1.load(Ordering::Relaxed), 2, "the first trigger is still invoked");

        assert!(gix::interrupt::is_triggered(), "this happens automatically");
        // now the registration is actually removed.
        reg2.with_reset(true).deregister()?;
        assert!(
            !gix::interrupt::is_triggered(),
            "the deregistration succeeded and this is an optional side-effect"
        );

        // SAFETY: The closure doesn't use mutexes or memory allocation, so it should be safe to call from a signal handler.
        let reg1 = unsafe {
            gix::interrupt::init_handler(3, || {
                V1.fetch_add(1, Ordering::SeqCst);
            })
        }
        .expect("succeeds");
        assert_eq!(V1.load(Ordering::Relaxed), 2, "nothing changed yet");
        // SAFETY: The closure doesn't use mutexes or memory allocation, so it should be safe to call from a signal handler.
        let reg2 = unsafe { gix::interrupt::init_handler(2, || V2.store(true, Ordering::SeqCst)) }
            .expect("multi-initialization is OK");
        assert!(!V2.load(Ordering::Relaxed));

        signal_hook::low_level::raise(SIGTERM).expect("signal can be raised");
        assert_eq!(V1.load(Ordering::Relaxed), 3, "the first trigger is invoked");
        assert!(!V2.load(Ordering::Relaxed), "the second trigger was ignored");

        reg2.auto_deregister();
        reg1.with_reset(true).auto_deregister();

        assert!(
            !gix::interrupt::is_triggered(),
            "the deregistration succeeded and this is an optional side-effect"
        );

        Ok(())
    }
}
