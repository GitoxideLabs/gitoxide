use std::{
    io::{self, Read, Write},
    os::unix::{fs::PermissionsExt, net::UnixStream},
    path::Path,
    time::{Duration, Instant},
};

use rustix::{
    event::{PollFd, PollFlags, Timespec, poll},
    io::Errno,
    net::{self, AddressFamily, SocketAddrUnix, SocketType},
};

pub(super) struct Stream {
    socket: UnixStream,
    deadline: Option<Instant>,
}

impl Stream {
    pub(super) fn connect(path: &Path, timeout: Option<Duration>) -> io::Result<Self> {
        #[cfg(not(any(target_vendor = "apple", target_os = "aix", target_os = "haiku")))]
        let socket = net::socket_with(AddressFamily::UNIX, SocketType::STREAM, net::SocketFlags::CLOEXEC, None)?;
        #[cfg(any(target_vendor = "apple", target_os = "aix", target_os = "haiku"))]
        let socket = {
            let socket = net::socket(AddressFamily::UNIX, SocketType::STREAM, None)?;
            rustix::io::fcntl_setfd(&socket, rustix::io::FdFlags::CLOEXEC)?;
            socket
        };
        // UnixStream::from() does not apply the constructor's per-socket SIGPIPE suppression.
        #[cfg(any(
            target_vendor = "apple",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "netbsd"
        ))]
        net::sockopt::set_socket_nosigpipe(&socket, true)?;
        let socket = UnixStream::from(socket);
        socket.set_nonblocking(true)?;
        let mut stream = Self { socket, deadline: None };
        stream.set_timeout(timeout)?;
        let connect = |path: &Path| {
            stream.remaining()?;
            let address =
                SocketAddrUnix::new(path).map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
            if let Err(error) = net::connect(&stream.socket, &address) {
                if error != Errno::INPROGRESS {
                    return Err(error.into());
                }
                stream.wait(PollFlags::OUT)?;
                if let Some(error) = stream.socket.take_error()? {
                    return Err(error);
                }
            }
            stream.remaining()?;
            Ok(())
        };
        match connect(path) {
            Err(error) if error.kind() == io::ErrorKind::InvalidInput => {
                gix_trace::debug!("using a short filesystem monitor socket alias");
                // Shorten overlong socket addresses without changing the process-wide directory.
                let directory = tempfile::Builder::new()
                    .prefix("gix-fsm-")
                    .permissions(std::fs::Permissions::from_mode(0o700))
                    .tempdir_in("/tmp")?;
                let alias = directory.path().join("socket");
                std::os::unix::fs::symlink(path, &alias)?;
                connect(&alias)?;
            }
            result => result?,
        }
        Ok(stream)
    }

    pub(super) fn set_timeout(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        self.deadline = timeout
            .map(|timeout| {
                Instant::now().checked_add(timeout).ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "filesystem monitor timeout is too large")
                })
            })
            .transpose()?;
        Ok(())
    }

    fn remaining(&self) -> io::Result<Option<Duration>> {
        self.deadline
            .map(|deadline| {
                deadline
                    .checked_duration_since(Instant::now())
                    .filter(|remaining| !remaining.is_zero())
                    .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "filesystem monitor exchange timed out"))
            })
            .transpose()
    }

    fn wait(&self, events: PollFlags) -> io::Result<()> {
        loop {
            // Platforms without ppoll accept only signed 32-bit millisecond timeouts.
            let timeout = self
                .remaining()?
                .map(|timeout| Timespec::try_from(timeout.min(Duration::from_millis(i32::MAX as u64))))
                .transpose()
                .map_err(io::Error::other)?;
            let mut descriptors = [PollFd::new(&self.socket, events)];
            match poll(&mut descriptors, timeout.as_ref()) {
                Err(Errno::INTR) | Ok(0) => {}
                result => return result.map(|_| ()).map_err(Into::into),
            }
        }
    }
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            self.remaining()?;
            match self.socket.read(buf) {
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => self.wait(PollFlags::IN)?,
                result => {
                    self.remaining()?;
                    return result;
                }
            }
        }
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        loop {
            self.remaining()?;
            match self.socket.write(buf) {
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => self.wait(PollFlags::OUT)?,
                result => {
                    self.remaining()?;
                    return result;
                }
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.remaining().map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_constructor_preserves_descriptor_and_signal_safeguards() -> gix_testtools::TestResult {
        let directory = gix_testtools::tempfile::tempdir_in("/tmp")?;
        let path = directory.path().join("socket");
        let _listener = std::os::unix::net::UnixListener::bind(&path)?;
        let stream = Stream::connect(&path, Some(Duration::from_secs(1)))?;
        assert!(
            rustix::io::fcntl_getfd(&stream.socket)?.contains(rustix::io::FdFlags::CLOEXEC),
            "a concurrently launched subprocess must not inherit the client connection"
        );
        #[cfg(any(
            target_vendor = "apple",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "netbsd"
        ))]
        assert!(
            net::sockopt::socket_nosigpipe(&stream.socket)?,
            "a disconnected daemon must report an I/O error instead of raising SIGPIPE"
        );
        Ok(())
    }
}
