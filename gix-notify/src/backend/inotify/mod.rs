//! One worker owns the inotify descriptor, watch topology, and marker fences. Each bounded
//! native read is published before a marker in that read can certify delivery.
use std::{
    collections::BTreeMap,
    io,
    os::{fd::OwnedFd, unix::fs::MetadataExt},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, SyncSender},
    thread::JoinHandle,
    time::Instant,
};

use gix_error::{ErrorExt, Result, ResultExt, conflict, message, validation};
use gix_parallel::OwnShared;
use inotify::{EventMask, Inotify, WatchDescriptor, WatchMask};
use rustix::event::{EventfdFlags, PollFd, PollFlags, Timespec, eventfd, poll};

use crate::{Event, EventKind, Fence, Loss, PathKind, SynchronizeError, Watch, queue};

mod mounts;

enum Command {
    Replace(Vec<Watch>, SyncSender<Result>),
    Synchronize {
        directory: PathBuf,
        generation: u64,
        deadline: Instant,
        reply: SyncSender<std::result::Result<Fence, SynchronizeError>>,
    },
}

pub(crate) struct Backend {
    commands: Option<SyncSender<Command>>,
    wake: OwnShared<OwnedFd>,
    worker: Option<JoinHandle<()>>,
}

impl Backend {
    pub(crate) fn new(watches: &[Watch], sender: queue::Sender, _options: crate::Options) -> Result<Self> {
        let wake = OwnShared::new(
            eventfd(0, EventfdFlags::NONBLOCK | EventfdFlags::CLOEXEC)
                .or_raise(|| message("could not create inotify worker wakeup"))?,
        );
        let awakened = wake.clone();
        let (commands, receive) = mpsc::sync_channel(1);
        let (ready, initialized) = mpsc::sync_channel(1);
        let watches = watches.to_vec();
        let worker = std::thread::Builder::new()
            .name("gix-notify inotify".into())
            .spawn(move || {
                let sender = OwnShared::new(sender);
                let mut stream = match Stream::new(&watches, sender.clone()) {
                    Ok(stream) => {
                        let _ = ready.send(Ok(()));
                        stream
                    }
                    Err(error) => {
                        let _ = ready.send(Err(error));
                        return;
                    }
                };
                if let Err(error) = stream.run(&receive, &awakened) {
                    sender.publish(Vec::new(), Some(Loss::BackendError), Some(error));
                }
            })
            .or_raise(|| message("could not start the inotify worker"))?;
        let out = Self {
            commands: Some(commands),
            wake,
            worker: Some(worker),
        };
        initialized
            .recv()
            .or_raise(|| conflict("inotify stopped during initialization"))??;
        Ok(out)
    }

    fn wake(&self) -> rustix::io::Result<()> {
        loop {
            match rustix::io::write(&*self.wake, &1u64.to_ne_bytes()) {
                // A saturated counter already provides a wakeup for the queued command.
                Err(rustix::io::Errno::AGAIN) => return Ok(()),
                Err(rustix::io::Errno::INTR) => continue,
                result => return result.map(|_| ()),
            }
        }
    }

    pub(crate) fn replace(&mut self, watches: &[Watch]) -> Result {
        let (reply, receive) = mpsc::sync_channel(1);
        self.commands
            .as_ref()
            .ok_or_else(|| conflict("inotify worker stopped").raise())?
            .send(Command::Replace(watches.to_vec(), reply))
            .or_raise(|| conflict("inotify stopped before replacing watches"))?;
        self.wake()
            .or_raise(|| message("could not wake inotify to replace watches"))?;
        receive
            .recv()
            .or_raise(|| conflict("inotify stopped while replacing watches"))?
    }

    pub(crate) fn synchronize_at(
        &mut self,
        directory: &Path,
        deadline: Instant,
        generation: u64,
    ) -> std::result::Result<Fence, SynchronizeError> {
        if Instant::now() >= deadline {
            return Err(SynchronizeError::TimedOut);
        }
        let (reply, receive) = mpsc::sync_channel(1);
        self.commands
            .as_ref()
            .ok_or(SynchronizeError::Stopped)?
            .try_send(Command::Synchronize {
                directory: directory.to_owned(),
                generation,
                deadline,
                reply,
            })
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => SynchronizeError::Busy,
                mpsc::TrySendError::Disconnected(_) => SynchronizeError::Stopped,
            })?;
        self.wake().map_err(|_| SynchronizeError::Stopped)?;
        receive
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .map_err(|error| match error {
                mpsc::RecvTimeoutError::Timeout => SynchronizeError::TimedOut,
                mpsc::RecvTimeoutError::Disconnected => SynchronizeError::Stopped,
            })?
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        self.commands = None;
        let _ = self.wake();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

