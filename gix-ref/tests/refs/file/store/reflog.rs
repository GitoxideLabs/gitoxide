fn store() -> crate::Result<crate::file::Store> {
    Ok(crate::file::Store::at_opts(
        crate::scripted_fixture_read_only("make_repo_for_reflog.sh")?.join(".git"),
        crate::fixture_hash_kind(),
        gix_ref::store::init::Options {
            write_reflog: gix_ref::store::WriteReflog::Disable,
            ..Default::default()
        },
    ))
}

mod iter_and_iter_rev {
    use crate::file::store::reflog::store;

    #[cfg(unix)]
    #[test]
    fn read_failures_preserve_context() -> crate::Result {
        let store = store()?;
        let name = "refs/heads/main/child";
        let mut buf = vec![0; 256];
        for err in [
            store.reflog_iter(name, &mut buf).err().expect("main is a file"),
            store.reflog_iter_rev(name, &mut buf).err().expect("main is a file"),
        ] {
            let err = err.into_error();
            let details = err.metadata().next().expect("reflog read context");
            assert_eq!(
                details.message, "Could not read reflog",
                "both directions retain the read message"
            );
            assert_eq!(details.values.len(), 1, "read context contains only the path");
            assert_eq!(
                details.values["path"],
                gix_error::Value::Path(store.git_dir().join("logs").join(name)),
                "both directions retain the resolved reflog path as a native path"
            );
            assert_eq!(
                err.downcast_any_ref::<std::io::Error>()
                    .expect("the original open error is retained")
                    .kind(),
                std::io::ErrorKind::NotADirectory,
                "a file at a path prefix retains its I/O error kind on Unix"
            );
            assert!(
                !err.is_corrupted(),
                "a path collision does not imply corrupt reflog contents"
            );
        }
        Ok(())
    }

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
