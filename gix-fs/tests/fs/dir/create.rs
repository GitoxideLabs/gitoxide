mod all {
    use crate::TestResult;
    use gix_fs::dir::create;

    #[test]
    #[cfg(unix)]
    fn shared_permissions_apply_only_to_new_directories() -> TestResult {
        use std::{fs, os::unix::fs::PermissionsExt};

        let dir = tempfile::tempdir()?;
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700))?;
        let target = dir.path().join("one/two");
        create::all(&target, Default::default(), -0o640)?;
        assert_eq!(
            fs::metadata(dir.path())?.permissions().mode() & 0o7777,
            0o700,
            "existing ancestors retain their mode"
        );
        for path in [dir.path().join("one"), target.clone()] {
            let mode = fs::metadata(&path)?.permissions().mode();
            assert_eq!(
                mode & 0o777,
                0o750,
                "directories gain search access wherever the sharing policy grants read access"
            );
            if cfg!(target_os = "linux") {
                assert_ne!(mode & 0o2000, 0, "shared directories inherit their group on Linux");
            }
        }
        create::all(&target, Default::default(), 0o660)?;
        assert_eq!(
            fs::metadata(target)?.permissions().mode() & 0o777,
            0o750,
            "an existing destination is not chmodded by directory creation"
        );
        Ok(())
    }

    #[test]
    fn single_and_existing_directories_succeed() -> TestResult {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join("new");
        assert_eq!(
            create::all(&target, Default::default(), 0)?,
            target,
            "creating a single directory returns the input path"
        );
        assert!(target.is_dir(), "the destination was created");
        assert_eq!(
            create::all(&target, Default::default(), 0)?,
            target,
            "an existing directory is also a successful destination"
        );
        Ok(())
    }

    #[test]
    fn missing_parents_exhaust_the_configured_retries() -> TestResult {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join("one/two");
        let retries = create::Retries {
            to_create_entire_directory: 5,
            on_create_directory_failure: 1,
            on_interrupt: 10,
        };
        let failure = create::all(&target, retries, 0).expect_err("a single attempt cannot create missing parents");
        assert_eq!(
            failure.kind(),
            std::io::ErrorKind::NotFound,
            "the stable helper returns the underlying permanent failure"
        );
        assert!(
            !dir.path().join("one").exists(),
            "exhausting retries before visiting a parent leaves it uncreated"
        );
        Ok(())
    }

    #[test]
    fn an_existing_file_cannot_be_a_destination_or_parent() -> TestResult {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("file");
        std::fs::write(&file, b"keep")?;
        for target in [file.clone(), file.join("child")] {
            let failure =
                create::all(&target, Default::default(), 0).expect_err("a file cannot be used as a directory");
            assert_eq!(
                failure.kind(),
                std::io::ErrorKind::NotADirectory,
                "the stable helper reports file collisions as directory errors"
            );
        }
        assert_eq!(std::fs::read(&file)?, b"keep", "the existing file remains untouched");
        Ok(())
    }

    #[test]
    fn a_deeply_nested_directory() -> TestResult {
        let dir = tempfile::tempdir()?;
        let target = &dir.path().join("1").join("2").join("3").join("4").join("5").join("6");
        let dir = create::all(target, Default::default(), 0)?;
        assert_eq!(dir, target, "all subdirectories can be created");
        assert!(target.is_dir(), "the deepest destination was created");
        Ok(())
    }
}
mod iter {
    use crate::TestResult;
    pub use std::io::ErrorKind::*;

    use gix_fs::dir::{
        create,
        create::{Error::*, Retries},
    };

    #[test]
    fn an_existing_directory_causes_immediate_success() -> TestResult {
        let dir = tempfile::tempdir()?;
        let mut it = create::Iter::new(dir.path(), 0);
        let created = it.next().expect("item").expect("success");
        assert_eq!(created, dir.path(), "first iteration is immediately successful");
        assert!(it.next().is_none(), "iterator exhausted afterwards");
        Ok(())
    }

    #[test]
    fn a_single_directory_can_be_created_too() -> TestResult {
        let dir = tempfile::tempdir()?;
        let new_dir = dir.path().join("new");
        let mut it = create::Iter::new(&new_dir, 0);
        let created = it.next().expect("item").expect("success");
        assert_eq!(created, &new_dir, "first iteration is immediately successful");
        assert!(it.next().is_none(), "iterator exhausted afterwards");
        assert!(new_dir.is_dir(), "the directory exists");
        Ok(())
    }

