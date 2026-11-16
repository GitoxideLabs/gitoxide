#![cfg(windows)]
#![allow(unsafe_code)]

use std::{
    fs::File,
    io,
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle},
    },
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use bstr::ByteSlice;
use gix_fsmonitor::client::daemon::{Options, query};
use gix_testtools::TestResult;
use windows_sys::Win32::{
    Foundation::{
        ERROR_BROKEN_PIPE, ERROR_NO_DATA, ERROR_PIPE_CONNECTED, ERROR_PIPE_LISTENING, ERROR_PIPE_NOT_CONNECTED,
        INVALID_HANDLE_VALUE,
    },
    Storage::FileSystem::{FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX, ReadFile, WriteFile},
    System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_NOWAIT, PIPE_READMODE_BYTE,
        PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE,
    },
};

const TIMEOUT: Duration = Duration::from_millis(200);
const MOCK_LIFETIME: Duration = Duration::from_secs(3);

#[derive(Clone, Copy)]
enum Reply {
    Stalled,
    Fragmented,
    Successful,
    UnreadRequest,
}

struct Control {
    stop: Receiver<()>,
    deadline: Instant,
}

impl Control {
    fn wait(&self, duration: Duration) -> bool {
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return false;
        }
        matches!(
            self.stop.recv_timeout(duration.min(remaining)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ) && Instant::now() < self.deadline
    }
}

// Use native nonblocking server I/O, independently of the implementation under test. Every loop
// shares a hard deadline; disconnecting at that deadline also releases an unbounded client read/write.
struct Pipe(File);

