use gix_testtools::TestResult;

fn paths(repo: &gix::Repository) -> gix::Result<Vec<String>> {
    let mut paths = repo
        .status(gix::progress::Discard)?
        .into_index_worktree_iter(None)?
        .map(|item| item.map(|item| item.rela_path().to_string()))
        .collect::<gix::Result<Vec<_>>>()?;
    paths.sort();
    Ok(paths)
}

#[test]
fn hook_skips_unreported_files_but_keeps_dirty_entries_and_untracked_files() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let root = gix_testtools::scripted_fixture_read_only("make_status_fsmonitor.sh")?;
    let repo = gix::open_opts(&root, gix::open::Options::isolated())?;
    let mut expected: Vec<_> = std::fs::read_to_string(root.join(".git/expected-status"))?
        .lines()
        .map(|line| line[3..].to_owned())
        .collect();
    expected.sort();
    assert_eq!(expected, ["dir/a", "dirty", "reported", "untracked"]);
    assert_eq!(
        paths(&repo)?,
        expected,
        "a successful query permits Git's scan omissions"
    );
    assert_eq!(paths(&repo)?, expected, "each call refreshes its own index snapshot");
    Ok(())
}

#[test]
fn disabled_failed_and_malformed_hooks_fall_back_to_full_status() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let root = gix_testtools::scripted_fixture_read_only("make_status_fsmonitor.sh")?;
    for value in [
        "false",
        ".git/fsmonitor-fail",
        ".git/fsmonitor-malformed",
        "missing-fsmonitor-hook",
    ] {
        let repo = gix::open_opts(
            &root,
            gix::open::Options::isolated().config_overrides([
                format!("core.fsmonitor={value}"),
                "gitoxide.core.externalCommandStderr=false".into(),
            ]),
        )?;
        assert_eq!(
            paths(&repo)?,
            ["dir/a", "directory", "dirty", "reported", "unreported", "untracked"],
            "{value}"
        );
    }
    for (version, events) in [
        (5, b"\0".as_slice()),
        (6, b"\0reported\0".as_slice()),
        (7, b"reported\0\0dir\0".as_slice()),
        (8, b"reported\0\0".as_slice()),
    ] {
        let (root, ()) =
            gix_testtools::scripted_fixture_read_only_with_post("make_status_fsmonitor.sh", version, |fixture| {
                if let gix_testtools::FixtureState::Uninitialized(path) = fixture {
                    std::fs::write(path.join(".git/fsmonitor-paths"), events)?;
                }
                Ok(())
            })?;
        for protocol in [1, 2] {
            let repo = gix::open_opts(
                &root,
                gix::open::Options::isolated().config_overrides([format!("core.fsmonitorHookVersion={protocol}")]),
            )?;
            assert_eq!(
                paths(&repo)?,
                ["dir/a", "directory", "dirty", "reported", "unreported", "untracked"],
                "empty path records invalidate the full checkpoint with protocol {protocol}"
            );
        }
    }
    Ok(())
}

#[test]
fn hook_protocol_selection_and_shell_commands() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let root = gix_testtools::scripted_fixture_read_only("make_status_fsmonitor.sh")?;
    for command in [
        ".git/fsmonitor-hook",
        ".git/fsmonitor-v1",
        "sh .git/fsmonitor-hook",
        "GIX_FSMONITOR_TEST=1 .git/fsmonitor-hook",
    ] {
        for version in [None, Some(1), Some(2)] {
            let mut overrides = vec![format!("core.fsmonitor={command}")];
            if let Some(version) = version {
                overrides.push(format!("core.fsmonitorHookVersion={version}"));
            }
            let repo = gix::open_opts(&root, gix::open::Options::isolated().config_overrides(overrides))?;
            let expected = if command == ".git/fsmonitor-v1" && version == Some(2) {
                6
            } else {
                4
            };
            assert_eq!(paths(&repo)?.len(), expected, "{command}, version {version:?}");
        }
    }
    let repo = gix::open_opts(
        &root,
        gix::open::Options::isolated().config_overrides(["core.fsmonitorHookVersion=3"]),
    )?;
    assert!(
        repo.status(gix::progress::Discard)?
            .into_index_worktree_iter(None)
            .and_then(Iterator::collect::<gix::Result<Vec<_>>>)
            .is_err(),
        "invalid protocol versions are configuration errors"
    );
    Ok(())
}

