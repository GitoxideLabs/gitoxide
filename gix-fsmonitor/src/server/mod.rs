//! An application-owned filesystem monitor serving Git's native IPC protocol.
//!
//! [`spawn()`](crate::server::spawn) registers watches before returning. Keep the returned
//! [`Handle`](crate::server::Handle) alive while
//! clients use `core.fsmonitor=true`; each worktree has its own endpoint and token history.
//! Shutting down releases watches and the endpoint. Restarting requires a fresh client baseline.
//! Clients may also stop the service with Git's native `quit` request.

use std::{
    io,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use gix_error::{ErrorExt, Result, ResultExt, bail, message};
use gix_notify::{Batch, Budget, Event, Fence, PathKind, SynchronizeError, Watch, Watcher};
use gix_parallel::OwnShared;

use crate::protocol;

mod cookie;
pub(crate) mod filesystem;
/// Probe and control an existing native IPC service, without starting another process.
pub mod ipc;
mod journal;
#[allow(unsafe_code)]
mod transport;

/// One stream of raw filesystem notifications and same-stream delivery fences.
///
/// Sources stay on the server thread and need not be [`Send`].
/// Source failures, unknown event locations, and lost coverage cannot certify incremental results.
trait EventSource {
    fn close(self) -> Result
    where
        Self: Sized,
    {
        drop(self);
        Ok(())
    }

    /// Install all `watches` on this source's single stream before reporting success.
    ///
    /// The set includes the worktree root, synchronization cookies, and parent sentinels. It may
    /// include a worktree root already owned by the source; additional watches must use the same stream.
    /// The server calls this before readiness and again to repair coverage. Repeated registration must
    /// preserve generation and sequence continuity while coverage remains intact. Coverage changes or
    /// restarts may invalidate the generation, but must report any coverage gap as loss rather than
    /// silently replacing the source with an unrelated watcher. Repeating a healthy set may be a no-op.
    fn register(&mut self, watches: &[Watch]) -> Result<()>;

    /// Consume a batch bounded by `budget` without blocking, preserving loss, diagnostics, generation, and sequence.
    ///
    /// Return raw events for all watched paths, including ignored files, administrative paths, cookies,
    /// and parent sentinels. Do not filter these through repository scopes or maintenance outcomes:
    /// the server performs path conversion and invalidation. An empty batch does not certify delivery.
    fn drain(&mut self, budget: Budget) -> Batch;

    /// Fence preceding writes using `directory`, which is covered by this same stream, before `deadline`.
    ///
    /// A successful fence must certify publication of the complete native batch containing the marker.
    /// The server drains through its sequence in the same generation; failures, intervening loss, or
    /// unsupported synchronization require full verification unless the source supports the
    /// server's cookie fallback. Never certify using a separate watcher.
    fn synchronize_at(&mut self, directory: &Path, deadline: Instant) -> std::result::Result<Fence, SynchronizeError>;

    /// Whether observing a server-created cookie can synchronize worktree notifications when
    /// [`synchronize_at()`](Self::synchronize_at) returns [`SynchronizeError::Unsupported`].
    ///
    /// This is an opt-in for Git-style cookie synchronization, used by the native Windows source.
    /// The recursive worktree watch must include cookies within the worktree. For external cookie
    /// directories, publish all ready worktree completions, including those obtained by rearming
    /// native reads, before exposing a cookie. Publish complete native batches atomically and report
    /// loss if that ordering cannot be maintained. Arbitrary unordered sources must leave this false.
    fn supports_cookie_synchronization(&self) -> bool {
        false
    }

    /// Install a lightweight `wake` callback for the server when events, loss, or diagnostics arrive.
    /// It may run on a backend thread and may be called spuriously; it must not run under a queue lock.
    fn set_waker(&mut self, wake: Box<dyn Fn() + Send + Sync + 'static>);
}

impl EventSource for Watcher {
    fn register(&mut self, watches: &[Watch]) -> Result<()> {
        self.replace(watches.iter().cloned())
    }

    fn drain(&mut self, budget: Budget) -> Batch {
        Watcher::drain(self, budget)
    }

    fn synchronize_at(&mut self, directory: &Path, deadline: Instant) -> std::result::Result<Fence, SynchronizeError> {
        Watcher::synchronize_at(self, directory, deadline)
    }

    fn supports_cookie_synchronization(&self) -> bool {
        cfg!(windows)
    }

    fn set_waker(&mut self, wake: Box<dyn Fn() + Send + Sync + 'static>) {
        Watcher::set_waker(self, wake);
    }
}

/// Owned repository context, prepared by the application after discovery and configuration.
/// All paths must be absolute; roots and the endpoint's parent are resolved to their native filesystem
/// spelling at startup. The endpoint itself need not exist.
#[derive(Debug, Clone)]
pub struct Context {
    /// The worktree whose changes the server reports, independently of ignore rules or an index.
    pub worktree: PathBuf,
    /// This worktree's private Git directory, used for administrative synchronization markers.
    pub git_dir: PathBuf,
    /// The shared Git directory, which may differ from `git_dir` for linked worktrees.
    pub common_dir: PathBuf,
    /// The native IPC endpoint, normally `<git_dir>/fsmonitor--daemon.ipc`.
    /// Windows derives Git's named-pipe name from this path.
    pub endpoint: PathBuf,
    /// Whether reported Git paths use composed Unicode, from `core.precomposeUnicode`.
    pub precompose_unicode: bool,
    /// Whether matching filesystem paths ignores ASCII case, from `core.ignoreCase`.
    pub ignore_case: bool,
}

/// Configure filesystem eligibility, lifetime, and operation timings of an application-owned server.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// Permit monitoring remote worktrees. Defaults to false, like `fsmonitor.allowRemote`.
    /// Filesystem-probe errors still fail startup, and incomplete notifications still require full verification.
    pub allow_remote: bool,
    /// Stop after this much time without a client connection, or stay running with `None` (the default).
    /// The duration must be positive. Completed connections, including status probes, restart the timer;
    /// filesystem events do not. An active exchange finishes before idle shutdown is considered.
    pub idle_timeout: Option<Duration>,
    /// Maximum wait for a native delivery fence or synchronization cookie. Defaults to one second.
    /// An incomplete fence requires a full scan.
    pub fence_timeout: Duration,
    /// Delay before retrying failed watch registration or synchronization. Defaults to five seconds.
    pub retry_interval: Duration,
    /// Maximum polling interval for synchronization cookies. Defaults to 20 milliseconds.
    /// IPC clients, source notifications, and shutdown requests wake the server directly.
    pub idle_interval: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            allow_remote: false,
            idle_timeout: None,
            fence_timeout: Duration::from_secs(1),
            retry_interval: Duration::from_secs(5),
            idle_interval: Duration::from_millis(20),
        }
    }
}

