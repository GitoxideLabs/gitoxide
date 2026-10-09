use std::fs;

use gix_worktree::remove::Options;

#[test]
fn relative_roots_are_rejected_before_either_root_is_removed() -> gix_testtools::TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let tmp = gix_testtools::tempfile::tempdir()?;
    let _cwd = gix_testtools::set_current_dir(tmp.path())?;
    let work_dir = tmp.path().join("checkout");
    let git_dir = tmp.path().join("repo.git/worktrees/linked");
    fs::create_dir(&work_dir)?;
    fs::create_dir_all(&git_dir)?;
    fs::write(work_dir.join("keep"), b"checkout content")?;
    fs::write(git_dir.join("keep"), b"administrative content")?;

    let relative_work_dir = std::path::Path::new("checkout");
    let relative_git_dir = std::path::Path::new("repo.git/worktrees/linked");
    for (work_dir_input, git_dir_input) in [
        (relative_work_dir, git_dir.as_path()),
        (work_dir.as_path(), relative_git_dir),
        (relative_work_dir, relative_git_dir),
        (std::path::Path::new(""), git_dir.as_path()),
        (work_dir.as_path(), std::path::Path::new("")),
        (std::path::Path::new("."), git_dir.as_path()),
        (work_dir.as_path(), std::path::Path::new(".")),
    ] {
        let err = gix_worktree::remove(
            work_dir_input,
            git_dir_input,
            gix_utils::progress::Discard,
            Options::default(),
        )
        .expect_err("both roots must be absolute before deletion can begin");
        let rejected = if work_dir_input.is_absolute() {
            git_dir_input
        } else {
            work_dir_input
        };
        assert!(
            matches!(err, gix_worktree::remove::Error::RelativePath { ref path } if path.as_path() == rejected),
            "the error identifies the first relative root without reporting a partial deletion: {err:?}"
        );
        assert_eq!(
            fs::read(work_dir.join("keep"))?,
            b"checkout content",
            "rejecting either argument leaves the checkout untouched"
        );
        assert_eq!(
            fs::read(git_dir.join("keep"))?,
            b"administrative content",
            "rejecting either argument leaves the private Git directory untouched"
        );
    }
    Ok(())
}

#[test]
fn removes_both_roots_with_configured_thread_limits_without_retries() -> gix_testtools::TestResult {
    for thread_limit in [None, Some(0), Some(1), Some(2)] {
        let tmp = gix_testtools::tempfile::tempdir()?;
        let worktrees_dir = tmp.path().join("repo.git/worktrees");
        let git_dir = worktrees_dir.join("linked");
        let work_dir = tmp.path().join("linked");
        for root in [&work_dir, &git_dir] {
            fs::create_dir_all(root.join("nested"))?;
            for idx in 0..8 {
                fs::write(root.join("nested").join(format!("file-{idx}")), b"content")?;
            }
        }

        gix_worktree::remove(
            &work_dir,
            &git_dir,
            gix_utils::progress::Discard,
            Options {
                thread_limit,
                max_retries: 0,
                ..Options::default()
            },
        )?;

        assert!(!work_dir.exists(), "the checkout is removed even with retries disabled");
        assert!(!git_dir.exists(), "the same options apply to the private Git directory");
        assert!(!worktrees_dir.exists(), "the empty worktrees parent is removed");
    }
    Ok(())
}

