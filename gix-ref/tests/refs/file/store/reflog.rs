fn store() -> crate::Result<crate::file::Store> {
    Ok(crate::file::Store::at(
        crate::scripted_fixture_read_only("make_repo_for_reflog.sh")?.join(".git"),
        gix_ref::store::init::Options {
            write_reflog: gix_ref::store::WriteReflog::Disable,
            object_hash: crate::fixture_hash_kind(),
            ..Default::default()
        },
    ))
}

mod iter_and_iter_rev {
    use crate::file::store::reflog::store;

    #[test]
    fn non_existing_and_directory_returns_none() -> crate::Result {
        let store = store()?;
        let mut buf = Vec::new();
        for name in &["FAILURE_NONEXISTING", "refs/heads"] {
            assert!(
                matches!(store.reflog_iter(*name, &mut buf), Ok(None)),
                "this one does not exist"
            );
        }
        Ok(())
    }

    #[test]
    fn for_head_and_main() -> crate::Result {
        let store = store()?;
        let mut buf = Vec::new();

        let log = store.reflog_iter("HEAD", &mut buf)?.expect("exists");
        assert_eq!(log.filter_map(Result::ok).count(), 5);

        let log = store.reflog_iter("refs/heads/main", &mut buf)?.expect("exists");
        assert_eq!(log.filter_map(Result::ok).count(), 5);
        Ok(())
    }
}

mod iter_rev {
    use crate::file::store::reflog::store;

    #[test]
    fn non_existing_and_directory_returns_none() -> crate::Result {
        let store = store()?;
        let mut buf = [0u8; 256];
        for name in &["FAILURE_NONEXISTING", "refs/heads"] {
            assert!(
                matches!(store.reflog_iter_rev(*name, &mut buf), Ok(None)),
                "this one does not exist"
            );
        }
        Ok(())
    }

    #[test]
    fn for_head_and_main() -> crate::Result {
        let store = store()?;
        let mut buf = [0u8; 256];

        let log = store.reflog_iter_rev("HEAD", &mut buf)?.expect("exists");
        assert_eq!(log.filter_map(Result::ok).count(), 5);

        let log = store.reflog_iter_rev("refs/heads/main", &mut buf)?.expect("exists");
        assert_eq!(log.filter_map(Result::ok).count(), 5);
        Ok(())
    }
}