/// Ownership of a server thread, its native watches, and its IPC endpoint.
///
/// Dropping the handle requests shutdown and joins the thread. Prefer explicit [`shutdown()`](Self::shutdown)
/// to observe errors. Joining can wait for an active client exchange or native synchronization;
/// perform it away from an application's UI thread.
#[must_use = "dropping the handle shuts the server down"]
pub struct Handle {
    endpoint: PathBuf,
    shutdown: OwnShared<AtomicBool>,
    wake: OwnShared<transport::Wake>,
    worker: Option<JoinHandle<Result<()>>>,
}

impl Handle {
    /// The endpoint owned by this server, typically `<git_dir>/fsmonitor--daemon.ipc`.
    pub fn endpoint(&self) -> &Path {
        &self.endpoint
    }

    /// Inspect whether the server thread has exited without waiting or consuming its result.
    /// Call [`wait()`](Self::wait) to join it and observe errors after completion.
    pub fn is_finished(&self) -> bool {
        self.worker.as_ref().is_none_or(JoinHandle::is_finished)
    }

    /// Signal this server instance to stop, without waiting for it. Repeated requests are harmless.
    /// This does not send IPC commands and cannot stop a replacement server at the same endpoint.
    pub fn request_shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
        self.wake.wake();
        if let Some(worker) = &self.worker {
            worker.thread().unpark();
        }
    }

    /// Wait until this server exits and all native watches and the endpoint have been released.
    /// It may exit after [`request_shutdown()`](Self::request_shutdown), an IPC `quit`, an idle timeout, or an error.
    pub fn wait(mut self) -> Result<()> {
        self.join()
    }

    /// Request shutdown and wait for cleanup, returning any error from the server thread.
    pub fn shutdown(self) -> Result<()> {
        self.request_shutdown();
        self.wait()
    }

    fn join(&mut self) -> Result<()> {
        match self.worker.take() {
            Some(worker) => worker
                .join()
                .map_err(|_| message("filesystem monitor thread panicked").raise())?,
            None => Ok(()),
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.request_shutdown();
        if let Err(_err) = self.join() {
            gix_trace::warn!(error = %_err, "filesystem monitor stopped with an error");
        }
    }
}

