//! Watch a small number of names without registering their parents as FSEvents roots.
use std::{
    fs::{self, File, Metadata, OpenOptions},
    io,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
    thread::JoinHandle,
    time::{Duration, Instant},
};

use gix_error::{ErrorExt, Result, ResultExt, conflict, message};
use gix_parallel::OwnShared;

use crate::{Event, EventKind, Loss, Options, PathKind, Watch, queue};

pub(crate) struct Backend {
    queue: OwnShared<OwnedFd>,
    worker: Option<JoinHandle<()>>,
}

impl Backend {
    pub(crate) fn new(watches: &[Watch], sender: queue::Sender, options: Options) -> Result<Self> {
        // SAFETY: kqueue has no arguments and returns a newly owned descriptor on success.
        let fd = unsafe { libc::kqueue() };
        if fd < 0 {
            return Err(io::Error::last_os_error().and_raise(message("could not create the file watch queue")));
        }
        // SAFETY: the successful call transferred ownership of this descriptor.
        let queue = OwnShared::new(unsafe { OwnedFd::from_raw_fd(fd) });
        // SAFETY: queue keeps fd alive and F_SETFD takes an integer flag value.
        if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
            return Err(io::Error::last_os_error().and_raise(message("could not protect the file watch descriptor")));
        }
        submit(&queue, user_event(libc::EV_ADD | libc::EV_CLEAR, 0))
            .or_raise(|| message("could not register the file watcher shutdown event"))?;
        let files = watches
            .iter()
            .map(|watch| FileWatch::new(&queue, watch.path.clone()))
            .collect::<Result<Vec<_>>>()?;
        let worker_queue = queue.clone();
        let worker = std::thread::Builder::new()
            .name("gix-notify vnode".into())
            .spawn(move || {
                if let Err(error) = observe(&worker_queue, files, &sender, options.root_check_interval) {
                    sender.publish(Vec::new(), Some(Loss::BackendError), Some(error));
                }
            })
            .or_raise(|| message("could not start the file watcher"))?;
        Ok(Self {
            queue,
            worker: Some(worker),
        })
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        let _ = submit(&self.queue, user_event(0, libc::NOTE_TRIGGER));
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn observe(queue: &OwnedFd, mut files: Vec<FileWatch>, sender: &queue::Sender, interval: Option<Duration>) -> Result {
    let mut checked = Instant::now();
    let mut events = [user_event(0, 0); 64];
    loop {
        let timeout = interval.map(|interval| interval.saturating_sub(checked.elapsed()));
        let count =
            kevent(queue, &[], &mut events, timeout).or_raise(|| message("could not read file notifications"))?;
        let events = &events[..count];
        if events.iter().any(|event| event.filter == libc::EVFILT_USER) {
            return Ok(());
        }
        if let Some(event) = events.iter().find(|event| event.flags & libc::EV_ERROR != 0) {
            return Err(io::Error::from_raw_os_error(event.data as i32).and_raise(message("file notification failed")));
        }
        if events.iter().any(|event| event.fflags & libc::NOTE_REVOKE != 0) {
            return Err(conflict("file watch access was revoked").raise());
        }
        let verify = interval.is_some_and(|interval| checked.elapsed() >= interval);
        if verify {
            checked = Instant::now();
        }
        let mut changes = Vec::new();
        for file in &mut files {
            let direct = events.iter().any(|event| {
                file.direct
                    .iter()
                    .any(|handle| event.ident == handle.as_raw_fd() as usize)
            });
            let parent = file
                .parent
                .as_ref()
                .is_some_and(|handle| events.iter().any(|event| event.ident == handle.as_raw_fd() as usize));
            if (direct || parent || verify) && file.recheck(queue, direct)? {
                changes.push(Event {
                    paths: vec![file.path.clone()],
                    kind: EventKind::Modify,
                    path_kind: PathKind::File,
                });
            }
        }
        if !changes.is_empty() {
            sender.publish(changes, None, None);
        }
    }
}

struct FileWatch {
    path: PathBuf,
    state: State,
    direct: Vec<File>,
    parent: Option<File>,
}

impl FileWatch {
    fn new(queue: &OwnedFd, path: PathBuf) -> Result<Self> {
        let state = State::read(&path)?;
        Self::bind(queue, path, state)
    }

    fn bind(queue: &OwnedFd, path: PathBuf, state: State) -> Result<Self> {
        let mut direct = Vec::new();
        if let Some(entry) = state.entry {
            if entry.kind.is_symlink() {
                direct.push(watch(queue, &path, libc::O_SYMLINK, entry.identity)?);
            } else if entry.kind.is_file() {
                direct.push(watch(queue, &path, libc::O_NOFOLLOW, entry.identity)?);
            }
        }
        if let Some(target) = state.target.filter(|target| target.kind.is_file()) {
            direct.push(watch(queue, &path, 0, target.identity)?);
        }
        let parent = state
            .parent
            .as_ref()
            .map(|(path, identity)| watch(queue, path, 0, *identity))
            .transpose()?;
        let after = State::read(&path)?;
        if !state.same_bindings(&after) {
            return Err(message!("file watch changed while registering {}", path.display()).conflict_error());
        }
        Ok(Self {
            path,
            state: after,
            direct,
            parent,
        })
    }

