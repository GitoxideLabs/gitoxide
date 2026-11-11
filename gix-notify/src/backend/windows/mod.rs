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

use crate::{Event, Fence, Loss, SynchronizeError, Watch, queue};

mod decode;
mod native;

const MAX_READY_COMPLETIONS: usize = 1024;
const MAX_READY_DURATION: Duration = Duration::from_millis(20);

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
    pub(crate) fn new(watches: &[Watch], sender: queue::Sender, options: crate::Options) -> Result<Self> {
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
                    // poll() caps the wait even when periodic checks are disabled or far away.
                    let timeout = options
                        .root_check_interval
                        .map_or(Duration::MAX, |interval| interval.saturating_sub(checked.elapsed()));
                    let result = streams.poll(timeout, &sender, options);
                    let result = result.and_then(|()| {
                        if options
                            .root_check_interval
                            .is_some_and(|interval| checked.elapsed() >= interval)
                        {
                            checked = Instant::now();
                            streams.check_roots()?;
                        }
                        Ok(())
                    });
                    if let Err(error) = result {
                        let _ = streams.clear();
                        sender.publish(
                            Vec::new(),
                            Some(Loss::BackendError),
                            Some(
                                error
                                    .and_raise(message("Windows filesystem notification coverage was lost").conflict()),
                            ),
                        );
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
            .or_raise(|| message("Windows filesystem notifications stopped during initialization").conflict())??;
        Ok(out)
    }

    pub(crate) fn replace(&mut self, watches: &[Watch]) -> Result {
        let (reply, receive) = mpsc::sync_channel(1);
        self.commands
            .as_ref()
            .ok_or_else(|| message("Windows filesystem notification worker has stopped").conflict_error())?
            .send(Replace {
                watches: watches.to_vec(),
                reply,
            })
            .or_raise(|| message("Windows filesystem notifications stopped before replacing watches").conflict())?;
        self.wake()
            .or_raise(|| message("could not wake Windows filesystem notification worker"))?;
        receive
            .recv()
            .or_raise(|| message("Windows filesystem notifications stopped while replacing watches").conflict())?
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

enum Completion {
    Wake,
    Events(Vec<Event>),
    Rescan,
}

fn collect_ready(
    timeout: Duration,
    options: crate::Options,
    mut receive: impl FnMut(Duration) -> Result<Option<Completion>>,
) -> Result<(Vec<Event>, Option<Loss>)> {
    let mut completion = receive(timeout)?;
    let deadline = Instant::now() + MAX_READY_DURATION;
    let mut events = Vec::new();
    let mut bytes = 0usize;
    for _ in 0..MAX_READY_COMPLETIONS {
        match completion {
            None => return Ok((events, None)),
            Some(Completion::Wake) => {}
            Some(Completion::Rescan) => return Ok((Vec::new(), Some(Loss::Rescan))),
            Some(Completion::Events(next)) => {
                bytes = next
                    .iter()
                    .fold(bytes, |sum, event| sum.saturating_add(queue::event_bytes(event)));
                if next.len() > options.max_events.saturating_sub(events.len()) || bytes > options.max_bytes {
                    return Ok((Vec::new(), Some(Loss::Overflow)));
                }
                events.extend(next);
            }
        }
        // A marker from one directory must not be published ahead of older changes still
        // buffered on another handle. Include the immediately ready completions from reads
        // rearmed while draining, as Git's Windows listener does for its worktree watch.
        completion = receive(Duration::ZERO)?;
        if completion.is_some() && Instant::now() >= deadline {
            return Ok((Vec::new(), Some(Loss::Rescan)));
        }
    }
    if completion.is_none() {
        Ok((events, None))
    } else {
        // Persistent activity must not delay commands indefinitely or certify a partial burst.
        Ok((Vec::new(), Some(Loss::Rescan)))
    }
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
                    .ok_or_else(|| message("a watched file has no parent directory").validation_error())?;
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
        // ReadDirectoryChangesW observes a directory's contents, not the directory itself. Reopening
        // each root also detects ancestor renames and replacement while the original handle stays valid.
        for read in self.reads.values() {
            read.check_root()
                .or_raise(|| message!("lost watched directory {}", read.path.display()))?;
        }
        Ok(())
    }

    fn poll(&mut self, timeout: Duration, sender: &queue::Sender, options: crate::Options) -> Result {
        let (events, loss) = collect_ready(timeout, options, |timeout| self.receive(timeout))?;
        if !events.is_empty() || loss.is_some() {
            sender.publish(events, loss, None);
        }
        Ok(())
    }

    fn receive(&mut self, timeout: Duration) -> Result<Option<Completion>> {
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
                Some(_) => Ok(None),
                None => Ok(Some(Completion::Wake)),
            };
        }
        let Some(read) = self.reads.get_mut(&key) else {
            return Ok(Some(Completion::Wake));
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
        Ok(Some(match events {
            Ok(events) => Completion::Events(events),
            Err(_) => Completion::Rescan,
        }))
    }
}