struct Directory {
    path: PathBuf,
    identity: (u64, u64),
}

struct Stream {
    native: Inotify,
    directories: BTreeMap<WatchDescriptor, Directory>,
    paths: BTreeMap<PathBuf, WatchDescriptor>,
    logical: BTreeMap<PathBuf, bool>,
    roots: BTreeMap<PathBuf, bool>,
    mounts: mounts::Snapshot,
    sender: OwnShared<queue::Sender>,
    failed: bool,
}

impl Stream {
    fn new(watches: &[Watch], sender: OwnShared<queue::Sender>) -> Result<Self> {
        let mut logical = BTreeMap::new();
        let mut roots = BTreeMap::new();
        for watch in watches {
            let path = watch
                .path
                .canonicalize()
                .or_raise(|| message!("could not resolve watched path {}", watch.path.display()))?;
            let metadata =
                std::fs::metadata(&path).or_raise(|| message!("could not inspect watched path {}", path.display()))?;
            let recursive = metadata.is_dir() && watch.recursive;
            let directory = if metadata.is_dir() {
                path.clone()
            } else {
                path.parent()
                    .ok_or_else(|| validation("a watched file has no parent directory").raise())?
                    .to_owned()
            };
            logical
                .entry(path)
                .and_modify(|recurse| *recurse |= recursive)
                .or_insert(recursive);
            roots
                .entry(directory)
                .and_modify(|recurse| *recurse |= recursive)
                .or_insert(recursive);
        }
        let mounts = mounts::snapshot(&roots)?;
        let mut out = Self {
            native: Inotify::init().or_raise(|| message("could not create inotify instance"))?,
            directories: BTreeMap::new(),
            paths: BTreeMap::new(),
            logical,
            roots,
            mounts,
            sender,
            failed: false,
        };
        for (path, recursive) in out.roots.clone() {
            out.add_tree(&path, recursive)?;
        }
        out.verify()?;
        Ok(out)
    }

    fn run(&mut self, commands: &Receiver<Command>, wake: &OwnedFd) -> Result {
        loop {
            match commands.try_recv() {
                Ok(Command::Replace(watches, reply)) => {
                    // A new descriptor prevents queued events from an old watch descriptor being
                    // attributed to a reused descriptor in the replacement registration.
                    match Self::new(&watches, self.sender.clone()) {
                        Ok(replacement) => {
                            *self = replacement;
                            let _ = reply.send(Ok(()));
                        }
                        Err(error) => {
                            let _ = reply.send(Err(error));
                            return Ok(());
                        }
                    }
                }
                Ok(Command::Synchronize {
                    directory,
                    generation,
                    deadline,
                    reply,
                }) => {
                    let _ = reply.send(self.synchronize_at(&directory, generation, deadline));
                }
                Err(mpsc::TryRecvError::Disconnected) => return Ok(()),
                Err(mpsc::TryRecvError::Empty) => {}
            }
            let (native_ready, command_ready) = {
                let mut descriptors = [
                    PollFd::new(&self.native, PollFlags::IN),
                    PollFd::new(wake, PollFlags::IN),
                ];
                match poll(&mut descriptors, None) {
                    Err(rustix::io::Errno::INTR) => continue,
                    result => {
                        result.or_raise(|| message("could not wait for inotify events"))?;
                    }
                }
                (
                    !descriptors[0].revents().is_empty(),
                    !descriptors[1].revents().is_empty(),
                )
            };
            if command_ready {
                match rustix::io::read(wake, &mut [0; 8]) {
                    Ok(_) => {}
                    Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => {}
                    Err(error) => return Err(error.and_raise(message("could not receive inotify worker wakeup"))),
                }
            }
            // Bound work per iteration so a busy filesystem cannot starve commands or shutdown.
            if native_ready {
                self.read_batch(None)?;
            }
        }
    }

