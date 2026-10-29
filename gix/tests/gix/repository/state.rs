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

#[test]
fn clear_operation_state_matches_git_quit() -> gix_testtools::TestResult {
    for (script, command) in [
        ("make_am_repo.sh", "am --quit"),
        ("make_merge_repo.sh", "merge --quit"),
        ("make_cherry_pick_repo.sh", "cherry-pick --quit"),
        ("make_cherry_pick_sequence_repo.sh", "cherry-pick --quit"),
        ("make_revert_repo.sh", "revert --quit"),
        ("make_revert_sequence_repo.sh", "revert --quit"),
        ("make_rebase_i_repo.sh", "rebase --quit"),
    ] {
        for use_git in [true, false] {
            let fixture = gix_testtools::scripted_fixture_writable(script)?;
            let repo = gix::open_opts(fixture.path(), crate::restricted())?;
            let head = std::fs::read(repo.git_dir().join("HEAD"))?;
            let index = std::fs::read(repo.index_path())?;
            let files = std::fs::read_dir(fixture.path())?
                .filter_map(|entry| match entry {
                    Ok(entry) if entry.path().is_file() => Some(Ok(entry.path())),
                    Ok(_) => None,
                    Err(err) => Some(Err(err)),
                })
                .map(|path| {
                    let path = path?;
                    std::fs::read(&path).map(|content| (path, content))
                })
                .collect::<std::io::Result<Vec<_>>>()?;
            if use_git {
                gix_testtools::git(fixture.path(), command)?;
            } else {
                repo.clear_operation_state()?;
                repo.clear_operation_state()?;
            }
            assert_eq!(repo.state(), None, "{script}: the operation is no longer active");
            for name in [
                "rebase-apply",
                "rebase-merge",
                "sequencer",
                "CHERRY_PICK_HEAD",
                "REVERT_HEAD",
            ] {
                assert!(!repo.git_dir().join(name).exists(), "{script}: {name} is cleared");
            }
            assert_eq!(
                std::fs::read(repo.git_dir().join("HEAD"))?,
                head,
                "HEAD stays unchanged"
            );
            assert_eq!(std::fs::read(repo.index_path())?, index, "the index stays unchanged");
            for (path, content) in files {
                assert_eq!(std::fs::read(&path)?, content, "{} stays unchanged", path.display());
            }
        }
    }
    Ok(())
}

#[test]
fn clear_operation_state_preserves_rebase_autostashes_like_git() -> gix_testtools::TestResult {
    for backend in ["apply", "merge"] {
        for use_git in [true, false] {
            let fixture = gix_testtools::scripted_fixture_writable_with_args(
                "make_rebase_autostash_repo.sh",
                [backend],
                gix_testtools::Creation::Execute,
            )?;
            let repo = gix::open_opts(fixture.path(), crate::restricted())?;
            let state_dir = repo.git_dir().join(format!("rebase-{backend}"));
            let autostash = std::fs::read_to_string(state_dir.join("autostash"))?;
            let older_stash_commit_id = repo.find_reference("refs/stash")?.id();
            let index = std::fs::read(repo.index_path())?;
            let head = std::fs::read(repo.git_dir().join("HEAD"))?;
            let local = std::fs::read(fixture.path().join("local"))?;
            if use_git {
                gix_testtools::git(fixture.path(), "rebase --quit")?;
            } else {
                repo.clear_operation_state()?;
            }
            assert!(!state_dir.exists(), "the rebase directory is removed");
            assert_eq!(
                gix_testtools::git(fixture.path(), "stash list --format=%H:%gs")?,
                format!(
                    "{}:autostash
{older_stash_commit_id}:On main: older
",
                    autostash.trim()
                ),
                "both stash entries stay recoverable"
            );
            assert_eq!(
                std::fs::read(repo.git_dir().join("HEAD"))?,
                head,
                "HEAD stays unchanged"
            );
            assert_eq!(std::fs::read(repo.index_path())?, index, "the index stays unchanged");
            assert_eq!(
                std::fs::read(fixture.path().join("local"))?,
                local,
                "the stash is not applied"
            );
        }
    }
    Ok(())
}

