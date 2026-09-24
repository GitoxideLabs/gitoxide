mod acquire {
    use std::time::{Duration, Instant};

    use gix_lock::acquire::Fail;

    #[test]
    fn fail_mode_immediately_produces_a_descriptive_error() -> gix_error::TestResult {
        let dir = tempfile::tempdir()?;
        let resource = dir.path().join("the-resource");
        let guard = gix_lock::Marker::acquire_to_hold_resource(&resource, Fail::Immediately, None)?;
        assert!(guard.lock_path().ends_with("the-resource.lock"));
        assert!(guard.resource_path().ends_with("the-resource"));
        let err = gix_lock::Marker::acquire_to_hold_resource(resource, Fail::Immediately, None)
            .expect_err("the lock is taken and there is a failure obtaining it again");
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&(err), &[(&(dir.path()).to_string_lossy(), "<tmp>")]), "lock contention is retryable", @r#"
        The lock for resource '<tmp>/the-resource' could not be obtained immediately after 1 attempt(s). The lockfile at '<tmp>/the-resource.lock' might need manual deletion.
        |
        └─ I/O error (AlreadyExists)
        |
        └─ AlreadyExists at path "<tmp>/the-resource.lock"
        "#);
        assert!(err.is_retryable(), "lock contention is retryable");
        Ok(())
    }

    #[test]
    fn fail_mode_after_duration_fails_after_a_given_duration_or_more() -> gix_error::TestResult {
        let dir = tempfile::tempdir()?;
        let resource = dir.path().join("the-resource");
        let _guard = gix_lock::Marker::acquire_to_hold_resource(&resource, Fail::Immediately, None)?;
        let start = Instant::now();
        let time_to_wait = Duration::from_millis(50);
        let err =
            gix_lock::Marker::acquire_to_hold_resource(resource, Fail::AfterDurationWithBackoff(time_to_wait), None)
                .expect_err("the lock is taken and there is a failure obtaining it again after some delay");
        assert!(
            start.elapsed() >= time_to_wait,
            "it should never wait less than the given wait time"
        );
        insta::with_settings!({ filters => vec![(r"after \d+ attempt\(s\)", "after <attempts> attempt(s)")] }, {
            insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[(&dir.path().to_string_lossy(), "<tmp>")]), "lock contention reports the requested wait duration and lockfile", @r#"
            The lock for resource '<tmp>/the-resource' could not be obtained after 0.05s after <attempts> attempt(s). The lockfile at '<tmp>/the-resource.lock' might need manual deletion.
            |
            └─ I/O error (AlreadyExists)
            |
            └─ AlreadyExists at path "<tmp>/the-resource.lock"
            "#);
        });
        Ok(())
    }
}
mod commit {
    use gix_lock::acquire::Fail;

    #[test]
    fn failure_to_commit_does_return_a_registered_marker() {
        let dir = tempfile::tempdir().unwrap();
        let resource = dir.path().join("the-resource");
        let file = gix_lock::File::acquire_to_update_resource(&resource, Fail::Immediately, None).unwrap();
        let mark = file.close().unwrap();
        let resource_lock_path = mark.lock_path().to_owned();

        std::fs::create_dir(&resource).unwrap();
        let err = mark.commit().expect_err("it fails as the resource path is a directory");
        assert!(
            resource_lock_path.is_file(),
            "the underlying lock wasn't consumed after all"
        );
        drop(err);
        assert!(
            !resource_lock_path.is_file(),
            "and is linked to the err which makes the lock recoverable"
        );
    }

    #[test]
    fn fails_for_ordinary_marker_that_was_never_writable() -> gix_error::TestResult {
        let dir = tempfile::tempdir()?;
        let resource = dir.path().join("the-resource");
        let mark = gix_lock::Marker::acquire_to_hold_resource(resource, Fail::Immediately, None)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = mark.lock_path().metadata()?.permissions();
            assert_ne!(
                perms.mode() & !0o170000,
                0o600,
                "mode is more permissive now, even after passing the umask"
            );
        }
        let err = mark.commit().expect_err("should always fail");
        assert_eq!(err.error.kind(), std::io::ErrorKind::Other);
        insta::assert_debug_snapshot!(err.error.get_ref().expect("custom error"), "fails for ordinary marker that was never writable", @r#""refusing to commit marker that was never opened""#);
        Ok(())
    }
}
