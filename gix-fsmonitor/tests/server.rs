#![cfg(feature = "server")]

use std::{
    path::{Path, PathBuf},
    rc::Rc,
    sync::mpsc,
    thread::ThreadId,
    time::{Duration, Instant},
};

use bstr::ByteSlice;
use gix_error::{ErrorExt, Result, message};
use gix_fsmonitor::{client, server};
use gix_notify::{Batch, Budget, Fence, SynchronizeError, Watch, Watcher};
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

#[derive(Debug)]
enum SourceCall {
    Created(server::Context, ThreadId),
    Register(Vec<Watch>),
    Drain,
    Fence(PathBuf),
    Waker,
    Dropped,
}

struct RecordingSource {
    watcher: Watcher,
    calls: mpsc::Sender<SourceCall>,
    // Deliberately not Send: only the factory crosses into the server thread.
    thread: Rc<ThreadId>,
    fail_registration: bool,
}

impl RecordingSource {
    fn new(context: &server::Context, calls: mpsc::Sender<SourceCall>, fail_registration: bool) -> Result<Self> {
        let thread = std::thread::current().id();
        calls
            .send(SourceCall::Created(context.clone(), thread))
            .expect("the test observes source creation until shutdown");
        Ok(Self {
            watcher: Watcher::new(Default::default())?,
            calls,
            thread: Rc::new(thread),
            fail_registration,
        })
    }

    fn record(&self, call: SourceCall) {
        assert_eq!(
            std::thread::current().id(),
            *self.thread,
            "the source stays on its factory's thread throughout its lifetime"
        );
        self.calls
            .send(call)
            .expect("the test observes the source until shutdown");
    }
}

impl server::EventSource for RecordingSource {
    fn register(&mut self, watches: &[Watch]) -> Result<()> {
        assert!(
            watches.iter().all(|watch| watch.path.is_dir()),
            "the server prepares cookies before registering the full watch set"
        );
        self.record(SourceCall::Register(watches.to_vec()));
        self.watcher.replace(watches.iter().cloned())?;
        if self.fail_registration {
            return Err(message("injected registration failure").raise());
        }
        Ok(())
    }

    fn drain(&mut self, budget: Budget) -> Batch {
        self.record(SourceCall::Drain);
        self.watcher.drain(budget)
    }

    fn synchronize_at(&mut self, directory: &Path, deadline: Instant) -> std::result::Result<Fence, SynchronizeError> {
        self.record(SourceCall::Fence(directory.to_owned()));
        self.watcher.synchronize_at(directory, deadline)
    }

    fn set_waker(&mut self, wake: Box<dyn Fn() + Send + Sync + 'static>) {
        self.record(SourceCall::Waker);
        self.watcher.set_waker(wake);
    }
}

impl Drop for RecordingSource {
    fn drop(&mut self) {
        self.record(SourceCall::Dropped);
    }
}

#[test]
fn injected_source_uses_normalized_context_and_owns_the_ready_server_lifecycle() -> TestResult {
    let fixture = Fixture::new()?;
    let mut expected = fixture.context();
    // Separate administrative roots exercise all parent sentinels, as in linked worktrees.
    expected.git_dir = fixture._parent.path().join("private/git");
    expected.common_dir = fixture._parent.path().join("shared/git");
    std::fs::create_dir_all(&expected.git_dir)?;
    std::fs::create_dir_all(&expected.common_dir)?;
    expected.git_dir = expected.git_dir.canonicalize()?;
    expected.common_dir = expected.common_dir.canonicalize()?;
    expected.endpoint = expected.git_dir.join("fsmonitor--daemon.ipc");
    expected.precompose_unicode = true;
    expected.ignore_case = true;
    let mut context = expected.clone();
    context.worktree.push(".git/..");
    context.git_dir.push("../git");
    context.common_dir.push("../git");
    context.endpoint = context.git_dir.join("fsmonitor--daemon.ipc");

    let caller = std::thread::current().id();
    let (send, receive) = mpsc::channel();
    let handle = server::spawn_with_source(context, server::Options::default(), move |context| {
        RecordingSource::new(context, send, false)
    })?;
    assert_eq!(
        handle.endpoint(),
        expected.endpoint,
        "the owned endpoint uses normalized paths"
    );
    let calls: Vec<_> = receive.try_iter().collect();
    let Some(SourceCall::Created(actual, worker)) = calls.first() else {
        return Err("the source factory must run before readiness".into());
    };
    assert_ne!(*worker, caller, "the factory runs on the server's worker thread");
    assert_eq!(actual.worktree, expected.worktree, "the worktree uses native spelling");
    assert_eq!(
        actual.git_dir, expected.git_dir,
        "the private Git directory is normalized"
    );
    assert_eq!(
        actual.common_dir, expected.common_dir,
        "the shared Git directory is normalized"
    );
    assert_eq!(
        actual.endpoint, expected.endpoint,
        "the endpoint's parent is normalized"
    );
    assert!(
        actual.precompose_unicode && actual.ignore_case,
        "filesystem settings reach the factory"
    );
    assert!(
        matches!(calls.get(1), Some(SourceCall::Waker)),
        "the server installs the source waker"
    );
    let Some(SourceCall::Register(watches)) = calls.get(2) else {
        return Err("the complete watch set must be registered before readiness".into());
    };
    let cookie_dir = expected.git_dir.join("fsmonitor--daemon/cookies");
    for path in [&expected.worktree, &cookie_dir] {
        assert!(
            watches.contains(&Watch {
                path: path.clone(),
                recursive: true
            }),
            "registration includes recursive worktree and cookie coverage"
        );
    }
    for root in [
        &expected.worktree,
        &expected.git_dir,
        &expected.common_dir,
        &expected.worktree.join(".git"),
    ] {
        assert!(
            watches.contains(&Watch {
                path: root.parent().ok_or("fixture roots have parents")?.to_owned(),
                recursive: false,
            }),
            "registration includes every root's parent sentinel"
        );
    }
    let initial = query(&handle, b"")?;
    assert!(
        initial.ends_with(b"\0/\0"),
        "readiness permits an immediate baseline query"
    );
    let _ = query(&handle, token(&initial)?)?;
    handle.shutdown()?;
    let calls: Vec<_> = receive.try_iter().collect();
    assert!(
        calls.iter().any(|call| matches!(call, SourceCall::Drain)),
        "the server drains the injected stream"
    );
    assert!(
        calls
            .iter()
            .any(|call| matches!(call, SourceCall::Fence(path) if path == &cookie_dir)),
        "query fences use the injected stream's cookie coverage"
    );
    assert!(
        matches!(calls.last(), Some(SourceCall::Dropped)),
        "shutdown drops the source before returning"
    );
    server::spawn(expected)?.shutdown()?;
    Ok(())
}