/// Start a server on its own thread and return only after binding and watch registration succeed.
///
/// An occupied endpoint is an error; applications can use the existing service instead.
/// Failed startup releases resources before returning. The worker uses repository `context`
/// and resolves filesystem paths independently of the process working directory.
/// The `options` control synchronization, recovery, and idle behavior; [`Options::default()`]
/// disables idle shutdown.
pub fn spawn(context: Context, options: Options) -> Result<Handle> {
    spawn_with_source(context, options, |_| Watcher::new(Default::default()))
}

/// Like [`spawn()`] with `context` and `options`, using the notification stream from `make_source`.
///
/// The factory runs on the server worker thread after validating and normalizing `context` paths.
/// Only the factory must be [`Send`]; its source is created, used, and dropped on that worker thread.
/// The server uses this source exclusively, without constructing an independent native watcher.
/// Readiness requires endpoint binding, cookie directory preparation, and successful registration of
/// the full watch set. Factory or registration failure releases resources before returning an error.
fn spawn_with_source<S: EventSource + 'static>(
    context: Context,
    options: Options,
    make_source: impl FnOnce(&Context) -> Result<S> + Send + 'static,
) -> Result<Handle> {
    if options.idle_timeout.is_some_and(|timeout| timeout.is_zero()) {
        bail!(message("filesystem monitor idle timeout must be positive"));
    }
    let endpoint = context.endpoint.clone();
    let shutdown = OwnShared::new(AtomicBool::new(false));
    let stop = OwnShared::clone(&shutdown);
    let wake =
        OwnShared::new(transport::Wake::new().or_raise(|| message("creating the filesystem monitor wake signal"))?);
    let worker_wake = OwnShared::clone(&wake);
    let (ready, receive) = mpsc::sync_channel(1);
    let worker = std::thread::Builder::new()
        .name("gix-fsmonitor".into())
        .spawn(gix_trace::in_thread(move || {
            let context = context.normalize()?;
            context.check_worktree(options.allow_remote)?;
            let listener = transport::Listener::bind(&context.endpoint)
                .or_raise(|| message("acquiring the native IPC endpoint"))?;
            let mut source = make_source(&context)?;
            let thread = std::thread::current();
            let source_wake = OwnShared::clone(&worker_wake);
            source.set_waker(Box::new(move || {
                thread.unpark();
                source_wake.wake();
            }));
            let monitor = Monitor::new(Layout::from_context(context), source, options)?;
            ready
                .send(monitor.layout.endpoint.clone())
                .or_raise(|| message("reporting filesystem monitor readiness"))?;
            run(listener, monitor, &stop, options, &worker_wake)
        }))
        .or_raise(|| message("starting the filesystem monitor thread"))?;
    let mut handle = Handle {
        endpoint,
        shutdown,
        wake,
        worker: Some(worker),
    };
    match receive.recv() {
        Ok(endpoint) => handle.endpoint = endpoint,
        Err(_) => {
            handle.wait()?;
            bail!(message("filesystem monitor exited before reporting readiness"));
        }
    }
    Ok(handle)
}

