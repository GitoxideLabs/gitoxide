mod all {
    use crate::Result;
    use gix_fs::dir::create;

    #[test]
    fn a_deeply_nested_directory() -> Result {
        let dir = tempfile::tempdir()?;
        let target = &dir.path().join("1").join("2").join("3").join("4").join("5").join("6");
        let dir = create::all(target, Default::default())?;
        assert_eq!(dir, target, "all subdirectories can be created");
        Ok(())
    }
}
mod iter {
    use crate::Result;
    pub use std::io::ErrorKind::*;

    use gix_fs::dir::{
        create,
        create::{Error::*, Retries},
    };

    #[test]
    fn an_existing_directory_causes_immediate_success() -> Result {
        let dir = tempfile::tempdir()?;
        let mut it = create::Iter::new(dir.path());
        assert_eq!(
            it.next().expect("item").expect("success"),
            dir.path(),
            "first iteration is immediately successful"
        );
        assert!(it.next().is_none(), "iterator exhausted afterwards");
        Ok(())
    }

    #[test]
    fn a_single_directory_can_be_created_too() -> Result {
        let dir = tempfile::tempdir()?;
        let new_dir = dir.path().join("new");
        let mut it = create::Iter::new(&new_dir);
        assert_eq!(
            it.next().expect("item").expect("success"),
            &new_dir,
            "first iteration is immediately successful"
        );
        assert!(it.next().is_none(), "iterator exhausted afterwards");
        assert!(new_dir.is_dir(), "the directory exists");
        Ok(())
    }

    #[test]
    fn multiple_intermediate_directories_are_created_automatically() -> Result {
        let dir = tempfile::tempdir()?;
        let new_dir = dir.path().join("s1").join("s2").join("new");
        let mut it = create::Iter::new(&new_dir);
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
        assert_eq!(
            it.next().expect("item").expect("success"),
            new_dir,
            "target directory is created"
        );
        assert!(it.next().is_none(), "iterator depleted");
        assert!(new_dir.is_dir(), "the directory exists");
        Ok(())
    }

    #[test]
    fn multiple_intermediate_directories_are_created_up_to_retries_limit() -> Result {
        let dir = tempfile::tempdir()?;
        let new_dir = dir.path().join("s1").join("s2").join("new");
        let mut it = create::Iter::new_with_retries(
            &new_dir,
            Retries {
                on_create_directory_failure: 1,
                ..Default::default()
            },
        );
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
            matches!(failure, Permanent{ retries_left, dir, err, ..} if retries_left.on_create_directory_failure == 0
                                                                    && err.kind() == NotFound
                                                                    && dir == new_dir),
            "parent dir is not present and we run out of attempts"
        );
        assert!(it.next().is_none(), "iterator depleted");
        assert!(!new_dir.is_dir(), "the wasn't created");
        Ok(())
    }

    #[test]
    fn an_existing_file_makes_directory_creation_fail_permanently() -> Result {
        let dir = tempfile::tempdir()?;
        let new_dir = dir.path().join("also-file");
        std::fs::write(&new_dir, [42])?;
        assert!(new_dir.is_file());

        let mut it = create::Iter::new(&new_dir);
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
    fn racy_directory_creation_with_new_directory_being_deleted_not_enough_retries() -> Result {
        let dir = tempfile::tempdir()?;
        let new_dir = dir.path().join("a").join("new");
        let parent_dir = new_dir.parent().unwrap();
        let mut it = create::Iter::new_with_retries(
            &new_dir,
            Retries {
                to_create_entire_directory: 2,
                on_create_directory_failure: 2,
                ..Default::default()
            },
        );

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
    fn racy_directory_creation_with_new_directory_being_deleted() -> Result {
        let dir = tempfile::tempdir()?;
        let new_dir = dir.path().join("a").join("new");
        let parent_dir = new_dir.parent().unwrap();
        let mut it = create::Iter::new(&new_dir);

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
