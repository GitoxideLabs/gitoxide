use std::time::Duration;

use gix::fsmonitor::server::{Options, ipc};

#[test]
fn embedded_server_uses_linked_worktree_layout_and_idle_options() -> gix_error::TestResult {
    let (main, _fixture) = crate::basic_rw_repo()?;
    let root = main.workdir().expect("the fixture has a worktree");
    gix_testtools::git(root, "worktree add --detach linked")?;
    let linked = gix::open_opts(root.join("linked"), gix::open::Options::isolated())?;
    let endpoint = linked.git_dir().canonicalize()?.join("fsmonitor--daemon.ipc");
    assert_ne!(
        linked.git_dir(),
        linked.common_dir(),
        "the fixture distinguishes private and shared administrative paths"
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
        ipc::query(handle.endpoint(), ipc::Request::Message(b""))?.ends_with(b"\0/\0"),
        "the owned server keeps serving clients after its repository handle is dropped"
    );
    std::thread::sleep(Duration::from_millis(700));
    assert!(
        !ipc::is_running(handle.endpoint())?,
        "repository startup forwards the configured idle timeout and releases the endpoint"
    );
    handle.wait()?;

    let handle = main.fsmonitor_server(Options::default())?;
    assert!(
        ipc::query(handle.endpoint(), ipc::Request::Message(b""))?.ends_with(b"\0/\0"),
        "default options start an application-owned server"
    );
    handle.shutdown()?;
    Ok(())
}

#[cfg(all(feature = "fsmonitor-daemon", any(target_os = "macos", target_os = "linux", windows)))]
#[test]
fn daemon_context_and_commands_are_available_through_gix() -> gix_error::TestResult {
    use gix::fsmonitor::daemon::Command;

    let repo = crate::named_repo("make_basic_repo.sh")?;
    let context = repo.fsmonitor_daemon_context()?;
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
        bare.fsmonitor_server(Options::default()).is_err(),
        "a filesystem monitor needs a worktree"
    );
    #[cfg(feature = "fsmonitor-daemon")]
    assert!(
        bare.fsmonitor_daemon_context().is_err(),
        "daemon control also requires a worktree"
    );
    Ok(())
}

#[cfg(feature = "async-fsmonitor-server")]
#[tokio::test(flavor = "current_thread")]
async fn async_repository_server_preserves_linked_worktree_context_and_shutdown() -> gix_error::TestResult {
    let (main, _fixture) = crate::basic_rw_repo()?;
    let root = main.workdir().expect("the fixture has a worktree");
    gix_testtools::git(root, "worktree add --detach linked")?;
    let linked = gix::open_opts(root.join("linked"), gix::open::Options::isolated())?;
    let endpoint = linked.git_dir().canonicalize()?.join("fsmonitor--daemon.ipc");
    let startup = linked.fsmonitor_server_async(Options::default());
    drop(linked);
    let server = tokio::spawn(startup).await??;
    assert_eq!(
        server.endpoint(),
        endpoint,
        "async startup uses the private Git directory too"
    );
    assert!(
        ipc::query_async(&endpoint, ipc::Request::Message(b""))
            .await?
            .ends_with(b"\0/\0"),
        "the server retains no repository handle"
    );
    server.shutdown().await?;
    assert!(
        !ipc::is_running_async(&endpoint).await?,
        "awaited shutdown releases the native endpoint"
    );
    Ok(())
}