    fn recheck(&mut self, queue: &OwnedFd, direct: bool) -> Result<bool> {
        let next = State::read(&self.path)?;
        let changed = direct || self.state.entry != next.entry || self.state.target != next.target;
        if self.state.same_bindings(&next) {
            self.state = next;
        } else {
            *self = Self::bind(queue, self.path.clone(), next)?;
        }
        Ok(changed)
    }
}

type Identity = (u64, u64);

#[derive(Clone, Copy, Eq, PartialEq)]
struct Stamp {
    identity: Identity,
    kind: fs::FileType,
    modified: (i64, i64),
    changed: (i64, i64),
    len: u64,
}

impl From<Metadata> for Stamp {
    fn from(metadata: Metadata) -> Self {
        Self {
            identity: (metadata.dev(), metadata.ino()),
            kind: metadata.file_type(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
            len: metadata.len(),
        }
    }
}

struct State {
    entry: Option<Stamp>,
    target: Option<Stamp>,
    parent: Option<(PathBuf, Identity)>,
}

impl State {
    fn read(path: &Path) -> Result<Self> {
        let entry = stamp(path, fs::symlink_metadata)?;
        let symlink = entry.is_some_and(|entry| entry.kind.is_symlink());
        let target = if symlink { stamp(path, fs::metadata)? } else { None };
        let mut parent = None;
        if target.or(entry).is_none_or(|file| !file.kind.is_file()) {
            let candidate = if symlink {
                path.with_file_name(
                    fs::read_link(path).or_raise(|| message!("could not read link {}", path.display()))?,
                )
            } else {
                path.to_owned()
            };
            for ancestor in candidate.ancestors().skip(1) {
                if let Some(directory) = stamp(ancestor, fs::metadata)?.filter(|stamp| stamp.kind.is_dir()) {
                    parent = Some((ancestor.to_owned(), directory.identity));
                    break;
                }
            }
        }
        Ok(Self { entry, target, parent })
    }

    fn same_bindings(&self, other: &Self) -> bool {
        let binding = |stamp: Option<Stamp>| stamp.map(|stamp| (stamp.identity, stamp.kind));
        binding(self.entry) == binding(other.entry)
            && binding(self.target) == binding(other.target)
            && self.parent == other.parent
    }
}

fn stamp<'a>(path: &'a Path, metadata: impl FnOnce(&'a Path) -> io::Result<Metadata>) -> Result<Option<Stamp>> {
    match metadata(path) {
        Ok(metadata) => Ok(Some(metadata.into())),
        Err(error) if matches!(error.kind(), io::ErrorKind::NotFound | io::ErrorKind::NotADirectory) => Ok(None),
        Err(error) => Err(error.and_raise(message!("could not inspect file watch path {}", path.display()))),
    }
}

fn watch(queue: &OwnedFd, path: &Path, flags: i32, identity: Identity) -> Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_EVTONLY | libc::O_NONBLOCK | flags)
        .open(path)
        .or_raise(|| message!("could not open watched file path {}", path.display()))?;
    let metadata = file
        .metadata()
        .or_raise(|| message!("could not verify {}", path.display()))?;
    if (metadata.dev(), metadata.ino()) != identity {
        return Err(message!("watched file path changed while opening {}", path.display()).conflict_error());
    }
    submit(
        queue,
        libc::kevent {
            ident: file.as_raw_fd() as usize,
            filter: libc::EVFILT_VNODE,
            flags: libc::EV_ADD | libc::EV_CLEAR,
            fflags: libc::NOTE_WRITE
                | libc::NOTE_EXTEND
                | libc::NOTE_ATTRIB
                | libc::NOTE_DELETE
                | libc::NOTE_RENAME
                | libc::NOTE_LINK
                | libc::NOTE_REVOKE,
            data: 0,
            udata: std::ptr::null_mut(),
        },
    )
    .or_raise(|| message!("could not watch {}", path.display()))?;
    Ok(file)
}

fn user_event(flags: u16, fflags: u32) -> libc::kevent {
    libc::kevent {
        ident: 0,
        filter: libc::EVFILT_USER,
        flags,
        fflags,
        data: 0,
        udata: std::ptr::null_mut(),
    }
}

fn submit(queue: &OwnedFd, event: libc::kevent) -> io::Result<()> {
    kevent(queue, &[event], &mut [], None).map(|_| ())
}

fn kevent(
    queue: &OwnedFd,
    changes: &[libc::kevent],
    events: &mut [libc::kevent],
    timeout: Option<Duration>,
) -> io::Result<usize> {
    let timeout = timeout.map(|timeout| libc::timespec {
        tv_sec: timeout.as_secs().try_into().unwrap_or(i64::MAX),
        tv_nsec: timeout.subsec_nanos().into(),
    });
    loop {
        // SAFETY: queue is live, the slices bound the syscall's reads and writes, and the timeout
        // remains live for the call. Darwin removes vnode registrations when their fd is closed.
        let count = unsafe {
            libc::kevent(
                queue.as_raw_fd(),
                changes.as_ptr(),
                changes.len() as i32,
                events.as_mut_ptr(),
                events.len() as i32,
                timeout.as_ref().map_or(std::ptr::null(), std::ptr::from_ref),
            )
        };
        if count >= 0 {
            return Ok(count as usize);
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}
