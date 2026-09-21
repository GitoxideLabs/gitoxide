//! FSEvents ownership stays on one control thread. A private serial dispatch queue publishes
//! complete native callbacks; native flushes never run on that queue.
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::{CStr, CString, OsString, c_void},
    os::unix::{
        ffi::{OsStrExt, OsStringExt},
        fs::MetadataExt,
    },
    path::{Path, PathBuf},
    ptr::NonNull,
    sync::mpsc::{self, SyncSender},
    thread::JoinHandle,
    time::Instant,
};

use dispatch2::{DispatchQueue, DispatchRetained};
use fsevent_sys::{self as fs, core_foundation as cf};
use gix_error::{ErrorExt, ResultExt, message};
use gix_features::threading::{Mutable, OwnShared, lock};
use std::io::Write;

use crate::{Error, Event, EventKind, Fence, Loss, PathKind, SynchronizeError, Watch, queue};

mod mounts;

// SDK event flags through ItemCloned occupy bits 0 through 22. Treat future bits as loss.
const KNOWN_FLAGS: u32 = 0x007f_ffff;

// fsevent-sys exposes the rest of FSEvents but omits its modern scheduling function.
unsafe extern "C" {
    fn FSEventStreamSetDispatchQueue(stream: fs::FSEventStreamRef, queue: *const DispatchQueue);
}

enum Command {
    Replace(Vec<Watch>, SyncSender<Result<(), Error>>),
    Synchronize {
        directory: PathBuf,
        generation: u64,
        deadline: Instant,
        reply: SyncSender<Result<Fence, SynchronizeError>>,
    },
}

pub(crate) struct Backend {
    commands: Option<SyncSender<Command>>,
    worker: Option<JoinHandle<()>>,
}

