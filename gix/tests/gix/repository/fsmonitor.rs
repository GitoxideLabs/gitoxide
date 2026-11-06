use std::path::Path;

use gix::bstr::ByteSlice;
use gix_testtools::TestResult;

fn open(path: &Path) -> gix::Result<gix::Repository> {
    gix::open_opts(
        path,
        gix::open::Options::isolated().config_overrides(["core.fsmonitor=true"]),
    )
}

fn query(server: &gix::fsmonitor::server::Handle) -> TestResult<Vec<u8>> {
    Ok(gix::fsmonitor::client::daemon::query(
        server.endpoint(),
        b"".as_bstr(),
        || panic!("an application-owned server must already be listening"),
        &Default::default(),
    )?)
}

#[test]
fn linked_worktrees_have_independent_servers_without_persisting_configuration() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    // Keep parent sentinels private to this fixture and finish layout changes before watching.
    let parent = gix_testtools::tempfile::tempdir()?;
    let main = parent.path().join("main");
    let linked = parent.path().join("linked");
    std::fs::create_dir(&main)?;
    gix_testtools::git(&main, "init --initial-branch=main")?;
    std::fs::write(main.join("tracked"), "initial\n")?;
    gix_testtools::git(&main, "add tracked")?;
    gix_testtools::git(
        &main,
        "-c user.name=Test -c user.email=test@example.com commit -m baseline",
    )?;
    let output = gix_testtools::git_command(&main)
        .args(["worktree", "add", "--detach"])
        .arg(&linked)
        .output()?;
    assert!(
        output.status.success(),
        "linked-worktree setup failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    gix_testtools::git(&main, "init --bare ../bare")?;
    assert!(
        open(&parent.path().join("bare"))?.fsmonitor_server().is_err(),
        "a bare repository has no worktree to monitor"
    );
    let config_path = main.join(".git/config");
    let original_config = std::fs::read(&config_path)?;
    let main_repo = open(&main)?;
    let linked_repo = open(&linked)?;
    let main_server = main_repo.fsmonitor_server()?;
    let linked_server = linked_repo.fsmonitor_server()?;
    assert_eq!(
        main_server.endpoint(),
        main_repo.git_dir().canonicalize()?.join("fsmonitor--daemon.ipc"),
        "the main worktree uses its private administrative directory"
    );
    assert_eq!(
        linked_server.endpoint(),
        linked_repo.git_dir().canonicalize()?.join("fsmonitor--daemon.ipc"),
        "a linked worktree uses its own administrative directory"
    );
    assert_ne!(main_server.endpoint(), linked_server.endpoint());
    assert!(query(&main_server)?.starts_with(b"gix:"));
    assert!(query(&linked_server)?.starts_with(b"gix:"));

    #[cfg(feature = "status")]
    for (repo, root) in [(&main_repo, &main), (&linked_repo, &linked)] {
        let mut status = repo.status(gix::progress::Discard)?.into_index_worktree_iter(None)?;
        let changes = status.by_ref().collect::<gix::Result<Vec<_>>>()?;
        assert!(changes.is_empty(), "each worktree starts clean");
        status
            .into_outcome()
            .expect("consumed status has an outcome")
            .write_changes()
            .transpose()?;
        let index = repo.open_index()?;
        assert!(
            matches!(
                index.fs_monitor().map(gix::index::extension::FsMonitor::token),
                Some(gix::index::extension::fs_monitor::Token::V2 { token }) if token.starts_with(b"gix:")
            ),
            "ordinary status discovers the embedded server and writes its checkpoint"
        );
        std::fs::write(root.join("tracked"), "changed after checkpoint\n")?;
        let changes = repo
            .status(gix::progress::Discard)?
            .into_index_worktree_iter(None)?
            .map(|item| item.map(|item| item.rela_path().to_owned()))
            .collect::<gix::Result<Vec<_>>>()?;
        assert_eq!(changes, ["tracked"], "server-backed status includes subsequent writes");
    }

    main_server.shutdown()?;
    assert!(
        query(&linked_server)?.starts_with(b"gix:"),
        "stopping the main worktree's server leaves its linked worktree serving"
    );
    linked_server.shutdown()?;
    assert_eq!(
        std::fs::read(config_path)?,
        original_config,
        "server startup and status configuration overrides never rewrite repository configuration"
    );
    Ok(())
}
