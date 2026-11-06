use std::{
    cell::UnsafeCell,
    fs::File,
    io,
    mem::size_of,
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::{Path, PathBuf},
};

use windows_sys::Win32::{
    Foundation::{
        ERROR_INVALID_PARAMETER, ERROR_IO_PENDING, ERROR_NOT_FOUND, ERROR_NOT_SUPPORTED, INVALID_HANDLE_VALUE,
    },
    Globalization::{CSTR_EQUAL, CompareStringOrdinal},
    Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, CreateFileW, FILE_CASE_SENSITIVE_INFO, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAG_OVERLAPPED, FILE_LIST_DIRECTORY, FILE_NOTIFY_CHANGE_ATTRIBUTES, FILE_NOTIFY_CHANGE_CREATION,
        FILE_NOTIFY_CHANGE_DIR_NAME, FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE,
        FILE_NOTIFY_CHANGE_SECURITY, FILE_NOTIFY_CHANGE_SIZE, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ,
        FILE_SHARE_WRITE, FileCaseSensitiveInfo, GetFileInformationByHandle, GetFileInformationByHandleEx,
        OPEN_EXISTING, ReadDirectoryChangesW,
    },
    System::IO::{CancelIoEx, CreateIoCompletionPort, OVERLAPPED},
};

use crate::Event;

// Network directory notifications reject buffers larger than 64 KiB. u32 provides DWORD alignment.
const BUFFER_WORDS: usize = 16 * 1024;
const FILTER: u32 = FILE_NOTIFY_CHANGE_FILE_NAME
    | FILE_NOTIFY_CHANGE_DIR_NAME
    | FILE_NOTIFY_CHANGE_ATTRIBUTES
    | FILE_NOTIFY_CHANGE_SIZE
    | FILE_NOTIFY_CHANGE_LAST_WRITE
    | FILE_NOTIFY_CHANGE_CREATION
    | FILE_NOTIFY_CHANGE_SECURITY;

pub(super) struct Read {
    pub(super) path: PathBuf,
    scope: super::Scope,
    file: File,
    identity: (u32, u32, u32),
    case_sensitive: bool,
    // The kernel mutates these allocations while the worker inspects the remaining fields.
    buffer: Box<UnsafeCell<[u32; BUFFER_WORDS]>>,
    overlapped: Box<UnsafeCell<OVERLAPPED>>,
    pending: bool,
}

impl Read {
    pub(super) fn new(port: &OwnedHandle, key: usize, path: &Path, scope: super::Scope) -> io::Result<Self> {
        let file = open(path, true)?;
        let identity = identity(&file)?;
        let case_sensitive = case_sensitive(&file)?;
        // SAFETY: Both handles are live, and the nonzero key uniquely identifies this read's lifetime.
        if unsafe { CreateIoCompletionPort(file.as_raw_handle(), port.as_raw_handle(), key, 0) }.is_null() {
            return Err(io::Error::last_os_error());
        }
        let out = Self {
            path: path.to_owned(),
            scope,
            file,
            identity,
            case_sensitive,
            buffer: Box::new(UnsafeCell::new([0; BUFFER_WORDS])),
            overlapped: Box::new(UnsafeCell::new(OVERLAPPED::default())),
            pending: false,
        };
        out.check_root()?;
        Ok(out)
    }

    pub(super) fn start(&mut self) -> io::Result<()> {
        if self.pending {
            return Err(io::Error::other("a directory notification read is already pending"));
        }
        // SAFETY: The preceding completion was dequeued (or this is the first read), so no native
        // operation accesses the status block. Its stable allocation is exclusively owned here.
        unsafe { self.overlapped.get().write(OVERLAPPED::default()) };
        // SAFETY: No read is pending. Both boxed allocations remain at fixed addresses until
        // Streams dequeues the completion, including after cancellation. UnsafeCell permits the
        // kernel's writes while the worker reads unrelated fields through shared references.
        let success = unsafe {
            ReadDirectoryChangesW(
                self.file.as_raw_handle(),
                self.buffer.get().cast(),
                size_of::<[u32; BUFFER_WORDS]>() as u32,
                self.scope.recursive.into(),
                FILTER,
                std::ptr::null_mut(),
                self.overlapped.get(),
                None,
            )
        } != 0;
        if !success {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(ERROR_IO_PENDING as i32) {
                return Err(error);
            }
        }
        self.pending = true;
        Ok(())
    }

