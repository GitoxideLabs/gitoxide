use std::{
    ffi::{OsStr, OsString},
    io,
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
        process::ExitStatusExt,
    },
    path::{Path, PathBuf},
    process::ExitStatus,
};

use windows_sys::Win32::{
    Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::Threading::{
        CREATE_NEW_PROCESS_GROUP, CreateProcessW, DETACHED_PROCESS, GetExitCodeProcess, INFINITE, PROCESS_INFORMATION,
        STARTUPINFOW, TerminateProcess, WaitForSingleObject,
    },
};

pub(crate) struct Child(OwnedHandle);

pub(crate) fn spawn(repo: &Path) -> io::Result<Child> {
    let executable = wide(std::env::current_exe()?.as_os_str())?;
    let directory = working_directory(repo)?;
    // The executable and worktree have dedicated API arguments, so paths need no argv quoting.
    let mut command: Vec<_> = "gix-fsmonitor run\0".encode_utf16().collect();
    let startup = STARTUPINFOW {
        cb: std::mem::size_of::<STARTUPINFOW>() as u32,
        ..Default::default()
    };
    let mut process = PROCESS_INFORMATION::default();
    // SAFETY: All strings are terminated and remain alive; the command buffer is writable.
    // No handles are inherited, including capture pipes belonging to any caller or ancestor.
    if unsafe {
        CreateProcessW(
            executable.as_ptr(),
            command.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP,
            std::ptr::null(),
            directory.as_ptr(),
            &startup,
            &mut process,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: Successful process creation transfers two distinct, valid handles to us.
    let process_handle = unsafe { OwnedHandle::from_raw_handle(process.hProcess) };
    let thread_handle = unsafe { OwnedHandle::from_raw_handle(process.hThread) };
    drop(thread_handle);
    Ok(Child(process_handle))
}

fn wide(value: &OsStr) -> io::Result<Vec<u16>> {
    let mut out: Vec<_> = value.encode_wide().collect();
    if out.contains(&0) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "NUL in process path"));
    }
    out.push(0);
    Ok(out)
}

fn working_directory(repo: &Path) -> io::Result<Vec<u16>> {
    // CreateProcessW cannot use a verbatim path as its working directory. Remove its prefix
    // only if the ordinary spelling still resolves to the same directory.
    let canonical = repo.canonicalize()?;
    let mut directory: Vec<_> = canonical.as_os_str().encode_wide().collect();
    let verbatim: Vec<_> = "\\\\?\\".encode_utf16().collect();
    let verbatim_unc: Vec<_> = "\\\\?\\UNC\\".encode_utf16().collect();
    if directory.starts_with(&verbatim_unc) {
        directory.splice(..verbatim_unc.len(), "\\\\".encode_utf16());
    } else if directory.starts_with(&verbatim) {
        directory.drain(..verbatim.len());
    }
    let directory = PathBuf::from(OsString::from_wide(&directory));
    if !directory.is_absolute() || directory.canonicalize()? != canonical {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "worktree has no equivalent non-verbatim process working directory",
        ));
    }
    wide(directory.as_os_str())
}

impl Child {
    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        // SAFETY: The process handle is owned and remains valid during this nonblocking wait.
        match unsafe { WaitForSingleObject(self.0.as_raw_handle(), 0) } {
            WAIT_TIMEOUT => Ok(None),
            WAIT_OBJECT_0 => self.status().map(Some),
            _ => Err(io::Error::last_os_error()),
        }
    }

    pub fn kill(&mut self) -> io::Result<()> {
        // SAFETY: This owned handle refers only to the daemon that we created.
        if unsafe { TerminateProcess(self.0.as_raw_handle(), 1) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        // SAFETY: The owned process handle remains valid until this wait completes.
        if unsafe { WaitForSingleObject(self.0.as_raw_handle(), INFINITE) } != WAIT_OBJECT_0 {
            return Err(io::Error::last_os_error());
        }
        self.status()
    }

    fn status(&self) -> io::Result<ExitStatus> {
        let mut code = 0;
        // SAFETY: The process handle is valid and the output points to writable stack storage.
        if unsafe { GetExitCodeProcess(self.0.as_raw_handle(), &mut code) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(ExitStatus::from_raw(code))
    }
}
