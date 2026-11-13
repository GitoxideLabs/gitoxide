use std::{
    fs::File,
    io::{self, Read, Write},
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::Path,
    time::{Duration, Instant},
};

use crate::named_pipe::{self, Operation, event, milliseconds};

use windows_sys::Win32::{
    Foundation::{ERROR_PIPE_BUSY, INVALID_HANDLE_VALUE, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT},
    Storage::FileSystem::{FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED, PIPE_ACCESS_DUPLEX},
    System::{
        Pipes::{
            CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE,
            PIPE_WAIT, WaitNamedPipeW,
        },
        Threading::{SetEvent, WaitForMultipleObjects},
    },
};

const TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) struct Wake {
    event: OwnedHandle,
}

impl Wake {
    pub fn new() -> io::Result<Self> {
        Ok(Self { event: event(false)? })
    }

    pub fn wake(&self) {
        // SAFETY: This live auto-reset event coalesces notifications without blocking the callback.
        if unsafe { SetEvent(self.event.as_raw_handle()) } == 0 {
            let _error = io::Error::last_os_error();
            gix_trace::warn!(error = %_error, "could not wake the filesystem monitor");
        }
    }
}

pub(crate) struct Listener {
    connection: Operation,
}
impl Listener {
    pub fn bind(path: &Path) -> io::Result<Self> {
        let name: Vec<_> = crate::pipe::name(path)?.encode_wide().chain(Some(0)).collect();
        let security = super::windows_security::Security::new().map_err(io::Error::other)?;
        // SAFETY: name is terminated; security and its descriptor allocations remain alive for the
        // call. FIRST_PIPE_INSTANCE enforces singleton ownership, and remote clients are rejected.
        let pipe = unsafe {
            CreateNamedPipeW(
                name.as_ptr(),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE | FILE_FLAG_OVERLAPPED,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                65536,
                65536,
                5000,
                security.attributes(),
            )
        };
        if pipe == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: The successful constructor transfers this unique handle to File.
        Ok(Self {
            connection: Operation::new(unsafe { File::from_raw_handle(pipe) })?,
        })
    }

    pub fn accept(&mut self) -> io::Result<Option<Stream>> {
        if !self.connection.accept()? {
            return Ok(None);
        }
        Ok(Some(Stream {
            operation: Operation::new(self.connection.file().try_clone()?)?,
            deadline: Instant::now() + TIMEOUT,
            server: true,
        }))
    }

    pub fn owns_endpoint(&self) -> bool {
        true
    }

    pub fn wait(&self, wake: &Wake, timeout: Duration) -> io::Result<()> {
        let handles = [self.connection.event().as_raw_handle(), wake.event.as_raw_handle()];
        // SAFETY: Both events remain owned and live for the duration of this wait. The wake event
        // auto-resets only when selected, so a simultaneous connection cannot consume its signal.
        match unsafe { WaitForMultipleObjects(handles.len() as u32, handles.as_ptr(), 0, milliseconds(timeout)) } {
            status if status == WAIT_OBJECT_0 || status == WAIT_TIMEOUT => Ok(()),
            status if status == WAIT_OBJECT_0 + 1 => Ok(()),
            WAIT_FAILED => Err(io::Error::last_os_error()),
            _ => Err(io::Error::other("filesystem monitor event wait failed")),
        }
    }
}

pub(crate) fn connect(path: &Path) -> io::Result<Stream> {
    let pipe = crate::pipe::name(&crate::normalize_endpoint(path)?)?;
    let name: Vec<_> = pipe.encode_wide().chain(Some(0)).collect();
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "named pipe stayed busy"))?;
        match named_pipe::Stream::connect(&pipe, Some(remaining)) {
            Ok(stream) => {
                return Ok(Stream {
                    operation: stream.into_operation(),
                    deadline,
                    server: false,
                });
            }
            Err(error) if error.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) => {}
            Err(error) => return Err(error),
        }
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "named pipe stayed busy"))?;
        // SAFETY: name is terminated and remains alive for the bounded wait.
        unsafe {
            // Zero selects the server's default wait, not an immediate timeout.
            WaitNamedPipeW(name.as_ptr(), milliseconds(remaining).max(1));
        }
    }
}