    #[test]
    fn multiple_intermediate_directories_are_created_automatically() -> TestResult {
        let dir = tempfile::tempdir()?;
        let new_dir = dir.path().join("s1").join("s2").join("new");
        let mut it = create::Iter::new(&new_dir, 0);
        let failure = it
            .next()
            .expect("the iterator reports its next creation attempt")
            .expect_err("the operation must fail");
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&failure, &[(&new_dir.to_string_lossy(), "<target>"), (&new_dir.parent().expect("target has a parent").to_string_lossy(), "<parent>")]), "dir is not present", @r#"
        Intermediate {
            dir: "<target>",
            kind: NotFound,
        }
        "#);
        assert!(
            matches!(failure, Intermediate{dir, kind: k} if k == NotFound && dir == new_dir),
            "dir is not present"
        );
        let failure = it
            .next()
            .expect("the iterator reports its next creation attempt")
            .expect_err("the operation must fail");
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&failure, &[(&new_dir.to_string_lossy(), "<target>"), (&new_dir.parent().expect("target has a parent").to_string_lossy(), "<parent>")]), "parent dir is not present", @r#"
        Intermediate {
            dir: "<parent>",
            kind: NotFound,
        }
        "#);
        assert!(
            matches!(failure, Intermediate{dir, kind:k} if k == NotFound && dir == new_dir.parent().unwrap()),
            "parent dir is not present"
        );
        assert_eq!(
            it.next().expect("item").expect("success"),
            new_dir.parent().unwrap().parent().unwrap(),
            "first subdir is created"
        );
        assert_eq!(
            it.next().expect("item").expect("success"),
            new_dir.parent().unwrap(),
            "second subdir is created"
        );
        let created = it.next().expect("item").expect("success");
        assert_eq!(created, new_dir, "target directory is created");
        assert!(it.next().is_none(), "iterator depleted");
        assert!(new_dir.is_dir(), "the directory exists");
        Ok(())
    }

    #[test]
    fn multiple_intermediate_directories_are_created_up_to_retries_limit() -> TestResult {
        let dir = tempfile::tempdir()?;
        let new_dir = dir.path().join("s1").join("s2").join("new");
        let limits = Retries {
            on_create_directory_failure: 1,
            ..Default::default()
        };
        let mut it = create::Iter::new(&new_dir, 0).retries(limits);
        let failure = it
            .next()
            .expect("the iterator reports its next creation attempt")
            .expect_err("the operation must fail");
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&failure, &[(&new_dir.to_string_lossy(), "<target>"), (&new_dir.parent().expect("target has a parent").to_string_lossy(), "<parent>")]), "parent dir is not present and we run out of attempts", @r#"
        Permanent {
            dir: "<target>",
            err: Kind(
                NotFound,
            ),
            retries_left: Retries {
                to_create_entire_directory: 5,
                on_create_directory_failure: 0,
                on_interrupt: 10,
            },
            retries: Retries {
                to_create_entire_directory: 5,
                on_create_directory_failure: 1,
                on_interrupt: 10,
            },
        }
        "#);
        assert!(
            matches!(failure, Permanent{ retries_left, retries, dir, err } if retries_left.on_create_directory_failure == 0
                                                                    && retries == limits
                                                                    && err.kind() == NotFound
                                                                    && dir == new_dir),
            "the configured retries are exhausted and retained in the error"
        );
        assert!(it.next().is_none(), "iterator depleted");
        assert!(!new_dir.is_dir(), "the wasn't created");
        Ok(())
    }

    #[test]
    fn an_existing_file_makes_directory_creation_fail_permanently() -> TestResult {
        let dir = tempfile::tempdir()?;
        let new_dir = dir.path().join("also-file");
        std::fs::write(&new_dir, [42])?;
        assert!(new_dir.is_file());

        let mut it = create::Iter::new(&new_dir, 0);
        let failure = it
            .next()
            .expect("the iterator reports its next creation attempt")
            .expect_err("the operation must fail");
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&failure, &[(&new_dir.to_string_lossy(), "<target>"), (&new_dir.parent().expect("target has a parent").to_string_lossy(), "<parent>")]), "a non-directory in the path is reported precisely as NotADirectory", @r#"
        Permanent {
            dir: "<target>",
            err: Custom {
                kind: NotADirectory,
                error: AlreadyExists,
            },
            retries_left: Retries {
                to_create_entire_directory: 5,
                on_create_directory_failure: 25,
                on_interrupt: 10,
            },
            retries: Retries {
                to_create_entire_directory: 5,
                on_create_directory_failure: 25,
                on_interrupt: 10,
            },
        }
        "#);
        assert!(
            matches!(failure, Permanent{ dir, err, .. } if err.kind() == NotADirectory
                                                                    && dir == new_dir),
            "a non-directory in the path is reported precisely as NotADirectory"
        );
        assert!(it.next().is_none(), "iterator depleted");
        assert!(new_dir.is_file(), "file is untouched");
        Ok(())
    }
    #[test]
    fn racy_directory_creation_with_new_directory_being_deleted_not_enough_retries() -> TestResult {
        let dir = tempfile::tempdir()?;
        let new_dir = dir.path().join("a").join("new");
        let parent_dir = new_dir.parent().unwrap();
        let mut it = create::Iter::new(&new_dir, 0).retries(Retries {
            to_create_entire_directory: 2,
            on_create_directory_failure: 2,
            ..Default::default()
        });

        assert!(
            matches!(it.nth(1), Some(Ok(dir)) if dir == parent_dir),
            "parent dir is created"
        );
        // Someone deletes the new directory
        std::fs::remove_dir(parent_dir)?;

        assert!(
            matches!(it.nth(1), Some(Ok(dir)) if dir == parent_dir),
            "parent dir is created"
        );
        // Someone deletes the new directory, again
        std::fs::remove_dir(parent_dir)?;

        let failure = it
            .next()
            .expect("the iterator reports its next creation attempt")
            .expect_err("the operation must fail");
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&failure, &[(&new_dir.to_string_lossy(), "<target>"), (&new_dir.parent().expect("target has a parent").to_string_lossy(), "<parent>")]), "we run out of attempts to retry to combat against raciness", @r#"
        Permanent {
            dir: "<target>",
            err: Kind(
                NotFound,
            ),
            retries_left: Retries {
                to_create_entire_directory: 0,
                on_create_directory_failure: 1,
                on_interrupt: 10,
            },
            retries: Retries {
                to_create_entire_directory: 2,
                on_create_directory_failure: 2,
                on_interrupt: 10,
            },
        }
        "#);
        assert!(
            matches!(failure, Permanent{ retries_left, dir, err, .. } if retries_left.to_create_entire_directory == 0
                                                                    && retries_left.on_create_directory_failure == 1
                                                                    && err.kind() == NotFound
                                                                    && dir == new_dir),
            "we run out of attempts to retry to combat against raciness"
        );
        Ok(())
    }

    #[test]
    fn racy_directory_creation_with_new_directory_being_deleted() -> TestResult {
        let dir = tempfile::tempdir()?;
        let new_dir = dir.path().join("a").join("new");
        let parent_dir = new_dir.parent().unwrap();
        let mut it = create::Iter::new(&new_dir, 0);

        let failure = it
            .next()
            .expect("the iterator reports its next creation attempt")
            .expect_err("the operation must fail");
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&failure, &[(&new_dir.to_string_lossy(), "<target>"), (&new_dir.parent().expect("target has a parent").to_string_lossy(), "<parent>")]), "dir is not present, and we go up a level", @r#"
        Intermediate {
            dir: "<target>",
            kind: NotFound,
        }
        "#);
        assert!(
            matches!(failure, Intermediate{dir, kind:k} if k == NotFound && dir == new_dir),
            "dir is not present, and we go up a level"
        );
        assert!(
            matches!(it.next(), Some(Ok(dir)) if dir == parent_dir),
            "parent dir is created"
        );
        // Someone deletes the new directory
        std::fs::remove_dir(parent_dir)?;

        let failure = it
            .next()
            .expect("the iterator reports its next creation attempt")
            .expect_err("the operation must fail");
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&failure, &[(&new_dir.to_string_lossy(), "<target>"), (&new_dir.parent().expect("target has a parent").to_string_lossy(), "<parent>")]), "now when it tries the actual dir its not found", @r#"
        Intermediate {
            dir: "<target>",
            kind: NotFound,
        }
        "#);
        assert!(
            matches!(failure, Intermediate{dir, kind:k} if k == NotFound && dir == new_dir),
            "now when it tries the actual dir its not found"
        );
        assert!(
            matches!(it.next(), Some(Ok(dir)) if dir == parent_dir),
            "parent dir is created as it retries"
        );
        assert!(
            matches!(it.next(), Some(Ok(dir)) if dir == new_dir),
            "target dir is created successfully"
        );
        assert!(it.next().is_none(), "iterator depleted");
        assert!(new_dir.is_dir());

        Ok(())
    }
}