#[derive(Clone)]
struct Layout {
    worktree: PathBuf,
    administrative: Vec<PathBuf>,
    cookie_dir: PathBuf,
    endpoint: PathBuf,
    precompose_unicode: bool,
    ignore_case: bool,
}

impl Context {
    pub(crate) fn check_worktree(&self, allow_remote: bool) -> Result<()> {
        check_worktree(&self.worktree, allow_remote, filesystem::is_remote)
    }

    fn normalize(self) -> Result<Self> {
        for (field, path) in [
            ("worktree", &self.worktree),
            ("git_dir", &self.git_dir),
            ("common_dir", &self.common_dir),
            ("endpoint", &self.endpoint),
        ] {
            if !path.is_absolute() {
                bail!(message!(
                    "filesystem monitor {field} must be absolute: {}",
                    path.display()
                ));
            }
        }
        // Native watchers need the stored case and Unicode spelling, including when callers use aliases.
        let worktree = self
            .worktree
            .canonicalize()
            .or_raise(|| message("resolving the monitored worktree"))?;
        let git_dir = self
            .git_dir
            .canonicalize()
            .or_raise(|| message("resolving the private Git directory"))?;
        let common_dir = self
            .common_dir
            .canonicalize()
            .or_raise(|| message("resolving the shared Git directory"))?;
        for path in [&worktree, &git_dir, &common_dir] {
            if !path.is_dir() {
                bail!(message!(
                    "filesystem monitor root must be a directory: {}",
                    path.display()
                ));
            }
        }
        let endpoint = crate::normalize_endpoint(&self.endpoint)
            .or_raise(|| message("resolving the native IPC endpoint directory"))?;
        Ok(Self {
            worktree,
            git_dir,
            common_dir,
            endpoint,
            ..self
        })
    }
}

fn check_worktree(
    worktree: &Path,
    allow_remote: bool,
    is_remote: impl FnOnce(&Path) -> io::Result<bool>,
) -> Result<()> {
    if is_remote(worktree).or_raise(|| {
        message!(
            "determining whether worktree '{}' is on a remote filesystem",
            worktree.display()
        )
    })? && !allow_remote
    {
        bail!(message!(
            "remote worktree '{}' requires fsmonitor.allowRemote=true",
            worktree.display()
        ));
    }
    Ok(())
}

impl Layout {
    fn from_context(context: Context) -> Self {
        let cookie_dir = context.git_dir.join("fsmonitor--daemon/cookies");
        let administrative = vec![context.git_dir, context.common_dir, context.worktree.join(".git")];
        Self {
            worktree: context.worktree,
            administrative,
            cookie_dir,
            endpoint: context.endpoint,
            precompose_unicode: context.precompose_unicode,
            ignore_case: context.ignore_case,
        }
    }

    fn relative_to(&self, path: &Path, root: &Path) -> Option<PathBuf> {
        let mut components = path.components();
        for prefix in root.components() {
            let component = components.next()?;
            let (a, b) = (
                component.as_os_str().as_encoded_bytes(),
                prefix.as_os_str().as_encoded_bytes(),
            );
            if !(a == b || self.ignore_case && a.eq_ignore_ascii_case(b)) {
                return None;
            }
        }
        Some(components.as_path().to_owned())
    }

    fn roots(&self) -> impl Iterator<Item = &Path> {
        std::iter::once(self.worktree.as_path()).chain(self.administrative.iter().map(PathBuf::as_path))
    }

    fn root_lost(&self, event: &Event) -> bool {
        matches!(
            event.kind,
            gix_notify::EventKind::Remove | gix_notify::EventKind::Rename
        ) && event
            .paths
            .iter()
            .any(|path| self.roots().any(|root| self.relative_to(root, path).is_some()))
    }

