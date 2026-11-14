use std::{
    fs,
    io::{self, Read, Write},
    os::unix::{
        ffi::OsStrExt,
        fs::{FileTypeExt, MetadataExt, PermissionsExt, symlink},
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use rustix::{
    event::{PollFd, PollFlags, Timespec, poll},
    io::Errno,
    net::{self, AddressFamily, SocketAddrUnix, SocketType},
};

const TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) struct Wake {
    receive: UnixStream,
    send: UnixStream,
}

impl Wake {
    pub fn new() -> io::Result<Self> {
        let (receive, send) = UnixStream::pair()?;
        receive.set_nonblocking(true)?;
        send.set_nonblocking(true)?;
        Ok(Self { receive, send })
    }

    pub fn wake(&self) {
        // A full socket already holds a wake notification; the callback must never block.
        if let Err(_error) = (&self.send).write_all(&[1])
            && _error.kind() != io::ErrorKind::WouldBlock
        {
            gix_trace::warn!(error = %_error, "could not wake the filesystem monitor");
        }
    }
}

pub(crate) struct Listener {
    socket: UnixListener,
    path: PathBuf,
    identity: (u64, u64),
}

impl Listener {
    pub fn bind(path: &Path) -> io::Result<Self> {
        // Git uses a temporary create-new <socket>.lock while probing and binding. Sharing
        // that protocol prevents concurrent native/Git starts from stealing each other's socket.
        let lock = gix_lock::Marker::acquire_to_hold_resource(path, Duration::from_millis(100).into(), None, 0)
            .map_err(|error| {
                let kind = if error.is_retryable() {
                    io::ErrorKind::WouldBlock
                } else {
                    error
                        .downcast_any_ref::<io::Error>()
                        .map_or(io::ErrorKind::Other, io::Error::kind)
                };
                io::Error::new(kind, error)
            })?;
        match connect(path) {
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    "a daemon already owns the IPC endpoint",
                ));
            }
            Err(err) if matches!(err.kind(), io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused) => {}
            Err(err) => return Err(err),
        }
        match fs::symlink_metadata(path) {
            Ok(meta) if meta.file_type().is_socket() => fs::remove_file(path)?,
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "IPC endpoint exists and is not a socket",
                ));
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
        let socket = at_short_path(path, |path| UnixListener::bind(path))?;
        socket.set_nonblocking(true)?;
        let metadata = fs::symlink_metadata(path)?;
        drop(lock);
        Ok(Self {
            socket,
            path: path.to_owned(),
            identity: (metadata.dev(), metadata.ino()),
        })
    }

    pub fn accept(&mut self) -> io::Result<Option<Stream>> {
        match self.socket.accept() {
            Ok((socket, _)) => {
                socket.set_nonblocking(true)?;
                Ok(Some(Stream {
                    socket,
                    deadline: Instant::now() + TIMEOUT,
                }))
            }
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(err) => Err(err),
        }
    }

    pub fn owns_endpoint(&self) -> bool {
        owns_endpoint(&self.path, self.identity)
    }

    #[cfg(feature = "async")]
    pub(crate) fn ownership_check(&self) -> impl FnOnce() -> io::Result<bool> + Send + 'static {
        let path = self.path.clone();
        let identity = self.identity;
        move || Ok(owns_endpoint(&path, identity))
    }

    #[cfg(feature = "async")]
    pub(crate) fn async_socket(&self) -> io::Result<tokio::net::UnixListener> {
        tokio::net::UnixListener::from_std(self.socket.try_clone()?)
    }

    pub fn wait(&self, wake: &Wake, timeout: Duration) -> io::Result<()> {
        let mut descriptors = [
            PollFd::new(&self.socket, PollFlags::IN),
            PollFd::new(&wake.receive, PollFlags::IN),
        ];
        match poll_with_timeout(&mut descriptors, timeout) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => return Ok(()),
            result => {
                result?;
            }
        }
        if descriptors.iter().any(|descriptor| {
            descriptor
                .revents()
                .intersects(PollFlags::ERR | PollFlags::HUP | PollFlags::NVAL)
        }) {
            return Err(io::Error::other("filesystem monitor wait descriptor failed"));
        }
        if descriptors[1].revents().contains(PollFlags::IN) {
            let mut notifications = [0; 1024];
            match (&wake.receive).read(&mut notifications) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "monitor wake socket closed",
                    ));
                }
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}

fn owns_endpoint(path: &Path, identity: (u64, u64)) -> bool {
    fs::symlink_metadata(path).is_ok_and(|meta| (meta.dev(), meta.ino()) == identity)
}