#[test]
fn failed_rebase_autostash_preservation_keeps_operation_state() -> gix_testtools::TestResult {
    for backend in ["apply", "merge"] {
        let fixture = gix_testtools::scripted_fixture_writable_with_args(
            "make_rebase_autostash_repo.sh",
            [backend],
            gix_testtools::Creation::Execute,
        )?;
        let repo = gix::open_opts(
            fixture.path(),
            crate::restricted().config_overrides(["core.filesRefLockTimeout=0"]),
        )?;
        let state_dir = repo.git_dir().join(format!("rebase-{backend}"));
        let autostash_path = state_dir.join("autostash");
        let original_autostash = std::fs::read(&autostash_path)?;
        let stash_lock = repo.common_dir().join("refs/stash.lock");
        std::fs::write(&stash_lock, b"another writer")?;
        repo.clear_operation_state().expect_err("the stash is locked");
        assert_eq!(
            std::fs::read(&autostash_path)?,
            original_autostash,
            "autostash reachability is retained"
        );
        assert!(repo.state().is_some(), "the rebase remains recoverable");
        std::fs::remove_file(stash_lock)?;
        std::fs::write(&autostash_path, b"invalid object ID")?;
        repo.clear_operation_state().expect_err("the autostash ID is invalid");
        assert!(state_dir.exists(), "invalid autostashes are not discarded");
    }
    Ok(())
}

#[test]
fn clear_operation_state_removes_only_registered_rebase_labels() -> gix_testtools::TestResult {
    for use_git in [true, false] {
        let fixture = gix_testtools::scripted_fixture_writable("make_rebase_i_repo.sh")?;
        let repo = gix::open_opts(fixture.path(), crate::restricted())?;
        for name in ["refs/rewritten/to-delete", "refs/rewritten/to-retain"] {
            repo.reference(
                name,
                repo.head_id()?,
                gix::refs::transaction::PreviousValue::MustNotExist,
                "test label",
            )?;
        }
        std::fs::write(
            repo.git_dir().join("rebase-merge/refs-to-delete"),
            "refs/rewritten/to-delete
refs/rewritten/to-delete
",
        )?;
        if use_git {
            gix_testtools::git(fixture.path(), "rebase --quit")?;
        } else {
            repo.clear_operation_state()?;
        }
        assert!(
            repo.try_find_reference("refs/rewritten/to-delete")?.is_none(),
            "registered labels are removed"
        );
        assert!(
            repo.try_find_reference("refs/rewritten/to-retain")?.is_some(),
            "unregistered labels stay intact"
        );
    }
    let fixture = gix_testtools::scripted_fixture_writable("make_rebase_i_repo.sh")?;
    let repo = gix::open_opts(fixture.path(), crate::restricted())?;
    std::fs::write(repo.git_dir().join("rebase-merge/refs-to-delete"), "refs/heads/main\n")?;
    repo.clear_operation_state()
        .expect_err("rebase cleanup cannot delete ordinary branches");
    assert!(repo.find_reference("refs/heads/main").is_ok(), "the branch is retained");
    assert!(
        repo.git_dir().join("rebase-merge").exists(),
        "the invalid state is retained for recovery"
    );
    Ok(())
}