    fn add_tree(&mut self, path: &Path, recursive: bool) -> Result {
        let mut pending = vec![path.to_owned()];
        while let Some(path) = pending.pop() {
            if !self.paths.contains_key(&path) {
                let before = identity(&path)?;
                let descriptor = self
                    .native
                    .watches()
                    .add(
                        &path,
                        WatchMask::CREATE
                            | WatchMask::DELETE
                            | WatchMask::MODIFY
                            | WatchMask::ATTRIB
                            | WatchMask::CLOSE_WRITE
                            | WatchMask::MOVED_FROM
                            | WatchMask::MOVED_TO
                            | WatchMask::DELETE_SELF
                            | WatchMask::MOVE_SELF
                            | WatchMask::ONLYDIR
                            | WatchMask::DONT_FOLLOW,
                    )
                    .or_raise(|| message!("could not install inotify watch for {}", path.display()))?;
                if self.directories.contains_key(&descriptor) {
                    return Err(
                        message!("inotify directory aliases are not supported: {}", path.display()).unsupported_error(),
                    );
                }
                if identity(&path)? != before {
                    return Err(
                        message!("directory changed while installing inotify watch: {}", path.display())
                            .conflict_error(),
                    );
                }
                self.paths.insert(path.clone(), descriptor.clone());
                self.directories.insert(
                    descriptor,
                    Directory {
                        path: path.clone(),
                        identity: before,
                    },
                );
            }
            if recursive {
                // Watch before enumeration: a newly created child is either enumerated here or
                // queued on its already-watched parent. Never follow descendant symlinks.
                for entry in std::fs::read_dir(&path)
                    .or_raise(|| message!("could not enumerate watched directory {}", path.display()))?
                {
                    let entry = entry.or_raise(|| message!("could not read watched directory {}", path.display()))?;
                    if entry.file_type().or_error()?.is_dir() {
                        pending.push(entry.path());
                    }
                }
            }
        }
        Ok(())
    }

    fn accepts(&self, path: &Path) -> bool {
        self.logical.contains_key(path)
            || path.parent().is_some_and(|parent| self.logical.contains_key(parent))
            || path
                .ancestors()
                .skip(2)
                .any(|parent| self.logical.get(parent) == Some(&true))
    }

    fn recursive(&self, path: &Path) -> bool {
        path.ancestors()
            .skip(1)
            .any(|parent| self.logical.get(parent) == Some(&true))
    }

    fn read_batch(&mut self, marker: Option<&Path>) -> Result<bool> {
        let mut buffer = [0; 64 * 1024];
        let native_events = match self.native.read_events(&mut buffer) {
            Ok(events) => events,
            Err(error) if matches!(error.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted) => {
                return Ok(false);
            }
            Err(error) => return Err(error.and_raise(message("could not read inotify events"))),
        };
        Ok(self.publish_batch(native_events, marker))
    }

