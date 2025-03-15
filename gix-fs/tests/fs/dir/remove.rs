mod empty_upwards_until_boundary {
    use std::{io, path::Path};

    use gix_fs::dir::remove;

    #[test]
    fn boundary_must_contain_target_dir() -> crate::Result {
        let dir = tempfile::tempdir()?;
        let (target, boundary) = (dir.path().join("a"), dir.path().join("b"));
        std::fs::create_dir(&target)?;
        std::fs::create_dir(&boundary)?;
        assert!(matches!(remove::empty_upward_until_boundary(&target, &boundary),
                            Err(err) if err.kind() == io::ErrorKind::InvalidInput));
        assert!(target.is_dir());
        assert!(boundary.is_dir());
        Ok(())
    }
    #[test]
    fn target_directory_non_existing_causes_existing_parents_not_to_be_deleted() -> crate::Result {
        let dir = tempfile::tempdir()?;
        let parent = dir.path().join("a");
        std::fs::create_dir(&parent)?;
        let target = parent.join("not-existing");
        assert_eq!(remove::empty_upward_until_boundary(&target, dir.path())?, target);
        assert!(
            parent.is_dir(),
            "the parent wasn't touched if the target already wasn't present"
        );
        Ok(())
    }

    #[test]
    fn target_directory_being_a_file_immediately_fails() -> crate::Result {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join("actually-a-file");
        std::fs::write(&target, [42])?;
        assert!(remove::empty_upward_until_boundary(&target, dir.path()).is_err()); // TODO: check for IsNotADirectory when it becomes stable
        assert!(target.is_file(), "it didn't touch the file");
        assert!(dir.path().is_dir(), "it won't touch the boundary");
        Ok(())
    }
    #[test]
    fn boundary_being_the_target_dir_always_succeeds_and_we_do_nothing() -> crate::Result {
        let dir = tempfile::tempdir()?;
        assert_eq!(remove::empty_upward_until_boundary(dir.path(), dir.path())?, dir.path());
        assert!(dir.path().is_dir(), "it won't touch the boundary");
        Ok(())
    }
    #[test]
    fn a_directory_which_doesnt_exist_to_start_with_is_ok() -> crate::Result {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join("does-not-exist");
        assert_eq!(remove::empty_upward_until_boundary(&target, dir.path())?, target);
        assert!(dir.path().is_dir(), "it won't touch the boundary");
        Ok(())
    }
    #[test]
    fn boundary_directory_doesnt_have_to_exist_either_if_the_target_doesnt() -> crate::Result {
        let boundary = Path::new("/boundary");
        let target = Path::new("/boundary/target");
        assert_eq!(remove::empty_upward_until_boundary(target, boundary)?, target);
        Ok(())
    }
    #[test]
    fn nested_directory_deletion_works() -> crate::Result {
        let dir = tempfile::tempdir()?;
        let nested = dir.path().join("a").join("b").join("to-delete");
        std::fs::create_dir_all(&nested)?;
        assert_eq!(remove::empty_upward_until_boundary(&nested, dir.path())?, nested);
        assert!(!nested.is_dir(), "it actually deleted the nested directory");
        assert!(!nested.parent().unwrap().is_dir(), "parent one was deleted");
        assert!(
            !nested.parent().unwrap().parent().unwrap().is_dir(),
            "parent two was deleted"
        );
        assert!(dir.path().is_dir(), "it won't touch the boundary");
        Ok(())
    }
}

mod empty_depth_first {
    use std::{
        fs::{create_dir, create_dir_all},
        path::Path,
    };

    #[test]
    fn non_empty_anywhere_and_deletion_fails() -> crate::Result {
        let dir = tempfile::TempDir::new()?;
        let touch = |base: &Path, name: &str| create_dir_all(base).and_then(|_| std::fs::write(base.join(name), b""));

        let nested_parent = dir.path().join("a");
        touch(&nested_parent, "hello.ext")?;

        let tree_parent = dir.path().join("tree");
        touch(&tree_parent.join("a").join("b"), "hello.ext")?;
        create_dir_all(tree_parent.join("one").join("two").join("empty"))?;

        assert!(gix_fs::dir::remove::empty_depth_first(nested_parent).is_err());
        Ok(())
    }

    #[test]
    fn nested_empty_and_single_empty_delete_successfully() {
        let dir = tempfile::TempDir::new().unwrap();
        let nested_parent = dir.path().join("a");
        let nested = nested_parent.join("b").join("leaf");
        create_dir_all(nested).unwrap();

        let single_parent = dir.path().join("single");
        create_dir(&single_parent).unwrap();

        let tree_parent = dir.path().join("tree");
        create_dir_all(tree_parent.join("a").join("b")).unwrap();
        create_dir_all(tree_parent.join("one").join("two").join("three")).unwrap();
        create_dir_all(tree_parent.join("c")).unwrap();
        for empty in &[nested_parent, single_parent, tree_parent] {
            gix_fs::dir::remove::empty_depth_first(empty.into()).unwrap();
        }
    }
}

/// We assume that all checks above also apply to the iterator, so won't repeat them here
/// Test outside interference only
mod iter {
    use gix_fs::dir::remove;

    #[test]
    fn racy_directory_creation_during_deletion_always_wins_immediately() -> crate::Result {
        let dir = tempfile::tempdir()?;
        let nested = dir.path().join("a").join("b").join("to-delete");
        std::fs::create_dir_all(&nested)?;

        let mut it = remove::Iter::new(&nested, dir.path())?;
        assert_eq!(it.next().expect("item")?, nested, "delete leaves directory");

        // recreate the deleted directory in racy fashion, causing the next-to-delete directory not to be empty.
        std::fs::create_dir(&nested)?;
        assert!(
            it.next().expect("err item").is_err(),
            "cannot delete non-empty directory" // TODO: check for IsADirectory when it becomes stable
        );
        assert!(it.next().is_none(), "iterator is depleted");
        Ok(())
    }
}
