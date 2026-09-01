mod mark_path {
    use gix_tempfile::{AutoRemove, ContainingDirectory};

    #[test]
    fn it_persists_markers_along_with_newly_created_directories() -> crate::Result {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join("a").join("b").join("file.tmp");
        let new_filename = target.parent().unwrap().join("file.ext");
        let handle = gix_tempfile::mark_at(
            &target,
            ContainingDirectory::CreateAllRaceProof(Default::default()),
            AutoRemove::TempfileAndEmptyParentDirectoriesUntil {
                boundary_directory: dir.path().into(),
            },
        )?;

        std::fs::create_dir(&new_filename)?;
        let err = handle
            .persist(&new_filename)
            .expect_err("cannot persist onto directory");
        let handle = err.handle;
        std::fs::remove_dir(&new_filename)?;

        handle.take().expect("still there").persist(&new_filename)?;
        assert!(!target.exists(), "tempfile was renamed");
        assert!(
            new_filename.is_file(),
            "new file was placed (and parent directories still exist)"
        );
        Ok(())
    }

    #[test]
    fn it_can_create_the_containing_directory_and_remove_it_on_drop() -> crate::Result {
        let dir = tempfile::tempdir()?;
        let first_dir = "dir";
        let filename = dir.path().join(first_dir).join("subdir").join("file.tmp");
        let tempfile = gix_tempfile::mark_at(
            &filename,
            ContainingDirectory::CreateAllRaceProof(Default::default()),
            AutoRemove::TempfileAndEmptyParentDirectoriesUntil {
                boundary_directory: dir.path().into(),
            },
        )?;
        assert!(filename.is_file(), "specified file should exist precisely");
        drop(tempfile);
        assert!(
            !filename.is_file(),
            "after drop named files are deleted as well as extra directories"
        );
        assert!(
            !dir.path().join(first_dir).is_dir(),
            "previously created and now empty directories are deleted, too"
        );
        Ok(())
    }
}
mod at_path {
    use gix_tempfile::{AutoRemove, ContainingDirectory};

    #[test]
    fn reduce_resource_usage_by_converting_files_to_markers_and_persist_them() -> crate::Result {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join("a").join("file.tmp");
        let new_filename = target.parent().unwrap().join("file.ext");
        let mut file = gix_tempfile::writable_at(
            &target,
            ContainingDirectory::CreateAllRaceProof(Default::default()),
            AutoRemove::TempfileAndEmptyParentDirectoriesUntil {
                boundary_directory: dir.path().into(),
            },
        )?;
        file.with_mut(|f| f.as_file_mut().write_all(b"hello world"))??;
        let mark = file.close()?;
        mark.take().expect("still there").persist(&new_filename)?;
        assert!(!target.exists(), "tempfile was renamed");
        assert!(
            new_filename.is_file(),
            "new file was placed (and parent directories still exist)"
        );
        assert_eq!(
            std::fs::read(new_filename)?,
            &b"hello world"[..],
            "written content is persisted, too"
        );
        Ok(())
    }
    use std::io::{ErrorKind, Write};

    #[test]
    fn it_persists_tempfiles_along_with_newly_created_directories() -> crate::Result {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join("a").join("b").join("file.tmp");
        let new_filename = target.parent().unwrap().join("file.ext");
        assert!(
            !new_filename.is_file(),
            "the filename for persistence doesn't exist yet"
        );
        let handle = gix_tempfile::writable_at(
            &target,
            ContainingDirectory::CreateAllRaceProof(Default::default()),
            AutoRemove::TempfileAndEmptyParentDirectoriesUntil {
                boundary_directory: dir.path().into(),
            },
        )?;
        std::fs::create_dir(&new_filename)?;
        let err = handle
            .persist(&new_filename)
            .expect_err("cannot persist onto directory");
        let handle = err.handle;
        std::fs::remove_dir(&new_filename)?;

        let mut file = handle.take().expect("still there");
        file.write_all(b"hello world")?;
        drop(file.persist(&new_filename)?);
        assert!(!target.exists(), "tempfile was renamed");
        assert!(
            new_filename.is_file(),
            "new file was placed (and parent directories still exist)"
        );
        assert_eq!(
            std::fs::read(new_filename)?,
            &b"hello world"[..],
            "written content is persisted, too"
        );
        Ok(())
    }

    #[test]
    #[cfg(windows)]
    fn persistence_replaces_readonly_files_and_retains_the_tempfiles_permissions() -> crate::Result {
        let dir = tempfile::tempdir()?;
        let tempfile_path = dir.path().join("file.lock");
        let destination = dir.path().join("file");
        std::fs::write(&destination, b"old")?;
        let mut destination_permissions = std::fs::metadata(&destination)?.permissions();
        destination_permissions.set_readonly(true);
        std::fs::set_permissions(&destination, destination_permissions)?;

        let mut handle = gix_tempfile::writable_at(&tempfile_path, ContainingDirectory::Exists, AutoRemove::Tempfile)?;
        handle.with_mut(|file| {
            file.write_all(b"new")?;
            let mut permissions = file.as_file().metadata()?.permissions();
            permissions.set_readonly(true);
            file.as_file().set_permissions(permissions)
        })??;

        drop(handle.persist(&destination)?);
        assert_eq!(std::fs::read(&destination)?, b"new", "the destination was replaced");
        let mut permissions = std::fs::metadata(&destination)?.permissions();
        let is_readonly = permissions.readonly();
        #[expect(clippy::permissions_set_readonly_false, reason = "this test only runs on Windows")]
        permissions.set_readonly(false);
        std::fs::set_permissions(&destination, permissions)?;
        assert!(is_readonly, "the persisted tempfile retained its read-only permission");
        Ok(())
    }

