//! One completion-port worker owns all directory reads and serializes watch replacement.
//! Windows directory streams have no shared delivery fence, even when they share a completion port.
use std::{
    collections::BTreeMap,
    io,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    path::{Path, PathBuf},
    sync::mpsc::{self, SyncSender},
    thread::JoinHandle,
    time::{Duration, Instant},
};

use gix_error::{ErrorExt, Result, ResultExt, message};
use gix_parallel::OwnShared;
use windows_sys::Win32::{
    Foundation::{ERROR_NOTIFY_ENUM_DIR, INVALID_HANDLE_VALUE, WAIT_TIMEOUT},
    System::IO::{CreateIoCompletionPort, GetQueuedCompletionStatus, PostQueuedCompletionStatus},
};

use crate::{Fence, Loss, SynchronizeError, Watch, queue};

mod decode;
mod native;

// ReadDirectoryChangesW observes a directory's contents, not the directory itself. Reopening
// each root also detects ancestor renames and replacement while the original handle stays valid.
const IDENTITY_INTERVAL: Duration = Duration::from_secs(1);

struct Replace {
    watches: Vec<Watch>,
    reply: SyncSender<Result>,
}

pub(crate) struct Backend {
    port: OwnShared<OwnedHandle>,
    commands: Option<SyncSender<Replace>>,
    worker: Option<JoinHandle<()>>,
}

impl Backend {
    pub(crate) fn new(watches: &[Watch], sender: queue::Sender) -> Result<Self> {
        // SAFETY: INVALID_HANDLE_VALUE creates a new port without associating a file.
        let port = unsafe { CreateIoCompletionPort(INVALID_HANDLE_VALUE, std::ptr::null_mut(), 0, 1) };
        if port.is_null() {
            return Err(io::Error::last_os_error().and_raise(message("could not create Windows notification port")));
        }
        // SAFETY: The successful call transfers ownership of this unique handle.
        let port = OwnShared::new(unsafe { OwnedHandle::from_raw_handle(port) });
        let (commands, receive) = mpsc::sync_channel::<Replace>(1);
        let (ready, initialized) = mpsc::sync_channel(1);
        let watches = watches.to_vec();
        let worker_port = port.clone();
        let worker = std::thread::Builder::new()
            .name("gix-notify-windows".into())
            .spawn(move || {
                let mut streams = Streams {
                    port: worker_port,
                    reads: BTreeMap::new(),
                    next_key: 0,
                };
                if let Err(error) = streams.replace(&watches) {
                    let _ = ready.send(Err(error));
                    return;
                }
                let _ = ready.send(Ok(()));
                let mut checked = Instant::now();
                loop {
                    match receive.try_recv() {
                        Ok(command) => {
                            let _ = command.reply.send(streams.replace(&command.watches));
                        }
                        Err(mpsc::TryRecvError::Disconnected) => break,
                        Err(mpsc::TryRecvError::Empty) => {}
                    }
                    let result = streams.poll(IDENTITY_INTERVAL.saturating_sub(checked.elapsed()), &sender);
                    let result = result.and_then(|()| {
                        if checked.elapsed() >= IDENTITY_INTERVAL {
                            checked = Instant::now();
                            streams.check_roots()?;
                        }
                        Ok(())
                    });
                    if let Err(error) = result {
                        let _ = streams.clear();
                        sender.publish(Vec::new(), Some(Loss::BackendError), Some(error));
                    }
                }
            })
            .or_raise(|| message("could not start the Windows filesystem notification worker"))?;
        let out = Self {
            port,
            commands: Some(commands),
            worker: Some(worker),
        };
        initialized
            .recv()
            .or_raise(|| message("Windows filesystem notifications stopped during initialization"))??;
        Ok(out)
    }

    pub(crate) fn replace(&mut self, watches: &[Watch]) -> Result {
        let (reply, receive) = mpsc::sync_channel(1);
        self.commands
            .as_ref()
            .ok_or_else(|| message("Windows filesystem notification worker has stopped").raise())?
            .send(Replace {
                watches: watches.to_vec(),
                reply,
            })
            .or_raise(|| message("Windows filesystem notifications stopped before replacing watches"))?;
        self.wake()
            .or_raise(|| message("could not wake Windows filesystem notification worker"))?;
        receive
            .recv()
            .or_raise(|| message("Windows filesystem notifications stopped while replacing watches"))?
    }

    pub(crate) fn synchronize_at(
        &mut self,
        _: &Path,
        _: Instant,
        _: u64,
    ) -> std::result::Result<Fence, SynchronizeError> {
        Err(SynchronizeError::Unsupported)
    }

