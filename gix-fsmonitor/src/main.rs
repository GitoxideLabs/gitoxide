//! Filesystem change provider for Git's hook v2 and native Simple IPC clients.

use std::{
    ffi::OsString,
    io::{self, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[cfg(unix)]
use std::process::{Child, Command, Stdio};

use anyhow::{Context, bail};
use gix_notify::{Budget, Event, PathKind, Watch, Watcher};

mod endpoint;
mod journal;
#[cfg(windows)]
mod launch;
mod protocol;
mod transport;

const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(5);
const FENCE_TIMEOUT: Duration = Duration::from_secs(1);
const RETRY_INTERVAL: Duration = Duration::from_secs(5);
const SAFETY_INTERVAL: Duration = Duration::from_secs(60);
const IDLE_INTERVAL: Duration = Duration::from_millis(20);
const MAX_REPLY: usize = 32 * 1024 * 1024;

struct Layout {
    worktree: PathBuf,
    administrative: Vec<PathBuf>,
    cookie_dir: PathBuf,
    endpoint: PathBuf,
    precompose_unicode: bool,
    ignore_case: bool,
}

impl Layout {
    fn open(path: &Path) -> anyhow::Result<Self> {
        let repo = gix::discover(path).context("opening the repository for filesystem monitoring")?;
        // Match the native watcher's pathname namespace, including the stored case and Unicode
        // spelling on macOS. Resolving only symlinks can preserve an alias that FSEvents never emits.
        let worktree = repo
            .workdir()
            .context("filesystem monitoring requires a worktree")?
            .canonicalize()?;
        let endpoint = endpoint::for_repository(&repo, &worktree)?;
        let git_dir = repo.git_dir().canonicalize()?;
        let cookie_dir = git_dir.join("fsmonitor--daemon/cookies");
        let administrative = vec![git_dir, repo.common_dir().canonicalize()?, worktree.join(".git")];
        let capabilities = repo.filesystem_options().context("reading filesystem capabilities")?;
        Ok(Self {
            worktree,
            administrative,
            cookie_dir,
            endpoint,
            precompose_unicode: capabilities.precompose_unicode,
            ignore_case: capabilities.ignore_case,
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
            let path = gix::path::to_unix_separators_on_windows(gix::path::try_into_bstr(relative).ok()?);
            let path = if self.precompose_unicode {
                gix::utils::str::precompose_bstr(path)
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
    fn new(layout: Layout) -> anyhow::Result<Self> {
        let mut watcher = Watcher::new(Default::default()).map_err(gix::Exn::into_error)?;
        let thread = std::thread::current();
        watcher.set_waker(move || thread.unpark());
        let now = Instant::now();
        let mut monitor = Self {
            watcher,
            layout,
            journal: journal::Journal::new(16384, 16 * 1024 * 1024, Duration::from_secs(300))
                .context("generating a fresh filesystem monitor token epoch")?,
            generation: None,
            next_safety: now + SAFETY_INTERVAL,
            retry: None,
            healthy: false,
            root_lost: false,
        };
        monitor.repair(now);
        Ok(monitor)
    }

    fn repair(&mut self, now: Instant) {
        let result = self
            .layout
            .prepare_cookies()
            .context("preparing the native synchronization directory")
            .and_then(|()| {
                self.watcher
                    .replace(self.layout.watches())
                    .map_err(|err| anyhow::Error::from(err.into_error()))
                    .context("registering complete worktree coverage")
            });
        match result {
            Ok(()) => {
                self.healthy = true;
                self.retry = None;
            }
            Err(err) => {
                eprintln!("filesystem monitoring will retry: {err:#}");
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
        for error in batch.errors {
            eprintln!("filesystem notification error: {}", error.into_error());
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
                Err(error) => {
                    eprintln!("filesystem synchronization requires a full response: {error}");
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

fn run(layout: Layout) -> anyhow::Result<()> {
    // Bind before constructing watcher threads: Unix long-path handling temporarily changes CWD.
    let mut listener = transport::Listener::bind(&layout.endpoint).context("acquiring the native IPC endpoint")?;
    let mut monitor = Monitor::new(layout)?;
    loop {
        let more = monitor.maintenance(Instant::now());
        if monitor.root_lost || !listener.owns_endpoint() {
            bail!("repository root or filesystem monitor endpoint was removed or replaced");
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
            Err(err) => return Err(err).context("accepting an IPC client"),
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
}

fn query(endpoint: &Path, request: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut stream = transport::connect(endpoint).context("connecting to the filesystem monitor")?;
    protocol::write_message(&mut stream, request).context("sending an IPC query")?;
    protocol::read_message(&mut stream, MAX_REPLY)?.context("daemon closed without a response")
}

#[cfg(unix)]
fn spawn(repo: &Path) -> anyhow::Result<Child> {
    use std::os::unix::process::CommandExt;

    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg("run")
        .arg("--repo")
        .arg(repo)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // SAFETY: Only the async-signal-safe setsid system call runs between fork and exec.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
    command.spawn().context("starting the filesystem monitor")
}

#[cfg(windows)]
fn spawn(repo: &Path) -> anyhow::Result<launch::Child> {
    launch::spawn(repo).context("starting the filesystem monitor")
}

fn start(layout: &Layout) -> anyhow::Result<()> {
    if transport::connect(&layout.endpoint).is_ok() {
        bail!("a daemon already owns the native IPC endpoint");
    }
    let mut child = spawn(&layout.worktree)?;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait()? {
            bail!("filesystem monitor exited during startup: {status}");
        }
        if query(&layout.endpoint, b"").is_ok() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            bail!("filesystem monitor did not become ready before the startup deadline");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn main() -> anyhow::Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let Some(command) = arguments.next() else {
        bail!("usage: gix-fsmonitor <run|start|stop|status|flush|query|hook|path> [--repo PATH]");
    };
    if command == "--help" || command == "-h" {
        println!("gix-fsmonitor <run|start|stop|status|flush|query [TOKEN]|hook 2 TOKEN|path> [--repo PATH]");
        return Ok(());
    }
    let mut repo = PathBuf::from(".");
    let mut positional = Vec::<OsString>::new();
    while let Some(argument) = arguments.next() {
        if argument == "--repo" {
            repo = arguments.next().context("--repo needs a path")?.into();
        } else {
            positional.push(argument);
        }
    }
    let layout = Layout::open(&repo)?;
    match command.to_str() {
        Some("run") if positional.is_empty() => run(layout),
        Some("start") if positional.is_empty() => start(&layout),
        Some("status") if positional.is_empty() => {
            transport::connect(&layout.endpoint).context("filesystem monitor is not running")?;
            Ok(())
        }
        Some("path") if positional.is_empty() => {
            println!("{}", layout.endpoint.display());
            Ok(())
        }
        Some("stop") if positional.is_empty() => {
            query(&layout.endpoint, b"quit")?;
            let deadline = Instant::now() + EXCHANGE_TIMEOUT;
            while transport::connect(&layout.endpoint).is_ok() {
                if Instant::now() >= deadline {
                    bail!("filesystem monitor did not stop");
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok(())
        }
        Some("flush") if positional.is_empty() => {
            io::stdout().write_all(&query(&layout.endpoint, b"flush")?)?;
            Ok(())
        }
        Some("query") if positional.len() <= 1 => {
            let token = positional
                .first()
                .map(|value| value.as_encoded_bytes())
                .unwrap_or_default();
            io::stdout().write_all(&query(&layout.endpoint, token)?)?;
            Ok(())
        }
        Some("hook") if positional.len() == 2 && positional[0] == "2" => {
            let token = positional[1].as_encoded_bytes();
            let response = match query(&layout.endpoint, token) {
                Ok(response) => response,
                Err(_) => {
                    // An autostart race may be won by either compatible daemon; query the winner.
                    let _ = start(&layout);
                    query(&layout.endpoint, token)?
                }
            };
            io::stdout().write_all(&response)?;
            Ok(())
        }
        _ => bail!("invalid command or arguments; use --help"),
    }
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
    }
}
