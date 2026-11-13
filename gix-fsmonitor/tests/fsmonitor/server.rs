use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use bstr::ByteSlice;
use gix_fsmonitor::{client, server};
use gix_testtools::{TestResult, tempfile::TempDir};

#[cfg(feature = "async")]
#[path = "asynchronous.rs"]
mod asynchronous;

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
        client::daemon::Options {
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
fn default_options_preserve_server_timings() -> TestResult {
    let options = server::Options::default();
    assert!(!options.allow_remote, "remote monitoring remains opt-in");
    #[cfg(feature = "daemon")]
    {
        let daemon_options = gix_fsmonitor::daemon::Options::default();
        assert!(
            !daemon_options.allow_remote,
            "daemon commands reject remote worktrees by default"
        );
        assert_eq!(
            daemon_options.start_timeout,
            Duration::from_secs(60),
            "daemon startup uses Git's default sixty-second readiness timeout"
        );
    }
    assert!(options.idle_timeout.is_none(), "idle shutdown remains opt-in");
    assert_eq!(
        options.fence_timeout,
        Duration::from_secs(1),
        "fences retain a one-second deadline"
    );
    assert_eq!(
        options.retry_interval,
        Duration::from_secs(5),
        "recovery retains a five-second retry delay"
    );
    assert_eq!(
        options.idle_interval,
        Duration::from_millis(20),
        "synchronization cookie polling retains a twenty-millisecond interval"
    );
    Ok(())
}

#[test]
fn clients_resolve_endpoint_directory_aliases() -> TestResult {
    let fixture = Fixture::new()?;
    let alias = fixture._parent.path().join("alias");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&fixture.worktree, &alias)?;
    #[cfg(windows)]
    {
        let output = std::process::Command::new("cmd.exe")
            .args(["/D", "/C", "mklink", "/J", "alias", "worktree"])
            .current_dir(fixture._parent.path())
            .output()?;
        assert!(output.status.success(), "junction creation failed: {output:?}");
    }
    let aliases = vec![alias];
    #[cfg(windows)]
    let aliases = {
        let mut aliases = aliases;
        let symlink = fixture._parent.path().join("symlink");
        match std::os::windows::fs::symlink_dir(&fixture.worktree, &symlink) {
            Ok(()) => aliases.push(symlink),
            Err(err) if err.raw_os_error() == Some(1314) => {
                eprintln!("Skipping directory symlink: symlink creation requires a privilege");
            }
            Err(err) => return Err(err.into()),
        }
        aliases
    };
    let handle = server::spawn(fixture.context(), server::Options::default())?;
    #[cfg(windows)]
    assert!(!handle.endpoint().exists(), "named pipes have no filesystem endpoint");
    for alias in aliases {
        let endpoint = alias.join(".git/fsmonitor--daemon.ipc");
        let response = client::daemon::query(
            &endpoint,
            b"".as_bstr(),
            || panic!("an alias must reach the running server without starting a daemon"),
            client::daemon::Options::default(),
        )?;
        assert!(
            response.ends_with(b"\0/\0"),
            "the alias returns a fresh client baseline"
        );
        assert!(
            server::ipc::is_running(&endpoint)?,
            "status probes resolve the same alias"
        );
        assert!(
            server::ipc::query(&endpoint, server::ipc::Request::Message(b""))?.ends_with(b"\0/\0"),
            "IPC queries resolve the same alias"
        );
    }
    handle.shutdown()?;
    Ok(())
}

