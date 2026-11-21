use crate::Result;

fn store() -> Result<crate::file::Store> {
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
    fn read_failures_preserve_context() -> gix_testtools::TestResult {
        let mut error_snapshots = Vec::new();
        let store = store()?;
        let name = "refs/heads/main/child";
        let mut buf = vec![0; 256];
        for err in [
            store.reflog_iter(name, &mut buf).err().expect("main is a file"),
            store.reflog_iter_rev(name, &mut buf).err().expect("main is a file"),
        ] {
            let details = err.metadata().next().expect("reflog read context");
            assert!(
                err.downcast_any_ref::<gix_error::Message>().is_some(),
                "the reflog read diagnostic retains its concrete type"
            );
            assert_eq!(details.len(), 1, "read context contains only the path");
            assert_eq!(
                details["path"],
                gix_error::MetadataValue::Path(store.git_dir().join("logs").join(name)),
                "both directions retain the resolved reflog path as a native path"
            );
            error_snapshots.push(gix_testtools::redact_debug_snapshot(
                &(err),
                &[(&(store.git_dir()).to_string_lossy(), "<git-dir>")],
            ));
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
        insta::assert_debug_snapshot!(error_snapshots, "read failures preserve context", @r#"
        [
            Could not read reflog, path="<git-dir>/logs/refs/heads/main/child"
            
            Caused by:
                0: NotADirectory,
            Could not read reflog, path="<git-dir>/logs/refs/heads/main/child"
            
            Caused by:
                0: NotADirectory,
        ]
        "#);
        Ok(())
    }

    #[test]
    fn non_existing_and_directory_returns_none() -> gix_testtools::TestResult {
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
    fn for_head_and_main() -> gix_testtools::TestResult {
        let store = store()?;
        let mut buf = Vec::new();

        let log = store.reflog_iter("HEAD", &mut buf)?.expect("exists");
        assert_eq!(log.filter_map(std::result::Result::ok).count(), 5);

        let log = store.reflog_iter("refs/heads/main", &mut buf)?.expect("exists");
        assert_eq!(log.filter_map(std::result::Result::ok).count(), 5);
        Ok(())
    }
}

mod iter_rev {
    use crate::file::store::reflog::store;

    #[test]
    fn non_existing_and_directory_returns_none() -> gix_testtools::TestResult {
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
    fn for_head_and_main() -> gix_testtools::TestResult {
        let store = store()?;
        let mut buf = [0u8; 256];

        let log = store.reflog_iter_rev("HEAD", &mut buf)?.expect("exists");
        assert_eq!(log.filter_map(std::result::Result::ok).count(), 5);

        let log = store.reflog_iter_rev("refs/heads/main", &mut buf)?.expect("exists");
        assert_eq!(log.filter_map(std::result::Result::ok).count(), 5);
        Ok(())
    }
}

mod retain {
    use gix_testtools::TestResult;
    use std::fs;

    #[test]
    fn filtering_matches_git_expiry_and_preserves_reference_values() -> TestResult {
        let mut expected = None;
        for use_git in [true, false] {
            let fixture = gix_testtools::scripted_fixture_writable("make_reflog_retention.sh")?;
            let git_dir = fixture.path().join(".git");
            let path = git_dir.join("logs/refs/heads/main");
            let original = fs::read(&path)?;
            let reference = fs::read(git_dir.join("refs/heads/main"))?;
            if use_git {
                gix_testtools::git(
                    fixture.path(),
                    "reflog expire --expire=@1009843200 --expire-unreachable=never refs/heads/main",
                )?;
                expected = Some(fs::read(&path)?);
            } else {
                let store = gix_ref::file::Store::at(git_dir.clone(), gix_testtools::object_hash());
                let select =
                    |line: gix_ref::file::log::LineRef<'_>| Ok(line.signature.time()?.seconds >= 1_009_843_200);
                let outcome = store.reflog_retain(
                    "refs/heads/main",
                    gix_ref::file::log::retain::Options { dry_run: true },
                    select,
                )?;
                assert_eq!((outcome.retained, outcome.removed), (1, 2));
                assert_eq!(fs::read(&path)?, original, "dry run does not rewrite the log");
                let outcome = store.reflog_retain("refs/heads/main", Default::default(), select)?;
                assert_eq!((outcome.retained, outcome.removed), (1, 2));
                assert_eq!(
                    fs::read(&path)?,
                    expected.as_deref().expect("Git ran first"),
                    "retained entries match Git byte for byte"
                );
            }
            assert_eq!(
                fs::read(git_dir.join("refs/heads/main"))?,
                reference,
                "filtering does not update the reference"
            );
        }
        Ok(())
    }

    #[test]
    fn locks_and_callback_errors_preserve_the_original_log() -> TestResult {
        let fixture = gix_testtools::scripted_fixture_writable("make_reflog_retention.sh")?;
        let git_dir = fixture.path().join(".git");
        let store = gix_ref::file::Store::at(git_dir.clone(), gix_testtools::object_hash());
        let path = git_dir.join("logs/refs/heads/main");
        let original = fs::read(&path)?;
        for name in ["refs/heads/main.lock", "logs/refs/heads/main.lock"] {
            let lock = git_dir.join(name);
            fs::write(&lock, b"another writer")?;
            store
                .reflog_retain("refs/heads/main", Default::default(), |_| Ok(false))
                .expect_err("reference and log locks exclude rewrites");
            assert_eq!(fs::read(&path)?, original);
            fs::remove_file(lock)?;
        }
        let mut count = 0;
        store
            .reflog_retain("refs/heads/main", Default::default(), |_| {
                count += 1;
                if count == 2 {
                    return Err(gix_error::message("callback failed").validation_error());
                }
                Ok(false)
            })
            .expect_err("callback failure must not publish partial output");
        assert_eq!(fs::read(&path)?, original);
        let mut malformed = original.clone();
        malformed.extend_from_slice(b"invalid log entry\n");
        fs::write(&path, &malformed)?;
        store
            .reflog_retain("refs/heads/main", Default::default(), |_| Ok(false))
            .expect_err("malformed logs must not publish partial output");
        assert_eq!(fs::read(&path)?, malformed);
        assert!(!git_dir.join("refs/heads/main.lock").exists());
        assert!(!git_dir.join("logs/refs/heads/main.lock").exists());
        Ok(())
    }
}