impl Drop for Listener {
    fn drop(&mut self) {
        if self.owns_endpoint() {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// Run `operation` on `path`, using a private parent-directory alias when needed for `sun_path` limits.
fn at_short_path<T>(path: &Path, operation: impl FnOnce(&Path) -> io::Result<T>) -> io::Result<T> {
    if path.as_os_str().as_bytes().len() < 100 {
        return operation(path);
    }
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "socket has no parent"))?;
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "socket has no filename"))?;
    let directory = tempfile::Builder::new()
        .prefix("gix-fsm-")
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir_in("/tmp")?;
    let alias = directory.path().join("parent");
    symlink(parent.canonicalize()?, &alias)?;
    operation(&alias.join(name))
}

pub(crate) fn connect(path: &Path) -> io::Result<Stream> {
    at_short_path(path, |path| {
        let deadline = Instant::now() + TIMEOUT;
        let address = SocketAddrUnix::new(path)?;
        let socket = UnixStream::from(net::socket(AddressFamily::UNIX, SocketType::STREAM, None)?);
        socket.set_nonblocking(true)?;
        if let Err(error) = net::connect(&socket, &address) {
            if error != Errno::INPROGRESS {
                return Err(error.into());
            }
            ready(&socket, PollFlags::OUT, deadline)?;
            if let Some(error) = socket.take_error()? {
                return Err(error);
            }
        }
        Ok(Stream { socket, deadline })
    })
}

fn poll_with_timeout(descriptors: &mut [PollFd<'_>], timeout: Duration) -> io::Result<usize> {
    // Platforms without ppoll only accept a signed 32-bit millisecond timeout.
    let timeout = Timespec::try_from(timeout.min(Duration::from_millis(i32::MAX as u64))).map_err(io::Error::other)?;
    Ok(poll(descriptors, Some(&timeout))?)
}

fn ready(socket: &UnixStream, events: PollFlags, deadline: Instant) -> io::Result<()> {
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "IPC exchange timed out"))?;
        let mut descriptors = [PollFd::new(socket, events)];
        match poll_with_timeout(&mut descriptors, remaining) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Ok(0) => return Err(io::Error::new(io::ErrorKind::TimedOut, "IPC exchange timed out")),
            result => return result.map(|_| ()),
        }
    }
}

