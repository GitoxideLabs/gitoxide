mod repo_with_small_packs {
    use gix_object::Find;

    use crate::{db_small_packs, hex_to_id};

    #[test]
    fn all_packed_objects_can_be_found() -> crate::Result {
        let store = db_small_packs();
        let mut buf = Vec::new();
        assert!(
            store
                .try_find(&hex_to_id("ecc68100297fff843a7eef8df0d0fb80c1c8bac5"), &mut buf)?
                .is_some(),
            "object is present and available"
        );
        Ok(())
    }

    #[test]
    #[cfg(feature = "parallel")]
    fn multi_threaded_access_will_not_panic() -> crate::Result {
        for arg in ["no", "without-multi-index"] {
            let base = crate::scripted_fixture_read_only_with_args("make_repo_multi_index.sh", Some(arg))?
                .join(".git")
                .join("objects");
            let store = gix_odb::at(base)?;
            let (tx, barrier) = crossbeam_channel::unbounded::<()>();
            let handles = (0..std::thread::available_parallelism()?.get()).map(|tid| {
                std::thread::spawn({
                    let store = store.clone();
                    let barrier = barrier.clone();
                    move || -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
                        barrier.recv().ok();
                        let mut buf = Vec::new();
                        let mut count = 0;
                        for id in store.iter()? {
                            let id = id?;
                            assert!(
                                store.try_find(&id, &mut buf).is_ok(),
                                "Thread {} could not find {}",
                                tid,
                                id
                            );
                            count += 1;
                        }
                        Ok(count)
                    }
                })
            });

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
                let actual = handle.join().expect("no panic").expect("no error in thread");
                assert_eq!(actual, expected);
            }
        }
        Ok(())
    }
}
