mod validate_relative_path {
    use std::path::Path;

    use gix_testtools::TestResult;

    #[test]
    fn relative_paths_stay_below_the_root_and_can_be_missing() -> TestResult {
        let dir = gix_testtools::tempfile::tempdir()?;
        for path in ["", "file", "dir/missing", "dir/../file"] {
            assert_eq!(
                gix_fs::symlink::validate_relative_path(dir.path(), Path::new(path))?,
                dir.path().join(path),
                "validation preserves the caller's path spelling"
            );
        }
        for path in [Path::new("../outside"), Path::new("dir/../../outside"), dir.path()] {
            let err = gix_fs::symlink::validate_relative_path(dir.path(), path)
                .expect_err("the relative path must remain below the root");
            assert!(err.is_validation(), "escaping paths are invalid: {err:?}");
        }
        #[cfg(windows)]
        for path in [r"C:relative", r"C:\absolute", r"\\host\share\file", r"\rooted"] {
            assert!(
                gix_fs::symlink::validate_relative_path(dir.path(), Path::new(path)).is_err(),
                "Windows prefixes must not replace the trusted root"
            );
        }
        Ok(())
    }

    #[test]
    #[cfg(any(unix, windows))]
    fn links_below_the_root_are_rejected_but_root_ancestors_are_allowed() -> TestResult {
        let fixture = gix_testtools::scripted_fixture_read_only("symlink_paths.sh")?;
        if !gix_testtools::fixture_has_symlinks(&fixture)? {
            return Ok(());
        }
        for root in [fixture.join("base"), fixture.join("root-link")] {
            for path in ["file", "dir/file", "blocked/child", "missing/child"] {
                assert_eq!(
                    gix_fs::symlink::validate_relative_path(&root, Path::new(path))?,
                    root.join(path),
                    "non-directory obstructions and missing checkouts remain valid"
                );
            }
            for path in [
                "file-link",
                "dir-link/file",
                "dir-link/../file",
                "dir-link/",
                "dir-link/.",
                "dangling",
                "dangling/child",
                "outside/file",
            ] {
                let err = gix_fs::symlink::validate_relative_path(&root, Path::new(path))
                    .expect_err("every component below the root must be checked before normalization");
                assert!(err.is_validation(), "links are invalid: {path}: {err:?}");
            }
        }
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn other_io_errors_are_preserved() -> TestResult {
        let dir = gix_testtools::tempfile::tempdir()?;
        let name = "a".repeat(300);
        let err = gix_fs::symlink::validate_relative_path(dir.path(), Path::new(&name))
            .expect_err("an overlong name must not be treated as an absent checkout");
        assert!(
            err.downcast_any_ref::<std::io::Error>().is_some(),
            "the original filesystem error remains available: {err:?}"
        );
        Ok(())
    }
}

#[cfg(windows)]
mod create {
    use std::{
        os::windows::fs::FileTypeExt,
        path::{Path, PathBuf},
    };