pub(crate) struct Stream {
    socket: UnixStream,
    deadline: Instant,
}
impl Stream {
    pub fn finish(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            match self.socket.read(buf) {
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    ready(&self.socket, PollFlags::IN, self.deadline)?;
                }
                result => return result,
            }
        }
    }
}
impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        loop {
            match self.socket.write(buf) {
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    ready(&self.socket, PollFlags::OUT, self.deadline)?;
                }
                result => return result,
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_connections_wake_the_listener_without_source_notifications() -> gix_testtools::TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let dir = gix_testtools::tempfile::tempdir()?;
        let path = dir.path().join("socket");
        let listener = Listener::bind(&path)?;
        let wake = Wake::new()?;
        std::thread::scope(|scope| -> gix_testtools::TestResult {
            let (started, receive_started) = std::sync::mpsc::sync_channel(0);
            let (finished, receive_finished) = std::sync::mpsc::sync_channel(1);
            let listener = &listener;
            let worker_wake = &wake;
            scope.spawn(move || {
                started
                    .send(())
                    .expect("the test observes the listener starting its wait");
                let result = listener.wait(worker_wake, Duration::from_secs(60));
                finished
                    .send(result)
                    .expect("the test observes the completed listener wait");
            });
            receive_started.recv_timeout(Duration::from_secs(1))?;
            assert_eq!(
                receive_finished.recv_timeout(Duration::from_millis(50)).map(|_| ()),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout),
                "the listener remains asleep without clients or source notifications"
            );
            let client = connect(&path);
            let result = receive_finished.recv_timeout(Duration::from_secs(1));
            wake.wake();
            let _client = client?;
            result.expect("a client connection wakes the listener without awaiting its maintenance timeout")?;
            Ok(())
        })
    }

    #[test]
    fn readiness_waits_support_zero_and_large_timeouts() -> gix_testtools::TestResult {
        let (receive, mut send) = UnixStream::pair()?;
        let mut descriptors = [PollFd::new(&receive, PollFlags::IN)];
        assert_eq!(
            poll_with_timeout(&mut descriptors, Duration::ZERO)?,
            0,
            "a zero timeout checks readiness without blocking"
        );
        send.write_all(&[1])?;
        assert_eq!(
            poll_with_timeout(&mut descriptors, Duration::MAX)?,
            1,
            "large timeouts are clamped for platforms with millisecond poll without delaying ready sockets"
        );
        Ok(())
    }

    #[test]
    fn incomplete_clients_have_a_bounded_deadline() -> io::Result<()> {
        let (socket, _client) = UnixStream::pair()?;
        socket.set_nonblocking(true)?;
        let mut stream = Stream {
            socket,
            deadline: Instant::now() + Duration::from_millis(10),
        };
        let error = stream
            .read(&mut [0])
            .expect_err("a client that sends nothing must time out");
        assert_eq!(
            error.kind(),
            io::ErrorKind::TimedOut,
            "nonblocking polling bounds the whole exchange"
        );
        Ok(())
    }

    #[test]
    fn singleton_stale_recovery_and_replacement_ownership() -> gix_testtools::Result {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let dir = gix_testtools::tempfile::tempdir()?;
        let path = dir.path().join("socket");
        drop(at_short_path(&path, |path| UnixListener::bind(path))?);
        let listener = Listener::bind(&path)?;
        assert!(Listener::bind(&path).is_err(), "an active endpoint cannot be stolen");
        fs::remove_file(&path)?;
        fs::write(&path, "replacement")?;
        drop(listener);
        assert_eq!(
            fs::read(&path)?,
            b"replacement",
            "dropping an obsolete listener preserves a replacement inode"
        );
        assert!(Listener::bind(&path).is_err(), "a nonsocket endpoint is never removed");
        Ok(())
    }
    #[test]
    fn native_startup_lock_is_respected_and_removed_on_release() -> gix_testtools::Result {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let dir = gix_testtools::tempfile::tempdir()?;
        let path = dir.path().join("socket");
        let lock = gix_lock::Marker::acquire_to_hold_resource(&path, Default::default(), None, 0)?;
        let lock_path = lock.lock_path().to_owned();
        assert!(
            matches!(Listener::bind(&path), Err(error) if error.kind() == io::ErrorKind::WouldBlock),
            "Git's create-new startup lock excludes our listener too"
        );
        assert!(
            lock_path.exists(),
            "a competing process must not remove the native startup lock"
        );
        drop(lock);
        let listener = Listener::bind(&path)?;
        assert!(
            !lock_path.exists(),
            "successful binding releases the temporary native startup lock"
        );
        drop(listener);
        fs::write(&path, "not a socket")?;
        assert!(Listener::bind(&path).is_err(), "nonsocket endpoints prevent binding");
        assert!(!lock_path.exists(), "failed binding also releases its own startup lock");
        assert!(
            matches!(Listener::bind(&dir.path().join("missing/socket")), Err(error) if error.kind() == io::ErrorKind::NotFound),
            "startup lock I/O failures retain their original kind"
        );
        Ok(())
    }

    #[test]
    fn long_socket_paths_roundtrip_without_changing_the_process_directory() -> gix_testtools::TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let dir = gix_testtools::tempfile::tempdir()?;
        let parent = dir.path().join("long-path-".repeat(16));
        fs::create_dir(&parent)?;
        let path = parent.join("socket");
        let original_directory = std::env::current_dir()?;
        assert!(
            path.as_os_str().as_bytes().len() > 128,
            "the fixture exceeds Unix sockaddr path limits"
        );
        at_short_path(&path, |_| {
            assert_eq!(
                std::env::current_dir()?,
                original_directory,
                "even an in-progress socket operation preserves the application's working directory"
            );
            Ok(())
        })?;

        let mut listener = Listener::bind(&path)?;
        assert_eq!(
            std::env::current_dir()?,
            original_directory,
            "binding preserves the process directory"
        );
        let mut client = connect(&path)?;
        assert_eq!(
            std::env::current_dir()?,
            original_directory,
            "connecting preserves the process directory"
        );
        let mut server = listener.accept()?.expect("a connected Unix client is ready to accept");
        client.write_all(b"request")?;
        let mut request = [0; 7];
        server.read_exact(&mut request)?;
        assert_eq!(&request, b"request", "long-path endpoints carry client requests");
        server.write_all(b"reply")?;
        let mut reply = [0; 5];
        client.read_exact(&mut reply)?;
        assert_eq!(&reply, b"reply", "long-path endpoints carry server responses");
        drop(server);
        drop(client);
        drop(listener);
        assert!(!path.exists(), "the long-path listener removes its own endpoint");
        assert!(connect(&path).is_err(), "the endpoint is unavailable after shutdown");
        assert_eq!(
            std::env::current_dir()?,
            original_directory,
            "failed connecting also preserves the process directory"
        );
        Ok(())
    }
}
