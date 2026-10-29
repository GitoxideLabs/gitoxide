use crate::named_repo;

#[test]
fn apply_mailbox() -> gix_testtools::TestResult {
    let repo = named_repo("make_am_repo.sh")?;

    assert_eq!(repo.head_name()?.unwrap().shorten(), "main");
    assert_eq!(repo.state(), Some(gix::state::InProgress::ApplyMailbox));
    Ok(())
}

#[test]
fn bisect() -> gix_testtools::TestResult {
    let repo = named_repo("make_bisect_repo.sh")?;

    assert_eq!(repo.head_name()?.unwrap().shorten(), "main");
    assert_eq!(repo.state(), Some(gix::state::InProgress::Bisect));

    Ok(())
}

#[test]
fn cherry_pick() -> gix_testtools::TestResult {
    let repo = named_repo("make_cherry_pick_repo.sh")?;

    assert_eq!(repo.head_name()?.unwrap().shorten(), "main");
    assert_eq!(repo.state(), Some(gix::state::InProgress::CherryPick));
    Ok(())
}

#[test]
fn cherry_pick_sequence() -> gix_testtools::TestResult {
    let repo = named_repo("make_cherry_pick_sequence_repo.sh")?;

    assert_eq!(repo.head_name()?.unwrap().shorten(), "main");
    assert_eq!(repo.state(), Some(gix::state::InProgress::CherryPickSequence));

    Ok(())
}

#[test]
fn merge() -> gix_testtools::TestResult {
    let repo = named_repo("make_merge_repo.sh")?;

    assert_eq!(repo.head_name()?.unwrap().shorten(), "main");
    assert_eq!(repo.state(), Some(gix::state::InProgress::Merge));

    Ok(())
}

#[test]
fn rebase_interactive() -> gix_testtools::TestResult {
    let repo = named_repo("make_rebase_i_repo.sh")?;

    assert!(repo.head()?.is_detached());
    assert_eq!(repo.state(), Some(gix::state::InProgress::RebaseInteractive));

    Ok(())
}

#[test]
fn revert() -> gix_testtools::TestResult {
    let repo = named_repo("make_revert_repo.sh")?;

    assert_eq!(repo.head_name()?.unwrap().shorten(), "main");
    assert_eq!(repo.state(), Some(gix::state::InProgress::Revert));

    Ok(())
}

#[test]
fn revert_sequence() -> gix_testtools::TestResult {
    let repo = named_repo("make_revert_sequence_repo.sh")?;

    assert_eq!(repo.head_name()?.unwrap().shorten(), "main");
    assert_eq!(repo.state(), Some(gix::state::InProgress::RevertSequence));

    Ok(())
}

#[test]
fn clear_merge_state_matches_git_quit_and_preserves_checkout_state() -> gix_testtools::TestResult {
    for use_git in [true, false] {
        let fixture = gix_testtools::scripted_fixture_writable("make_merge_repo.sh")?;
        let repo = gix::open_opts(fixture.path(), crate::restricted())?;
        for marker in ["MERGE_RR", "MERGE_CUSTOM"] {
            std::fs::write(repo.git_dir().join(marker), b"marker")?;
        }
        // MERGE_LABELS is cleared by current Git, but is not known to older installed versions.
        if !use_git {
            std::fs::write(repo.git_dir().join("MERGE_LABELS"), b"marker")?;
        }
        let head = std::fs::read(repo.git_dir().join("HEAD"))?;
        let index = std::fs::read(repo.index_path())?;
        let conflict = std::fs::read(fixture.path().join("file"))?;
        let original_head = std::fs::read(repo.git_dir().join("ORIG_HEAD"))?;
        if use_git {
            gix_testtools::git(fixture.path(), "merge --quit")?;
        } else {
            repo.clear_merge_state()?;
            repo.clear_merge_state()?;
        }
        assert_eq!(repo.state(), None, "quitting removes the merge state");
        for marker in [
            "MERGE_HEAD",
            "MERGE_MODE",
            "MERGE_MSG",
            "MERGE_RR",
            "MERGE_LABELS",
            "AUTO_MERGE",
        ] {
            assert!(
                !repo.git_dir().join(marker).exists(),
                "use_git={use_git}: {marker} is cleared"
            );
        }
        assert_eq!(std::fs::read(repo.git_dir().join("HEAD"))?, head, "HEAD is not moved");
        assert_eq!(
            std::fs::read(repo.index_path())?,
            index,
            "the conflicted index is untouched"
        );
        assert_eq!(
            std::fs::read(fixture.path().join("file"))?,
            conflict,
            "conflict markers remain in the checkout"
        );
        assert_eq!(
            std::fs::read(repo.git_dir().join("ORIG_HEAD"))?,
            original_head,
            "the original HEAD remains available"
        );
        assert_eq!(
            std::fs::read(repo.git_dir().join("MERGE_CUSTOM"))?,
            b"marker",
            "unrecognized files are retained"
        );
    }
    Ok(())
}

