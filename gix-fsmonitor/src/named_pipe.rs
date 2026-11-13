//! Blocking named-pipe I/O with cancellable, event-based overlapped operations.

use std::{
    cell::UnsafeCell,
    ffi::OsStr,
    fs::File,
    io::{self, Read, Write},
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    time::{Duration, Instant},
};

use windows_sys::Win32::{
    Foundation::{
        ERROR_BROKEN_PIPE, ERROR_IO_INCOMPLETE, ERROR_IO_PENDING, ERROR_NOT_FOUND, ERROR_OPERATION_ABORTED,
        INVALID_HANDLE_VALUE, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT,
    },
    Storage::FileSystem::{
        CreateFileW, FILE_FLAG_OVERLAPPED, FILE_GENERIC_READ, FILE_GENERIC_WRITE, OPEN_EXISTING, ReadFile, WriteFile,
    },
    System::{
        IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED},
        Threading::{CreateEventW, INFINITE, ResetEvent, WaitForSingleObject},
    },
};

pub(crate) fn milliseconds(timeout: Duration) -> u32 {
    // Round up so submillisecond deadlines do not spin; reserve INFINITE for truly unbounded waits.
    timeout.as_nanos().div_ceil(1_000_000).min((INFINITE - 1) as u128) as u32
}

pub(crate) fn event(manual_reset: bool) -> io::Result<OwnedHandle> {
    // SAFETY: No name or security pointers are supplied, and the returned event is uniquely owned.
    let handle = unsafe { CreateEventW(std::ptr::null(), manual_reset.into(), 0, std::ptr::null()) };
    if handle.is_null() {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: A successful constructor transfers ownership of this unique event handle.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

fn deadline(timeout: Option<Duration>) -> io::Result<Option<Instant>> {
    timeout
        .map(|timeout| {
            Instant::now()
                .checked_add(timeout)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "named-pipe timeout is too large"))
        })
        .transpose()
}

fn check_deadline(deadline: Option<Instant>) -> io::Result<()> {
    if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
        return Err(io::Error::new(io::ErrorKind::TimedOut, "IPC exchange timed out"));
    }
    Ok(())
}

pub(crate) struct Operation {
    // Own the handle used for submission, polling, and cancellation, including during Drop.
    file: File,
    event: OwnedHandle,
    // Windows retains and mutates the status block until completion, even after cancellation.
    overlapped: Box<UnsafeCell<OVERLAPPED>>,
    pending: bool,
}

impl Operation {
    pub(crate) fn new(file: File) -> io::Result<Self> {
        Ok(Self {
            file,
            event: event(true)?,
            overlapped: Box::new(UnsafeCell::new(OVERLAPPED::default())),
            pending: false,
        })
    }

    #[cfg(feature = "server")]
    pub(crate) fn file(&self) -> &File {
        &self.file
    }

    #[cfg(feature = "server")]
    pub(crate) fn event(&self) -> &OwnedHandle {
        &self.event
    }

