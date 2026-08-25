use std::{fs, io};

use gix_testtools::tempfile::tempdir;

#[test]
fn prepares_git_compatible_links_and_unique_sanitized_names() -> crate::Result {
    let tmp = tempdir()?;
    let common_dir = tmp.path().join("repo.git");
    fs::create_dir(&common_dir)?;

    let first = gix_worktree::create::prepare(&common_dir, tmp.path().join("a b.lock"))?;
    assert!(first.common_dir().is_absolute(), "the common directory is absolute");
    assert!(first.work_dir().is_absolute(), "the worktree directory is absolute");
    assert_eq!(
        first.git_dir().file_name(),
        Some("a-b".as_ref()),
        "the destination basename is sanitized like a reference component"
    );
    assert_eq!(fs::read(first.git_dir().join("locked"))?, b"initializing\n");
    assert_eq!(fs::read(first.git_dir().join("commondir"))?, b"../..\n");
    assert_eq!(
        fs::read_to_string(first.git_dir().join("gitdir"))?,
        format!("{}\n", first.work_dir().join(".git").display())
    );
    assert_eq!(
        fs::read_to_string(first.work_dir().join(".git"))?,
        format!("gitdir: {}\n", first.git_dir().display())
    );

    let second_parent = tmp.path().join("other");
    let second = gix_worktree::create::prepare(&common_dir, second_parent.join("a b.lock"))?;
    assert_eq!(
        second.git_dir().file_name(),
        Some("a-b1".as_ref()),
        "an atomic numeric suffix avoids an existing ID"
    );
    let second_git_dir = second.git_dir().to_owned();
    let second_work_dir = second.work_dir().to_owned();
    drop(second);
    assert!(
        !second_git_dir.exists(),
        "dropping rolls back the private Git directory"
    );
    assert!(
        !second_work_dir.exists(),
        "dropping removes a worktree directory it created"
    );

    let first_git_dir = first.git_dir().to_owned();
    let first_work_dir = first.work_dir().to_owned();
    first.persist()?;
    assert!(
        !first_git_dir.join("locked").exists(),
        "persisting removes the initialization lock"
    );
    assert!(first_git_dir.is_dir(), "persisting retains the private Git directory");
    assert!(first_work_dir.is_dir(), "persisting retains the worktree directory");
    Ok(())
}

#[test]
fn rollback_preserves_a_caller_owned_empty_directory() -> crate::Result {
    let tmp = tempdir()?;
    let common_dir = tmp.path().join("repo.git");
    let work_dir = tmp.path().join("existing");
    fs::create_dir(&common_dir)?;
    fs::create_dir(&work_dir)?;

    let prepared = gix_worktree::create::prepare(&common_dir, &work_dir)?;
    let git_dir = prepared.git_dir().to_owned();
    fs::write(work_dir.join("checkout-file"), b"created after preparation")?;
    fs::create_dir(work_dir.join("checkout-dir"))?;
    fs::write(work_dir.join("checkout-dir/file"), b"created after preparation")?;
    drop(prepared);

    assert!(work_dir.is_dir(), "a caller-owned destination remains");
    assert_eq!(
        fs::read_dir(&work_dir)?.count(),
        0,
        "all operation-owned contents are removed"
    );
    assert!(!git_dir.exists(), "the private Git directory is removed");
    Ok(())
}

#[test]
fn rollback_removes_directory_symlinks_without_touching_their_target() -> crate::Result {
    let tmp = tempdir()?;
    let common_dir = tmp.path().join("repo.git");
    let work_dir = tmp.path().join("existing");
    let target = tmp.path().join("target");
    fs::create_dir(&common_dir)?;
    fs::create_dir(&work_dir)?;
    fs::create_dir(&target)?;
    fs::write(target.join("keep"), b"user data")?;

    let prepared = gix_worktree::create::prepare(&common_dir, &work_dir)?;
    let link = work_dir.join("checkout-link");
    if let Err(err) = gix_fs::symlink::create(&target, &link) {
        #[cfg(windows)]
        if err.kind() == io::ErrorKind::PermissionDenied {
            return Ok(());
        }
        return Err(err.into());
    }
    drop(prepared);

    assert!(!link.exists(), "the directory symlink is removed");
    assert_eq!(
        fs::read(target.join("keep"))?,
        b"user data",
        "rollback does not follow the directory symlink"
    );
    Ok(())
}

#[test]
#[cfg(unix)]
fn linking_paths_resolve_symlinked_parent_directories() -> crate::Result {
    let tmp = tempdir()?;
    let actual_parent = tmp.path().join("actual");
    let linked_parent = tmp.path().join("linked");
    fs::create_dir(&actual_parent)?;
    std::os::unix::fs::symlink(&actual_parent, &linked_parent)?;
    let common_dir = linked_parent.join("repo.git");
    fs::create_dir(&common_dir)?;

    let prepared = gix_worktree::create::prepare(&common_dir, linked_parent.join("worktree"))?;
    let actual_common_dir = gix_path::realpath(actual_parent.join("repo.git"))?;
    let actual_work_dir = gix_path::realpath(actual_parent.join("worktree"))?;
    assert_eq!(
        prepared.common_dir(),
        actual_common_dir,
        "the common directory's real path is stored"
    );
    assert_eq!(
        prepared.work_dir(),
        actual_work_dir,
        "the worktree directory's real path is stored"
    );
    assert_eq!(
        fs::read_to_string(prepared.git_dir().join("gitdir"))?,
        format!("{}\n", actual_work_dir.join(".git").display()),
        "the private Git directory links to the real worktree path"
    );
    assert_eq!(
        fs::read_to_string(actual_work_dir.join(".git"))?,
        format!("gitdir: {}\n", prepared.git_dir().display()),
        "the worktree links to the real private Git directory"
    );
    Ok(())
}

#[test]
fn rejects_occupied_destinations() -> crate::Result {
    let tmp = tempdir()?;
    let common_dir = tmp.path().join("repo.git");
    fs::create_dir(&common_dir)?;

    let nonempty = tmp.path().join("nonempty");
    fs::create_dir(&nonempty)?;
    fs::write(nonempty.join("file"), b"content")?;
    assert_eq!(
        gix_worktree::create::prepare(&common_dir, &nonempty)
            .expect_err("nonempty directories are rejected")
            .kind(),
        io::ErrorKind::AlreadyExists
    );

    let file = tmp.path().join("file");
    fs::write(&file, b"content")?;
    assert_eq!(
        gix_worktree::create::prepare(&common_dir, &file)
            .expect_err("files are rejected")
            .kind(),
        io::ErrorKind::AlreadyExists
    );

    #[cfg(unix)]
    {
        let link = tmp.path().join("link");
        std::os::unix::fs::symlink(tmp.path().join("missing"), &link)?;
        assert_eq!(
            gix_worktree::create::prepare(&common_dir, &link)
                .expect_err("symbolic links are rejected")
                .kind(),
            io::ErrorKind::AlreadyExists
        );
    }
    Ok(())
}