    fn paths(&self, event: Event) -> Option<Vec<Vec<u8>>> {
        if event.paths.is_empty() {
            return None;
        }
        let mut paths = Vec::new();
        for path in event.paths {
            if !path.is_absolute() {
                return None;
            }
            if self
                .administrative
                .iter()
                .any(|root| self.relative_to(&path, root).is_some())
            {
                continue;
            }
            let Some(relative) = self.relative_to(&path, &self.worktree) else {
                continue;
            };
            if relative.as_os_str().is_empty() {
                // Windows parent sentinels report root metadata changes alongside child events.
                if cfg!(windows) && event.kind == gix_notify::EventKind::Modify {
                    continue;
                }
                return None;
            }
            let path = gix_path::to_unix_separators_on_windows(gix_path::into_bstr(relative).ok()?);
            let path = if self.precompose_unicode {
                gix_utils::str::precompose_bstr(path)
            } else {
                path
            };
            let path = path.into_owned().to_vec();
            if path.contains(&0) {
                return None;
            }
            if event.path_kind != PathKind::Directory {
                paths.push(path.clone());
            }
            if event.path_kind != PathKind::File {
                let mut directory = path;
                directory.push(b'/');
                paths.push(directory);
            }
        }
        Some(paths)
    }

    fn watches(&self) -> Vec<Watch> {
        let mut watches = vec![
            Watch {
                path: self.worktree.clone(),
                recursive: true,
            },
            Watch {
                path: self.cookie_dir.clone(),
                recursive: true,
            },
        ];
        // Parent sentinels detect root replacement even when the original inode lost coverage.
        for root in self.roots() {
            if let Some(parent) = root.parent() {
                watches.push(Watch {
                    path: parent.to_owned(),
                    recursive: false,
                });
            }
        }
        watches
    }

    fn prepare_cookies(&self) -> io::Result<()> {
        // Do not create a missing Git directory if it disappeared during startup or recovery.
        for path in [
            self.cookie_dir
                .parent()
                .expect("cookie directory has an administrative parent"),
            &self.cookie_dir,
        ] {
            match std::fs::create_dir(path) {
                Ok(()) => {}
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists && path.is_dir() => {}
                Err(err) => return Err(err),
            }
        }
        Ok(())
    }
}

mod blocking {
    use super::*;
    use gix_macros::sync as maybe_async;

    fn prepare_cookies<S>(_source: &S, layout: &Layout) -> io::Result<()> {
        layout.prepare_cookies()
    }
    fn create_cookie<S>(_source: &S, directory: &Path) -> io::Result<cookie::Cookie> {
        cookie::Cookie::create(directory)
    }
    fn roots_lost<S>(_source: &S, layout: &Layout) -> io::Result<bool> {
        Ok(layout.roots().any(|root| !root.exists()))
    }
    fn wait_for_cookie<S>(_source: &S, remaining: Duration, interval: Duration) {
        std::thread::park_timeout(remaining.min(interval));
    }
    include!("monitor.rs");
}
use blocking::Monitor;

#[cfg(feature = "async")]
/// Tokio orchestration using the shared notification interpretation and token journal.
pub mod asynchronous;
mod state;