    pub(super) fn complete(&mut self) {
        self.pending = false;
    }

    pub(super) fn is_pending(&self) -> bool {
        self.pending
    }

    pub(super) fn cancel(&self) -> io::Result<()> {
        if self.pending {
            // SAFETY: The file and status block stay alive until Streams dequeues the completion.
            // Cancellation only requests completion; it never releases the kernel's pointers.
            if unsafe { CancelIoEx(self.file.as_raw_handle(), self.overlapped.get()) } == 0 {
                let error = io::Error::last_os_error();
                if error.raw_os_error() != Some(ERROR_NOT_FOUND as i32) {
                    return Err(error);
                }
            }
        }
        Ok(())
    }

    pub(super) fn events(&self, bytes: u32) -> io::Result<Vec<Event>> {
        if self.pending {
            return Err(io::Error::other("a directory notification read is still pending"));
        }
        if bytes == 0 || bytes as usize > size_of::<[u32; BUFFER_WORDS]>() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "directory notification buffer overflow",
            ));
        }
        // SAFETY: The completed read no longer accesses this initialized allocation; the byte
        // count has been bounded by its size. Decoding copies all paths before the next native read.
        let buffer = unsafe { std::slice::from_raw_parts(self.buffer.get().cast::<u8>(), bytes as usize) };
        let mut events = super::decode::events(&self.path, buffer)?;
        if !self.scope.directory {
            events.retain(|event| event.paths.iter().any(|path| self.accepts_file(path)));
        }
        Ok(events)
    }

    fn accepts_file(&self, path: &Path) -> bool {
        self.scope.files.iter().any(|file| {
            if self.case_sensitive {
                return file == path;
            }
            let file: Vec<_> = file.as_os_str().encode_wide().collect();
            let path: Vec<_> = path.as_os_str().encode_wide().collect();
            // SAFETY: These UTF-16 slices are live and Windows paths fit in i32 lengths. Ordinal
            // comparison follows Windows casing without lossy Unicode conversion or locale rules.
            unsafe {
                CompareStringOrdinal(file.as_ptr(), file.len() as i32, path.as_ptr(), path.len() as i32, 1)
                    == CSTR_EQUAL
            }
        })
    }

    pub(super) fn check_root(&self) -> io::Result<()> {
        if identity(&open(&self.path, false)?)? != self.identity {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "watched directory was replaced",
            ));
        }
        Ok(())
    }
}

fn open(path: &Path, notifications: bool) -> io::Result<File> {
    let path: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: The canonical path is NUL-terminated, and the returned handle is not inherited.
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            FILE_READ_ATTRIBUTES | if notifications { FILE_LIST_DIRECTORY } else { 0 },
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | if notifications { FILE_FLAG_OVERLAPPED } else { 0 },
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: CreateFileW transferred ownership of this unique handle.
    Ok(unsafe { File::from_raw_handle(handle) })
}

fn identity(file: &File) -> io::Result<(u32, u32, u32)> {
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: The handle and output structure remain valid for the synchronous query.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((info.dwVolumeSerialNumber, info.nFileIndexHigh, info.nFileIndexLow))
}

fn case_sensitive(file: &File) -> io::Result<bool> {
    let mut info = FILE_CASE_SENSITIVE_INFO::default();
    // SAFETY: The information class matches the output structure and its size.
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileCaseSensitiveInfo,
            (&raw mut info).cast(),
            size_of::<FILE_CASE_SENSITIVE_INFO>() as u32,
        )
    } == 0
    {
        let error = io::Error::last_os_error();
        return match error.raw_os_error().map(|code| code as u32) {
            Some(ERROR_INVALID_PARAMETER | ERROR_NOT_SUPPORTED) => Ok(false),
            _ => Err(error),
        };
    }
    // FILE_CS_FLAG_CASE_SENSITIVE_DIR, without requiring the unrelated SystemServices feature.
    Ok(info.Flags & 1 != 0)
}