#[test]
fn clear_merge_state_preserves_autostashes_in_the_stash_reflog_like_git() -> gix_testtools::TestResult {
    for use_git in [true, false] {
        let fixture = gix_testtools::scripted_fixture_writable("make_merge_autostash_repo.sh")?;
        let mut repo = gix::open_opts(fixture.path(), crate::restricted())?;
        let stash_id = repo.find_reference("MERGE_AUTOSTASH")?.id().detach();
        let older_stash_id = repo.find_reference("refs/stash")?.id().detach();
        let index = std::fs::read(repo.index_path())?;
        let local = std::fs::read(fixture.path().join("local"))?;
        gix_testtools::git(fixture.path(), "config core.logAllRefUpdates false")?;
        repo.reload()?;
        if use_git {
            gix_testtools::git(fixture.path(), "merge --quit")?;
        } else {
            repo.clear_merge_state()?;
        }
        assert_eq!(
            repo.find_reference("refs/stash")?.id(),
            stash_id,
            "the autostash is now the newest stash"
        );
        assert!(
            repo.try_find_reference("MERGE_AUTOSTASH")?.is_none(),
            "the saved autostash marker is removed"
        );
        assert_eq!(
            gix_testtools::git(fixture.path(), "stash list --format=%H:%gs")?,
            format!(
                "{stash_id}:autostash
{older_stash_id}:On main: older
"
            ),
            "quitting retains both stash entries even with reflogs disabled"
        );
        assert_eq!(
            std::fs::read(repo.index_path())?,
            index,
            "saving the stash does not change the index"
        );
        assert_eq!(
            std::fs::read(fixture.path().join("local"))?,
            local,
            "the autostash is saved without applying it"
        );
    }
    Ok(())
}

#[test]
fn failed_autostash_preservation_keeps_the_merge_recoverable() -> gix_testtools::TestResult {
    let fixture = gix_testtools::scripted_fixture_writable("make_merge_autostash_repo.sh")?;
    let repo = gix::open_opts(
        fixture.path(),
        crate::restricted().config_overrides(["core.filesRefLockTimeout=0"]),
    )?;
    let autostash = std::fs::read(repo.git_dir().join("MERGE_AUTOSTASH"))?;
    let merge_head = std::fs::read(repo.git_dir().join("MERGE_HEAD"))?;
    let previous_stash = repo.find_reference("refs/stash")?.id();
    let lock_path = repo.common_dir().join("refs/stash.lock");
    std::fs::write(&lock_path, b"another writer")?;
    repo.clear_merge_state()
        .expect_err("an existing stash lock blocks preservation");
    assert_eq!(
        std::fs::read(repo.git_dir().join("MERGE_AUTOSTASH"))?,
        autostash,
        "failed preservation retains autostash reachability"
    );
    assert_eq!(
        std::fs::read(repo.git_dir().join("MERGE_HEAD"))?,
        merge_head,
        "the merge remains active after failure"
    );
    assert_eq!(
        repo.find_reference("refs/stash")?.id(),
        previous_stash,
        "the existing stash is unchanged"
    );
    std::fs::remove_file(lock_path)?;
    std::fs::write(repo.git_dir().join("MERGE_AUTOSTASH"), format!("{}\n", repo.head_id()?))?;
    repo.clear_merge_state()
        .expect_err("a regular commit is not a valid stash");
    assert!(
        repo.git_dir().join("MERGE_AUTOSTASH").exists(),
        "invalid autostash markers are never silently discarded"
    );
    Ok(())
}