fn run<S: EventSource>(
    mut listener: transport::Listener,
    mut monitor: Monitor<S>,
    shutdown: &AtomicBool,
    options: Options,
    wake: &transport::Wake,
) -> Result<()> {
    let mut last_activity = Instant::now();
    while !shutdown.load(Ordering::Relaxed) {
        let more = monitor.maintenance(Instant::now());
        if monitor.root_lost || !listener.owns_endpoint() {
            bail!(message(
                "repository root or filesystem monitor endpoint was removed or replaced"
            ));
        }
        let mut stream = match listener.accept() {
            Ok(Some(stream)) => stream,
            Ok(None) => {
                if options
                    .idle_timeout
                    .is_some_and(|timeout| last_activity.elapsed() >= timeout)
                {
                    return Ok(());
                }
                if !more {
                    // Like Git, periodically check ownership even if no native event arrives.
                    let mut timeout = Duration::from_secs(60);
                    if let Some(retry) = monitor.retry {
                        timeout = timeout.min(retry.saturating_duration_since(Instant::now()));
                    }
                    if let Some(idle_timeout) = options.idle_timeout {
                        timeout = timeout.min(idle_timeout.saturating_sub(last_activity.elapsed()));
                    }
                    listener
                        .wait(wake, timeout)
                        .or_raise(|| message("waiting for filesystem monitor activity"))?;
                }
                continue;
            }
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(err).or_raise(|| message("accepting an IPC client")),
        };
        let request = protocol::read_message(&mut stream, 16384);
        last_activity = Instant::now();
        let request = match request {
            Ok(Some(request)) if !request.contains(&0) => request,
            Ok(None) => continue, // Git's native status probe only connects and closes.
            Ok(Some(_)) | Err(_) => continue,
        };
        if request == b"quit" {
            monitor.close()?;
            let _ = protocol::write_message(&mut stream, &[]);
            let _ = stream.finish();
            return Ok(());
        }
        if request == b"flush" {
            monitor.journal.reset();
        }
        let response = monitor.response(&request, Instant::now());
        if protocol::write_message(&mut stream, &response).is_ok() {
            let _ = stream.finish();
        }
        last_activity = Instant::now();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    mod cookies;
    mod source;

    use super::*;
    use gix_notify::EventKind;

    #[test]
    fn remote_worktree_permission_does_not_override_probe_errors() -> gix_error::TestResult {
        let worktree = Path::new("worktree");
        for allow_remote in [false, true] {
            check_worktree(worktree, allow_remote, |path| {
                assert_eq!(
                    path, worktree,
                    "eligibility probes the worktree, not an administrative directory"
                );
                Ok(false)
            })?;
            assert!(
                check_worktree(worktree, allow_remote, |_| Err(io::Error::other("probe failed"))).is_err(),
                "filesystem errors remain failures even when remote monitoring is allowed"
            );
        }
        check_worktree(worktree, true, |_| Ok(true))?;
        let error =
            check_worktree(worktree, false, |_| Ok(true)).expect_err("remote worktrees require explicit permission");
        assert!(
            error.to_string().contains("fsmonitor.allowRemote=true"),
            "remote rejection explains how to opt in: {error}"
        );
        Ok(())
    }

    #[test]
    fn idle_maintenance_preserves_history_until_loss_is_reported() -> gix_error::TestResult {
        let directory = gix_testtools::tempfile::tempdir()?;
        let root = directory.path().canonicalize()?.join("worktree");
        let git_dir = root.join(".git");
        std::fs::create_dir_all(&git_dir)?;
        let options = Options {
            retry_interval: Duration::from_secs(7),
            ..Default::default()
        };
        let mut monitor = Monitor::new(
            Layout {
                administrative: vec![git_dir.clone()],
                endpoint: git_dir.join("socket"),
                cookie_dir: git_dir.join("fsmonitor--daemon/cookies"),
                worktree: root.clone(),
                precompose_unicode: false,
                ignore_case: false,
            },
            Watcher::new(Default::default())?,
            options,
        )?;
        // Control time and loss directly, without racing native callbacks from fixture setup.
        monitor.watcher = Watcher::new(Default::default())?;
        let now = Instant::now();
        monitor.maintenance(now);
        let initial = monitor.journal.response(b"", true, now);
        let token = initial
            .split(|byte| *byte == 0)
            .next()
            .expect("responses begin with a token");
        monitor.journal.record(b"changed".to_vec(), now);
        let later = now + Duration::from_secs(120);
        monitor.maintenance(later);
        assert_eq!(
            monitor
                .journal
                .response(token, true, later)
                .splitn(2, |byte| *byte == 0)
                .nth(1),
            Some(b"changed\0".as_slice()),
            "retained changes remain available to the client while idle"
        );
        assert!(monitor.retry.is_none(), "idle time does not request watch registration");

        std::fs::remove_dir_all(root)?;
        let mut failure = monitor.watcher.drain(Budget::default());
        failure.error = Some(message("native root identity check failed").raise());
        monitor.ingest(failure, later);
        assert!(
            monitor.root_lost && !monitor.healthy,
            "a pathless native error rechecks roots so an obsolete server can stop"
        );
        assert_eq!(
            monitor.retry,
            Some(later + options.retry_interval),
            "native failures schedule recovery with the configured retry interval"
        );
        assert!(
            monitor.response(token, later).ends_with(b"\0/\0"),
            "native failure still requires full verification"
        );
        Ok(())
    }

    #[test]
    fn complete_coverage_keeps_ignored_candidates_and_directory_ambiguity() {
        let root = PathBuf::from(if cfg!(windows) { "C:/work" } else { "/work" });
        let layout = Layout {
            administrative: vec![root.join(".git")],
            endpoint: root.join(".git/socket"),
            cookie_dir: root.join(".git/fsmonitor--daemon/cookies"),
            worktree: root.clone(),
            precompose_unicode: false,
            ignore_case: true,
        };
        let paths = layout.paths(Event {
            paths: vec![root.join("ignored/target/file"), root.join(".git/index")],
            kind: EventKind::Modify,
            path_kind: PathKind::Any,
        });
        assert_eq!(
            paths,
            Some(vec![b"ignored/target/file".to_vec(), b"ignored/target/file/".to_vec()]),
            "provider coverage ignores Git ignore rules and excludes only administrative files"
        );
        #[cfg(windows)]
        assert_eq!(
            layout.paths(Event {
                paths: vec![root.clone(), root.join("created")],
                kind: EventKind::Modify,
                path_kind: PathKind::Any,
            }),
            Some(vec![b"created".to_vec(), b"created/".to_vec()]),
            "Windows root metadata changes preserve the incremental child candidates"
        );
        assert!(
            layout
                .paths(Event {
                    paths: vec![root],
                    kind: EventKind::Any,
                    path_kind: PathKind::Directory
                })
                .is_none(),
            "root-level directory events require full invalidation"
        );
    }
    #[cfg(unix)]
    #[test]
    fn git_path_conversion_preserves_non_utf8_bytes_and_rejects_unknown_paths() {
        use std::os::unix::ffi::OsStrExt;
        let root = PathBuf::from("/work");
        let layout = Layout {
            administrative: vec![root.join(".git")],
            endpoint: root.join(".git/socket"),
            cookie_dir: root.join(".git/fsmonitor--daemon/cookies"),
            worktree: root.clone(),
            precompose_unicode: true,
            ignore_case: false,
        };
        let path = std::ffi::OsStr::from_bytes(b"raw-\xff");
        assert_eq!(
            layout.paths(Event {
                paths: vec![root.join(path)],
                kind: EventKind::Modify,
                path_kind: PathKind::File
            }),
            Some(vec![b"raw-\xff".to_vec()]),
            "Git's Unix path conversion preserves non-UTF-8 names"
        );
        for paths in [Vec::new(), vec![PathBuf::new()]] {
            assert!(
                layout
                    .paths(Event {
                        paths,
                        kind: EventKind::Any,
                        path_kind: PathKind::Any
                    })
                    .is_none(),
                "an unlocated event cannot certify an empty change list"
            );
        }
        #[cfg(target_os = "macos")]
        {
            let mut layout = layout;
            for (precompose_unicode, expected) in [(true, "ä"), (false, "a\u{308}")] {
                layout.precompose_unicode = precompose_unicode;
                assert_eq!(
                    layout.paths(Event {
                        paths: vec![root.join("a\u{308}")],
                        kind: EventKind::Modify,
                        path_kind: PathKind::File
                    }),
                    Some(vec![expected.as_bytes().to_vec()]),
                    "Unicode composition follows repository configuration independently of native event delivery"
                );
            }
        }
    }
}