impl Backend {
    pub(crate) fn new(watches: &[Watch], sender: queue::Sender) -> Result<Self, Error> {
        let (commands, receive) = mpsc::sync_channel(1);
        let (ready, initialized) = mpsc::sync_channel(1);
        let watches = watches.to_vec();
        let worker = std::thread::Builder::new()
            .name("gix-notify FSEvents".into())
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
                while let Ok(command) = receive.recv() {
                    match command {
                        Command::Replace(watches, reply) => {
                            // Outer Watcher already invalidated coverage. Fully tear down the old
                            // stream before reusing callback state for the new registration.
                            drop(stream);
                            match Stream::new(&watches, sender.clone()) {
                                Ok(replacement) => {
                                    stream = replacement;
                                    let _ = reply.send(Ok(()));
                                }
                                Err(error) => {
                                    let _ = reply.send(Err(error));
                                    return;
                                }
                            }
                        }
                        Command::Synchronize {
                            directory,
                            generation,
                            deadline,
                            reply,
                        } => {
                            let _ = reply.send(stream.synchronize_at(&directory, generation, deadline));
                        }
                    }
                }
            })
            .or_raise(|| message("could not start the FSEvents control worker"))?;
        let out = Self {
            commands: Some(commands),
            worker: Some(worker),
        };
        initialized
            .recv()
            .or_raise(|| message("FSEvents stopped during initialization"))??;
        Ok(out)
    }

    pub(crate) fn replace(&mut self, watches: &[Watch]) -> Result<(), Error> {
        let (reply, receive) = mpsc::sync_channel(1);
        let Some(commands) = self.commands.as_ref() else {
            return Err(message("FSEvents control worker has stopped").raise());
        };
        commands
            .send(Command::Replace(watches.to_vec(), reply))
            .or_raise(|| message("FSEvents stopped before replacing its roots"))?;
        receive
            .recv()
            .or_raise(|| message("FSEvents stopped while replacing its roots"))?
    }

    pub(crate) fn synchronize_at(
        &mut self,
        directory: &Path,
        deadline: Instant,
        generation: u64,
    ) -> Result<Fence, SynchronizeError> {
        if Instant::now() >= deadline {
            return Err(SynchronizeError::TimedOut);
        }
        let (reply, receive) = mpsc::sync_channel(1);
        let commands = self.commands.as_ref().ok_or(SynchronizeError::Stopped)?;
        commands
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
        // Closing the command channel stops the worker after any active native flush. FlushSync
        // has no cancellation primitive; caller deadlines do not pretend to cancel it.
        self.commands = None;
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

struct PendingFence {
    path: PathBuf,
    generation: u64,
    reply: SyncSender<Result<Fence, SynchronizeError>>,
}

struct Context {
    pending: Mutable<Option<PendingFence>>,
    sender: OwnShared<queue::Sender>,
    watches: BTreeMap<PathBuf, bool>,
}

impl Context {
    fn accepts(&self, path: &Path) -> bool {
        self.watches.contains_key(path)
            || path.parent().is_some_and(|parent| self.watches.contains_key(parent))
            || path
                .ancestors()
                .skip(2)
                .any(|parent| self.watches.get(parent) == Some(&true))
    }
}

struct Root {
    path: PathBuf,
    identity: (u64, u64),
}

struct Stream {
    stream: NonNull<c_void>,
    callback_queue: DispatchRetained<DispatchQueue>,
    context: OwnShared<Context>,
    roots: Vec<Root>,
    local: bool,
    started: bool,
    _paths: CfOwned,
}

impl Stream {
    fn new(watches: &[Watch], sender: OwnShared<queue::Sender>) -> Result<Self, Error> {
        let mut logical = BTreeMap::new();
        let mut directories = BTreeSet::new();
        let mut local = true;
        for watch in watches {
            let path = watch
                .path
                .canonicalize()
                .or_raise(|| message!("could not resolve watched path {}", watch.path.display()))?;
            let metadata =
                std::fs::metadata(&path).or_raise(|| message!("could not inspect watched path {}", path.display()))?;
            local &= is_local(&path)?;
            let directory = if metadata.is_dir() {
                path.clone()
            } else {
                path.parent()
                    .ok_or_else(|| message("a watched file has no parent directory").raise())?
                    .to_owned()
            };
            logical
                .entry(path)
                .and_modify(|recursive| *recursive |= watch.recursive)
                .or_insert(watch.recursive);
            directories.insert(directory);
        }
        let directories: Vec<_> = directories
            .iter()
            .filter(|path| !path.ancestors().skip(1).any(|parent| directories.contains(parent)))
            .cloned()
            .collect();
        let paths = CfOwned::new(unsafe {
            // SAFETY: callbacks describe ordinary retained CF objects; no borrowed data escapes.
            cf::CFArrayCreateMutable(cf::kCFAllocatorDefault, 0, &cf::kCFTypeArrayCallBacks)
        })?;
        let mut roots = Vec::new();
        for path in directories {
            let string = path_string(&path)?;
            unsafe {
                // SAFETY: paths is a mutable CFArray and string is a live CFString. The array retains it.
                cf::CFArrayAppendValue(paths.0.as_ptr(), string.0.as_ptr());
            }
            roots.push(Root {
                identity: identity(&path)?,
                path,
            });
        }
        let context = OwnShared::new(Context {
            pending: Mutable::new(None),
            sender,
            watches: logical,
        });
        let native_context = fs::FSEventStreamContext {
            version: 0,
            info: OwnShared::as_ptr(&context).cast_mut().cast(),
            retain: Some(retain_context),
            release: Some(release_context),
            copy_description: None,
        };
        let stream = unsafe {
            // SAFETY: paths and context remain alive; context retain/release manage its native ownership.
            fs::FSEventStreamCreate(
                cf::kCFAllocatorDefault,
                callback,
                &native_context,
                paths.0.as_ptr(),
                fs::kFSEventStreamEventIdSinceNow,
                0.01,
                fs::kFSEventStreamCreateFlagFileEvents
                    | fs::kFSEventStreamCreateFlagNoDefer
                    | fs::kFSEventStreamCreateFlagWatchRoot,
            )
        };
        let stream = NonNull::new(stream).ok_or_else(|| message("could not create an FSEvents stream").raise())?;
        let callback_queue = DispatchQueue::new("org.gitoxide.notify", None);
        unsafe {
            // SAFETY: the queue is serial, active, retained for the entire scheduled stream lifetime.
            FSEventStreamSetDispatchQueue(stream.as_ptr(), DispatchRetained::as_ptr(&callback_queue).as_ptr());
        }
        let mut out = Self {
            stream,
            callback_queue,
            context,
            roots,
            local,
            started: false,
            _paths: paths,
        };
        out.started = unsafe {
            // SAFETY: the stream has been scheduled, and control operations stay on this worker.
            fs::FSEventStreamStart(out.stream.as_ptr()) != 0
        };
        if !out.started {
            return Err(message("the FSEvents service refused to start the stream").raise());
        }
        Ok(out)
    }

    fn synchronize_at(&self, directory: &Path, generation: u64, deadline: Instant) -> Result<Fence, SynchronizeError> {
        if !self.local
            || mounts::has_nonlocal_coverage(&self.context.watches).map_err(|error| self.marker_error(error))?
        {
            return Err(SynchronizeError::Unsupported);
        }
        if Instant::now() >= deadline {
            return Err(SynchronizeError::TimedOut);
        }
        self.context.sender.fence(generation)?;
        self.verify_roots()?;
        let directory = directory.canonicalize().map_err(|error| self.marker_error(error))?;
        if !self.roots.iter().any(|root| directory.starts_with(&root.path)) {
            return Err(SynchronizeError::Unsupported);
        }
        if !is_local(&directory).map_err(|error| {
            self.context.sender.publish(Vec::new(), Some(Loss::Rescan), Some(error));
            SynchronizeError::CoverageLost
        })? {
            return Err(SynchronizeError::Unsupported);
        }
        let anchor_identity = identity(&directory).map_err(|error| {
            self.context.sender.publish(Vec::new(), Some(Loss::Rescan), Some(error));
            SynchronizeError::CoverageLost
        })?;
        let mut cookie = tempfile::Builder::new()
            .prefix(".gix-notify-sync-")
            .tempfile_in(&directory)
            .map_err(|error| self.marker_error(error))?;
        let (reply, receive) = mpsc::sync_channel(1);
        *lock(&self.context.pending) = Some(PendingFence {
            path: cookie.path().to_owned(),
            generation,
            reply,
        });
        // Creation already follows the caller's filesystem writes. Writing after installing the
        // pending marker also guarantees another event if creation was delivered exceptionally fast.
        if let Err(error) = cookie.write_all(b"fence") {
            lock(&self.context.pending).take();
            return Err(self.marker_error(error));
        }
        // Close the file handle but keep its name present until observation (or timeout), so
        // native create/remove coalescing cannot erase the marker before it is delivered.
        let _cookie = cookie.into_temp_path();
        unsafe {
            // SAFETY: only the control worker calls this on a started stream. FlushSync reduces
            // latency, but does NOT fence writes whose kernel events have not reached the service.
            // The callback must actually observe our unique marker before returning a fence.
            fs::FSEventStreamFlushSync(self.stream.as_ptr());
        }
        let result = receive
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .map_err(|error| match error {
                mpsc::RecvTimeoutError::Timeout => SynchronizeError::TimedOut,
                mpsc::RecvTimeoutError::Disconnected => SynchronizeError::Stopped,
            });
        lock(&self.context.pending).take();
        let fence = result??;
        if Instant::now() >= deadline {
            return Err(SynchronizeError::TimedOut);
        }
        self.verify_roots()?;
        if identity(&directory).ok() != Some(anchor_identity) {
            self.context.sender.publish(Vec::new(), Some(Loss::Rescan), None);
            return Err(SynchronizeError::CoverageLost);
        }
        self.context.sender.fence(generation)?;
        Ok(fence)
    }

    fn marker_error(&self, error: std::io::Error) -> SynchronizeError {
        self.context.sender.publish(
            Vec::new(),
            Some(Loss::BackendError),
            Some(error.and_raise(message("could not create a filesystem synchronization marker"))),
        );
        SynchronizeError::CoverageLost
    }

    fn verify_roots(&self) -> Result<(), SynchronizeError> {
        for root in &self.roots {
            match identity(&root.path) {
                Ok(identity) if identity == root.identity => {}
                result => {
                    let error = match result {
                        Err(error) => error,
                        Ok(_) => message!("watched directory identity changed: {}", root.path.display()).raise(),
                    };
                    self.context.sender.publish(Vec::new(), Some(Loss::Rescan), Some(error));
                    return Err(SynchronizeError::CoverageLost);
                }
            }
        }
        Ok(())
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: only the control worker can tear down this live, scheduled stream.
            if self.started {
                fs::FSEventStreamStop(self.stream.as_ptr());
            }
            fs::FSEventStreamInvalidate(self.stream.as_ptr());
        }
        self.callback_queue.exec_sync(|| {});
        unsafe {
            // SAFETY: no callback is active; invalidation removed future callback scheduling.
            fs::FSEventStreamRelease(self.stream.as_ptr());
        }
    }
}

