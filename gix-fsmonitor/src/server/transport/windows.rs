use std::{
    cell::UnsafeCell,
    fs::File,
    io::{self, Read, Write},
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::Path,
    time::{Duration, Instant},
};

use windows_sys::Win32::{
    Foundation::{
        ERROR_BROKEN_PIPE, ERROR_FILE_NOT_FOUND, ERROR_IO_INCOMPLETE, ERROR_IO_PENDING, ERROR_NO_DATA, ERROR_NOT_FOUND,
        ERROR_OPERATION_ABORTED, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED, INVALID_HANDLE_VALUE, WAIT_FAILED,
        WAIT_OBJECT_0, WAIT_TIMEOUT,
    },
    Storage::FileSystem::{
        CreateFileW, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
        OPEN_EXISTING, PIPE_ACCESS_DUPLEX, ReadFile, WriteFile,
    },
    System::{
        IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED},
        Pipes::{
            ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_NOWAIT, PIPE_READMODE_BYTE,
            PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT, SetNamedPipeHandleState, WaitNamedPipeW,
        },
        Threading::{CreateEventW, ResetEvent, SetEvent, WaitForMultipleObjects, WaitForSingleObject},
    },
};

const TIMEOUT: Duration = Duration::from_secs(5);

fn milliseconds(timeout: Duration) -> u32 {
    // Round up so submillisecond deadlines do not spin; reserve INFINITE for truly unbounded waits.
    timeout.as_millis().saturating_add(1).min((u32::MAX - 1) as u128) as u32
}

fn event(manual_reset: bool) -> io::Result<OwnedHandle> {
    // SAFETY: No name or security pointers are supplied, and the returned event is uniquely owned.
    let handle = unsafe { CreateEventW(std::ptr::null(), manual_reset.into(), 0, std::ptr::null()) };
    if handle.is_null() {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: A successful constructor transfers ownership of this unique event handle.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

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

struct Operation {
    event: OwnedHandle,
    // Windows retains and mutates the status block until completion, even after cancellation.
    overlapped: Box<UnsafeCell<OVERLAPPED>>,
    pending: bool,
}

impl Operation {
    fn new() -> io::Result<Self> {
        Ok(Self {
            event: event(true)?,
            overlapped: Box::new(UnsafeCell::new(OVERLAPPED::default())),
            pending: false,
        })
    }

    fn prepare(&mut self) -> io::Result<*mut OVERLAPPED> {
        if self.pending {
            return Err(io::Error::other("a named-pipe operation is already pending"));
        }
        // SAFETY: The event is live and no operation still accesses this stable status allocation.
        if unsafe { ResetEvent(self.event.as_raw_handle()) } == 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: The previous operation completed before reuse; UnsafeCell permits kernel writes.
        unsafe {
            self.overlapped.get().write(OVERLAPPED {
                hEvent: self.event.as_raw_handle(),
                ..OVERLAPPED::default()
            });
        }
        Ok(self.overlapped.get())
    }

    fn result(&mut self, file: &File, wait: bool) -> io::Result<Option<u32>> {
        let mut bytes = 0;
        // SAFETY: The pipe and its stable status block remain live until completion is observed.
        if unsafe { GetOverlappedResult(file.as_raw_handle(), self.overlapped.get(), &mut bytes, wait.into()) } != 0 {
            self.pending = false;
            return Ok(Some(bytes));
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(ERROR_IO_INCOMPLETE as i32) {
            return Ok(None);
        }
        self.pending = false;
        Err(error)
    }

    fn cancel(&mut self, file: &File) -> io::Result<()> {
        if !self.pending {
            return Ok(());
        }
        // SAFETY: Cancellation requests completion; the buffers and status block stay alive below.
        let error = if unsafe { CancelIoEx(file.as_raw_handle(), self.overlapped.get()) } == 0 {
            Some(io::Error::last_os_error()).filter(|error| error.raw_os_error() != Some(ERROR_NOT_FOUND as i32))
        } else {
            None
        };
        // Observe completion even if cancellation raced an already finished operation. Event-based
        // I/O needs no completion-port packet, but cancellation alone cannot release native pointers.
        while self.pending {
            match self.result(file, true) {
                Ok(_) => {}
                Err(error) if error.raw_os_error() == Some(ERROR_OPERATION_ABORTED as i32) => {}
                Err(error) => return Err(error),
            }
        }
        error.map_or(Ok(()), Err)
    }

    fn execute(
        &mut self,
        file: &File,
        deadline: Instant,
        submit: impl FnOnce(*mut OVERLAPPED) -> i32,
    ) -> io::Result<u32> {
        let overlapped = self.prepare()?;
        if submit(overlapped) == 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(ERROR_IO_PENDING as i32) {
                return Err(error);
            }
        }
        self.pending = true;
        if let Some(bytes) = self.result(file, false)? {
            return Ok(bytes);
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        // SAFETY: The event and native allocations stay alive throughout the bounded wait.
        let status = unsafe { WaitForSingleObject(self.event.as_raw_handle(), milliseconds(remaining)) };
        if status == WAIT_OBJECT_0
            && let Some(bytes) = self.result(file, true)?
        {
            return Ok(bytes);
        }
        let error = if status == WAIT_TIMEOUT {
            io::Error::new(io::ErrorKind::TimedOut, "IPC exchange timed out")
        } else if status == WAIT_FAILED {
            io::Error::last_os_error()
        } else {
            io::Error::other("named-pipe operation wait failed")
        };
        self.cancel(file)?;
        Err(error)
    }
}

fn name(path: &Path) -> io::Result<Vec<u16>> {
    let path: Vec<_> = path.as_os_str().encode_wide().collect();
    if path.contains(&0) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "NUL in pipe name"));
    }
    Ok(super::pipe_name(path))
}