    fn wake(&self) -> io::Result<()> {
        // SAFETY: The shared port is alive. Key zero and a null OVERLAPPED identify control wakeups.
        if unsafe { PostQueuedCompletionStatus(self.port.as_raw_handle(), 0, 0, std::ptr::null()) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        self.commands = None;
        // A failed wake still leaves the bounded completion wait to notice the disconnected channel.
        let _ = self.wake();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[derive(Default)]
struct Scope {
    directory: bool,
    recursive: bool,
    files: Vec<PathBuf>,
}

struct Streams {
    port: OwnShared<OwnedHandle>,
    reads: BTreeMap<usize, native::Read>,
    next_key: usize,
}

impl Streams {
    fn replace(&mut self, watches: &[Watch]) -> Result {
        // The outer watcher invalidates coverage first. Cancellation waits for every outstanding
        // completion packet before freeing its buffers, including on partial registration failure.
        self.clear()
            .or_raise(|| message("could not cancel Windows filesystem notifications"))?;
        let result = self.install(watches);
        if result.is_err() {
            self.clear()
                .or_raise(|| message("could not cancel partially installed Windows filesystem notifications"))?;
        }
        result
    }

    fn install(&mut self, watches: &[Watch]) -> Result {
        let mut directories = BTreeMap::<PathBuf, Scope>::new();
        for watch in watches {
            let path = watch
                .path
                .canonicalize()
                .or_raise(|| message!("could not resolve watched path {}", watch.path.display()))?;
            let metadata =
                std::fs::metadata(&path).or_raise(|| message!("could not inspect watched path {}", path.display()))?;
            if metadata.is_dir() {
                let scope = directories.entry(path).or_default();
                scope.directory = true;
                scope.recursive |= watch.recursive;
            } else {
                let parent = path
                    .parent()
                    .ok_or_else(|| message("a watched file has no parent directory").raise())?;
                directories.entry(parent.to_owned()).or_default().files.push(path);
            }
        }
        for (path, scope) in directories {
            self.next_key = self
                .next_key
                .checked_add(1)
                .ok_or_else(|| message("Windows filesystem notification keys exhausted").raise())?;
            let read = native::Read::new(&self.port, self.next_key, &path, scope)
                .or_raise(|| message!("could not watch {}", path.display()))?;
            // Own the allocation before submitting any I/O, so even unwinding drains completion.
            let read = self.reads.entry(self.next_key).or_insert(read);
            read.start()
                .or_raise(|| message!("could not start watching {}", path.display()))?;
            read.check_root()
                .or_raise(|| message!("lost watched directory {}", path.display()))?;
        }
        // All initial reads are armed before acknowledging registration.
        Ok(())
    }

    fn clear(&mut self) -> io::Result<()> {
        let mut pending = self.reads.values().filter(|read| read.is_pending()).count();
        let mut cancellation_error = None;
        for read in self.reads.values() {
            if let Err(error) = read.cancel() {
                cancellation_error = Some(error);
            }
        }
        while pending != 0 {
            let (mut bytes, mut key, mut overlapped) = (0, 0, std::ptr::null_mut());
            // SAFETY: All outstanding allocations remain alive. Windows may update an OVERLAPPED
            // status block when its packet is removed, so cancelling or waiting on an event alone
            // is insufficient. Even failed/cancelled I/O supplies its completion packet here.
            let success = unsafe {
                GetQueuedCompletionStatus(self.port.as_raw_handle(), &mut bytes, &mut key, &mut overlapped, 1000)
            } != 0;
            if !overlapped.is_null() {
                if let Some(read) = self.reads.get_mut(&key).filter(|read| read.is_pending()) {
                    read.complete();
                    pending -= 1;
                }
            } else if !success {
                let error = io::Error::last_os_error();
                if error.raw_os_error() != Some(WAIT_TIMEOUT as i32) {
                    // An unusable port cannot normally occur while we own its handle. Should it
                    // happen, retain the kernel's target allocations rather than risk use-after-free.
                    std::mem::forget(std::mem::take(&mut self.reads));
                    return Err(error);
                }
            }
        }
        self.reads.clear();
        cancellation_error.map_or(Ok(()), Err)
    }

    fn check_roots(&self) -> Result {
        for read in self.reads.values() {
            read.check_root()
                .or_raise(|| message!("lost watched directory {}", read.path.display()))?;
        }
        Ok(())
    }

    fn poll(&mut self, timeout: Duration, sender: &queue::Sender) -> Result {
        let (mut bytes, mut key, mut overlapped) = (0, 0, std::ptr::null_mut());
        // SAFETY: The port and every outstanding read allocation are live; output pointers are
        // valid for this call. Control wakeups have a null OVERLAPPED and never refer to a read.
        let success = unsafe {
            GetQueuedCompletionStatus(
                self.port.as_raw_handle(),
                &mut bytes,
                &mut key,
                &mut overlapped,
                timeout.as_millis().min(1000) as u32,
            )
        } != 0;
        let error = (!success).then(io::Error::last_os_error);
        if overlapped.is_null() {
            return match error {
                Some(error) if error.raw_os_error() != Some(WAIT_TIMEOUT as i32) => {
                    Err(error.and_raise(message("could not receive Windows filesystem notifications")))
                }
                _ => Ok(()),
            };
        }
        let Some(read) = self.reads.get_mut(&key) else {
            return Ok(());
        };
        read.complete();
        if let Some(error) = error {
            if error.raw_os_error() != Some(ERROR_NOTIFY_ENUM_DIR as i32) {
                return Err(error.and_raise(message!("filesystem notifications failed for {}", read.path.display())));
            }
            bytes = 0;
        }
        let events = read.events(bytes);
        // Re-arm before publishing (and before invoking the user's waker). Windows retains changes
        // between reads in the handle's fixed native buffer and reports overflow explicitly.
        read.start()
            .or_raise(|| message!("could not rearm filesystem notifications for {}", read.path.display()))?;
        read.check_root()
            .or_raise(|| message!("lost watched directory {}", read.path.display()))?;
        match events {
            Ok(events) => sender.publish(events, None, None),
            Err(_) => sender.publish(Vec::new(), Some(Loss::Rescan), None),
        }
        Ok(())
    }
}

impl Drop for Streams {
    fn drop(&mut self) {
        let _ = self.clear();
    }
}