struct CfOwned(NonNull<c_void>);

impl CfOwned {
    fn new(value: cf::CFRef) -> Result<Self, Error> {
        NonNull::new(value)
            .map(Self)
            .ok_or_else(|| message("could not create a Core Foundation value").raise())
    }
}

impl Drop for CfOwned {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: each wrapper owns one retained CF value.
            cf::CFRelease(self.0.as_ptr());
        }
    }
}

fn path_string(path: &Path) -> Result<CfOwned, Error> {
    let bytes = path.as_os_str().as_bytes();
    let length = cf::CFIndex::try_from(bytes.len()).or_raise(|| message("FSEvents root path is too long"))?;
    let url = CfOwned::new(unsafe {
        // SAFETY: the slice is valid for length bytes; Core Foundation copies its filesystem representation.
        cf::CFURLCreateFromFileSystemRepresentation(cf::kCFAllocatorDefault, bytes.as_ptr().cast(), length, true)
    })?;
    CfOwned::new(unsafe {
        // SAFETY: url is a live CFURL; the Copy rule transfers ownership of the returned CFString.
        cf::CFURLCopyFileSystemPath(url.0.as_ptr(), cf::kCFURLPOSIXPathStyle)
    })
}

fn identity(path: &Path) -> Result<(u64, u64), Error> {
    let metadata =
        std::fs::metadata(path).or_raise(|| message!("could not verify watched directory {}", path.display()))?;
    Ok((metadata.dev(), metadata.ino()))
}