#[test]
fn startup_is_ready_and_shutdown_releases_the_endpoint() -> TestResult {
    let fixture = Fixture::new()?;
    let handle = server::spawn(fixture.context(), server::Options::default())?;
    assert!(!handle.is_finished(), "a ready server remains active before shutdown");
    assert_eq!(
        handle.endpoint(),
        fixture.context().endpoint,
        "applications can discover the endpoint owned by their server"
    );
    let initial = query(&handle, b"")?;
    assert!(initial.ends_with(b"\0/\0"), "a new client needs a full baseline");

    handle.request_shutdown();
    handle.request_shutdown();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !handle.is_finished() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        handle.is_finished(),
        "thread completion can be observed before joining the owned server"
    );
    handle.wait()?;
    let restarted = server::spawn(fixture.context(), server::Options::default())?;
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
fn native_ipc_requests_query_flush_and_quit() -> TestResult {
    use server::ipc::{Request, query};

    let fixture = Fixture::new()?;
    let handle = server::spawn(fixture.context(), server::Options::default())?;
    let initial = query(handle.endpoint(), Request::Message(b"unknown-token"))?;
    assert!(
        initial.ends_with(b"\0/\0"),
        "arbitrary messages are interpreted as tokens, with unknown tokens requiring a full baseline"
    );

    let flushed = query(handle.endpoint(), Request::Flush)?;
    assert!(flushed.ends_with(b"\0/\0"), "flush returns a fresh full baseline");
    let fresh = query(handle.endpoint(), Request::Message(token(&initial)?))?;
    assert!(
        fresh.ends_with(b"\0/\0"),
        "flush invalidates checkpoints issued before the command"
    );
    let raw_flushed = query(handle.endpoint(), Request::Message(b"flush"))?;
    assert_ne!(
        token(&raw_flushed)?,
        token(&fresh)?,
        "raw control-command payloads retain their native interpretation"
    );

    assert!(
        query(handle.endpoint(), Request::Quit)?.is_empty(),
        "quit acknowledges shutdown with an empty response"
    );
    handle.wait()?;
    assert!(
        !server::ipc::is_running(&fixture.context().endpoint)?,
        "quit releases the endpoint"
    );
    Ok(())
}

#[test]
fn servers_have_independent_ownership_and_drop_releases_the_endpoint() -> TestResult {
    let first = Fixture::new()?;
    let second = Fixture::new()?;
    let first_handle = server::spawn(first.context(), server::Options::default())?;
    let second_handle = server::spawn(second.context(), server::Options::default())?;
    assert!(
        server::spawn(first.context(), server::Options::default()).is_err(),
        "a second owner must not steal a live endpoint"
    );
    assert!(
        query(&first_handle, b"")?.ends_with(b"\0/\0"),
        "refusing a duplicate owner leaves the original service available"
    );

    drop(first_handle);
    let replacement = server::spawn(first.context(), server::Options::default())?;
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
        assert!(
            server::spawn(context, server::Options::default()).is_err(),
            "{field} must be an absolute path"
        );
    }
    server::spawn(fixture.context(), server::Options::default())?.shutdown()?;
    Ok(())
}

#[test]
fn failed_startup_does_not_leave_an_endpoint_owned() -> TestResult {
    let fixture = Fixture::new()?;
    let mut missing = fixture.context();
    missing.worktree.push("missing");
    assert!(
        server::spawn(missing, server::Options::default()).is_err(),
        "missing worktrees cannot be watched"
    );

    // Cookie setup follows endpoint binding, so this exercises cleanup after startup has begun.
    let blocked = fixture.worktree.join(".git/fsmonitor--daemon");
    std::fs::write(&blocked, b"not a directory")?;
    assert!(
        server::spawn(fixture.context(), server::Options::default()).is_err(),
        "startup must report failure when synchronization coverage cannot be installed"
    );
    std::fs::remove_file(blocked)?;
    let handle = server::spawn(fixture.context(), server::Options::default())?;
    assert!(
        query(&handle, b"")?.ends_with(b"\0/\0"),
        "a failed startup releases the endpoint for the next owner"
    );
    handle.shutdown()?;
    Ok(())
}

#[test]
fn idle_shutdown_ignores_filesystem_activity_and_releases_the_endpoint() -> gix_error::TestResult {
    let fixture = Fixture::new()?;
    let handle = server::spawn(
        fixture.context(),
        server::Options {
            idle_timeout: Some(Duration::from_millis(300)),
            idle_interval: Duration::from_millis(5),
            ..Default::default()
        },
    )?;
    let initial = query(&handle, b"")?;
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        std::fs::write(
            fixture.worktree.join("changing"),
            b"filesystem activity is not client activity",
        )?;
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        !server::ipc::is_running(handle.endpoint())?,
        "filesystem changes must not keep an unused server alive"
    );
    handle.wait()?;

    assert!(
        server::Options::default().idle_timeout.is_none(),
        "idle shutdown is opt-in"
    );
    let restarted = server::spawn(fixture.context(), server::Options::default())?;
    std::thread::sleep(Duration::from_millis(800));
    let fresh = query(&restarted, token(&initial)?)?;
    assert!(
        fresh.ends_with(b"\0/\0"),
        "the default server stays alive and rejects checkpoints from before idle shutdown"
    );
    assert_ne!(
        token(&initial)?,
        token(&fresh)?,
        "restart establishes a fresh token epoch"
    );
    restarted.shutdown()?;
    Ok(())
}

