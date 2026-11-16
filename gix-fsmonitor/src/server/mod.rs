//! An application-owned filesystem monitor serving Git's native IPC protocol.
//!
//! [`endpoint()`](crate::server::endpoint) selects Git's native IPC endpoint from caller-resolved repository paths.
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
mod endpoint;
pub(crate) mod filesystem;
/// Probe and control an existing native IPC service, without starting another process.
pub mod ipc;
mod journal;
#[allow(unsafe_code)]
mod transport;

pub use endpoint::{endpoint, endpoint_for_canonical_paths};

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
    /// The native IPC endpoint selected by [`endpoint()`], normally `<git_dir>/fsmonitor--daemon.ipc`.
    /// Windows derives Git's named-pipe name from this path.
    pub endpoint: PathBuf,
    /// Whether reported Git paths use composed Unicode, from `core.precomposeUnicode`.
    pub precompose_unicode: bool,
    /// Whether matching filesystem paths ignores ASCII case, from `core.ignoreCase`.
    pub ignore_case: bool,
}

/// Configure filesystem eligibility, lifetime, and operation timings of an application-owned server.
/// Fence and retry durations must fit the platform's monotonic clock range; startup rejects overflow.
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

impl Options {
    fn validate(&self) -> Result<()> {
        if self.idle_timeout.is_some_and(|timeout| timeout.is_zero()) {
            bail!(message("filesystem monitor idle timeout must be positive"));
        }
        let now = Instant::now();
        for (name, duration) in [
            ("fence timeout", self.fence_timeout),
            ("retry interval", self.retry_interval),
        ] {
            if now.checked_add(duration).is_none() {
                bail!(message!("filesystem monitor {name} exceeds the supported clock range").validation());
            }
        }
        Ok(())
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
    /// Use it with [`ipc::query()`] to query or control this server.
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
    options.validate()?;
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
                if cfg!(windows)
                    && event.kind == gix_notify::EventKind::Modify
                    && event.path_kind != PathKind::Directory
                {
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

mod blocking;
use blocking::Monitor;

#[cfg(feature = "async-tokio")]
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
mod tests;