fn is_local(path: &Path) -> Result<bool, Error> {
    let path = CString::new(path.as_os_str().as_bytes()).or_raise(|| message("filesystem path contains a NUL byte"))?;
    let mut stat = std::mem::MaybeUninit::<libc::statfs>::uninit();
    let result = unsafe {
        // SAFETY: path is NUL-terminated and stat points at writable storage of the required size.
        libc::statfs(path.as_ptr(), stat.as_mut_ptr())
    };
    if result != 0 {
        return Err(std::io::Error::last_os_error().and_raise(message("could not inspect watched filesystem")));
    }
    let stat = unsafe {
        // SAFETY: successful statfs initialized the output.
        stat.assume_init()
    };
    Ok(stat.f_flags & libc::MNT_LOCAL as u32 != 0)
}

extern "C" fn retain_context(info: *const c_void) -> *const c_void {
    unsafe {
        // SAFETY: info originates from a live Arc<Context>; FSEvents pairs each retain with a release.
        OwnShared::<Context>::increment_strong_count(info.cast());
    }
    info
}

extern "C" fn release_context(info: *const c_void) {
    unsafe {
        // SAFETY: releases the matching strong reference retained for FSEvents.
        OwnShared::<Context>::decrement_strong_count(info.cast());
    }
}

extern "C" fn callback(
    _stream: fs::FSEventStreamRef,
    info: *mut c_void,
    count: usize,
    paths: *mut c_void,
    flags: *const fs::FSEventStreamEventFlags,
    _ids: *const fs::FSEventStreamEventId,
) {
    let context = unsafe {
        // SAFETY: the stream retains Context throughout each callback, including teardown draining.
        &*info.cast::<Context>()
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut events = Vec::new();
        let mut loss = None;
        let marker_path = lock(&context.pending).as_ref().map(|pending| pending.path.clone());
        let mut marker_seen = false;
        for index in 0..count {
            let (path, flags) = unsafe {
                // SAFETY: FSEvents provides count C-string pointers and flags valid until callback return.
                let path = *((paths as *const *const std::ffi::c_char).add(index));
                (CStr::from_ptr(path).to_bytes(), *flags.add(index))
            };
            if flags & !KNOWN_FLAGS != 0
                || flags
                    & (fs::kFSEventStreamEventFlagMustScanSubDirs
                        | fs::kFSEventStreamEventFlagUserDropped
                        | fs::kFSEventStreamEventFlagKernelDropped
                        | fs::kFSEventStreamEventFlagRootChanged
                        | fs::kFSEventStreamEventFlagEventIdsWrapped
                        | fs::kFSEventStreamEventFlagMount
                        | fs::kFSEventStreamEventFlagUnmount)
                    != 0
            {
                loss = Some(Loss::Rescan);
            }
            if flags & fs::kFSEventStreamEventFlagHistoryDone != 0 {
                continue;
            }
            let path = PathBuf::from(OsString::from_vec(path.to_vec()));
            if marker_path.as_ref() == Some(&path) {
                marker_seen = true;
                continue;
            }
            if context.accepts(&path) {
                let (kind, path_kind) = translate(flags);
                events.push(Event {
                    paths: vec![path],
                    kind,
                    path_kind,
                });
            }
        }
        if !events.is_empty() || loss.is_some() {
            context.sender.publish(events, loss, None);
        }
        // Publish the ENTIRE native callback, including paths after the marker, before completing
        // it. A timed-out marker cannot satisfy a newer request even if its callback arrives late.
        let mut pending = lock(&context.pending);
        if let Some(fence) = pending.as_ref() {
            let result = context.sender.fence(fence.generation);
            if (result.is_err() || (marker_seen && marker_path.as_ref() == Some(&fence.path)))
                && let Some(fence) = pending.take()
            {
                let _ = fence.reply.send(result);
            }
        }
    }));
    if result.is_err() {
        context.sender.publish(
            Vec::new(),
            Some(Loss::BackendError),
            Some(message("FSEvents callback failed").raise()),
        );
    }
}

fn translate(flags: u32) -> (EventKind, PathKind) {
    let path_kind = match flags & (fs::kFSEventStreamEventFlagItemIsDir | fs::kFSEventStreamEventFlagItemIsFile) {
        fs::kFSEventStreamEventFlagItemIsDir => PathKind::Directory,
        fs::kFSEventStreamEventFlagItemIsFile => PathKind::File,
        _ => PathKind::Any,
    };
    let kind = if flags & fs::kFSEventStreamEventFlagItemRenamed != 0 {
        EventKind::Rename
    } else if flags & fs::kFSEventStreamEventFlagItemRemoved != 0 {
        EventKind::Remove
    } else if flags & fs::kFSEventStreamEventFlagItemCreated != 0 {
        EventKind::Create
    } else if flags
        & (fs::kFSEventStreamEventFlagItemModified
            | fs::kFSEventStreamEventFlagItemInodeMetaMod
            | fs::kFSEventStreamEventFlagItemFinderInfoMod
            | fs::kFSEventStreamEventFlagItemChangeOwner
            | fs::kFSEventStreamEventFlagItemXattrMod)
        != 0
    {
        EventKind::Modify
    } else {
        EventKind::Any
    };
    (kind, path_kind)
}

#[cfg(test)]
mod tests;