    #[test]
    fn empty_and_root_link_paths_are_errors() -> gix_testtools::TestResult {
        for link in ["", "/", "\\", "C:", "C:\\", "C:/"] {
            let link = Path::new(link);
            assert!(link.parent().is_none(), "the link path has no parent: {link:?}");
            let err = gix_fs::symlink::create(Path::new("target"), link)
                .expect_err("a link path without a parent must return an error instead of panicking");
            assert_eq!(
                err.kind(),
                std::io::ErrorKind::InvalidInput,
                "parentless link paths are invalid when the target must be probed: {link:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn slash_prefixed_targets_with_empty_link_paths_return_native_errors() -> gix_testtools::TestResult {
        let link = Path::new("");
        for original in ["/target", "\\target"] {
            let original = Path::new(original);
            let expected =
                std::os::windows::fs::symlink_file(original, link).expect_err("an empty link path cannot be created");
            let actual = gix_fs::symlink::create(original, link).expect_err("an empty link path cannot be created");
            assert_eq!(
                actual.raw_os_error(),
                expected.raw_os_error(),
                "slash-prefixed targets bypass the parent check and preserve the native error: {original:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn slash_prefixed_targets_are_created_as_file_links() -> gix_testtools::TestResult {
        // Keep all targets local, including the verbatim path, while exercising the same
        // leading separators as UNC and NT namespace paths. Never contact a network share.
        let dir = gix_testtools::tempfile::tempdir_in(std::env::current_dir()?)?;
        let target = dir.path().join("target");
        std::fs::create_dir(&target)?;
        let rooted: PathBuf = target.components().skip(1).collect();
        for (name, original) in [
            ("backslash", rooted.clone()),
            ("slash", PathBuf::from(rooted.to_string_lossy().replace('\\', "/"))),
            ("verbatim", target.canonicalize()?),
        ] {
            assert!(
                original.is_dir(),
                "the local target exists as a directory: {original:?}"
            );
            let link = dir.path().join(name);
            gix_fs::symlink::create(&original, &link)?;
            assert!(
                link.symlink_metadata()?.file_type().is_symlink_file(),
                "slash-prefixed targets must use file links without probing the target: {original:?}"
            );
            let delayed_link = dir.path().join(format!("{name}-delayed"));
            assert!(
                gix_fs::symlink::create_or_delay(&original, &delayed_link)?,
                "slash-prefixed targets must not be queued for later probing"
            );
            assert!(
                delayed_link.symlink_metadata()?.file_type().is_symlink_file(),
                "the retry API uses the same guarded file fallback"
            );
        }
        Ok(())
    }

    #[test]
    fn relative_and_drive_qualified_directory_targets_are_detected() -> gix_testtools::TestResult {
        let dir = gix_testtools::tempfile::tempdir_in(std::env::current_dir()?)?;
        let target = dir.path().join("target");
        std::fs::create_dir(&target)?;
        for (name, original) in [("relative", PathBuf::from("target")), ("absolute", target)] {
            let link = dir.path().join(name);
            gix_fs::symlink::create(&original, &link)?;
            assert!(
                link.symlink_metadata()?.file_type().is_symlink_dir(),
                "local directory targets without a leading slash retain directory detection: {original:?}"
            );
            assert!(link.is_dir(), "the directory link can be followed");
        }
        Ok(())
    }

    #[test]
    fn indirect_slash_prefixed_targets_are_not_probed() -> gix_testtools::TestResult {
        // A local directory stands in for a network share: following it would wrongly
        // produce a directory link. This regression never accesses a network path.
        let dir = gix_testtools::tempfile::tempdir_in(std::env::current_dir()?)?;
        let target = dir.path().join("target");
        std::fs::create_dir_all(target.join("nested"))?;
        let rooted: PathBuf = target.components().skip(1).collect();
        std::os::windows::fs::symlink_dir(&rooted, dir.path().join("alias"))?;
        std::os::windows::fs::symlink_dir("alias", dir.path().join("chain"))?;

        for (index, original) in ["alias", "chain", "alias/nested", "chain\\nested"]
            .into_iter()
            .enumerate()
        {
            for delayed in [false, true] {
                let link = dir.path().join(format!("link-{index}-{delayed}"));
                if delayed {
                    assert!(
                        gix_fs::symlink::create_or_delay(Path::new(original), &link)?,
                        "unsafe indirect targets use the file fallback without retries"
                    );
                } else {
                    gix_fs::symlink::create(Path::new(original), &link)?;
                }
                assert!(
                    link.symlink_metadata()?.file_type().is_symlink_file(),
                    "a separator-prefixed target anywhere in a chain must prevent directory probing: {original}"
                );
            }
        }
        Ok(())
    }
}

#[cfg(windows)]
mod create_or_delay {
    use std::os::windows::fs::FileTypeExt;

    #[test]
    fn unresolved_targets_can_be_retried_before_creating_a_link() -> gix_testtools::TestResult {
        let dir = gix_testtools::tempfile::tempdir()?;
        let original = std::path::Path::new("target");
        let link = dir.path().join("link");
        assert!(
            !gix_fs::symlink::create_or_delay(original, &link)?,
            "an unknown target type delays creation"
        );
        assert_eq!(
            link.symlink_metadata()
                .expect_err("no provisional link is created")
                .kind(),
            std::io::ErrorKind::NotFound,
            "delaying a link leaves its destination available"
        );
        std::fs::create_dir(dir.path().join(original))?;
        assert!(
            gix_fs::symlink::create_or_delay(original, &link)?,
            "creation succeeds after the target becomes available"
        );
        assert!(
            link.symlink_metadata()?.file_type().is_symlink_dir(),
            "the retry creates a directory link"
        );
        Ok(())
    }
}