#[test]
fn removes_both_roots_and_does_not_follow_symlinks() -> gix_testtools::TestResult {
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

    gix_worktree::remove(&work_dir, &git_dir, gix_utils::progress::Discard, Options::default())?;

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
#[cfg(unix)]
fn symlink_roots_with_or_without_a_trailing_separator_are_unlinked() -> gix_testtools::TestResult {
    for trailing_separator in [false, true] {
        let tmp = gix_testtools::tempfile::tempdir()?;
        let work_dir = tmp.path().join("linked");
        let git_dir = tmp.path().join("repo.git/worktrees/linked");
        let outside = tmp.path().join("outside");
        fs::create_dir_all(&git_dir)?;
        fs::create_dir(&outside)?;
        fs::write(outside.join("keep"), b"outside")?;
        gix_fs::symlink::create(&outside, &work_dir)?;
        let root = if trailing_separator {
            work_dir.join("")
        } else {
            work_dir.clone()
        };

        gix_worktree::remove(&root, &git_dir, gix_utils::progress::Discard, Options::default())?;

        assert!(!work_dir.exists(), "the symlink itself was removed");
        assert!(!git_dir.exists(), "the private Git directory was removed");
        assert_eq!(
            fs::read(outside.join("keep"))?,
            b"outside",
            "a root symlink is not followed, even with a trailing separator"
        );
    }
    Ok(())
}

#[test]
fn missing_roots_are_already_removed() -> gix_testtools::TestResult {
    let tmp = gix_testtools::tempfile::tempdir()?;
    gix_worktree::remove(
        tmp.path().join("missing-worktree"),
        tmp.path().join("repo.git/worktrees/missing"),
        gix_utils::progress::Discard,
        Options::default(),
    )?;
    let non_directory = tmp.path().join("file");
    fs::write(&non_directory, b"not a directory")?;
    gix_worktree::remove(
        non_directory.join("missing-worktree"),
        tmp.path().join("repo.git/worktrees/missing"),
        gix_utils::progress::Discard,
        Options::default(),
    )?;
    Ok(())
}

#[test]
fn only_the_conventional_empty_worktrees_parent_is_removed() -> gix_testtools::TestResult {
    let tmp = gix_testtools::tempfile::tempdir()?;
    let parent = tmp.path().join("custom-parent");
    let git_dir = parent.join("linked");
    fs::create_dir_all(&git_dir)?;

    gix_worktree::remove(
        tmp.path().join("missing-worktree"),
        &git_dir,
        gix_utils::progress::Discard,
        Options::default(),
    )?;

    assert!(parent.exists(), "an arbitrary parent directory is retained");
    Ok(())
}

#[test]
#[cfg(unix)]
fn unreadable_directories_match_git() -> gix_testtools::TestResult {
    use std::os::unix::fs::PermissionsExt;

    for empty in [true, false] {
        for use_git in [true, false] {
            let tmp = gix_testtools::tempfile::tempdir()?;
            gix_testtools::git(tmp.path(), "init")?;
            gix_testtools::git(tmp.path(), "commit --allow-empty -m initial")?;
            gix_testtools::git(tmp.path(), "worktree add --detach linked HEAD")?;
            let work_dir = tmp.path().join("linked");
            let git_dir = tmp.path().join(".git/worktrees/linked");
            let unreadable = [work_dir.join("unreadable"), git_dir.join("unreadable")];
            for path in &unreadable {
                fs::create_dir(path)?;
                if !empty {
                    fs::write(path.join("keep"), b"content")?;
                }
                // Directory scans fail, but an empty directory can still be unlinked by its parent.
                fs::set_permissions(path, fs::Permissions::from_mode(0o000))?;
            }
            let scan_error = fs::read_dir(&unreadable[0]).err();

            let result: crate::Result = if use_git {
                gix_testtools::git(tmp.path(), "worktree remove --force linked").map(|_| ())
            } else {
                gix_worktree::remove(
                    &work_dir,
                    &git_dir,
                    gix_utils::progress::Discard,
                    Options {
                        max_retries: 0,
                        ..Options::default()
                    },
                )
                .map_err(Into::into)
            };
            for path in &unreadable {
                if path.exists() {
                    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
                }
            }
            // Root can bypass these permissions, so require a fixture that actually fails to scan.
            let Some(scan_error) = scan_error else {
                return Ok(());
            };
            assert_eq!(
                scan_error.kind(),
                std::io::ErrorKind::PermissionDenied,
                "mode-000 directories cannot be scanned by this user"
            );
            for root in [&work_dir, &git_dir] {
                assert_eq!(
                    root.exists(),
                    !empty,
                    "only empty unreadable directories can be removed"
                );
            }
            if empty {
                result?;
            } else {
                assert!(
                    result.is_err(),
                    "nonempty unreadable directories still report a failure"
                );
            }
        }
    }
    Ok(())
}

#[test]
#[cfg(unix)]
fn administrative_data_is_removed_after_checkout_removal_fails() -> gix_testtools::TestResult {
    use std::os::unix::fs::PermissionsExt;

    let tmp = gix_testtools::tempfile::tempdir()?;
    let work_dir = tmp.path().join("linked");
    let git_dir = tmp.path().join("repo.git/worktrees/linked");
    fs::create_dir(&work_dir)?;
    fs::write(work_dir.join("protected"), b"content")?;
    fs::set_permissions(&work_dir, fs::Permissions::from_mode(0o500))?;
    // Root can bypass these permissions, so first check that the fixture can fail.
    match fs::remove_file(work_dir.join("protected")) {
        Ok(()) => {
            fs::set_permissions(&work_dir, fs::Permissions::from_mode(0o700))?;
            return Ok(());
        }
        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {}
        Err(err) => {
            fs::set_permissions(&work_dir, fs::Permissions::from_mode(0o700))?;
            return Err(err.into());
        }
    }
    fs::create_dir_all(&git_dir)?;
    fs::write(git_dir.join("HEAD"), b"ref: refs/heads/topic\n")?;

    let result = gix_worktree::remove(&work_dir, &git_dir, gix_utils::progress::Discard, Options::default());
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
fn readonly_files_do_not_prevent_removal() -> gix_testtools::TestResult {
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

    gix_worktree::remove(&work_dir, &git_dir, gix_utils::progress::Discard, Options::default())?;

    assert!(!work_dir.exists(), "the checkout was removed");
    assert!(!git_dir.exists(), "the private Git directory was removed");
    Ok(())
}