#[test]
fn clear_merge_state_is_private_to_the_linked_worktree_like_git() -> gix_testtools::TestResult {
    for use_git in [true, false] {
        let fixture = gix_testtools::scripted_fixture_writable("make_merge_repo.sh")?;
        let main = gix::open_opts(fixture.path(), crate::restricted())?;
        let main_markers = ["MERGE_HEAD", "MERGE_MSG", "MERGE_MODE", "AUTO_MERGE"]
            .into_iter()
            .map(|name| std::fs::read(main.git_dir().join(name)).map(|content| (name, content)))
            .collect::<Result<Vec<_>, _>>()?;
        gix_testtools::git(fixture.path(), "worktree add --detach linked HEAD")?;
        let linked_dir = fixture.path().join("linked");
        gix_testtools::git(&linked_dir, "merge other-branch")
            .expect_err("the same merge conflicts in the linked worktree");
        let linked = gix::open_opts(&linked_dir, crate::restricted())?;
        let index = std::fs::read(linked.index_path())?;
        if use_git {
            gix_testtools::git(&linked_dir, "merge --quit")?;
        } else {
            linked.clear_merge_state()?;
        }
        assert_eq!(linked.state(), None, "the linked worktree's merge has ended");
        assert_eq!(
            main.state(),
            Some(gix::state::InProgress::Merge),
            "the main worktree's merge stays active"
        );
        for (name, content) in main_markers {
            assert_eq!(
                std::fs::read(main.git_dir().join(name))?,
                content,
                "the main worktree's {name} remains unchanged"
            );
            assert!(
                !linked.git_dir().join(name).exists(),
                "the linked worktree's {name} is cleared"
            );
        }
        assert_eq!(
            std::fs::read(linked.index_path())?,
            index,
            "the linked worktree's index stays unchanged"
        );
    }
    Ok(())
}

#[test]
fn clear_merge_state_ignores_reference_namespaces_like_git() -> gix_testtools::TestResult {
    for use_git in [true, false] {
        let fixture = gix_testtools::scripted_fixture_writable("make_merge_autostash_repo.sh")?;
        let repo = gix::open_opts(fixture.path(), crate::restricted())?;
        let autostash_commit_id = repo.find_reference("MERGE_AUTOSTASH")?.id();
        let older_stash_commit_id = repo.find_reference("refs/stash")?.id();
        let mut namespaced = repo.clone();
        namespaced.set_namespace("unrelated/nested")?;
        for name in ["MERGE_AUTOSTASH", "AUTO_MERGE", "refs/stash"] {
            namespaced.reference(
                name,
                older_stash_commit_id,
                gix::refs::transaction::PreviousValue::MustNotExist,
                "create namespaced state",
            )?;
        }
        if use_git {
            gix_testtools::git(fixture.path(), "--namespace=unrelated/nested merge --quit")?;
        } else {
            namespaced.clear_merge_state()?;
        }
        assert_eq!(
            repo.find_reference("refs/stash")?.id(),
            autostash_commit_id,
            "the local autostash is saved in the default namespace"
        );
        for name in ["MERGE_AUTOSTASH", "AUTO_MERGE"] {
            assert!(repo.try_find_reference(name)?.is_none(), "local {name} is removed");
        }
        for name in ["MERGE_AUTOSTASH", "AUTO_MERGE", "refs/stash"] {
            assert_eq!(
                namespaced.find_reference(name)?.id(),
                older_stash_commit_id,
                "the unrelated namespaced {name} remains unchanged"
            );
        }
        assert!(namespaced.namespace().is_some(), "the caller's namespace is retained");
    }
    Ok(())
}
