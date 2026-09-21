use std::{
    fs,
    io::{self, Read, Write},
    os::unix::{
        ffi::OsStrExt,
        fs::{FileTypeExt, MetadataExt},
        io::{AsRawFd, FromRawFd},
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) struct Listener {
    socket: UnixListener,
    path: PathBuf,
    identity: (u64, u64),
}

impl Listener {
    pub fn bind(path: &Path) -> io::Result<Self> {
        // Git uses a temporary create-new <socket>.lock while probing and binding. Sharing
        // that protocol prevents concurrent native/Git starts from stealing each other's socket.
        let lock = gix::lock::Marker::acquire_to_hold_resource(path, Duration::from_millis(100).into(), None).map_err(
            |error| match error {
                gix::lock::acquire::Error::Io(error) => error,
                error => io::Error::new(io::ErrorKind::WouldBlock, error),
            },
        )?;
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
        fs::symlink_metadata(&self.path).is_ok_and(|meta| (meta.dev(), meta.ino()) == self.identity)
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        if self.owns_endpoint() {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// This private binary helper runs before watcher threads exist, or in a standalone client process.
/// Git uses the same temporary-CWD technique to support Unix socket paths longer than sun_path.
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
    let previous = std::env::current_dir()?;
    std::env::set_current_dir(parent)?;
    let result = operation(Path::new(name));
    std::env::set_current_dir(previous)?;
    result
}

pub(crate) fn connect(path: &Path) -> io::Result<Stream> {
    at_short_path(path, |path| {
        let deadline = Instant::now() + TIMEOUT;
        // SAFETY: sockaddr_un consists of integer fields and a character array, all valid as zero.
        let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
        let bytes = path.as_os_str().as_bytes();
        if bytes.contains(&0) || bytes.len() >= address.sun_path.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid Unix IPC socket path",
            ));
        }
        address.sun_family = libc::AF_UNIX as _;
        for (to, from) in address.sun_path.iter_mut().zip(bytes) {
            *to = *from as _;
        }
        let length = std::mem::offset_of!(libc::sockaddr_un, sun_path) + bytes.len() + 1;
        #[cfg(any(
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "dragonfly"
        ))]
        {
            address.sun_len = length as _;
        }
        // SAFETY: socket returns a new uniquely owned descriptor on success.
        let descriptor = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0) };
        if descriptor == -1 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: descriptor is valid and ownership transfers to socket.
        let socket = unsafe { UnixStream::from_raw_fd(descriptor) };
        socket.set_nonblocking(true)?;
        // SAFETY: address is initialized and length includes its NUL-terminated pathname.
        if unsafe { libc::connect(socket.as_raw_fd(), std::ptr::from_ref(&address).cast(), length as _) } != 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::EINPROGRESS) {
                return Err(error);
            }
            ready(&socket, libc::POLLOUT, deadline)?;
            if let Some(error) = socket.take_error()? {
                return Err(error);
            }
        }
        Ok(Stream { socket, deadline })
    })
}

fn ready(socket: &UnixStream, events: libc::c_short, deadline: Instant) -> io::Result<()> {
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "IPC exchange timed out"))?;
        let mut descriptor = libc::pollfd {
            fd: socket.as_raw_fd(),
            events,
            revents: 0,
        };
        let timeout = remaining.as_millis().saturating_add(1).min(i32::MAX as u128) as i32;
        // SAFETY: descriptor is valid for one entry; poll neither retains it nor closes the socket.
        match unsafe { libc::poll(&mut descriptor, 1, timeout) } {
            -1 => {
                let error = io::Error::last_os_error();
                if error.kind() != io::ErrorKind::Interrupted {
                    return Err(error);
                }
            }
            0 => {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "IPC exchange timed out"));
            }
            _ => return Ok(()),
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
                    ready(&self.socket, libc::POLLIN, self.deadline)?;
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
                    ready(&self.socket, libc::POLLOUT, self.deadline)?;
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
        let lock = gix::lock::Marker::acquire_to_hold_resource(&path, Default::default(), None)?;
        let lock_path = lock.lock_path().to_owned();
        assert!(
            Listener::bind(&path).is_err(),
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
        Ok(())
    }

    #[test]
    fn long_socket_paths_roundtrip_and_restore_the_process_directory() -> gix_testtools::Result {
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

        let mut listener = Listener::bind(&path)?;
        assert_eq!(
            std::env::current_dir()?,
            original_directory,
            "binding restores the process directory"
        );
        let mut client = connect(&path)?;
        assert_eq!(
            std::env::current_dir()?,
            original_directory,
            "connecting restores the process directory"
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
            "failed connecting also restores the process directory"
        );
        Ok(())
    }
}
