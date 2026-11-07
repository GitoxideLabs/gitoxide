#![cfg(feature = "server")]

use std::{path::PathBuf, time::Duration};

use bstr::ByteSlice;
use gix_fsmonitor::{client, server};
use gix_testtools::{TestResult, tempfile::TempDir};

struct Fixture {
    _parent: TempDir,
    worktree: PathBuf,
}

impl Fixture {
    fn new() -> TestResult<Self> {
        // Root replacement watches the worktree's parent, so keep it private to this fixture.
        let parent = gix_testtools::tempfile::tempdir()?;
        let worktree = parent.path().join("worktree");
        std::fs::create_dir_all(worktree.join(".git"))?;
        Ok(Self {
            _parent: parent,
            worktree: worktree.canonicalize()?,
        })
    }

    fn context(&self) -> server::Context {
        let git_dir = self.worktree.join(".git");
        server::Context {
            worktree: self.worktree.clone(),
            common_dir: git_dir.clone(),
            endpoint: git_dir.join("fsmonitor--daemon.ipc"),
            git_dir,
            precompose_unicode: false,
            ignore_case: false,
        }
    }
}

fn query(handle: &server::Handle, token: &[u8]) -> TestResult<Vec<u8>> {
    Ok(client::daemon::query(
        handle.endpoint(),
        token.as_bstr(),
        || panic!("successful startup must make the server available without an external process"),
        &client::daemon::Options {
            socket_timeout: Some(Duration::from_secs(5)),
            ..Default::default()
        },
    )?)
}

fn token(response: &[u8]) -> TestResult<&[u8]> {
    let end = response
        .iter()
        .position(|byte| *byte == 0)
        .ok_or("server response has no token separator")?;
    assert!(end > 0, "server checkpoints have a nonempty opaque token");
    Ok(&response[..end])
}

#[test]
fn startup_is_ready_and_shutdown_releases_the_endpoint() -> TestResult {
    let fixture = Fixture::new()?;
    let handle = server::spawn(fixture.context())?;
    assert_eq!(
        handle.endpoint(),
        fixture.context().endpoint,
        "applications can discover the endpoint owned by their server"
    );
    let initial = query(&handle, b"")?;
    assert!(initial.ends_with(b"\0/\0"), "a new client needs a full baseline");

    handle.request_shutdown();
    handle.request_shutdown();
    handle.wait()?;
    let restarted = server::spawn(fixture.context())?;
    let fresh = query(&restarted, token(&initial)?)?;
    assert!(
        fresh.ends_with(b"\0/\0"),
        "a restart cannot reuse a checkpoint from the server that stopped watching"
    );
    assert_ne!(
        token(&initial)?,
        token(&fresh)?,
        "each server instance establishes an independent token epoch"
    );
    restarted.shutdown()?;
    Ok(())
}

#[test]
fn servers_have_independent_ownership_and_drop_releases_the_endpoint() -> TestResult {
    let first = Fixture::new()?;
    let second = Fixture::new()?;
    let first_handle = server::spawn(first.context())?;
    let second_handle = server::spawn(second.context())?;
    assert!(
        server::spawn(first.context()).is_err(),
        "a second owner must not steal a live endpoint"
    );
    assert!(
        query(&first_handle, b"")?.ends_with(b"\0/\0"),
        "refusing a duplicate owner leaves the original service available"
    );

    drop(first_handle);
    let replacement = server::spawn(first.context())?;
    assert!(
        query(&second_handle, b"")?.ends_with(b"\0/\0"),
        "stopping one worktree's server does not stop another worktree's service"
    );
    replacement.shutdown()?;
    second_handle.shutdown()?;
    Ok(())
}

#[test]
fn relative_paths_are_rejected_before_starting_a_server() -> TestResult {
    let fixture = Fixture::new()?;
    let mut contexts: [_; 4] = std::array::from_fn(|_| fixture.context());
    contexts[0].worktree = "relative".into();
    contexts[1].git_dir = "relative".into();
    contexts[2].common_dir = "relative".into();
    contexts[3].endpoint = "relative".into();
    for (field, context) in ["worktree", "git_dir", "common_dir", "endpoint"]
        .into_iter()
        .zip(contexts)
    {
        assert!(server::spawn(context).is_err(), "{field} must be an absolute path");
    }
    server::spawn(fixture.context())?.shutdown()?;
    Ok(())
}

#[test]
fn failed_startup_does_not_leave_an_endpoint_owned() -> TestResult {
    let fixture = Fixture::new()?;
    let mut missing = fixture.context();
    missing.worktree.push("missing");
    assert!(server::spawn(missing).is_err(), "missing worktrees cannot be watched");

    // Cookie setup follows endpoint binding, so this exercises cleanup after startup has begun.
    let blocked = fixture.worktree.join(".git/fsmonitor--daemon");
    std::fs::write(&blocked, b"not a directory")?;
    assert!(
        server::spawn(fixture.context()).is_err(),
        "startup must report failure when synchronization coverage cannot be installed"
    );
    std::fs::remove_file(blocked)?;
    let handle = server::spawn(fixture.context())?;
    assert!(
        query(&handle, b"")?.ends_with(b"\0/\0"),
        "a failed startup releases the endpoint for the next owner"
    );
    handle.shutdown()?;
    Ok(())
}