impl Drop for Streams {
    fn drop(&mut self) {
        let _ = self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(path: &str) -> Event {
        Event {
            paths: vec![PathBuf::from(path)],
            kind: crate::EventKind::Create,
            path_kind: crate::PathKind::File,
        }
    }

    #[test]
    fn ready_burst_limits_and_native_loss_never_publish_partial_changes() -> gix_error::TestResult {
        let events = vec![change("cookie"), change("buffered-worktree-change")];
        let options = crate::Options {
            max_events: events.len(),
            max_bytes: events.iter().map(queue::event_bytes).sum(),
            ..Default::default()
        };
        for (options, expected_loss) in [
            (options, None),
            (
                crate::Options {
                    max_events: options.max_events - 1,
                    ..options
                },
                Some(Loss::Overflow),
            ),
            (
                crate::Options {
                    max_bytes: options.max_bytes - 1,
                    ..options
                },
                Some(Loss::Overflow),
            ),
        ] {
            let mut completions = events.iter().cloned();
            let (published, loss) = collect_ready(Duration::ZERO, options, |timeout| {
                assert_eq!(timeout, Duration::ZERO, "draining ready completions never blocks");
                Ok(completions.next().map(|event| Completion::Events(vec![event])))
            })?;
            assert_eq!(loss, expected_loss, "only exceeding a configured limit loses coverage");
            assert_eq!(
                published,
                if loss.is_none() { events.clone() } else { Vec::new() },
                "a partial burst must not expose its cookie"
            );
        }

        let mut completions = [Completion::Events(vec![events[0].clone()]), Completion::Rescan].into_iter();
        let (published, loss) = collect_ready(Duration::ZERO, options, |_| Ok(completions.next()))?;
        assert_eq!(
            loss,
            Some(Loss::Rescan),
            "later native loss invalidates the whole burst"
        );
        assert!(
            published.is_empty(),
            "a cookie before native loss cannot certify delivery"
        );

        let mut received = 0;
        let (published, loss) = collect_ready(Duration::ZERO, options, |_| {
            received += 1;
            Ok(Some(Completion::Wake))
        })?;
        assert_eq!(
            loss,
            Some(Loss::Rescan),
            "continuous completions cannot certify an idle boundary"
        );
        assert!(published.is_empty(), "a bounded drain cannot publish a partial burst");
        assert!(
            received <= MAX_READY_COMPLETIONS + 1,
            "control wakeups also obey the completion bound"
        );
        Ok(())
    }

    #[test]
    fn ready_completions_include_changes_buffered_before_a_cookie() -> gix_error::TestResult {
        let directory = tempfile::tempdir()?;
        let worktree = directory.path().join("worktree");
        let cookies = directory.path().join("cookies");
        std::fs::create_dir(&worktree)?;
        std::fs::create_dir(&cookies)?;
        // SAFETY: INVALID_HANDLE_VALUE creates a new, unassociated completion port.
        let port = unsafe { CreateIoCompletionPort(INVALID_HANDLE_VALUE, std::ptr::null_mut(), 0, 1) };
        if port.is_null() {
            return Err(io::Error::last_os_error().into());
        }
        // SAFETY: The successful call transfers ownership of this unique handle.
        let port = OwnShared::new(unsafe { OwnedHandle::from_raw_handle(port) });
        let mut streams = Streams {
            port,
            reads: BTreeMap::new(),
            next_key: 0,
        };
        streams.install(&[
            Watch {
                path: worktree.clone(),
                recursive: true,
            },
            Watch {
                path: cookies.clone(),
                recursive: true,
            },
        ])?;
        let options = crate::Options::default();
        let state = OwnShared::new(gix_parallel::Mutable::new(queue::State::new(options)));
        let sender = queue::Sender::new(state.clone(), 0);

        // The first change completes the worktree read. The next remains in that handle's
        // native buffer until rearming, behind the independently completed cookie read.
        let first = worktree.join("first");
        let second = worktree.join("second");
        let cookie = cookies.join("cookie");
        std::fs::write(&first, b"first")?;
        std::fs::write(&second, b"second")?;
        std::fs::write(&cookie, b"")?;
        streams.poll(Duration::from_secs(1), &sender, options)?;
        let batch = gix_parallel::lock(&state).drain(crate::Budget {
            max_events: usize::MAX,
            max_bytes: usize::MAX,
        });
        assert_eq!(batch.loss, None, "the native buffers retain all fixture events");
        for expected in [first, second, cookie] {
            let expected = expected.canonicalize()?;
            assert!(
                batch.events.iter().any(|event| event.paths.contains(&expected)),
                "a publication containing a cookie must include all already buffered worktree changes: {expected:?}"
            );
        }
        Ok(())
    }

    fn backend() -> Result<Backend> {
        let options = crate::Options::default();
        let state = OwnShared::new(gix_parallel::Mutable::new(queue::State::new(options)));
        Backend::new(&[], queue::Sender::new(state, 0), options)
    }

    #[test]
    fn stopped_workers_require_new_registration_and_preserve_channel_causes() -> gix_error::TestResult {
        let mut backend = backend()?;
        backend.commands = None;
        backend.wake()?;
        backend
            .worker
            .take()
            .expect("the worker was started")
            .join()
            .expect("the worker shuts down cleanly");
        let error = backend
            .replace(&[])
            .expect_err("a stopped worker cannot replace watches");
        assert!(error.is_conflict(), "registration requires a live worker");
        assert!(
            !error.is_retryable(),
            "retrying the stopped worker does not restore coverage"
        );

        let (commands, receive) = mpsc::sync_channel(1);
        drop(receive);
        backend.commands = Some(commands);
        let error = backend
            .replace(&[])
            .expect_err("a disconnected worker cannot accept commands");
        assert!(
            error.is_conflict(),
            "a disconnected command channel requires a new worker"
        );
        assert!(
            error.downcast_any_ref::<mpsc::SendError<Replace>>().is_some(),
            "the original channel error is retained"
        );

        let (commands, receive) = mpsc::sync_channel::<Replace>(1);
        backend.commands = Some(commands);
        backend.worker = Some(std::thread::spawn(move || {
            drop(receive.recv().expect("the replacement command is sent"));
        }));
        let error = backend.replace(&[]).expect_err("the worker stopped before replying");
        assert!(
            error.is_conflict(),
            "an unacknowledged registration cannot certify coverage"
        );
        assert!(
            error.downcast_any_ref::<mpsc::RecvError>().is_some(),
            "the original reply channel error is retained"
        );
        Ok(())
    }

    #[test]
    fn native_errors_distinguish_replaced_roots_malformed_counts_and_overflow() -> gix_error::TestResult {
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("watched");
        std::fs::create_dir(&root)?;
        let backend = backend()?;
        let read = native::Read::new(&backend.port, 1, &root, Scope::default())?;

        let overflow = read
            .events(0)
            .expect_err("zero bytes signal native buffer loss")
            .raise();
        assert!(
            !overflow.is_corrupted(),
            "ordinary native overflow does not imply malformed data"
        );
        assert!(
            !overflow.is_conflict(),
            "overflow retains coverage and only needs a rescan"
        );
        assert!(
            !overflow.is_resource_exhausted(),
            "notification loss is not an allocation failure or configured allocation limit"
        );
        assert_eq!(
            overflow
                .downcast_any_ref::<io::Error>()
                .expect("the native overflow error is retained")
                .kind(),
            io::ErrorKind::InvalidData,
            "the overflow error kind is unchanged"
        );

        let malformed = read
            .events(u32::MAX)
            .expect_err("a completion cannot exceed its buffer")
            .raise();
        assert!(
            malformed.is_corrupted(),
            "an impossible byte count is malformed native data"
        );
        assert_eq!(
            malformed
                .downcast_any_ref::<io::Error>()
                .expect("the native malformed-buffer error is retained")
                .kind(),
            io::ErrorKind::InvalidData,
            "the malformed-buffer error kind is unchanged"
        );

        std::fs::rename(&root, directory.path().join("old-root"))?;
        std::fs::create_dir(&root)?;
        let replaced = read.check_root().expect_err("the directory has a new identity").raise();
        assert!(replaced.is_conflict(), "the new directory requires re-registration");
        assert!(
            replaced.is_not_found(),
            "the original missing-identity kind remains visible"
        );
        assert!(
            !replaced.is_retryable(),
            "rechecking the old handle cannot restore coverage"
        );
        assert_eq!(
            replaced
                .downcast_any_ref::<io::Error>()
                .expect("the native identity error is retained")
                .kind(),
            io::ErrorKind::NotFound,
            "the identity error kind is unchanged"
        );
        Ok(())
    }
}
