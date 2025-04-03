use crate::upwards::repo_path;

#[cfg(target_os = "macos")]
#[test]
fn verify_on_exfat() -> crate::Result<()> {
    use std::process::Command;

    use gix_discover::repository::Kind;

    let fixtures = gix_testtools::scripted_fixture_read_only("make_exfat_repo_darwin.sh")?;
    let mount_point = tempfile::tempdir()?;

    let _cleanup = {
        // Mount dmg file
        Command::new("hdiutil")
            .args(["attach", "-nobrowse", "-mountpoint"])
            .arg(mount_point.path())
            .arg(fixtures.as_path().join("exfat_repo.dmg"))
            .status()?;

        // Ensure that the mount point is always cleaned up
        defer::defer({
            let mount_point = mount_point.path().to_owned();
            move || {
                Command::new("hdiutil")
                    .arg("detach")
                    .arg(&mount_point)
                    .status()
                    .expect("detach temporary test dmg filesystem successfully");
            }
        })
    };

    let is_git = gix_discover::is_git(&mount_point.path().join(".git"));

    assert!(
        matches!(is_git, Ok(Kind::WorkTree { linked_git_dir: None })),
        "repo on exFAT is recognized as a valid worktree repo"
    );
    Ok(())
}

#[test]
fn missing_configuration_file_is_not_a_dealbreaker_in_bare_repo() -> crate::Result {
    for name in ["bare-no-config-after-init.git", "bare-no-config.git"] {
        let repo = repo_path()?.join(name);
        let kind = gix_discover::is_git(&repo)?;
        assert_eq!(kind, gix_discover::repository::Kind::PossiblyBare);
    }
    Ok(())
}

#[test]
fn bare_repo_with_index_file_looks_still_looks_like_bare() -> crate::Result {
    let repo = repo_path()?.join("bare-with-index.git");
    let kind = gix_discover::is_git(&repo)?;
    assert_eq!(kind, gix_discover::repository::Kind::PossiblyBare);
    Ok(())
}

#[test]
fn non_bare_repo_without_workdir() -> crate::Result {
    let repo = repo_path()?.join("non-bare-without-worktree");
    let kind = gix_discover::is_git(&repo)?;
    assert_eq!(
        kind,
        gix_discover::repository::Kind::PossiblyBare,
        "typically due to misconfiguration, but worktrees could also be configured in Git configuration"
    );
    Ok(())
}

#[test]
fn non_bare_repo_without_workdir_with_index() -> crate::Result {
    let repo = repo_path()?.join("non-bare-without-worktree-with-index");
    let kind = gix_discover::is_git(&repo)?;
    assert_eq!(
        kind,
        gix_discover::repository::Kind::PossiblyBare,
        "this means it has to be validated later"
    );
    Ok(())
}

#[test]
fn bare_repo_with_index_file_looks_still_looks_like_bare_if_it_was_renamed() -> crate::Result {
    for repo_name in ["bare-with-index-bare", "bare-with-index-no-config-bare"] {
        let repo = repo_path()?.join(repo_name);
        let kind = gix_discover::is_git(&repo)?;
        assert_eq!(kind, gix_discover::repository::Kind::PossiblyBare);
    }
    Ok(())
}

#[test]
fn no_bare_repo_without_index_file_looks_like_worktree() -> crate::Result {
    let repo = repo_path()?.join("non-bare-without-index").join(".git");
    let kind = gix_discover::is_git(&repo)?;
    assert_eq!(kind, gix_discover::repository::Kind::WorkTree { linked_git_dir: None });
    Ok(())
}

#[test]
fn missing_configuration_file_is_not_a_dealbreaker_in_nonbare_repo() -> crate::Result {
    for name in ["worktree-no-config-after-init/.git", "worktree-no-config/.git"] {
        let repo = repo_path()?.join(name);
        let kind = gix_discover::is_git(&repo)?;
        assert_eq!(kind, gix_discover::repository::Kind::WorkTree { linked_git_dir: None });
    }
    Ok(())
}

#[test]
fn split_worktree_using_configuration() -> crate::Result {
    for name in [
        "repo-with-worktree-in-config",
        "repo-with-worktree-in-config-unborn",
        "repo-with-worktree-in-config-unborn-no-worktreedir",
        "repo-with-worktree-in-config-unborn-empty-worktreedir",
        "repo-with-worktree-in-config-unborn-worktreedir-missing-value",
    ] {
        let repo = repo_path()?.join(name);
        let kind = gix_discover::is_git(&repo)?;
        assert_eq!(
            kind,
            gix_discover::repository::Kind::PossiblyBare,
            "{name}: we think these are bare as we don't read the configuration in this case - \
            a shortcoming to favor performance which still comes out correct in `gix`"
        );
    }
    Ok(())
}

#[test]
fn reftable() -> crate::Result {
    let repo = gix_testtools::scripted_fixture_read_only("make_reftable_repo.sh")?.join("reftable-clone");
    let kind = gix_discover::is_git(&repo)?;
    assert_eq!(kind, gix_discover::repository::Kind::WorkTree { linked_git_dir: None });
    Ok(())
}
