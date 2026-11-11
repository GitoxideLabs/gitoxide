use std::time::Duration;

use gix::fsmonitor::server::{Options, ipc};

#[test]
fn server_context_resolves_repository_paths_and_filesystem_options() -> gix_error::TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")?;
    let repo = gix::open_opts(
        &fixture,
        gix::open::Options::isolated().config_overrides([
            "core.precomposeUnicode=true",
            "core.ignoreCase=false",
            "fsmonitor.socketDir=ignored",
        ]),
    )?;
    let context = repo.fsmonitor_server_context()?;
    assert_eq!(
        context.worktree,
        repo.workdir().expect("the fixture has a worktree").canonicalize()?,
        "the worktree path uses its canonical filesystem spelling"
    );
    assert_eq!(
        context.git_dir,
        repo.git_dir().canonicalize()?,
        "the private Git directory uses its canonical filesystem spelling"
    );
    assert_eq!(
        context.common_dir,
        repo.common_dir().canonicalize()?,
        "the shared Git directory is canonicalized independently"
    );
    assert_eq!(
        context.endpoint,
        gix::fsmonitor::server::endpoint(
            repo.git_dir(),
            repo.workdir().expect("the fixture has a worktree"),
            || { panic!("local Git directories ignore fsmonitor.socketDir") }
        )?,
        "the repository context agrees with independent native endpoint resolution"
    );
    assert!(context.precompose_unicode, "core.precomposeUnicode is forwarded");
    assert!(!context.ignore_case, "core.ignoreCase is forwarded");
    assert!(
        !context.endpoint.exists(),
        "preparing the context does not start a server"
    );
    Ok(())
}

#[test]
fn embedded_server_uses_linked_worktree_layout_and_idle_options() -> gix_error::TestResult {
    let (main, _fixture) = crate::basic_rw_repo()?;
    let root = main.workdir().expect("the fixture has a worktree");
    gix_testtools::git(root, "worktree add --detach linked")?;
    let linked = gix::open_opts(root.join("linked"), gix::open::Options::isolated())?;
    let endpoint = gix::fsmonitor::server::endpoint(
        linked.git_dir(),
        linked.workdir().expect("the linked fixture has a worktree"),
        || panic!("the linked Git directory is local"),
    )?;
    assert_ne!(
        linked.git_dir(),
        linked.common_dir(),
        "the fixture distinguishes private and shared administrative paths"
    );
    let context = linked.fsmonitor_server_context()?;
    assert_eq!(
        context.git_dir,
        linked.git_dir().canonicalize()?,
        "linked worktree contexts retain their canonical private Git directory"
    );
    assert_eq!(
        context.common_dir,
        main.git_dir().canonicalize()?,
        "linked worktree contexts retain the main repository's shared Git directory"
    );
    let handle = linked.fsmonitor_server(Options {
        idle_timeout: Some(Duration::from_millis(200)),
        ..Default::default()
    })?;
    drop(linked);
    assert_eq!(
        handle.endpoint(),
        endpoint,
        "linked worktrees use their private administrative IPC endpoint"
    );
    assert!(
        ipc::query(&endpoint, ipc::Request::Message(b""))?.ends_with(b"\0/\0"),
        "clients independently locate the owned server after its repository handle is dropped"
    );
    std::thread::sleep(Duration::from_millis(700));
    assert!(
        !ipc::is_running(handle.endpoint())?,
        "repository startup forwards the configured idle timeout and releases the endpoint"
    );
    handle.wait()?;

    let handle = main.fsmonitor_server(Options::default())?;
    #[cfg(any(target_os = "macos", windows))]
    gix_testtools::git(root, "fsmonitor--daemon status")?;
    assert!(
        ipc::query(handle.endpoint(), ipc::Request::Message(b""))?.ends_with(b"\0/\0"),
        "default options start an application-owned server"
    );
    handle.shutdown()?;
    Ok(())
}

#[cfg(all(feature = "fsmonitor-daemon", any(target_os = "macos", target_os = "linux", windows)))]
#[test]
fn daemon_commands_use_the_server_context() -> gix_error::TestResult {
    use gix::fsmonitor::daemon::Command;

    let repo = crate::named_repo("make_basic_repo.sh")?;
    let context = repo.fsmonitor_server_context()?;
    assert_eq!(
        context.endpoint,
        repo.git_dir().canonicalize()?.join("fsmonitor--daemon.ipc"),
        "the gix facade resolves the worktree's native daemon endpoint"
    );
    let mut out = Vec::new();
    repo.fsmonitor_daemon_execute(
        Command::Path,
        &mut out,
        || panic!("printing a path does not poll for interruption"),
        |_| panic!("printing a path does not request a detached child"),
    )?;
    assert_eq!(
        out,
        format!("{}\n", context.endpoint.display()).into_bytes(),
        "the gix facade executes shared daemon commands"
    );
    Ok(())
}

#[test]
fn bare_repositories_cannot_start_a_worktree_monitor() -> gix_error::TestResult {
    let bare = crate::named_subrepo_opts("make_basic_repo.sh", "bare.git", gix::open::Options::isolated())?;
    assert!(
        bare.fsmonitor_server_context().is_err(),
        "preparing an embedded server context requires a worktree"
    );
    assert!(
        bare.fsmonitor_server(Options::default()).is_err(),
        "a filesystem monitor needs a worktree"
    );
    Ok(())
}