fn mode(file: &File, value: u32) -> io::Result<()> {
    // SAFETY: The handle remains owned by file and the mode pointer is valid for this call.
    if unsafe { SetNamedPipeHandleState(file.as_raw_handle(), &value, std::ptr::null(), std::ptr::null()) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub(crate) struct Listener {
    pipe: File,
    connection: Operation,
}
impl Listener {
    pub fn bind(path: &Path) -> io::Result<Self> {
        let name = name(path)?;
        // SAFETY: name is terminated and alive; default security belongs to the creating user.
        // FIRST_PIPE_INSTANCE enforces singleton ownership, and remote clients are rejected.
        let pipe = unsafe {
            CreateNamedPipeW(
                name.as_ptr(),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE | FILE_FLAG_OVERLAPPED,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                65536,
                65536,
                5000,
                std::ptr::null(),
            )
        };
        if pipe == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: The successful constructor transfers this unique handle to File.
        Ok(Self {
            pipe: unsafe { File::from_raw_handle(pipe) },
            connection: Operation::new()?,
        })
    }

    pub fn accept(&mut self) -> io::Result<Option<Stream>> {
        let result = if self.connection.pending {
            self.connection.result(&self.pipe, false)
        } else {
            let overlapped = self.connection.prepare()?;
            // SAFETY: This live asynchronous pipe retains its stable status block until completion.
            if unsafe { ConnectNamedPipe(self.pipe.as_raw_handle(), overlapped) } != 0 {
                Ok(Some(0))
            } else {
                let error = io::Error::last_os_error();
                if error.raw_os_error() == Some(ERROR_IO_PENDING as i32) {
                    self.connection.pending = true;
                    Ok(None)
                } else {
                    Err(error)
                }
            }
        };
        match result {
            Ok(None) => return Ok(None),
            Ok(Some(_)) => {}
            // A client may connect before ConnectNamedPipe, or close its status probe before accept.
            Err(error) if matches!(error.raw_os_error(), Some(code) if code == ERROR_PIPE_CONNECTED as i32 || code == ERROR_NO_DATA as i32) =>
                {}
            Err(error) => return Err(error),
        }
        Ok(Some(Stream {
            file: self.pipe.try_clone()?,
            deadline: Instant::now() + TIMEOUT,
            operation: Some(Operation::new()?),
        }))
    }

    pub fn owns_endpoint(&self) -> bool {
        true
    }

    pub fn wait(&self, wake: &Wake, timeout: Duration) -> io::Result<()> {
        let handles = [self.connection.event.as_raw_handle(), wake.event.as_raw_handle()];
        // SAFETY: Both events remain owned and live for the duration of this wait. The wake event
        // auto-resets only when selected, so a simultaneous connection cannot consume its signal.
        match unsafe { WaitForMultipleObjects(handles.len() as u32, handles.as_ptr(), 0, milliseconds(timeout)) } {
            WAIT_OBJECT_0 | WAIT_TIMEOUT => Ok(()),
            status if status == WAIT_OBJECT_0 + 1 => Ok(()),
            WAIT_FAILED => Err(io::Error::last_os_error()),
            _ => Err(io::Error::other("filesystem monitor event wait failed")),
        }
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        let _ = self.connection.cancel(&self.pipe);
    }
}

pub(crate) fn connect(path: &Path) -> io::Result<Stream> {
    let name = name(&crate::normalize_endpoint(path)?)?;
    let deadline = Instant::now() + TIMEOUT;
    loop {
        // SAFETY: name is terminated and alive; no handle is inherited by child processes.
        let handle = unsafe {
            CreateFileW(
                name.as_ptr(),
                FILE_GENERIC_READ | FILE_GENERIC_WRITE,
                0,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            )
        };
        if handle != INVALID_HANDLE_VALUE {
            // SAFETY: The successful open transfers this unique handle to File.
            let file = unsafe { File::from_raw_handle(handle) };
            mode(&file, PIPE_READMODE_BYTE | PIPE_NOWAIT)?;
            return Ok(Stream {
                file,
                deadline,
                operation: None,
            });
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(ERROR_FILE_NOT_FOUND as i32) {
            return Err(error);
        }
        if error.raw_os_error() != Some(ERROR_PIPE_BUSY as i32) {
            return Err(error);
        }
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "named pipe stayed busy"))?;
        // SAFETY: name is terminated and remains alive for the bounded wait.
        unsafe {
            WaitNamedPipeW(name.as_ptr(), remaining.as_millis().clamp(1, 5000) as u32);
        }
    }
}

pub(crate) struct Stream {
    file: File,
    deadline: Instant,
    operation: Option<Operation>,
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
    fn wait(&self) -> io::Result<()> {
        if Instant::now() >= self.deadline {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "IPC exchange timed out"));
        }
        std::thread::sleep(Duration::from_millis(1));
        Ok(())
    }
}
impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if let Some(operation) = &mut self.operation {
            // SAFETY: execute observes completion or drains cancellation before releasing buf.
            return match operation.execute(&self.file, self.deadline, |overlapped| unsafe {
                ReadFile(
                    self.file.as_raw_handle(),
                    buf.as_mut_ptr(),
                    buf.len().min(u32::MAX as usize) as u32,
                    std::ptr::null_mut(),
                    overlapped,
                )
            }) {
                Ok(bytes) => Ok(bytes as usize),
                Err(error) if error.raw_os_error() == Some(ERROR_BROKEN_PIPE as i32) => Ok(0),
                Err(error) => Err(error),
            };
        }
        loop {
            let mut read = 0;
            // File::read maps ERROR_NO_DATA to EOF, hiding a connected pipe that is only idle.
            // SAFETY: The handle is synchronous and live, and both output buffers remain valid
            // for the call. No overlapped operation can outlive this borrow of buf.
            if unsafe {
                ReadFile(
                    self.file.as_raw_handle(),
                    buf.as_mut_ptr(),
                    buf.len().min(u32::MAX as usize) as u32,
                    &mut read,
                    std::ptr::null_mut(),
                )
            } != 0
            {
                return Ok(read as usize);
            }
            match io::Error::last_os_error() {
                err if err.raw_os_error() == Some(ERROR_NO_DATA as i32) => self.wait()?,
                err if err.raw_os_error() == Some(ERROR_BROKEN_PIPE as i32) => return Ok(0),
                err => return Err(err),
            }
        }
    }
}
impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if let Some(operation) = &mut self.operation {
            // SAFETY: execute observes completion or drains cancellation before releasing buf.
            return operation
                .execute(&self.file, self.deadline, |overlapped| unsafe {
                    WriteFile(
                        self.file.as_raw_handle(),
                        buf.as_ptr(),
                        buf.len().min(u32::MAX as usize) as u32,
                        std::ptr::null_mut(),
                        overlapped,
                    )
                })
                .map(|bytes| bytes as usize);
        }
        loop {
            match self.file.write(buf) {
                Ok(0) => self.wait()?,
                Err(err) if err.raw_os_error() == Some(ERROR_NO_DATA as i32) => self.wait()?,
                result => return result,
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Drop for Stream {
    fn drop(&mut self) {
        if let Some(operation) = &mut self.operation {
            let _ = operation.cancel(&self.file);
            // SAFETY: The duplicated pipe handle remains live until File is dropped afterward.
            unsafe {
                DisconnectNamedPipe(self.file.as_raw_handle());
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
            listener.connection.pending,
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

        client.write_all(b"request")?;
        let mut request = [0; 7];
        server.read_exact(&mut request)?;
        assert_eq!(
            &request, b"request",
            "a pending request remains readable after an idle read"
        );
        server.write_all(b"response")?;
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
            !server.operation.as_ref().expect("server I/O is asynchronous").pending,
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
