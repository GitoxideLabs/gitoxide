use std::fs;

#[test]
fn removes_both_roots_and_does_not_follow_symlinks() -> crate::Result {
    let tmp = gix_testtools::tempfile::tempdir()?;
    let worktrees_dir = tmp.path().join("repo.git/worktrees");
    let git_dir = worktrees_dir.join("linked");
    let work_dir = tmp.path().join("linked");
    let outside = tmp.path().join("outside");
    fs::create_dir_all(work_dir.join("deep/nested"))?;
    fs::create_dir_all(&git_dir)?;
    fs::create_dir(&outside)?;
    fs::write(work_dir.join("deep/nested/file"), b"content")?;
    fs::write(git_dir.join("HEAD"), b"ref: refs/heads/topic\n")?;
    fs::write(outside.join("keep"), b"outside")?;

    let link = work_dir.join("link");
    if let Err(err) = gix_fs::symlink::create(&outside, &link) {
        #[cfg(windows)]
        if err.kind() == std::io::ErrorKind::PermissionDenied {
            return Ok(());
        }
        return Err(err.into());
    }

    gix_worktree::remove::remove(&work_dir, &git_dir, gix_features::progress::Discard)?;

    assert!(!work_dir.exists(), "the checkout was removed");
    assert!(!git_dir.exists(), "the private Git directory was removed");
    assert!(!worktrees_dir.exists(), "an empty worktrees directory was removed");
    assert_eq!(
        fs::read(outside.join("keep"))?,
        b"outside",
        "directory symlinks are not followed"
    );
    Ok(())
}

#[test]
fn missing_roots_are_already_removed() -> crate::Result {
    let tmp = gix_testtools::tempfile::tempdir()?;
    gix_worktree::remove::remove(
        tmp.path().join("missing-worktree"),
        tmp.path().join("repo.git/worktrees/missing"),
        gix_features::progress::Discard,
    )?;
    let non_directory = tmp.path().join("file");
    fs::write(&non_directory, b"not a directory")?;
    gix_worktree::remove::remove(
        non_directory.join("missing-worktree"),
        tmp.path().join("repo.git/worktrees/missing"),
        gix_features::progress::Discard,
    )?;
    Ok(())
}

#[test]
fn only_the_conventional_empty_worktrees_parent_is_removed() -> crate::Result {
    let tmp = gix_testtools::tempfile::tempdir()?;
    let parent = tmp.path().join("custom-parent");
    let git_dir = parent.join("linked");
    fs::create_dir_all(&git_dir)?;

    gix_worktree::remove::remove(
        tmp.path().join("missing-worktree"),
        &git_dir,
        gix_features::progress::Discard,
    )?;

    assert!(parent.exists(), "an arbitrary parent directory is retained");
    Ok(())
}

#[test]
#[cfg(unix)]
fn administrative_data_is_removed_after_checkout_removal_fails() -> crate::Result {
    use std::os::unix::fs::PermissionsExt;

    let tmp = gix_testtools::tempfile::tempdir()?;
    let work_dir = tmp.path().join("linked");
    let git_dir = tmp.path().join("repo.git/worktrees/linked");
    fs::create_dir(&work_dir)?;
    fs::write(work_dir.join("protected"), b"content")?;
    fs::set_permissions(&work_dir, fs::Permissions::from_mode(0o500))?;
    fs::create_dir_all(&git_dir)?;
    fs::write(git_dir.join("HEAD"), b"ref: refs/heads/topic\n")?;

    let result = gix_worktree::remove::remove(&work_dir, &git_dir, gix_features::progress::Discard);
    if work_dir.exists() {
        fs::set_permissions(&work_dir, fs::Permissions::from_mode(0o700))?;
    }
    let err = result.expect_err("a protected checkout cannot be removed");

    assert!(
        matches!(err, gix_worktree::remove::Error::Worktree(_)),
        "only the checkout failed"
    );
    assert!(
        !git_dir.exists(),
        "administrative data is removed despite checkout failure"
    );
    Ok(())
}

#[test]
#[cfg(windows)]
fn readonly_files_do_not_prevent_removal() -> crate::Result {
    let tmp = gix_testtools::tempfile::tempdir()?;
    let work_dir = tmp.path().join("linked");
    let git_dir = tmp.path().join("repo.git/worktrees/linked");
    fs::create_dir(&work_dir)?;
    fs::create_dir_all(&git_dir)?;
    let readonly = work_dir.join("readonly");
    fs::write(&readonly, b"content")?;
    let mut permissions = fs::metadata(&readonly)?.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&readonly, permissions)?;

    gix_worktree::remove::remove(&work_dir, &git_dir, gix_features::progress::Discard)?;

    assert!(!work_dir.exists(), "the checkout was removed");
    assert!(!git_dir.exists(), "the private Git directory was removed");
    Ok(())
}
