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
    })?;
    drop(linked);
    assert_eq!(
        handle.endpoint(),
        endpoint,
        "linked worktrees use their private administrative IPC endpoint"
    );
    assert!(
        ipc::query(handle.endpoint(), b"")?.ends_with(b"\0/\0"),
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
        ipc::query(handle.endpoint(), b"")?.ends_with(b"\0/\0"),
        "default options start an application-owned server"
    );
    handle.shutdown()?;
    Ok(())
}

#[test]
fn bare_repositories_cannot_start_a_worktree_monitor() -> gix_error::TestResult {
    let bare = crate::named_subrepo_opts("make_basic_repo.sh", "bare.git", gix::open::Options::isolated())?;
    assert!(
        bare.fsmonitor_server(Options::default()).is_err(),
        "a filesystem monitor needs a worktree"
    );
    Ok(())
}