#[test]
fn persisted_tokens_and_partial_status_keep_unvisited_dirty_entries() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_writable("make_status_fsmonitor.sh")?;
    let root = fixture.path();
    let repo = gix::open_opts(root, gix::open::Options::isolated())?;
    std::fs::write(root.join(".git/fsmonitor-paths"), b"reported\0dir\0unreported\0")?;
    let mut iter = repo
        .status(gix::progress::Discard)?
        .into_index_worktree_iter(["reported".into()])?;
    assert_eq!(iter.by_ref().collect::<gix::Result<Vec<_>>>()?.len(), 1);
    let mut outcome = iter.into_outcome().expect("fully consumed iterator has an outcome");
    assert!(outcome.has_changes(), "advancing the monitor token needs writeback");
    outcome.write_changes().expect("monitor state is pending")?;
    assert!(!outcome.has_changes(), "writeback consumes all pending state");
    assert!(outcome.write_changes().is_none(), "writeback happens once");
    let index = gix::index::File::at(root.join(".git/index"), repo.object_hash(), false, Default::default())?;
    let monitor = index.fs_monitor().expect("writeback retains FSMN");
    assert_eq!(
        monitor.token(),
        &gix::index::extension::fs_monitor::Token::V2 {
            token: "next-token".into()
        }
    );
    std::fs::write(root.join(".git/fsmonitor-paths"), b"")?;
    assert_eq!(
        paths(&repo)?,
        ["dir/a", "dirty", "reported", "unreported", "untracked"],
        "unvisited reported changes survive token advancement"
    );
    let git = gix_testtools::git_command(root)
        .args(["status", "--porcelain"])
        .output()?;
    assert!(git.status.success(), "Git accepts the written FSMN extension");
    let mut expected: Vec<_> = std::str::from_utf8(&git.stdout)?
        .lines()
        .map(|line| line[3..].to_owned())
        .collect();
    expected.sort();
    assert_eq!(
        paths(&repo)?,
        expected,
        "Git and gix read the refreshed bitmap identically"
    );
    Ok(())
}

#[test]
fn directory_events_ignore_case_and_full_invalidation() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    for (version, events, ignore_case, expected) in [
        (1, b"DIR/\0".as_slice(), true, vec!["dir/a", "dirty", "untracked"]),
        (2, b"dir/\0".as_slice(), false, vec!["dir/a", "dirty", "untracked"]),
        (4, b"reported".as_slice(), false, vec!["dirty", "reported", "untracked"]),
        (
            3,
            b"/\0".as_slice(),
            false,
            vec!["dir/a", "directory", "dirty", "reported", "unreported", "untracked"],
        ),
    ] {
        let (root, ()) =
            gix_testtools::scripted_fixture_read_only_with_post("make_status_fsmonitor.sh", version, |fixture| {
                if let gix_testtools::FixtureState::Uninitialized(path) = fixture {
                    std::fs::write(path.join(".git/fsmonitor-paths"), events)?;
                }
                Ok(())
            })?;
        let repo = gix::open_opts(
            &root,
            gix::open::Options::isolated().config_overrides([format!("core.ignoreCase={ignore_case}")]),
        )?;
        let git = gix_testtools::git_command(&root)
            .args(["-c", &format!("core.ignoreCase={ignore_case}"), "status", "--porcelain"])
            .env("GIT_OPTIONAL_LOCKS", "0")
            .output()?;
        assert!(
            git.status.success(),
            "Git supplies the framing and directory-event reference"
        );
        let mut git_paths: Vec<_> = std::str::from_utf8(&git.stdout)?
            .lines()
            .map(|line| &line[3..])
            .collect();
        git_paths.sort();
        assert_eq!(
            git_paths, expected,
            "Git also accepts the final path without a NUL terminator"
        );
        assert_eq!(
            paths(&repo)?,
            expected,
            "directory boundaries and full invalidation are preserved"
        );
    }
    Ok(())
}