#[test]
fn clear_operation_state_cleans_bisect_without_moving_head_like_git() -> gix_testtools::TestResult {
    for use_git in [true, false] {
        let fixture = gix_testtools::scripted_fixture_writable("make_bisect_repo.sh")?;
        gix_testtools::git(fixture.path(), "bisect reset")?;
        gix_testtools::git(fixture.path(), "bisect start --no-checkout HEAD HEAD~1")?;
        let repo = gix::open_opts(fixture.path(), crate::restricted())?;
        let head = std::fs::read(repo.git_dir().join("HEAD"))?;
        let index = std::fs::read(repo.index_path())?;
        std::fs::write(
            repo.common_dir().join("packed-refs"),
            format!(
                "# pack-refs with: peeled
{} refs/bisect/packed
",
                repo.head_id()?
            ),
        )?;
        for name in ["BISECT_ANCESTORS_OK", "BISECT_RUN", "BISECT_FIRST_PARENT"] {
            std::fs::write(repo.git_dir().join(name), b"marker")?;
        }
        if use_git {
            gix_testtools::git(fixture.path(), "bisect reset")?;
        } else {
            repo.clear_operation_state()?;
        }
        for name in [
            "BISECT_HEAD",
            "BISECT_EXPECTED_REV",
            "BISECT_ANCESTORS_OK",
            "BISECT_LOG",
            "BISECT_NAMES",
            "BISECT_RUN",
            "BISECT_TERMS",
            "BISECT_FIRST_PARENT",
            "BISECT_START",
        ] {
            assert!(!repo.git_dir().join(name).exists(), "{name} is cleared");
        }
        assert_eq!(
            repo.references()?.prefixed("refs/bisect/")?.count(),
            0,
            "loose and packed bisect refs are cleared"
        );
        assert_eq!(
            std::fs::read(repo.git_dir().join("HEAD"))?,
            head,
            "HEAD stays unchanged"
        );
        assert_eq!(std::fs::read(repo.index_path())?, index, "the index stays unchanged");
    }
    Ok(())
}

#[test]
fn clear_operation_state_is_private_to_the_linked_worktree() -> gix_testtools::TestResult {
    let fixture = gix_testtools::scripted_fixture_writable("make_rebase_i_repo.sh")?;
    let main = gix::open_opts(fixture.path(), crate::restricted())?;
    let main_state = main.state();
    let main_label = main.find_reference("refs/rewritten/onto")?.id();
    gix_testtools::git(fixture.path(), "worktree add --detach linked HEAD")?;
    let linked_dir = fixture.path().join("linked");
    gix_testtools::git(&linked_dir, "bisect start --no-checkout HEAD HEAD~1")?;
    let mut linked = gix::open_opts(&linked_dir, crate::restricted())?;
    linked.set_namespace("unrelated")?;
    linked.clear_operation_state()?;
    linked.clear_namespace();
    assert_eq!(linked.state(), None, "the linked bisect has ended");
    assert_eq!(
        linked.references()?.prefixed("refs/bisect/")?.count(),
        0,
        "linked bisect refs are removed"
    );
    assert_eq!(main.state(), main_state, "the main rebase stays active");
    assert_eq!(
        main.find_reference("refs/rewritten/onto")?.id(),
        main_label,
        "the main rebase label is unchanged"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn clear_operation_state_rejects_symlinked_operation_directories() -> gix_testtools::TestResult {
    let fixture = gix_testtools::scripted_fixture_writable("make_merge_autostash_repo.sh")?;
    let repo = gix::open_opts(fixture.path(), crate::restricted())?;
    let outside_state = fixture.path().join("outside-state");
    std::fs::create_dir(&outside_state)?;
    let stash_commit_id = repo.find_reference("MERGE_AUTOSTASH")?.id();
    let original_stash = repo.find_reference("refs/stash")?.id();
    std::fs::write(outside_state.join("autostash"), format!("{stash_commit_id}\n"))?;
    std::os::unix::fs::symlink(&outside_state, repo.git_dir().join("rebase-merge"))?;
    repo.clear_operation_state()
        .expect_err("operation directories cannot redirect cleanup");
    assert!(
        outside_state.join("autostash").exists(),
        "the external state file is untouched"
    );
    assert_eq!(
        repo.find_reference("refs/stash")?.id(),
        original_stash,
        "no autostash is imported from outside operation state"
    );
    assert!(
        repo.git_dir().join("MERGE_HEAD").exists(),
        "the original merge remains recoverable"
    );
    Ok(())
}
