use std::{
    fs::File,
    io::{self, Read, Write},
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle},
    },
    path::Path,
    time::{Duration, Instant},
};

use windows_sys::Win32::{
    Foundation::{
        ERROR_BROKEN_PIPE, ERROR_FILE_NOT_FOUND, ERROR_NO_DATA, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED,
        ERROR_PIPE_LISTENING, INVALID_HANDLE_VALUE,
    },
    Storage::FileSystem::{
        CreateFileW, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_GENERIC_READ, FILE_GENERIC_WRITE, OPEN_EXISTING,
        PIPE_ACCESS_DUPLEX, ReadFile,
    },
    System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_NOWAIT, PIPE_READMODE_BYTE,
        PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, SetNamedPipeHandleState, WaitNamedPipeW,
    },
};

const TIMEOUT: Duration = Duration::from_secs(5);

fn name(path: &Path) -> io::Result<Vec<u16>> {
    let path: Vec<_> = path.as_os_str().encode_wide().collect();
    if path.contains(&0) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "NUL in pipe name"));
    }
    Ok(crate::endpoint::pipe_name(path))
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
}
impl Listener {
    pub fn bind(path: &Path) -> io::Result<Self> {
        let name = name(path)?;
        // SAFETY: name is terminated and alive; default security belongs to the creating user.
        // FIRST_PIPE_INSTANCE enforces singleton ownership, and remote clients are rejected.
        let pipe = unsafe {
            CreateNamedPipeW(
                name.as_ptr(),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_NOWAIT | PIPE_REJECT_REMOTE_CLIENTS,
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
        })
    }

    pub fn accept(&mut self) -> io::Result<Option<Stream>> {
        // SAFETY: This is a live nonblocking pipe handle; no overlapped state is retained.
        if unsafe { ConnectNamedPipe(self.pipe.as_raw_handle(), std::ptr::null_mut()) } != 0 {
            // With PIPE_NOWAIT, success means a disconnected instance became available again;
            // an established client is reported as ERROR_PIPE_CONNECTED instead.
            return Ok(None);
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(ERROR_PIPE_LISTENING as i32) {
            return Ok(None);
        }
        if error.raw_os_error() == Some(ERROR_NO_DATA as i32) {
            // A connect-and-close status probe may disappear before accept begins.
            // SAFETY: Disconnect resets this still-owned instance for its next client.
            unsafe {
                DisconnectNamedPipe(self.pipe.as_raw_handle());
            }
            return Ok(None);
        }
        if error.raw_os_error() != Some(ERROR_PIPE_CONNECTED as i32) {
            return Err(error);
        }
        Ok(Some(Stream {
            file: self.pipe.try_clone()?,
            deadline: Instant::now() + TIMEOUT,
            server: true,
        }))
    }

    pub fn owns_endpoint(&self) -> bool {
        true
    }
}

pub(crate) fn connect(path: &Path) -> io::Result<Stream> {
    let name = name(path)?;
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
                server: false,
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
        if self.server {
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
    fn connected_idle_pipes_wait_for_data_instead_of_reporting_eof() -> gix_testtools::Result {
        let directory = gix_testtools::tempfile::tempdir()?;
        let path = directory.path().join("ipc");
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
        Ok(())
    }
}
