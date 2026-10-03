mod open_read_only_no_follow {

    use std::io::Read;

    use gix_fs::{FileOrSymlink, open_read_only_no_follow};

    #[test]
    fn regular_files_can_be_read() -> gix_testtools::Result {
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
    fn missing_paths_are_errors_not_symlinks() -> gix_testtools::Result {
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
    fn symlinks_are_not_followed() -> gix_testtools::Result {
        #[cfg(unix)]
        use std::os::unix::fs::symlink;
        #[cfg(windows)]
        use std::os::windows::fs::symlink_file as symlink;

        let dir = gix_testtools::tempfile::tempdir()?;
        let target = dir.path().join("file");
        let link = dir.path().join("link");
        std::fs::write(&target, b"contents")?;
        if let Err(err) = symlink(&target, &link) {
            if cfg!(windows) && err.kind() == std::io::ErrorKind::PermissionDenied {
                eprintln!("skipping symlink checks: {err}");
                return Ok(());
            }
            return Err(err.into());
        }

        for target_exists in [true, false] {
            if !target_exists {
                std::fs::remove_file(&target)?;
            }
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
