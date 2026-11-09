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
use gix_notify::{Budget, Event, PathKind, Watch, Watcher};
use gix_parallel::OwnShared;

/// Probe and control an existing native IPC service, without starting another process.
pub mod ipc;
mod journal;
mod protocol;
#[allow(unsafe_code)]
mod transport;

const FENCE_TIMEOUT: Duration = Duration::from_secs(1);
const RETRY_INTERVAL: Duration = Duration::from_secs(5);
const SAFETY_INTERVAL: Duration = Duration::from_secs(60);
const IDLE_INTERVAL: Duration = Duration::from_millis(20);

/// Owned repository context, prepared by the application after discovery and configuration.
/// All paths must be absolute; roots are resolved to their native filesystem spelling at startup.
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

/// Ownership of a server thread, its native watches, and its IPC endpoint.
///
/// Dropping the handle requests shutdown and joins the thread. Prefer explicit [`shutdown()`](Self::shutdown)
/// to observe errors. Joining can wait for an active client exchange or native synchronization;
/// perform it away from an application's UI thread.
#[must_use = "dropping the handle shuts the server down"]
pub struct Handle {
    endpoint: PathBuf,
    shutdown: OwnShared<AtomicBool>,
    worker: Option<JoinHandle<Result<()>>>,
}

impl Handle {
    /// The endpoint owned by this server.
    pub fn endpoint(&self) -> &Path {
        &self.endpoint
    }

    /// Signal this server instance to stop, without waiting for it. Repeated requests are harmless.
    /// This does not send IPC commands and cannot stop a replacement server at the same endpoint.
    pub fn request_shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(worker) = &self.worker {
            worker.thread().unpark();
        }
    }

    /// Wait until this server exits and all native watches and the endpoint have been released.
    /// It may exit after [`request_shutdown()`](Self::request_shutdown), an IPC `quit`, or an error.
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
/// Failed startup releases resources before returning. The worker uses caller-supplied repository
/// context and resolves filesystem paths independently of the process working directory.
pub fn spawn(context: Context) -> Result<Handle> {
    let endpoint = context.endpoint.clone();
    let shutdown = OwnShared::new(AtomicBool::new(false));
    let stop = OwnShared::clone(&shutdown);
    let (ready, receive) = mpsc::sync_channel(1);
    let worker = std::thread::Builder::new()
        .name("gix-fsmonitor".into())
        .spawn(gix_trace::in_thread(move || {
            let layout = Layout::from_context(context)?;
            let listener = transport::Listener::bind(&layout.endpoint)
                .or_raise(|| message("acquiring the native IPC endpoint"))?;
            let monitor = Monitor::new(layout)?;
            ready
                .send(())
                .or_raise(|| message("reporting filesystem monitor readiness"))?;
            run(listener, monitor, &stop)
        }))
        .or_raise(|| message("starting the filesystem monitor thread"))?;
    let handle = Handle {
        endpoint,
        shutdown,
        worker: Some(worker),
    };
    if receive.recv().is_err() {
        handle.wait()?;
        bail!(message("filesystem monitor exited before reporting readiness"));
    }
    Ok(handle)
}

struct Layout {
    worktree: PathBuf,
    administrative: Vec<PathBuf>,
    cookie_dir: PathBuf,
    endpoint: PathBuf,
    precompose_unicode: bool,
    ignore_case: bool,
}