    #[test]
    fn it_can_create_the_containing_directory_and_remove_it_on_drop() -> crate::Result {
        let dir = tempfile::tempdir()?;
        let first_dir = "dir";
        let filename = dir.path().join(first_dir).join("subdir").join("file.tmp");
        let tempfile = gix_tempfile::writable_at(
            &filename,
            ContainingDirectory::CreateAllRaceProof(Default::default()),
            AutoRemove::TempfileAndEmptyParentDirectoriesUntil {
                boundary_directory: dir.path().into(),
            },
        )?;
        assert!(filename.is_file(), "specified file should exist precisely");
        drop(tempfile);
        assert!(
            !filename.is_file(),
            "after drop named files are deleted as well as extra directories"
        );
        assert!(
            !dir.path().join(first_dir).is_dir(),
            "previously created and now empty directories are deleted, too"
        );
        assert!(dir.path().is_dir(), "it won't touch the containing directory");
        Ok(())
    }

    #[test]
    fn it_names_files_correctly_and_similarly_named_tempfiles_cannot_be_created() -> crate::Result {
        let dir = tempfile::tempdir()?;
        let filename = dir.path().join("something-specific.ext");
        let tempfile = gix_tempfile::writable_at(&filename, ContainingDirectory::Exists, AutoRemove::Tempfile)?;
        let res = gix_tempfile::writable_at(&filename, ContainingDirectory::Exists, AutoRemove::Tempfile);
        assert!(
            matches!(res, Err(err) if err.kind() == ErrorKind::AlreadyExists),
            "only one tempfile can be created at a time, they are exclusive"
        );
        assert!(filename.is_file(), "specified file should exist precisely");
        drop(tempfile);
        assert!(!filename.is_file(), "after drop named files are deleted as well");
        assert!(dir.path().is_dir(), "it won't touch the containing directory");
        Ok(())
    }
}

mod new {
    use std::{
        io::{ErrorKind, Write},
        path::Path,
    };

    use gix_tempfile::{AutoRemove, ContainingDirectory};

    fn filecount_in(path: impl AsRef<Path>) -> usize {
        std::fs::read_dir(path).expect("valid dir").count()
    }

    #[test]
    fn it_can_be_kept() -> crate::Result {
        let dir = tempfile::tempdir()?;
        drop(
            gix_tempfile::new(dir.path(), ContainingDirectory::Exists, AutoRemove::Tempfile)?
                .take()
                .expect("not taken yet")
                .keep()?,
        );
        assert_eq!(filecount_in(&dir), 1, "a temp file and persisted");
        Ok(())
    }

    #[test]
    fn it_is_removed_if_it_goes_out_of_scope() -> crate::Result {
        let dir = tempfile::tempdir()?;
        {
            let _keep = gix_tempfile::new(dir.path(), ContainingDirectory::Exists, AutoRemove::Tempfile)?;
            assert_eq!(filecount_in(&dir), 1, "a temp file was created");
        }
        assert_eq!(filecount_in(&dir), 0, "lock was automatically removed");
        Ok(())
    }

    #[test]
    fn it_can_create_the_containing_directory_and_remove_it_when_dropped() -> crate::Result {
        let dir = tempfile::tempdir()?;
        let containing_dir = dir.path().join("dir");
        assert!(!containing_dir.exists());
        {
            let mut writable = gix_tempfile::new(
                &containing_dir,
                ContainingDirectory::CreateAllRaceProof(Default::default()),
                AutoRemove::TempfileAndEmptyParentDirectoriesUntil {
                    boundary_directory: dir.path().into(),
                },
            )?;
            assert_eq!(
                filecount_in(&dir),
                1,
                "a temp file was created, as well as the directory"
            );
            writable.with_mut(|tf| tf.write_all(b"hello world"))??;
            assert_eq!(
                writable
                    .with_mut(|_tf| Err::<(), std::io::Error>(ErrorKind::Other.into()))?
                    .unwrap_err()
                    .kind(),
                ErrorKind::Other,
                "errors are propagated"
            );
            writable
                .with_mut(|tf| assert!(tf.path().is_file()))
                .expect("after seeing an error before the file still exists");
        }
        assert!(!containing_dir.is_dir(), "the now empty directory was deleted as well");
        assert!(dir.path().is_dir(), "it won't touch the containing directory");
        Ok(())
    }
}
