use std::path::Path;

use gix_testtools::tempfile::{TempDir, tempdir};
use gix_worktree::{Stack, stack};

const IS_FILE: Option<gix_index::entry::Mode> = Some(gix_index::entry::Mode::FILE);
const IS_DIR: Option<gix_index::entry::Mode> = Some(gix_index::entry::Mode::DIR);
const IS_SYMLINK: Option<gix_index::entry::Mode> = Some(gix_index::entry::Mode::SYMLINK);

#[test]
fn root_is_assumed_to_exist_and_files_in_root_do_not_create_directory() -> crate::Result {
    let dir = tempdir()?;
    let mut cache = Stack::new(
        dir.path().join("non-existing-root"),
        stack::State::for_checkout(false, Default::default(), Default::default()),
        Default::default(),
        Vec::new(),
        Default::default(),
    );
    assert_eq!(cache.statistics().delegate.num_mkdir_calls, 0);

    let path = cache.at_path("hello", IS_FILE, &gix_object::find::Never)?.path();
    assert!(!path.parent().unwrap().exists(), "prefix itself is never created");
    assert_eq!(cache.statistics().delegate.num_mkdir_calls, 0);
    Ok(())
}

#[test]
fn directory_paths_are_created_in_full() {
    let (mut cache, _tmp) = new_cache();

    for (name, mode) in [
        ("dir", IS_DIR),
        ("submodule", IS_DIR),
        ("file", IS_FILE),
        ("exe", IS_FILE),
        ("link", None),
    ] {
        let path = cache
            .at_path(Path::new("dir").join(name), mode, &gix_object::find::Never)
            .unwrap()
            .path();
        assert!(path.parent().unwrap().is_dir(), "dir exists");
    }

    assert_eq!(cache.statistics().delegate.num_mkdir_calls, 3);
}

#[test]
fn existing_directories_are_fine() -> crate::Result {
    let (mut cache, tmp) = new_cache();
    std::fs::create_dir(tmp.path().join("dir"))?;

    let path = cache.at_path("dir/file", IS_FILE, &gix_object::find::Never)?.path();
    assert!(path.parent().unwrap().is_dir(), "directory is still present");
    assert!(!path.exists(), "it won't create the file");
    assert_eq!(cache.statistics().delegate.num_mkdir_calls, 1);
    Ok(())
}

#[test]
fn validation_to_each_component() -> crate::Result {
    let (mut cache, tmp) = new_cache();

    let err = cache
        .at_path("valid/.gIt", IS_FILE, &gix_object::find::Never)
        .unwrap_err();
    assert_eq!(
        cache.statistics().delegate.num_mkdir_calls,
        1,
        "the valid directory was created"
    );
    assert!(tmp.path().join("valid").is_dir(), "it was actually created");
    assert_eq!(err.to_string(), "The .git name may never be used");
    Ok(())
}

#[test]
fn symlinks_or_files_in_path_are_forbidden_or_unlinked_when_forced() -> crate::Result {
    let (mut cache, tmp) = new_cache();
    let forbidden = tmp.path().join("forbidden");
    std::fs::create_dir(&forbidden)?;
    gix_fs::symlink::create(&forbidden, &tmp.path().join("link-to-dir"))?;
    std::fs::write(tmp.path().join("file-in-dir"), [])?;

    for dirname in &["file-in-dir", "link-to-dir"] {
        if let stack::State::CreateDirectoryAndAttributesStack {
            unlink_on_collision, ..
        } = cache.state_mut()
        {
            *unlink_on_collision = false;
        }
        let relative_path = format!("{dirname}/file");
        assert_eq!(
            cache
                .at_path(&*relative_path, IS_FILE, &gix_object::find::Never)
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::AlreadyExists
        );
    }
    assert_eq!(
        cache.statistics().delegate.num_mkdir_calls,
        2,
        "it tries to create each directory once, but it's a file"
    );
    cache.take_statistics();
    for dirname in &["link-to-dir", "file-in-dir"] {
        if let stack::State::CreateDirectoryAndAttributesStack {
            unlink_on_collision, ..
        } = cache.state_mut()
        {
            *unlink_on_collision = true;
        }
        let relative_path = format!("{dirname}/file");
        let path = cache
            .at_path(&*relative_path, IS_FILE, &gix_object::find::Never)?
            .path();
        assert!(path.parent().unwrap().is_dir(), "directory was forcefully created");
        assert!(!path.exists());
    }
    assert_eq!(
        cache.statistics().delegate.num_mkdir_calls,
        4,
        "like before, but it unlinks what's there and tries again"
    );
    Ok(())
}

#[test]
#[cfg(windows)]
fn terminal_symlinks_are_forbidden_without_force() -> crate::Result {
    let (mut cache, tmp) = new_cache();
    cache.enable_terminal_symlink_check();

    let target = tmp.path().join("target");
    let link = tmp.path().join("link");
    std::fs::write(&target, b"untouched")?;
    std::os::windows::fs::symlink_file(&target, &link)?;

    assert_eq!(
        cache
            .at_path("link", IS_FILE, &gix_object::find::Never)
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::AlreadyExists,
        "the terminal symlink must be rejected"
    );
    assert!(
        link.symlink_metadata()?.file_type().is_symlink(),
        "the terminal symlink must remain in place"
    );
    assert_eq!(
        std::fs::read(&target)?,
        b"untouched",
        "the symlink target must stay unchanged"
    );
    Ok(())
}

