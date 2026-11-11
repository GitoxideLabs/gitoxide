use std::{
    path::{Path, PathBuf},
    rc::Rc,
    sync::mpsc,
    thread::ThreadId,
    time::{Duration, Instant},
};

use bstr::ByteSlice;
use gix_error::{ErrorExt, Result, message};

use gix_notify::{Batch, Budget, Fence, SynchronizeError, Watch, Watcher};
use gix_testtools::{TestResult, tempfile::TempDir};

use crate::{client, server};

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

#[derive(Debug)]
enum SourceCall {
    Created(server::Context, ThreadId),
    Register(Vec<Watch>),
    Drain,
    Fence(PathBuf, Duration),
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
        self.record(SourceCall::Fence(
            directory.to_owned(),
            deadline.saturating_duration_since(Instant::now()),
        ));
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
fn idle_server_waits_for_source_notifications_clients_and_shutdown() -> TestResult {
    struct IdleSource {
        drained: mpsc::Sender<()>,
        registered: mpsc::Sender<()>,
        failed: mpsc::Receiver<()>,
        waker: mpsc::SyncSender<Box<dyn Fn() + Send + Sync>>,
    }

    impl server::EventSource for IdleSource {
        fn register(&mut self, _watches: &[Watch]) -> Result<()> {
            self.registered
                .send(())
                .expect("the test observes registration until shutdown");
            Ok(())
        }

        fn drain(&mut self, _budget: Budget) -> Batch {
            self.drained
                .send(())
                .expect("the test observes every drain until shutdown");
            Batch {
                events: Vec::new(),
                loss: self.failed.try_recv().ok().map(|()| gix_notify::Loss::BackendError),
                error: None,
                generation: 1,
                sequence: 0,
                more: false,
            }
        }

        fn synchronize_at(
            &mut self,
            _directory: &Path,
            _deadline: Instant,
        ) -> std::result::Result<Fence, SynchronizeError> {
            Err(SynchronizeError::Unsupported)
        }

        fn set_waker(&mut self, wake: Box<dyn Fn() + Send + Sync + 'static>) {
            self.waker.send(wake).expect("the test receives the source waker");
        }
    }

    let fixture = Fixture::new()?;
    let (drained, receive) = mpsc::channel();
    let (registered, receive_registration) = mpsc::channel();
    let (fail, failed) = mpsc::channel();
    let (waker, receive_waker) = mpsc::sync_channel(1);
    let handle = server::spawn_with_source(
        fixture.context(),
        server::Options {
            retry_interval: Duration::from_millis(50),
            ..Default::default()
        },
        |_| {
            Ok(IdleSource {
                drained,
                registered,
                failed,
                waker,
            })
        },
    )?;
    let wake = receive_waker.recv_timeout(Duration::from_secs(1))?;
    receive_registration.recv_timeout(Duration::from_secs(1))?;
    receive.recv_timeout(Duration::from_secs(1))?;
    assert_eq!(
        receive.recv_timeout(Duration::from_millis(100)),
        Err(mpsc::RecvTimeoutError::Timeout),
        "an idle server does not periodically drain its source"
    );
    wake();
    receive.recv_timeout(Duration::from_secs(1))?;
    assert_eq!(
        receive.recv_timeout(Duration::from_millis(100)),
        Err(mpsc::RecvTimeoutError::Timeout),
        "a consumed wake notification does not leave the server spinning"
    );
    assert!(
        query(&handle, b"")?.ends_with(b"\0/\0"),
        "a client connection wakes an idle server and receives its baseline"
    );
    for probe in [false, true, true] {
        if probe {
            assert!(
                server::ipc::is_running(handle.endpoint())?,
                "closed status probes wake the same listener after a completed query"
            );
        }
        // A query drains before accepting, while preparing its response, and after completion.
        // Probes return before the worker handles them, so await activity before checking quiescence.
        receive.recv_timeout(Duration::from_secs(1))?;
        let mut idle = false;
        for _ in 0..8 {
            match receive.recv_timeout(Duration::from_millis(100)) {
                Ok(()) => {}
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    idle = true;
                    break;
                }
                Err(error) => return Err(error.into()),
            }
        }
        assert!(
            idle,
            "completed clients and probes do not leave a stale readiness event spinning"
        );
    }
    fail.send(())?;
    wake();
    receive_registration
        .recv_timeout(Duration::from_secs(1))
        .expect("recovery wakes at its retry deadline without another client or source notification");
    let stopped = Instant::now();
    handle.request_shutdown();
    handle.request_shutdown();
    handle.wait()?;
    assert!(
        stopped.elapsed() < Duration::from_secs(1),
        "shutdown wakes the IPC wait without awaiting its maintenance timeout"
    );
    Ok(())
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
    let options = server::Options {
        fence_timeout: Duration::from_millis(250),
        retry_interval: Duration::from_millis(50),
        idle_interval: Duration::from_millis(2),
        ..Default::default()
    };
    let handle = server::spawn_with_source(context, options, move |context| {
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
            .any(|call| matches!(call, SourceCall::Fence(path, _) if path == &cookie_dir)),
        "query fences use the injected stream's cookie coverage"
    );
    assert!(
        calls.iter().all(|call| match call {
            SourceCall::Fence(_, remaining) => *remaining <= options.fence_timeout,
            _ => true,
        }),
        "query fences use the configured synchronization timeout"
    );
    assert!(
        matches!(calls.last(), Some(SourceCall::Dropped)),
        "shutdown drops the source before returning"
    );
    server::spawn(expected, server::Options::default())?.shutdown()?;
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
        server::spawn(fixture.context(), server::Options::default())?.shutdown()?;
    }
    Ok(())
}

#[test]
fn injected_relative_paths_are_rejected_before_starting_a_server() -> TestResult {
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
    server::spawn(fixture.context(), server::Options::default())?.shutdown()?;
    Ok(())
}
