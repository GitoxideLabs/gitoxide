mod close {

    use std::io::Write;

    use gix_lock::acquire::Fail;

    #[test]
    fn acquire_close_commit_to_existing_file() -> crate::Result {
        let dir = tempfile::tempdir()?;
        let resource = dir.path().join("resource-existing.ext");
        std::fs::write(&resource, b"old state")?;
        let resource_lock = resource.with_extension("ext.lock");
        let mut file = gix_lock::File::acquire_to_update_resource(&resource, Fail::Immediately, None)?;
        assert!(resource_lock.is_file());
        file.with_mut(|out| out.write_all(b"hello world"))?;
        let mark = file.close()?;
        assert_eq!(mark.lock_path(), resource_lock);
        assert_eq!(mark.resource_path(), resource);
        assert_eq!(mark.commit()?, resource, "returned and initial resource path match");
        assert_eq!(
            std::fs::read(resource)?,
            &b"hello world"[..],
            "it created the resource and wrote the data"
        );
        assert!(!resource_lock.is_file());
        Ok(())
    }
}

mod commit {
    use gix_lock::acquire::Fail;

    #[test]
    fn failure_to_commit_does_return_a_registered_marker() -> crate::Result {
        let dir = tempfile::tempdir()?;
        let resource = dir.path().join("resource-existing.ext");
        std::fs::create_dir(&resource)?;
        let mark = gix_lock::Marker::acquire_to_hold_resource(&resource, Fail::Immediately, None)?;
        let lock_path = mark.lock_path().to_owned();
        assert!(lock_path.is_file(), "the lock is placed");

        let err = mark
            .commit()
            .expect_err("cannot commit onto existing directory, empty or not");
        assert!(err.instance.lock_path().is_file(), "the lock is still present");

        drop(err);
        assert!(
            !lock_path.is_file(),
            "the lock file is still owned by the lock instance (and ideally still registered, but hard to test)"
        );
        Ok(())
    }

    #[test]
    fn failure_to_commit_does_return_a_registered_file() -> crate::Result {
        let dir = tempfile::tempdir()?;
        let resource = dir.path().join("resource-existing.ext");
        std::fs::create_dir(&resource)?;
        let file = gix_lock::File::acquire_to_update_resource(&resource, Fail::Immediately, None)?;
        let lock_path = file.lock_path().to_owned();
        assert!(lock_path.is_file(), "the lock is placed");

        let err = file
            .commit()
            .expect_err("cannot commit onto existing directory, empty or not");
        assert!(err.instance.lock_path().is_file(), "the lock is still present");
        std::fs::remove_dir(resource)?;
        let (resource, open_file) = err.instance.commit()?;
        let mut open_file = open_file.expect("file to be present as no interrupt has messed with us");

        assert!(
            !lock_path.is_file(),
            "the lock was moved into place, now it's the resource"
        );

        use std::io::Write;
        write!(open_file, "hello")?;
        drop(open_file);
        assert_eq!(
            std::fs::read(resource)?,
            b"hello".to_vec(),
            "and committing returned a writable file handle"
        );
        Ok(())
    }
}

mod acquire {
    use std::io::{ErrorKind, Write};

    use gix_lock::acquire;

    fn fail_immediately() -> gix_lock::acquire::Fail {
        acquire::Fail::Immediately
    }

    #[test]
    fn lock_create_dir_write_commit() -> crate::Result {
        let dir = tempfile::tempdir()?;
        let resource = dir.path().join("a").join("resource-nonexisting");
        let resource_lock = resource.with_extension("lock");
        let mut file =
            gix_lock::File::acquire_to_update_resource(&resource, fail_immediately(), Some(dir.path().into()))?;
        assert_eq!(file.lock_path(), resource_lock);
        assert_eq!(file.resource_path(), resource);
        assert!(resource_lock.is_file());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = resource_lock.metadata()?.permissions();
            assert_ne!(
                perms.mode() & !0o170000,
                0o600,
                "mode is more permissive now, even after passing the umask"
            );
        }
        file.with_mut(|out| out.write_all(b"hello world"))?;
        assert_eq!(file.commit()?.0, resource, "returned and computed resource path match");
        assert_eq!(
            std::fs::read(resource)?,
            &b"hello world"[..],
            "it created the resource and wrote the data"
        );
        assert!(!resource_lock.is_file());
        Ok(())
    }

    #[test]
    fn lock_write_drop() -> crate::Result {
        let dir = tempfile::tempdir()?;
        let resource = dir.path().join("resource-nonexisting.ext");
        {
            let mut file = gix_lock::File::acquire_to_update_resource(&resource, fail_immediately(), None)?;
            file.with_mut(|out| out.write_all(b"probably we will be interrupted"))?;
        }
        assert!(!resource.is_file(), "the file wasn't created");
        Ok(())
    }

    #[test]
    fn lock_non_existing_dir_fails() -> crate::Result {
        let dir = tempfile::tempdir()?;
        let resource = dir.path().join("a").join("resource.ext");
        let res = gix_lock::File::acquire_to_update_resource(&resource, fail_immediately(), None);
        assert!(matches!(res, Err(acquire::Error::Io(err)) if err.kind() == ErrorKind::NotFound));
        assert!(dir.path().is_dir(), "it won't meddle with the containing directory");
        assert!(!resource.is_file(), "the resource is not created");
        assert!(
            !resource.parent().unwrap().is_dir(),
            "parent dire wasn't created either"
        );
        Ok(())
    }
}