impl Layout {
    fn from_context(context: Context) -> Result<Self> {
        for (field, path) in [
            ("worktree", &context.worktree),
            ("git_dir", &context.git_dir),
            ("common_dir", &context.common_dir),
            ("endpoint", &context.endpoint),
        ] {
            if !path.is_absolute() {
                bail!(message!(
                    "filesystem monitor {field} must be absolute: {}",
                    path.display()
                ));
            }
        }
        // Native watchers need the stored case and Unicode spelling, including when callers use aliases.
        let worktree = context
            .worktree
            .canonicalize()
            .or_raise(|| message("resolving the monitored worktree"))?;
        let git_dir = context
            .git_dir
            .canonicalize()
            .or_raise(|| message("resolving the private Git directory"))?;
        let common_dir = context
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
        let cookie_dir = git_dir.join("fsmonitor--daemon/cookies");
        let administrative = vec![git_dir, common_dir, worktree.join(".git")];
        Ok(Self {
            worktree,
            administrative,
            cookie_dir,
            endpoint: context.endpoint,
            precompose_unicode: context.precompose_unicode,
            ignore_case: context.ignore_case,
        })
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

struct Monitor {
    watcher: Watcher,
    layout: Layout,
    journal: journal::Journal,
    generation: Option<u64>,
    next_safety: Instant,
    retry: Option<Instant>,
    healthy: bool,
    root_lost: bool,
}

impl Monitor {
    fn new(layout: Layout) -> Result<Self> {
        let mut watcher = Watcher::new(Default::default())?;
        let thread = std::thread::current();
        watcher.set_waker(move || thread.unpark());
        let now = Instant::now();
        let mut monitor = Self {
            watcher,
            layout,
            journal: journal::Journal::new(16384, 16 * 1024 * 1024, Duration::from_secs(300))
                .or_raise(|| message("generating a fresh filesystem monitor token epoch"))?,
            generation: None,
            next_safety: now + SAFETY_INTERVAL,
            retry: None,
            healthy: false,
            root_lost: false,
        };
        monitor.register()?;
        monitor.healthy = true;
        Ok(monitor)
    }

    fn register(&mut self) -> Result<()> {
        self.layout
            .prepare_cookies()
            .or_raise(|| message("preparing the native synchronization directory"))?;
        self.watcher
            .replace(self.layout.watches())
            .or_raise(|| message("registering complete worktree coverage"))
    }

    fn repair(&mut self, now: Instant) {
        let result = self.register();
        match result {
            Ok(()) => {
                self.healthy = true;
                self.retry = None;
            }
            Err(_err) => {
                gix_trace::warn!(error = %_err, "filesystem monitoring will retry");
                self.healthy = false;
                self.retry = Some(now + RETRY_INTERVAL);
                self.journal.reset();
            }
        }
    }

    fn ingest(&mut self, batch: gix_notify::Batch, now: Instant) {
        if self.generation != Some(batch.generation) || batch.loss.is_some() {
            self.journal.reset();
            self.generation = Some(batch.generation);
        }
        if matches!(
            batch.loss,
            Some(gix_notify::Loss::BackendError | gix_notify::Loss::BackendStopped)
        ) {
            self.healthy = false;
            self.retry.get_or_insert(now + RETRY_INTERVAL);
        }
        for _error in batch.errors {
            gix_trace::warn!(error = %_error, "filesystem notification error");
            self.healthy = false;
            self.retry.get_or_insert(now + RETRY_INTERVAL);
        }
        for event in batch.events {
            if self.layout.root_lost(&event) {
                self.root_lost = true;
                self.journal.reset();
                continue;
            }
            if event
                .paths
                .iter()
                .any(|path| self.layout.relative_to(&self.layout.cookie_dir, path).is_some())
            {
                self.journal.reset();
                self.healthy = false;
                self.retry.get_or_insert(now);
            }
            match self.layout.paths(event) {
                Some(paths) => {
                    for path in paths {
                        self.journal.record(path, now);
                    }
                }
                None => self.journal.reset(),
            }
        }
    }

    fn maintenance(&mut self, now: Instant) -> bool {
        let batch = self.watcher.drain(Budget::default());
        let more = batch.more;
        self.ingest(batch, now);
        self.journal.expire(now);
        if now >= self.next_safety {
            self.journal.reset();
            self.next_safety = now + SAFETY_INTERVAL;
            self.retry = Some(now);
            if self.layout.roots().any(|root| !root.exists()) {
                self.root_lost = true;
            }
        }
        if !self.root_lost && self.retry.is_some_and(|retry| now >= retry) {
            self.repair(now);
        }
        more
    }

    fn response(&mut self, requested: &[u8], now: Instant) -> Vec<u8> {
        self.maintenance(now);
        let deadline = Instant::now() + FENCE_TIMEOUT;
        let mut complete = false;
        if self.healthy && !self.root_lost {
            match self.watcher.synchronize_at(&self.layout.cookie_dir, deadline) {
                Ok(fence) => loop {
                    let batch = self.watcher.drain(Budget::default());
                    let generation = batch.generation;
                    let sequence = batch.sequence;
                    self.ingest(batch, now);
                    if generation != fence.generation || !self.healthy || self.root_lost {
                        break;
                    }
                    if sequence >= fence.sequence {
                        complete = true;
                        break;
                    }
                    if Instant::now() >= deadline {
                        break;
                    }
                },
                Err(gix_notify::SynchronizeError::Unsupported) => {}
                Err(_error) => {
                    gix_trace::warn!(error = %_error, "filesystem synchronization requires a full response");
                    self.healthy = false;
                    self.retry.get_or_insert(now + RETRY_INTERVAL);
                }
            }
        }
        if !complete {
            self.journal.reset();
        }
        self.journal.response(requested, complete, now)
    }

    fn close(self) {
        drop(self);
    }
}

fn run(mut listener: transport::Listener, mut monitor: Monitor, shutdown: &AtomicBool) -> Result<()> {
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
                if !more {
                    std::thread::park_timeout(IDLE_INTERVAL);
                }
                continue;
            }
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(err).or_raise(|| message("accepting an IPC client")),
        };
        let request = match protocol::read_message(&mut stream, 16384) {
            Ok(Some(request)) if !request.contains(&0) => request,
            Ok(None) => continue, // Git's native status probe only connects and closes.
            Ok(Some(_)) | Err(_) => continue,
        };
        if request == b"quit" {
            monitor.close();
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
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gix_notify::EventKind;

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