#[test]
fn untrusted_configuration_cannot_enable_a_monitor() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let root = gix_testtools::scripted_fixture_read_only("make_status_fsmonitor.sh")?;
    let repo = gix::open_opts(&root, gix::open::Options::isolated().filter_config_section(|_| false))?;
    assert_eq!(
        paths(&repo)?.len(),
        6,
        "a filtered hook must not authorize scan omissions"
    );
    Ok(())
}

#[test]
fn first_query_and_token_only_writeback_prime_clean_entries() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_writable("make_status_fsmonitor.sh")?;
    let root = fixture.path();
    let repo = gix::open_opts(root, gix::open::Options::isolated())?;
    let mut index = (**repo.index()?).clone();
    index.set_fs_monitor_token(None);
    index.write(Default::default())?;
    let mut iter = repo
        .status(gix::progress::Discard)?
        .into_index_worktree_iter(["dir/b".into()])?;
    assert!(iter.by_ref().collect::<gix::Result<Vec<_>>>()?.is_empty());
    let mut outcome = iter.into_outcome().expect("fully consumed status has an outcome");
    assert!(
        outcome.has_changes(),
        "a clean status still persists its first monitor checkpoint"
    );
    outcome.write_changes().expect("the monitor checkpoint is new")?;
    let index = repo.index()?;
    let validity = index
        .fs_monitor()
        .expect("checkpoint was written")
        .valid_entries(index.entries().len())
        .expect("valid bitmap");
    let clean = index.entry_index_by_path("dir/b".into()).expect("fixture entry exists");
    assert!(validity[clean], "the checked clean file becomes valid");
    assert_eq!(
        validity.iter().filter(|valid| **valid).count(),
        1,
        "unvisited files stay dirty after the initial scan"
    );
    Ok(())
}

#[test]
fn repeated_queries_do_not_request_redundant_index_writes() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    for hook in [".git/fsmonitor-hook", ".git/fsmonitor-fail"] {
        let fixture = gix_testtools::scripted_fixture_writable("make_status_fsmonitor.sh")?;
        let root = fixture.path();
        // Keep the only clean file non-racy so stat refreshes cannot request another write.
        std::fs::File::options()
            .write(true)
            .open(root.join("dir/b"))?
            .set_modified(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1))?;
        let repo = gix::open_opts(
            root,
            gix::open::Options::isolated().config_overrides([format!("core.fsmonitor={hook}")]),
        )?;
        let mut first = repo.status(gix::progress::Discard)?.into_index_worktree_iter(None)?;
        first.by_ref().collect::<gix::Result<Vec<_>>>()?;
        let mut outcome = first.into_outcome().expect("fully consumed status has an outcome");
        assert!(outcome.has_changes(), "the first {hook} query changes monitor state");
        outcome.write_changes().expect("the monitor state changed")?;
        assert_eq!(
            repo.index()?.fs_monitor().is_some(),
            hook == ".git/fsmonitor-hook",
            "only a successful hook retains the monitor checkpoint"
        );

        let mut repeated = repo.status(gix::progress::Discard)?.into_index_worktree_iter(None)?;
        repeated.by_ref().collect::<gix::Result<Vec<_>>>()?;
        let outcome = repeated.into_outcome().expect("fully consumed status has an outcome");
        assert!(
            !outcome.has_changes(),
            "repeating {hook} with identical monitor state must not rewrite the index"
        );
    }
    Ok(())
}