pub(crate) struct Stream {
    operation: Operation,
    deadline: Instant,
    server: bool,
}
impl Stream {
    pub fn finish(&mut self) -> io::Result<()> {
        // DisconnectNamedPipe discards unread output. Wait for the one-request client to close,
        // bounded by the exchange deadline, rather than using an unbounded FlushFileBuffers.
        let mut byte = [0];
        while self.read(&mut byte)? != 0 {
            if Instant::now() >= self.deadline {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "IPC client did not close"));
            }
        }
        Ok(())
    }
}
impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.operation.read(buf, Some(self.deadline))
    }
}
impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.operation.write(buf, Some(self.deadline))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Drop for Stream {
    fn drop(&mut self) {
        let _ = self.operation.cancel();
        if self.server {
            // SAFETY: Cancellation is drained, and the owned pipe stays live until Operation drops.
            unsafe {
                DisconnectNamedPipe(self.operation.file().as_raw_handle());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_waits_consume_wakes_and_pending_accepts_cancel_on_drop() -> gix_error::TestResult {
        let directory = gix_testtools::tempfile::tempdir()?;
        let path = directory.path().canonicalize()?.join("ipc");
        let wake = Wake::new()?;
        let mut listener = Listener::bind(&path)?;
        assert!(
            listener.accept()?.is_none(),
            "an idle listener arms asynchronous acceptance"
        );
        assert!(
            listener.connection.is_pending(),
            "the pipe retains its pending acceptance state"
        );

        wake.wake();
        wake.wake();
        let started = Instant::now();
        listener.wait(&wake, Duration::from_secs(1))?;
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "coalesced notifications wake the wait"
        );
        let started = Instant::now();
        listener.wait(&wake, Duration::from_millis(50))?;
        assert!(
            started.elapsed() >= Duration::from_millis(40),
            "a consumed wake does not leave the listener spinning"
        );

        drop(listener);
        let mut replacement = Listener::bind(&path)?;
        assert!(
            replacement.accept()?.is_none(),
            "cancelling pending acceptance releases the endpoint and its native state"
        );
        let client = connect(&path)?;
        replacement.wait(&wake, Duration::from_secs(1))?;
        let server = replacement
            .accept()?
            .expect("client readiness completes asynchronous acceptance");
        drop(client);
        drop(server);
        assert!(
            replacement.accept()?.is_none(),
            "a completed connection rearms acceptance"
        );
        let started = Instant::now();
        replacement.wait(&wake, Duration::from_millis(50))?;
        assert!(
            started.elapsed() >= Duration::from_millis(40),
            "rearming clears the previous connection's manual-reset event"
        );
        Ok(())
    }

    #[test]
    fn connected_idle_pipes_wait_for_data_instead_of_reporting_eof() -> gix_error::TestResult {
        let directory = gix_testtools::tempfile::tempdir()?;
        // Server startup supplies a normalized endpoint to the listener.
        let path = directory.path().canonicalize()?.join("ipc");
        let mut listener = Listener::bind(&path)?;
        let mut client = connect(&path)?;
        let mut server = listener.accept()?.expect("the client already opened the named pipe");

        for stream in [&mut client, &mut server] {
            stream.deadline = Instant::now();
            let error = stream
                .read(&mut [0])
                .expect_err("an idle connected pipe must wait until its exchange deadline");
            assert_eq!(
                error.kind(),
                io::ErrorKind::TimedOut,
                "no data yet is distinct from the peer closing the connection"
            );
            stream.deadline = Instant::now() + TIMEOUT;
        }

        for stream in [&mut client, &mut server] {
            stream.deadline = Instant::now();
            let error = stream
                .write(b"expired")
                .expect_err("an expired deadline must prevent even an immediately-ready write");
            assert_eq!(
                error.kind(),
                io::ErrorKind::TimedOut,
                "available pipe capacity does not bypass the exchange deadline"
            );
            stream.deadline = Instant::now() + TIMEOUT;
        }

        client.write_all(b"request")?;
        server.deadline = Instant::now();
        let error = server
            .read(&mut [0])
            .expect_err("buffered request data must not bypass an expired exchange deadline");
        assert_eq!(
            error.kind(),
            io::ErrorKind::TimedOut,
            "ready server reads check the deadline"
        );
        server.deadline = Instant::now() + TIMEOUT;
        let mut request = [0; 7];
        server.read_exact(&mut request)?;
        assert_eq!(
            &request, b"request",
            "a pending request remains readable after an idle read"
        );
        server.write_all(b"response")?;
        client.deadline = Instant::now();
        let error = client
            .read(&mut [0])
            .expect_err("buffered response data must not bypass an expired exchange deadline");
        assert_eq!(
            error.kind(),
            io::ErrorKind::TimedOut,
            "ready client reads check the deadline"
        );
        client.deadline = Instant::now() + TIMEOUT;
        let mut response = [0; 8];
        client.read_exact(&mut response)?;
        assert_eq!(&response, b"response", "the client receives the complete response");

        drop(client);
        assert_eq!(server.read(&mut [0])?, 0, "closing the peer produces actual EOF");
        server.finish()?;
        drop(server);

        assert!(
            listener.accept()?.is_none(),
            "disconnecting rearms the listener before another client connects"
        );
        drop(connect(&path)?);
        let mut probe = listener
            .accept()?
            .expect("closed probes still count as client activity");
        assert_eq!(probe.read(&mut [0])?, 0, "an already-closed status probe produces EOF");
        drop(probe);
        assert!(
            listener.accept()?.is_none(),
            "the listener is rearmed after a status probe"
        );
        let _client = connect(&path)?;
        assert!(
            listener.accept()?.is_some(),
            "handling a probe releases the pipe for reuse"
        );
        Ok(())
    }

    #[test]
    fn pending_server_writes_cancel_at_the_exchange_deadline() -> gix_error::TestResult {
        let directory = gix_testtools::tempfile::tempdir()?;
        let path = directory.path().canonicalize()?.join("ipc");
        let mut listener = Listener::bind(&path)?;
        let client = connect(&path)?;
        let mut server = listener
            .accept()?
            .expect("the client connects before acceptance is armed");
        server.deadline = Instant::now() + Duration::from_millis(50);
        let error = server
            .write_all(&vec![1; 1024 * 1024])
            .expect_err("a client that does not read cannot retain a pending server write indefinitely");
        assert_eq!(
            error.kind(),
            io::ErrorKind::TimedOut,
            "server writes obey the exchange deadline"
        );
        assert!(
            !server.operation.is_pending(),
            "timeout observes cancellation completion before releasing the caller's buffer"
        );
        drop(server);
        drop(client);
        assert!(listener.accept()?.is_none(), "a cancelled exchange rearms acceptance");
        let _client = connect(&path)?;
        assert!(
            listener.accept()?.is_some(),
            "a cancelled exchange releases the pipe for reuse"
        );
        Ok(())
    }

    #[test]
    fn server_writes_complete_after_a_client_drains_more_than_the_pipe_buffer() -> gix_error::TestResult {
        let directory = gix_testtools::tempfile::tempdir()?;
        let path = directory.path().canonicalize()?.join("ipc");
        let mut listener = Listener::bind(&path)?;
        let wake = Wake::new()?;
        assert!(
            listener.accept()?.is_none(),
            "acceptance is armed before the client connects"
        );
        let client = std::thread::spawn(move || -> gix_error::TestResult<Vec<u8>> {
            let mut client = connect(&path)?;
            std::thread::sleep(Duration::from_millis(50));
            client.write_all(b"request")?;
            let mut output = vec![0; 1024 * 1024];
            client.read_exact(&mut output)?;
            Ok(output)
        });
        listener.wait(&wake, TIMEOUT)?;
        let mut server = listener.accept()?.expect("the client wakes the pending acceptance");
        let mut request = [0; 7];
        server.read_exact(&mut request)?;
        assert_eq!(
            &request, b"request",
            "a pending server read wakes when delayed client data arrives"
        );
        server.write_all(&vec![1; 1024 * 1024])?;
        server.finish()?;
        let output = client.join().expect("the native pipe client does not panic")?;
        assert!(
            output.iter().all(|byte| *byte == 1),
            "the complete response survives pipe-buffer backpressure"
        );
        Ok(())
    }
}