#[test]
fn injected_factory_and_registration_failures_release_the_endpoint_before_returning() -> TestResult {
    let fixture = Fixture::new()?;
    for fail_factory in [true, false] {
        let (send, receive) = mpsc::channel();
        let result = server::spawn_with_source(fixture.context(), server::Options::default(), move |context| {
            if fail_factory {
                return Err(message("injected factory failure").raise());
            }
            RecordingSource::new(context, send, true)
        });
        let error = result.err().ok_or("failed source startup must not report readiness")?;
        assert!(
            format!("{error:#}").contains(if fail_factory {
                "injected factory failure"
            } else {
                "injected registration failure"
            }),
            "startup preserves the source's failure context"
        );
        let calls: Vec<_> = receive.try_iter().collect();
        if fail_factory {
            assert!(calls.is_empty(), "a failed factory creates no source");
        } else {
            assert!(
                calls.iter().any(|call| matches!(call, SourceCall::Register(_))),
                "registration was attempted"
            );
            assert!(
                matches!(calls.last(), Some(SourceCall::Dropped)),
                "failed registration drops the source before returning"
            );
        }
        assert!(
            !server::ipc::is_running(&fixture.context().endpoint)?,
            "failed startup releases the endpoint"
        );
        server::spawn(fixture.context())?.shutdown()?;
    }
    Ok(())
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
        assert!(
            server::spawn(context.clone()).is_err(),
            "{field} must be an absolute path"
        );
        let (send, receive) = mpsc::channel();
        assert!(
            server::spawn_with_source(context, server::Options::default(), move |context| {
                RecordingSource::new(context, send, false)
            })
            .is_err(),
            "injected sources also require an absolute {field}"
        );
        assert_eq!(
            receive.try_iter().count(),
            0,
            "invalid context is rejected before invoking the factory"
        );
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

#[test]
fn idle_shutdown_ignores_filesystem_activity_and_releases_the_endpoint() -> gix_error::TestResult {
    let fixture = Fixture::new()?;
    let handle = server::spawn_with_options(
        fixture.context(),
        server::Options {
            idle_timeout: Some(Duration::from_millis(300)),
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
    let restarted = server::spawn(fixture.context())?;
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
    let handle = server::spawn_with_options(
        fixture.context(),
        server::Options {
            idle_timeout: Some(Duration::from_millis(600)),
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

#[cfg(unix)]
#[test]
fn an_active_connection_gets_a_new_idle_window_when_it_finishes() -> gix_error::TestResult {
    let fixture = Fixture::new()?;
    // std's socket client needs a short path, unlike the server's long-path-aware transport.
    let endpoint_dir = gix_testtools::tempfile::tempdir_in("/tmp")?;
    let mut context = fixture.context();
    context.endpoint = endpoint_dir.path().join("ipc");
    let handle = server::spawn_with_options(
        context,
        server::Options {
            idle_timeout: Some(Duration::from_millis(500)),
        },
    )?;
    let stream = std::os::unix::net::UnixStream::connect(handle.endpoint())?;
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
        server::spawn_with_options(
            fixture.context(),
            server::Options {
                idle_timeout: Some(Duration::ZERO),
            },
        )
        .is_err(),
        "use None to disable idle shutdown; a configured timeout must be positive"
    );
    server::spawn(fixture.context())?.shutdown()?;
    Ok(())
}