    fn publish_batch<'a>(
        &mut self,
        native_events: impl IntoIterator<Item = inotify::Event<&'a std::ffi::OsStr>>,
        marker: Option<&Path>,
    ) -> bool {
        if self.failed {
            return false;
        }
        let mut events = Vec::new();
        let mut error = None;
        let mut marker_seen = false;
        for event in native_events {
            if event.mask.contains(EventMask::Q_OVERFLOW) {
                error =
                    Some(conflict("inotify queue overflowed; reinstall watches to recover recursive coverage").raise());
                continue;
            }
            let Some(directory) = self.directories.get(&event.wd) else {
                error = Some(conflict("inotify reported an unknown watch descriptor").raise());
                continue;
            };
            let path = event
                .name
                .map_or_else(|| directory.path.clone(), |name| directory.path.join(name));
            let directory_event = event.name.is_none() || event.mask.contains(EventMask::ISDIR);
            if event
                .mask
                .intersects(EventMask::DELETE_SELF | EventMask::MOVE_SELF | EventMask::UNMOUNT | EventMask::IGNORED)
                || (directory_event
                    && event.mask.intersects(EventMask::MOVED_FROM | EventMask::DELETE)
                    && self.paths.contains_key(&path))
            {
                error = Some(
                    message!(
                        "inotify directory coverage changed at {}; reinstall watches",
                        path.display()
                    )
                    .conflict_error(),
                );
            }
            if Some(path.as_path()) == marker {
                marker_seen = true;
                continue;
            }
            if directory_event
                && event.mask.intersects(EventMask::CREATE | EventMask::MOVED_TO)
                && self.recursive(&path)
                && error.is_none()
                && let Err(failure) = self.add_tree(&path, true)
            {
                error = Some(failure);
            }
            if self.accepts(&path) {
                events.push(Event {
                    paths: vec![path],
                    kind: translate(event.mask),
                    path_kind: if directory_event {
                        PathKind::Directory
                    } else {
                        PathKind::Any
                    },
                });
            }
        }
        let loss = error.as_ref().map(|_| Loss::Rescan);
        self.failed |= error.is_some();
        if !events.is_empty() || loss.is_some() {
            self.sender.publish(events, loss, error);
        }
        marker_seen
    }

    fn verify(&self) -> Result {
        for root in self.roots.keys() {
            let directory = self
                .paths
                .get(root)
                .and_then(|descriptor| self.directories.get(descriptor))
                .ok_or_else(|| message!("inotify root has no watch: {}", root.display()).conflict_error())?;
            if identity(root)? != directory.identity {
                return Err(message!("inotify root identity changed: {}", root.display()).conflict_error());
            }
        }
        if mounts::snapshot(&self.roots)? != self.mounts {
            return Err(conflict("mounts changed within inotify coverage; reinstall watches").raise());
        }
        Ok(())
    }

    fn coverage_lost(&mut self, error: gix_error::Error) -> SynchronizeError {
        self.failed = true;
        self.sender.publish(Vec::new(), Some(Loss::Rescan), Some(error));
        SynchronizeError::CoverageLost
    }

    fn synchronize_at(
        &mut self,
        directory: &Path,
        generation: u64,
        deadline: Instant,
    ) -> std::result::Result<Fence, SynchronizeError> {
        self.sender.fence(generation)?;
        if Instant::now() >= deadline {
            return Err(SynchronizeError::TimedOut);
        }
        if !self.mounts.supported {
            return Err(SynchronizeError::Unsupported);
        }
        self.verify().map_err(|error| self.coverage_lost(error))?;
        let directory = directory
            .canonicalize()
            .or_raise(|| message("could not resolve inotify marker directory"))
            .map_err(|error| self.coverage_lost(error))?;
        let Some(descriptor) = self.paths.get(&directory) else {
            return Err(SynchronizeError::Unsupported);
        };
        let anchor = self.directories[descriptor].identity;
        if identity(&directory).map_err(|error| self.coverage_lost(error))? != anchor {
            return Err(self.coverage_lost(conflict("inotify marker directory identity changed").raise()));
        }
        let cookie = tempfile::Builder::new()
            .prefix(".gix-notify-sync-")
            .tempfile_in(&directory)
            .or_raise(|| message("could not create an inotify synchronization marker"))
            .map_err(|error| self.coverage_lost(error))?;
        // All watches share this descriptor. The marker's creation follows the caller's writes
        // in that descriptor's ordered queue; keep its unique name until it has been observed.
        let cookie = cookie.into_temp_path();
        loop {
            self.sender.fence(generation)?;
            if Instant::now() >= deadline {
                return Err(SynchronizeError::TimedOut);
            }
            if self
                .read_batch(Some(&cookie))
                .map_err(|error| self.coverage_lost(error))?
            {
                self.verify().map_err(|error| self.coverage_lost(error))?;
                if identity(&directory).map_err(|error| self.coverage_lost(error))? != anchor {
                    return Err(self.coverage_lost(conflict("inotify marker directory identity changed").raise()));
                }
                if Instant::now() >= deadline {
                    return Err(SynchronizeError::TimedOut);
                }
                return self.sender.fence(generation);
            }
            self.sender.fence(generation)?;
            let timeout = Timespec::try_from(deadline.saturating_duration_since(Instant::now()))
                .map_err(|_| SynchronizeError::TimedOut)?;
            let mut descriptors = [PollFd::new(&self.native, PollFlags::IN)];
            match poll(&mut descriptors, Some(&timeout)) {
                Err(rustix::io::Errno::INTR) => {}
                result => {
                    result
                        .or_raise(|| message("could not wait for the inotify marker"))
                        .map_err(|error| self.coverage_lost(error))?;
                }
            }
        }
    }
}

fn identity(path: &Path) -> Result<(u64, u64)> {
    let metadata = std::fs::symlink_metadata(path)
        .or_raise(|| message!("could not verify watched directory {}", path.display()))?;
    if !metadata.is_dir() {
        return Err(message!("watched directory is no longer a directory: {}", path.display()).conflict_error());
    }
    Ok((metadata.dev(), metadata.ino()))
}

fn translate(mask: EventMask) -> EventKind {
    if mask.intersects(EventMask::MOVED_FROM | EventMask::MOVED_TO | EventMask::MOVE_SELF) {
        EventKind::Rename
    } else if mask.intersects(EventMask::DELETE | EventMask::DELETE_SELF) {
        EventKind::Remove
    } else if mask.contains(EventMask::CREATE) {
        EventKind::Create
    } else if mask.intersects(EventMask::MODIFY | EventMask::ATTRIB | EventMask::CLOSE_WRITE) {
        EventKind::Modify
    } else {
        EventKind::Any
    }
}

#[cfg(test)]
mod tests;
