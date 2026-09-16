use std::{fs, io};

use gix_path::{into_bstr, to_unix_separators_on_windows};
use gix_testtools::tempfile::tempdir;

#[test]
#[cfg(unix)]
fn shared_permissions_include_linking_files_but_preserve_existing_directories() -> crate::Result {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempdir()?;
    let common_dir = tmp.path().join("repo.git");
    fs::create_dir(&common_dir)?;
    fs::set_permissions(&common_dir, fs::Permissions::from_mode(0o700))?;
    let options = gix_worktree::add::Options {
        shared_repository_permissions: -0o640,
        ..Default::default()
    };
    let destination = tmp.path().join("parent/worktree");
    let prepared = gix_worktree::add::prepare(&common_dir, &destination, options)?;
    for path in [
        destination.join(".git"),
        prepared.git_dir().join("gitdir"),
        prepared.git_dir().join("commondir"),
        prepared.git_dir().join("locked"),
    ] {
        assert_eq!(
            fs::metadata(path)?.permissions().mode() & 0o777,
            0o640,
            "all linking files follow the explicit sharing policy"
        );
    }
    for path in [tmp.path().join("parent"), destination, common_dir.join("worktrees")] {
        assert_eq!(
            fs::metadata(path)?.permissions().mode() & 0o777,
            0o750,
            "new directories include search permissions"
        );
    }
    assert_eq!(
        fs::metadata(&common_dir)?.permissions().mode() & 0o777,
        0o700,
        "existing repository permissions are unchanged"
    );
    let existing = tmp.path().join("existing");
    fs::create_dir(&existing)?;
    fs::set_permissions(&existing, fs::Permissions::from_mode(0o700))?;
    let prepared = gix_worktree::add::prepare(common_dir, &existing, options)?;
    prepared.rollback()?;
    assert_eq!(
        fs::metadata(existing)?.permissions().mode() & 0o777,
        0o700,
        "a caller-owned destination retains its mode after rollback"
    );
    Ok(())
}

#[test]
fn relative_input_paths_are_made_absolute() -> crate::Result {
    let cwd = std::env::current_dir()?;
    let tmp = gix_testtools::tempfile::tempdir_in(&cwd)?;
    let relative = tmp.path().strip_prefix(&cwd)?;
    let common_dir = relative.join("repo.git");
    let destination = relative.join("worktree");
    fs::create_dir(&common_dir)?;

    let prepared = gix_worktree::add::prepare(&common_dir, &destination, Default::default())?;

    assert_eq!(
        prepared.common_dir(),
        gix_path::realpath(common_dir)?,
        "the common directory is absolute"
    );
    assert_eq!(
        prepared.work_dir(),
        gix_path::realpath(destination)?,
        "the destination is absolute"
    );
    Ok(())
}

