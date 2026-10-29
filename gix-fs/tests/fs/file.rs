mod clear_readonly_attribute {
    use gix_fs::clear_readonly_attribute;
    use gix_testtools::TestResult;

    #[test]
    fn clearing_readonly_is_platform_specific_and_idempotent() -> TestResult {
        let dir = gix_testtools::tempfile::tempdir()?;
        let path = dir.path().join("file");
        std::fs::write(&path, b"contents")?;
        let mut permissions = std::fs::metadata(&path)?.permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&path, permissions)?;
        #[cfg(unix)]
        let original_mode = {
            use std::os::unix::fs::PermissionsExt;
            std::fs::metadata(&path)?.permissions().mode()
        };

        clear_readonly_attribute(&path)?;
        clear_readonly_attribute(&path)?;
        assert_eq!(
            std::fs::metadata(&path)?.permissions().readonly(),
            !cfg!(windows),
            "only Windows read-only attributes are cleared, and repeated calls succeed"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path)?.permissions().mode(),
                original_mode,
                "Unix permissions remain unchanged"
            );
        }
        assert_eq!(std::fs::read(&path)?, b"contents", "file contents remain unchanged");
        Ok(())
    }

    #[test]
    fn missing_paths_are_only_accessed_on_windows() -> TestResult {
        let dir = gix_testtools::tempfile::tempdir()?;
        let path = dir.path().join("missing");
        #[cfg(windows)]
        {
            let err = clear_readonly_attribute(&path).expect_err("Windows metadata errors are propagated");
            assert_eq!(
                err.kind(),
                std::io::ErrorKind::NotFound,
                "missing paths retain their native I/O error"
            );
        }
        #[cfg(not(windows))]
        clear_readonly_attribute(&path)?;
        assert!(!path.exists(), "clearing an attribute never creates a missing file");
        Ok(())
    }
}

mod open_read_only_no_follow {

    use std::io::Read;

    use gix_fs::{FileOrSymlink, open_read_only_no_follow};

    #[test]
    fn regular_files_can_be_read() -> gix_testtools::TestResult {
        let dir = gix_testtools::tempfile::tempdir()?;
        let path = dir.path().join("file");
        std::fs::write(&path, b"contents")?;

        let mut buf = Vec::new();
        let FileOrSymlink::File(mut file) = open_read_only_no_follow(&path)? else {
            panic!("a regular file must not be classified as a symlink");
        };
        file.read_to_end(&mut buf)?;
        assert_eq!(
            buf, b"contents",
            "regular files remain readable without further options"
        );
        Ok(())
    }

    #[test]
    fn missing_paths_are_errors_not_symlinks() -> gix_testtools::TestResult {
        let dir = gix_testtools::tempfile::tempdir()?;
        let err = open_read_only_no_follow(&dir.path().join("missing"))
            .expect_err("missing paths must not be classified as symlinks");
        assert_eq!(
            err.kind(),
            std::io::ErrorKind::NotFound,
            "callers can distinguish missing paths from skipped symlinks"
        );
        Ok(())
    }

    #[test]
    #[cfg(any(unix, windows))]
    fn symlinks_are_not_followed() -> gix_testtools::TestResult {
        let dir = gix_testtools::scripted_fixture_read_only("symlinks.sh")?;
        if !gix_testtools::fixture_has_symlinks(&dir)? {
            return Ok(());
        }
        for (name, target_exists) in [("link", true), ("dangling-link", false)] {
            let link = dir.join(name);
            assert_eq!(
                link.exists(),
                target_exists,
                "the fixture covers both live and dangling symlinks"
            );
            assert!(
                link.symlink_metadata()?.file_type().is_symlink(),
                "native links remain symlinks whether their targets exist or not"
            );
            assert!(
                matches!(open_read_only_no_follow(&link)?, FileOrSymlink::Symlink),
                "live and dangling symlinks have the same explicit outcome on Unix and Windows"
            );
        }
        Ok(())
    }
}
