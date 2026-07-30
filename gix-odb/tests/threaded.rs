use gix_object::Find;

#[test]
fn multi_threaded_access_will_not_panic() -> gix_testtools::TestResult {
    for arg in ["no", "without-multi-index"] {
        let base = gix_testtools::scripted_fixture_read_only_with_args("make_repo_multi_index.sh", Some(arg))?
            .join(".git")
            .join("objects");
        let store = gix_odb::at(base, gix_testtools::object_hash())?;
        let (tx, barrier) = crossbeam_channel::unbounded::<()>();
        let handles: Vec<_> = (0..std::thread::available_parallelism()?.get().max(2))
            .map(|tid| {
                std::thread::spawn({
                    let store = store.clone();
                    let barrier = barrier.clone();
                    move || -> std::result::Result<usize, Box<dyn std::error::Error + Send + Sync>> {
                        barrier.recv().ok();
                        let mut buf = Vec::new();
                        let mut count = 0;
                        for id in store.iter()? {
                            let id = id?;
                            let found = store.try_find(&id, &mut buf)?;
                            assert!(found.is_some(), "Thread {tid} could not find {id}");
                            count += 1;
                        }
                        Ok(count)
                    }
                })
            })
            .collect();

        std::thread::sleep(std::time::Duration::from_millis(50));
        drop(tx);
        let expected = store.iter()?.count();
        assert_eq!(
            store
                .iter()?
                .with_ordering(gix_odb::store::iter::Ordering::PackAscendingOffsetThenLooseLexicographical)
                .count(),
            expected,
            "different ordering doesn't change the count"
        );
        for handle in handles {
            let actual = handle.join().expect("no panic")?;
            assert_eq!(actual, expected, "each worker finds every object");
        }
    }
    Ok(())
}