#[test]
fn prepares_git_compatible_links_and_unique_sanitized_names() -> crate::Result {
    let tmp = tempdir()?;
    let common_dir = tmp.path().join("repo.git");
    fs::create_dir(&common_dir)?;

    let first = gix_worktree::add::prepare(&common_dir, tmp.path().join("a b.lock"), Default::default())?;
    assert!(first.common_dir().is_absolute(), "the common directory is absolute");
    assert!(first.work_dir().is_absolute(), "the worktree directory is absolute");
    assert_eq!(
        first.git_dir().file_name(),
        Some("a-b".as_ref()),
        "the destination basename is sanitized like a reference component"
    );
    let lock_name = gix_worktree::add::prepare(&common_dir, tmp.path().join(".lock.lock"), Default::default())?;
    assert_eq!(
        lock_name.git_dir().file_name(),
        Some("-lock".as_ref()),
        "stripping lock suffixes preserves a non-empty worktree ID, like Git"
    );
    assert_eq!(fs::read(first.git_dir().join("locked"))?, b"initializing\n");
    assert_eq!(fs::read(first.git_dir().join("commondir"))?, b"../..\n");
    assert_eq!(
        fs::read_to_string(first.git_dir().join("gitdir"))?,
        format!(
            "{}\n",
            to_unix_separators_on_windows(into_bstr(first.work_dir().join(".git")))
        )
    );
    assert_eq!(
        fs::read_to_string(first.work_dir().join(".git"))?,
        format!(
            "gitdir: {}\n",
            to_unix_separators_on_windows(into_bstr(first.git_dir()))
        )
    );

    let second_parent = tmp.path().join("other");
    let second = gix_worktree::add::prepare(&common_dir, second_parent.join("a b.lock"), Default::default())?;
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

    let prepared = gix_worktree::add::prepare(&common_dir, &work_dir, Default::default())?;
    let git_dir = prepared.git_dir().to_owned();
    fs::write(work_dir.join("checkout-file"), b"created after preparation")?;
    fs::create_dir(work_dir.join("checkout-dir"))?;
    fs::write(work_dir.join("checkout-dir/file"), b"created after preparation")?;
    prepared.rollback()?;

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
fn explicit_rollback_removes_new_directories_but_preserves_their_parents() -> crate::Result {
    let tmp = tempdir()?;
    let common_dir = tmp.path().join("repo.git");
    let work_dir = tmp.path().join("new-parent/worktree");
    fs::create_dir(&common_dir)?;
    let prepared = gix_worktree::add::prepare(&common_dir, &work_dir, Default::default())?;
    let git_dir = prepared.git_dir().to_owned();
    fs::write(work_dir.join("checkout-file"), b"created after preparation")?;

    prepared.rollback()?;

    assert!(
        !work_dir.exists(),
        "a newly created destination is removed with its contents"
    );
    assert!(!git_dir.exists(), "the private Git directory is removed");
    assert!(
        common_dir.join("worktrees").is_dir(),
        "the administrative parent remains"
    );
    assert!(tmp.path().join("new-parent").is_dir(), "new destination parents remain");
    Ok(())
}

#[test]
fn explicit_rollback_attempts_both_directories_even_if_one_fails() -> crate::Result {
    let tmp = tempdir()?;
    let common_dir = tmp.path().join("repo.git");
    let work_dir = tmp.path().join("worktree");
    fs::create_dir(&common_dir)?;
    for remove_work_dir in [false, true] {
        let prepared = gix_worktree::add::prepare(&common_dir, &work_dir, Default::default())?;
        let git_dir = prepared.git_dir().to_owned();
        fs::remove_dir_all(if remove_work_dir { &work_dir } else { &git_dir })?;

        let err = prepared
            .rollback()
            .expect_err("the missing directory reports a cleanup error");

        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(!work_dir.exists(), "worktree cleanup is attempted independently");
        assert!(
            !git_dir.exists(),
            "private Git directory cleanup is attempted independently"
        );
    }
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

    let prepared = gix_worktree::add::prepare(&common_dir, &work_dir, Default::default())?;
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
fn linking_paths_preserve_backslashes_in_unix_filenames() -> crate::Result {
    let tmp = tempdir()?;
    let common_dir = tmp.path().join(r"repo\backslash.git");
    let work_dir = tmp.path().join(r"with\backslash");
    fs::create_dir(&common_dir)?;

    for relative_paths in [false, true] {
        let prepared = gix_worktree::add::prepare(
            &common_dir,
            &work_dir,
            gix_worktree::add::Options {
                relative_paths,
                ..Default::default()
            },
        )?;
        let dot_git = prepared.work_dir().join(".git");
        let git_dir = gix_discover::path::from_gitdir_file(&dot_git)?;
        let backlink = gix_discover::path::from_plain_file_relative_to_file(&prepared.git_dir().join("gitdir"))
            .expect("preparation writes the backlink file")?;
        assert_eq!(
            gix_path::realpath(git_dir)?,
            prepared.git_dir(),
            "relative_paths={relative_paths}: the forward link preserves backslashes in the common directory"
        );
        assert_eq!(
            gix_path::realpath(backlink)?,
            dot_git,
            "relative_paths={relative_paths}: the backlink preserves backslashes in the destination"
        );
    }
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

    let prepared = gix_worktree::add::prepare(&common_dir, linked_parent.join("worktree"), Default::default())?;
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
        gix_worktree::add::prepare(&common_dir, &nonempty, Default::default())
            .expect_err("nonempty directories are rejected")
            .kind(),
        io::ErrorKind::AlreadyExists
    );

    let file = tmp.path().join("file");
    fs::write(&file, b"content")?;
    assert_eq!(
        gix_worktree::add::prepare(&common_dir, &file, Default::default())
            .expect_err("files are rejected")
            .kind(),
        io::ErrorKind::AlreadyExists
    );

    #[cfg(unix)]
    {
        let link = tmp.path().join("link");
        std::os::unix::fs::symlink(tmp.path().join("missing"), &link)?;
        assert_eq!(
            gix_worktree::add::prepare(&common_dir, &link, Default::default())
                .expect_err("symbolic links are rejected")
                .kind(),
            io::ErrorKind::AlreadyExists
        );
    }
    Ok(())
}
