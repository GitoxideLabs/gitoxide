use gix_features::{parallel, trace};

fn assert_parent() {
    assert_eq!(
        trace::__tracing::Span::current()
            .metadata()
            .map(trace::__tracing::Metadata::name),
        Some("parent"),
        "worker callbacks retain the calling thread's span and subscriber"
    );
}

#[test]
fn workers_inherit_the_current_span_and_subscriber() {
    trace::__tracing::subscriber::with_default(tracing_subscriber::Registry::default(), || {
        let _parent = trace::coarse!("parent");
        parallel::join(assert_parent, || parallel::join(assert_parent, assert_parent));
        parallel::in_parallel(
            (0..2).inspect(|_| assert_parent()),
            Some(2),
            |_| assert_parent(),
            |item, _| {
                assert_parent();
                Ok(item)
            },
            parallel::reduce::IdentityWithResult::<_, ()>::default(),
        )
        .expect("workers complete successfully");
        parallel::in_parallel_with_finalize(
            (0..2).inspect(|_| assert_parent()),
            Some(2),
            |_| assert_parent(),
            |item, _| {
                assert_parent();
                Ok(item)
            },
            |_| {
                assert_parent();
                Ok(0)
            },
            parallel::reduce::IdentityWithResult::<_, ()>::default(),
        )
        .expect("workers complete successfully");
        let periodic_ran = std::sync::atomic::AtomicBool::new(false);
        parallel::in_parallel_with_slice(
            &mut [0, 1],
            Some(2),
            |_| assert_parent(),
            |_, _, _, _| {
                assert_parent();
                #[cfg(feature = "parallel")]
                while !periodic_ran.load(std::sync::atomic::Ordering::Relaxed) {
                    std::thread::yield_now();
                }
                Ok::<_, ()>(())
            },
            || {
                periodic_ran.store(true, std::sync::atomic::Ordering::Relaxed);
                assert_parent();
                Some(std::time::Duration::from_millis(1))
            },
            |_| assert_parent(),
        )
        .expect("workers complete successfully");
        assert_eq!(
            parallel::EagerIter::new((0..2).inspect(|_| assert_parent()), 1, 1).count(),
            2,
            "the eager worker visits every input"
        );
        parallel::reduce::Stepwise::new(
            (0..2).inspect(|_| assert_parent()),
            Some(2),
            |_| assert_parent(),
            |item, _| {
                assert_parent();
                Ok(item)
            },
            parallel::reduce::IdentityWithResult::<_, ()>::default(),
        )
        .finalize()
        .expect("workers complete successfully");
    });
}