#[test]
fn queries_and_status_probes_extend_the_idle_deadline() -> gix_error::TestResult {
    let fixture = Fixture::new()?;
    let handle = server::spawn(
        fixture.context(),
        server::Options {
            idle_timeout: Some(Duration::from_millis(600)),
            ..Default::default()
        },
    )?;
    for _ in 0..4 {
        std::thread::sleep(Duration::from_millis(200));
        let _ = query(&handle, b"")?;
    }
    for _ in 0..4 {
        std::thread::sleep(Duration::from_millis(200));
        assert!(
            server::ipc::is_running(handle.endpoint())?,
            "status probes keep the server alive beyond the last query's deadline"
        );
    }
    std::thread::sleep(Duration::from_millis(1000));
    assert!(
        !server::ipc::is_running(handle.endpoint())?,
        "the server shuts down once clients stop contacting it"
    );
    handle.wait()?;
    Ok(())
}

#[test]
fn an_active_connection_gets_a_new_idle_window_when_it_finishes() -> gix_error::TestResult {
    let fixture = Fixture::new()?;
    #[cfg(unix)]
    let mut context = fixture.context();
    #[cfg(windows)]
    let context = fixture.context();
    // std's socket client needs a short path, unlike the server's long-path-aware transport.
    #[cfg(unix)]
    let endpoint_dir = gix_testtools::tempfile::tempdir_in("/tmp")?;
    #[cfg(unix)]
    {
        context.endpoint = endpoint_dir.path().join("ipc");
    }
    let handle = server::spawn(
        context,
        server::Options {
            idle_timeout: Some(Duration::from_millis(500)),
            ..Default::default()
        },
    )?;
    #[cfg(unix)]
    let stream = std::os::unix::net::UnixStream::connect(handle.endpoint())?;
    #[cfg(windows)]
    let stream = {
        use std::os::windows::ffi::{OsStrExt, OsStringExt};

        // Git's native pipe spelling removes the verbatim prefix and replaces the drive colon.
        let mut path: Vec<_> = handle.endpoint().as_os_str().encode_wide().collect();
        let prefix: Vec<_> = "\\\\?\\".encode_utf16().collect();
        let unc_prefix: Vec<_> = "\\\\?\\UNC\\".encode_utf16().collect();
        if path.starts_with(&unc_prefix) {
            path.splice(..unc_prefix.len(), "\\\\".encode_utf16());
        } else if path.starts_with(&prefix) {
            path.drain(..prefix.len());
        }
        if path.get(1) == Some(&(b':' as u16)) {
            path[1] = b'_' as u16;
        }
        let mut pipe: Vec<_> = "\\\\.\\pipe\\".encode_utf16().collect();
        pipe.extend(path);
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(std::ffi::OsString::from_wide(&pipe))?
    };
    std::thread::sleep(Duration::from_millis(900));
    drop(stream);
    std::thread::sleep(Duration::from_millis(100));
    assert!(
        query(&handle, b"")?.ends_with(b"\0/\0"),
        "idle time starts after the active connection finishes, even when it exceeds the timeout"
    );
    std::thread::sleep(Duration::from_millis(900));
    assert!(
        !server::ipc::is_running(handle.endpoint())?,
        "completed connections do not prevent later idle shutdown"
    );
    handle.wait()?;
    Ok(())
}

#[test]
fn zero_idle_timeout_is_rejected_without_owning_the_endpoint() -> gix_error::TestResult {
    let fixture = Fixture::new()?;
    assert!(
        server::spawn(
            fixture.context(),
            server::Options {
                idle_timeout: Some(Duration::ZERO),
                ..Default::default()
            },
        )
        .is_err(),
        "use None to disable idle shutdown; a configured timeout must be positive"
    );
    server::spawn(fixture.context(), server::Options::default())?.shutdown()?;
    Ok(())
}