impl Pipe {
    fn bind(endpoint: &Path) -> io::Result<Self> {
        let name: Vec<_> = super::pipe_name(endpoint)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: The terminated name lives through the call; this unique local endpoint has no
        // inherited handle. PIPE_NOWAIT applies to the server, not the client's blocking pipe mode.
        let handle = unsafe {
            CreateNamedPipeW(
                name.as_ptr(),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_NOWAIT | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                1024,
                1024,
                0,
                std::ptr::null(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: The successful constructor transfers this unique handle to File.
        Ok(Self(unsafe { File::from_raw_handle(handle) }))
    }

    fn accept(&self, control: &Control) -> io::Result<bool> {
        loop {
            // SAFETY: This synchronous, nonblocking pipe stays owned throughout the native call.
            if unsafe { ConnectNamedPipe(self.0.as_raw_handle(), std::ptr::null_mut()) } == 0 {
                match io::Error::last_os_error().raw_os_error() {
                    Some(code) if code == ERROR_PIPE_CONNECTED as i32 => return Ok(true),
                    Some(code) if code == ERROR_NO_DATA as i32 => return Ok(false),
                    Some(code) if code == ERROR_PIPE_LISTENING as i32 => {}
                    _ => return Err(io::Error::last_os_error()),
                }
            }
            // A successful initial nonblocking ConnectNamedPipe only makes the pipe available;
            // a subsequent ERROR_PIPE_CONNECTED proves the client actually opened it.
            if !control.wait(Duration::from_millis(2)) {
                return Ok(false);
            }
        }
    }

    fn read_request(&self, control: &Control) -> io::Result<bool> {
        let mut request = [0; 17];
        let mut offset = 0;
        while offset < request.len() {
            let mut read = 0;
            // SAFETY: This synchronous nonblocking call cannot retain either output buffer.
            if unsafe {
                ReadFile(
                    self.0.as_raw_handle(),
                    request[offset..].as_mut_ptr(),
                    (request.len() - offset) as u32,
                    &mut read,
                    std::ptr::null_mut(),
                )
            } != 0
            {
                offset += read as usize;
                if offset == request.len() {
                    break;
                }
            } else {
                match io::Error::last_os_error().raw_os_error() {
                    Some(code) if code == ERROR_NO_DATA as i32 => {}
                    Some(code) if code == ERROR_BROKEN_PIPE as i32 || code == ERROR_PIPE_NOT_CONNECTED as i32 => {
                        return Ok(false);
                    }
                    _ => return Err(io::Error::last_os_error()),
                }
            }
            if !control.wait(Duration::from_millis(2)) {
                return Ok(false);
            }
        }
        assert_eq!(
            &request, b"000dold-token0000",
            "the native blocking client sends a packetized token and terminating flush"
        );
        Ok(true)
    }

    fn write_byte(&self, byte: u8, control: &Control) -> io::Result<bool> {
        loop {
            let mut written = 0;
            // SAFETY: The synchronous nonblocking call does not retain the byte or output count.
            if unsafe { WriteFile(self.0.as_raw_handle(), &byte, 1, &mut written, std::ptr::null_mut()) } != 0 {
                if written == 1 {
                    return Ok(true);
                }
            } else {
                let error = io::Error::last_os_error();
                if matches!(error.raw_os_error(), Some(code) if code == ERROR_NO_DATA as i32 || code == ERROR_BROKEN_PIPE as i32 || code == ERROR_PIPE_NOT_CONNECTED as i32)
                {
                    return Ok(false);
                }
                return Err(error);
            }
            if !control.wait(Duration::from_millis(2)) {
                return Ok(false);
            }
        }
    }

    fn serve(self, reply: Reply, control: Control) -> io::Result<()> {
        if !self.accept(&control)? {
            return Ok(());
        }
        if matches!(reply, Reply::UnreadRequest) {
            // Leave the small inbound buffer undrained so a large client token must block in write.
            control.wait(MOCK_LIFETIME);
            return Ok(());
        }
        if !self.read_request(&control)? {
            return Ok(());
        }
        let response: &[u8] = match reply {
            Reply::Fragmented => b"0030xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx0000",
            Reply::Successful => b"000dnew-token0000",
            Reply::Stalled => b"",
            Reply::UnreadRequest => unreachable!("the undrained request was handled before reading"),
        };
        for &byte in response {
            // Each gap is far shorter than TIMEOUT, while the entire valid response takes longer.
            if !control.wait(Duration::from_millis(30)) || !self.write_byte(byte, &control)? {
                return Ok(());
            }
        }
        // Disconnect discards unread output. Keep the pipe open until query returns, or until the
        // hard cleanup deadline releases a client whose timeout implementation has regressed.
        control.wait(MOCK_LIFETIME);
        Ok(())
    }
}

impl Drop for Pipe {
    fn drop(&mut self) {
        // SAFETY: No native I/O outlives a nonblocking call; the live owned handle closes next.
        unsafe {
            DisconnectNamedPipe(self.0.as_raw_handle());
        }
    }
}

struct MockDaemon {
    endpoint: PathBuf,
    _directory: gix_testtools::tempfile::TempDir,
    stop: Sender<()>,
    server: Option<JoinHandle<io::Result<()>>>,
}

impl MockDaemon {
    fn new(reply: Reply) -> io::Result<Self> {
        let directory = gix_testtools::tempfile::tempdir()?;
        let endpoint = directory.path().canonicalize()?.join("fsmonitor.ipc");
        let pipe = Pipe::bind(&endpoint)?;
        let (stop, receiver) = mpsc::channel();
        let control = Control {
            stop: receiver,
            deadline: Instant::now() + MOCK_LIFETIME,
        };
        let server = thread::spawn(move || pipe.serve(reply, control));
        Ok(Self {
            endpoint,
            _directory: directory,
            stop,
            server: Some(server),
        })
    }

    fn query(&self, token: &[u8], timeout: Option<Duration>) -> gix_error::Result<Vec<u8>> {
        query(
            &self.endpoint,
            token.as_bstr(),
            || panic!("the test owns an already-created native endpoint; startup must not run"),
            Options {
                connect_retries: 0,
                connect_retry_delay: Duration::ZERO,
                socket_timeout: timeout,
                ..Default::default()
            },
        )
    }

    fn finish(mut self) -> TestResult {
        let _ = self.stop.send(());
        self.server
            .take()
            .expect("the mock server has not been joined")
            .join()
            .expect("the native mock server must finish without panicking")?;
        Ok(())
    }
}

impl Drop for MockDaemon {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(server) = self.server.take() {
            let _ = server.join();
        }
    }
}

fn assert_timeout(result: gix_error::Result<Vec<u8>>, elapsed: Duration, invariant: &str) {
    let error = result.expect_err(invariant);
    assert_eq!(
        error
            .downcast_any_ref::<io::Error>()
            .expect("query preserves its native I/O error through contextual wrapping")
            .kind(),
        io::ErrorKind::TimedOut,
        "{invariant}"
    );
    assert!(
        elapsed < Duration::from_secs(1),
        "the configured deadline, not the mock's three-second cleanup, must release the client: {invariant}"
    );
}

#[test]
fn stalled_response_read_times_out() -> TestResult {
    let daemon = MockDaemon::new(Reply::Stalled)?;
    let started = Instant::now();
    let result = daemon.query(b"old-token", Some(TIMEOUT));
    let elapsed = started.elapsed();
    daemon.finish()?;
    assert_timeout(
        result,
        elapsed,
        "an idle connected pipe obeys the blocking read deadline",
    );
    Ok(())
}

#[test]
fn continuous_fragmented_response_shares_one_exchange_deadline() -> TestResult {
    let daemon = MockDaemon::new(Reply::Fragmented)?;
    let started = Instant::now();
    let result = daemon.query(b"old-token", Some(TIMEOUT));
    let elapsed = started.elapsed();
    daemon.finish()?;
    assert_timeout(
        result,
        elapsed,
        "continuous progress cannot reset the deadline for each header or payload fragment",
    );
    Ok(())
}

#[test]
fn token_larger_than_pipe_buffer_times_out_in_write() -> TestResult {
    let daemon = MockDaemon::new(Reply::UnreadRequest)?;
    let token = vec![b'x'; 1024 * 1024];
    let started = Instant::now();
    let result = daemon.query(&token, Some(TIMEOUT));
    let elapsed = started.elapsed();
    daemon.finish()?;
    assert_timeout(
        result,
        elapsed,
        "an undrained server buffer cannot retain a borrowed request buffer beyond the write deadline",
    );
    Ok(())
}

#[test]
fn disabled_timeout_accepts_a_slow_successful_response() -> TestResult {
    let daemon = MockDaemon::new(Reply::Successful)?;
    let started = Instant::now();
    let result = daemon.query(b"old-token", None);
    let elapsed = started.elapsed();
    daemon.finish()?;
    assert_eq!(result?, b"new-token", "None disables the cumulative exchange deadline");
    assert!(
        elapsed > TIMEOUT,
        "the successful response takes longer than the enabled test timeout"
    );
    Ok(())
}

#[test]
fn zero_timeout_fails_without_startup_or_waiting_for_io() -> TestResult {
    let daemon = MockDaemon::new(Reply::Stalled)?;
    let started = Instant::now();
    let result = daemon.query(b"old-token", Some(Duration::ZERO));
    let elapsed = started.elapsed();
    daemon.finish()?;
    assert_timeout(
        result,
        elapsed,
        "zero is an immediate timeout even though an available native pipe can connect immediately",
    );
    Ok(())
}