    #[cfg(all(test, feature = "server"))]
    pub(crate) fn is_pending(&self) -> bool {
        self.pending
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

    fn result(&mut self, wait: bool) -> io::Result<Option<u32>> {
        let mut bytes = 0;
        // SAFETY: The owned pipe and stable status block stay live until completion is observed.
        if unsafe {
            GetOverlappedResult(
                self.file.as_raw_handle(),
                self.overlapped.get(),
                &mut bytes,
                wait.into(),
            )
        } != 0
        {
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

    pub(crate) fn cancel(&mut self) -> io::Result<()> {
        if !self.pending {
            return Ok(());
        }
        // SAFETY: Cancellation requests completion; buffers and status stay alive until drained.
        let error = if unsafe { CancelIoEx(self.file.as_raw_handle(), self.overlapped.get()) } == 0 {
            Some(io::Error::last_os_error()).filter(|error| error.raw_os_error() != Some(ERROR_NOT_FOUND as i32))
        } else {
            None
        };
        // Cancellation can race completion. Never release native pointers merely because
        // CancelIoEx returned; event-based I/O must still observe the final operation status.
        while self.pending {
            match self.result(true) {
                Ok(_) => {}
                Err(error) if error.raw_os_error() == Some(ERROR_OPERATION_ABORTED as i32) => {}
                Err(error) => return Err(error),
            }
        }
        error.map_or(Ok(()), Err)
    }

    // Only the native calls below can submit borrowed buffers. Keep this closure private:
    // every call must submit at most one operation and cannot panic after submission.
    fn execute(&mut self, deadline: Option<Instant>, submit: impl FnOnce(*mut OVERLAPPED) -> i32) -> io::Result<u32> {
        // Check before submission, even when the pipe can make continuous immediate progress.
        check_deadline(deadline)?;
        let overlapped = self.prepare()?;
        if submit(overlapped) == 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(ERROR_IO_PENDING as i32) {
                return Err(error);
            }
        }
        self.pending = true;
        loop {
            if let Some(bytes) = self.result(false)? {
                check_deadline(deadline)?;
                return Ok(bytes);
            }
            if let Err(error) = check_deadline(deadline) {
                self.cancel()?;
                return Err(error);
            }
            let timeout = deadline.map_or(INFINITE, |deadline| {
                milliseconds(deadline.saturating_duration_since(Instant::now()))
            });
            // SAFETY: The event, status, and caller's buffer remain live throughout the wait.
            let status = unsafe { WaitForSingleObject(self.event.as_raw_handle(), timeout) };
            match status {
                // Recheck completion and the deadline after waking, including long waits
                // split into chunks to reserve INFINITE for disabled timeouts only.
                status if status == WAIT_OBJECT_0 || status == WAIT_TIMEOUT => {}
                _ => {
                    let error = if status == WAIT_FAILED {
                        io::Error::last_os_error()
                    } else {
                        io::Error::other("named-pipe operation wait failed")
                    };
                    self.cancel()?;
                    return Err(error);
                }
            }
        }
    }

    pub(crate) fn read(&mut self, buf: &mut [u8], deadline: Option<Instant>) -> io::Result<usize> {
        check_deadline(deadline)?;
        if buf.is_empty() {
            return Ok(0);
        }
        let handle = self.file.as_raw_handle();
        // SAFETY: execute observes completion or drains cancellation before releasing buf.
        match self.execute(deadline, |overlapped| unsafe {
            ReadFile(
                handle,
                buf.as_mut_ptr(),
                buf.len().min(u32::MAX as usize) as u32,
                std::ptr::null_mut(),
                overlapped,
            )
        }) {
            Ok(bytes) => Ok(bytes as usize),
            Err(error) if error.raw_os_error() == Some(ERROR_BROKEN_PIPE as i32) => Ok(0),
            Err(error) => Err(error),
        }
    }

    pub(crate) fn write(&mut self, buf: &[u8], deadline: Option<Instant>) -> io::Result<usize> {
        check_deadline(deadline)?;
        if buf.is_empty() {
            return Ok(0);
        }
        let handle = self.file.as_raw_handle();
        // SAFETY: execute observes completion or drains cancellation before releasing buf.
        self.execute(deadline, |overlapped| unsafe {
            WriteFile(
                handle,
                buf.as_ptr(),
                buf.len().min(u32::MAX as usize) as u32,
                std::ptr::null_mut(),
                overlapped,
            )
        })
        .map(|bytes| bytes as usize)
    }

    #[cfg(feature = "server")]
    pub(crate) fn accept(&mut self) -> io::Result<bool> {
        use windows_sys::Win32::{
            Foundation::{ERROR_NO_DATA, ERROR_PIPE_CONNECTED},
            System::Pipes::ConnectNamedPipe,
        };

        let result = if self.pending {
            self.result(false)
        } else {
            let overlapped = self.prepare()?;
            // SAFETY: The owned pipe retains its stable status until completion or Drop drains it.
            if unsafe { ConnectNamedPipe(self.file.as_raw_handle(), overlapped) } != 0 {
                Ok(Some(0))
            } else {
                let error = io::Error::last_os_error();
                if error.raw_os_error() == Some(ERROR_IO_PENDING as i32) {
                    self.pending = true;
                    Ok(None)
                } else {
                    Err(error)
                }
            }
        };
        match result {
            Ok(bytes) => Ok(bytes.is_some()),
            // A client may connect before acceptance, or close its status probe before accept.
            Err(error) if matches!(error.raw_os_error(), Some(code) if code == ERROR_PIPE_CONNECTED as i32 || code == ERROR_NO_DATA as i32) => {
                Ok(true)
            }
            Err(error) => Err(error),
        }
    }
}

impl Drop for Operation {
    fn drop(&mut self) {
        let _ = self.cancel();
    }
}

pub(crate) struct Stream {
    operation: Operation,
    deadline: Option<Instant>,
}

impl Stream {
    pub(crate) fn connect(pipe: &OsStr, timeout: Option<Duration>) -> io::Result<Self> {
        let deadline = deadline(timeout)?;
        check_deadline(deadline)?;
        let mut name: Vec<_> = pipe.encode_wide().collect();
        if name.contains(&0) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "NUL in pipe name"));
        }
        name.push(0);
        check_deadline(deadline)?;
        // SAFETY: The name is terminated and live; the handle is not inherited. Overlapped mode
        // permits cancellation of reads/writes. Opening a busy local pipe returns immediately;
        // callers retain their existing retry policy rather than waiting here with WaitNamedPipeW.
        let handle = unsafe {
            CreateFileW(
                name.as_ptr(),
                FILE_GENERIC_READ | FILE_GENERIC_WRITE,
                0,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                std::ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: A successful open transfers this unique handle to File.
        let operation = Operation::new(unsafe { File::from_raw_handle(handle) })?;
        check_deadline(deadline)?;
        Ok(Self { operation, deadline })
    }

    pub(crate) fn set_timeout(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        self.deadline = deadline(timeout)?;
        Ok(())
    }

    #[cfg(feature = "server")]
    pub(crate) fn into_operation(self) -> Operation {
        self.operation
    }
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.operation.read(buf, self.deadline)
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.operation.write(buf, self.deadline)
    }

    fn flush(&mut self) -> io::Result<()> {
        // No FlushFileBuffers: it waits for the peer to drain output and is not cancellable.
        check_deadline(self.deadline)
    }
}