#[test]
fn symlink_cached_as_file_is_revalidated_before_use_as_directory() -> crate::Result {
    let (mut cache, tmp) = new_cache();
    let forbidden = tmp.path().join("forbidden");
    std::fs::create_dir(&forbidden)?;

    let link_path = cache
        .at_path("link", IS_SYMLINK, &gix_object::find::Never)?
        .path()
        .to_owned();
    gix_fs::symlink::create(&forbidden, &link_path)?;

    let err = cache
        .at_path("link/file", IS_SYMLINK, &gix_object::find::Never)
        .unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
    assert!(
        link_path.symlink_metadata()?.file_type().is_symlink(),
        "the existing symlink must remain in place when collisions are forbidden"
    );
    Ok(())
}

#[test]
fn symlink_cached_as_file_is_unlinked_before_use_as_directory_when_forced() -> crate::Result {
    let (mut cache, tmp) = new_cache();
    let forbidden = tmp.path().join("forbidden");
    std::fs::create_dir(&forbidden)?;

    let link_path = cache
        .at_path("link", IS_SYMLINK, &gix_object::find::Never)?
        .path()
        .to_owned();
    gix_fs::symlink::create(&forbidden, &link_path)?;
    if let stack::State::CreateDirectoryAndAttributesStack {
        unlink_on_collision, ..
    } = cache.state_mut()
    {
        *unlink_on_collision = true;
    }

    let path = cache.at_path("link/file", IS_SYMLINK, &gix_object::find::Never)?.path();
    assert_eq!(path, tmp.path().join("link").join("file"));
    assert!(
        link_path.symlink_metadata()?.is_dir(),
        "the existing symlink must be replaced with a directory when collisions may be unlinked"
    );
    Ok(())
}

#[test]
fn cached_directory_returned_as_terminal_is_revalidated_before_descending() -> crate::Result {
    for relative in ["link", "parent/link", "parent/deeper/link"] {
        for force in [false, true] {
            let (mut cache, _tmp) = new_cache();
            let target = tempdir()?;
            if let stack::State::CreateDirectoryAndAttributesStack {
                unlink_on_collision, ..
            } = cache.state_mut()
            {
                *unlink_on_collision = force;
            }

            let relative = Path::new(relative);
            let directory_count = relative.components().count();
            for name in ["first", "sibling"] {
                let _ = cache.at_path(relative.join(name), IS_FILE, &gix_object::find::Never)?;
                assert_eq!(
                    cache.statistics().delegate.num_mkdir_calls,
                    directory_count,
                    "sibling entries reuse all cached leading directories"
                );
            }

            let link = cache
                .at_path(relative, IS_SYMLINK, &gix_object::find::Never)?
                .path()
                .to_owned();
            std::fs::remove_dir(&link)?;
            gix_fs::symlink::create(target.path(), &link)?;

            let result = cache.at_path(relative.join("child"), IS_FILE, &gix_object::find::Never);
            if force {
                let child = result?.path();
                assert!(
                    link.symlink_metadata()?.is_dir(),
                    "the returned terminal must be checked again and its symlink replaced"
                );
                std::fs::write(child, b"within the worktree")?;
            } else {
                assert_eq!(
                    result
                        .expect_err("the replaced directory must not remain trusted")
                        .kind(),
                    std::io::ErrorKind::AlreadyExists,
                    "a symlink collision must be rejected without force"
                );
                assert!(
                    link.symlink_metadata()?.file_type().is_symlink(),
                    "forbidden collisions leave the symlink in place"
                );
            }
            assert!(
                target.path().read_dir()?.next().is_none(),
                "descending through a replaced cached directory must not touch the symlink target"
            );
            assert_eq!(
                cache.statistics().delegate.num_mkdir_calls,
                directory_count + if force { 2 } else { 1 },
                "only the returned terminal needs revalidation; its parents remain cached"
            );
        }
    }
    Ok(())
}

#[test]
fn cached_terminal_is_revalidated_when_mode_changes() -> crate::Result {
    let (mut cache, _tmp) = new_cache();
    for relative in [".gitmodules", "parent/.gitmodules"] {
        let _ = cache.at_path(relative, IS_FILE, &gix_object::find::Never)?;
        let err = cache
            .at_path(relative, IS_SYMLINK, &gix_object::find::Never)
            .expect_err("a cached file path must still be validated with the new mode");
        assert_eq!(
            err.to_string(),
            "The .gitmodules file must not be a symlink",
            "changing the mode must apply the symlink-specific name restriction"
        );
    }
    Ok(())
}

fn new_cache() -> (Stack, TempDir) {
    let dir = tempdir().unwrap();
    let cache = Stack::new(
        dir.path(),
        stack::State::for_checkout(false, Default::default(), Default::default()),
        Default::default(),
        Vec::new(),
        Default::default(),
    );
    (cache, dir)
}
