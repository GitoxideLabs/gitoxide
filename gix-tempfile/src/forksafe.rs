use std::path::Path;

use tempfile::{NamedTempFile, TempPath};

use crate::{AutoRemove, handle};

/// Move `source` over `destination` while preserving its persistent Windows file attributes.
///
/// `tempfile` creates named temporary files with `FILE_ATTRIBUTE_TEMPORARY`. Its Windows persistence
/// implementation changes all attributes to `FILE_ATTRIBUTE_NORMAL` before moving the file, which also discards
/// attributes deliberately applied after creation, notably `FILE_ATTRIBUTE_READONLY`. `MoveFileExW()` additionally
/// refuses to replace a read-only destination. This function handles both constraints while keeping `source`
/// recoverable for a retry when the move fails.
///
/// # I/O per attempt
///
/// The ordinary successful path makes four Win32 filesystem calls:
///
/// 1. Read and remember all attributes of `source`.
/// 2. Remove only `FILE_ATTRIBUTE_TEMPORARY` from `source`; if that was its sole attribute, use
///    `FILE_ATTRIBUTE_NORMAL` because Windows doesn't accept an empty attribute set.
/// 3. Read `destination`'s attributes. `INVALID_FILE_ATTRIBUTES` is allowed here because a missing destination is a
///    valid target; `MoveFileExW()` will report other lookup failures authoritatively.
/// 4. Move `source` over `destination` with `MOVEFILE_REPLACE_EXISTING`. The moved file retains every original source
///    attribute except `FILE_ATTRIBUTE_TEMPORARY`.
///
/// A read-only destination adds one attribute write before the move to clear `FILE_ATTRIBUTE_READONLY`. On failure,
/// attribute writes restore the exact source attributes and, if changed, the exact destination attributes. Restoration
/// is best-effort so its errors don't hide the operation that failed; the original error is returned. Each outer retry
/// repeats this sequence.
///
/// These are metadata and namespace operations: file contents aren't read or copied, and cross-volume moves fail because
/// `MOVEFILE_COPY_ALLOWED` isn't requested. This function doesn't flush contents or retry transient errors; its caller
/// owns those responsibilities. Once the move succeeds, it disables `tempfile`'s cleanup before returning so dropping
/// the wrapper can't act on the stale source path.
#[cfg(windows)]
fn persist_windows(source: &mut TempfileOrTemppath, destination: &Path) -> std::io::Result<()> {
    use std::{iter, os::windows::ffi::OsStrExt};

    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_READONLY, FILE_ATTRIBUTE_TEMPORARY, GetFileAttributesW,
        INVALID_FILE_ATTRIBUTES, MOVEFILE_REPLACE_EXISTING, MoveFileExW, SetFileAttributesW,
    };

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(iter::once(0)).collect()
    }

    fn usable_attributes(attributes: u32) -> u32 {
        if attributes == 0 {
            FILE_ATTRIBUTE_NORMAL
        } else {
            attributes
        }
    }

    let source_path = wide(match &*source {
        TempfileOrTemppath::Tempfile(file) => file.path(),
        TempfileOrTemppath::Temppath(path) => path,
    });
    let destination_path = wide(destination);
    // SAFETY: Both paths are encoded as NUL-terminated UTF-16 strings and remain alive for all calls.
    #[expect(unsafe_code)]
    unsafe {
        let source_attributes = GetFileAttributesW(source_path.as_ptr());
        if source_attributes == INVALID_FILE_ATTRIBUTES {
            return Err(std::io::Error::last_os_error());
        }
        let persisted_attributes = usable_attributes(source_attributes & !FILE_ATTRIBUTE_TEMPORARY);
        if SetFileAttributesW(source_path.as_ptr(), persisted_attributes) == 0 {
            return Err(std::io::Error::last_os_error());
        }

        let destination_attributes = GetFileAttributesW(destination_path.as_ptr());
        let destination_was_readonly =
            destination_attributes != INVALID_FILE_ATTRIBUTES && destination_attributes & FILE_ATTRIBUTE_READONLY != 0;
        if destination_was_readonly
            && SetFileAttributesW(
                destination_path.as_ptr(),
                usable_attributes(destination_attributes & !FILE_ATTRIBUTE_READONLY),
            ) == 0
        {
            let err = std::io::Error::last_os_error();
            let _ = SetFileAttributesW(source_path.as_ptr(), source_attributes);
            return Err(err);
        }

        if MoveFileExW(
            source_path.as_ptr(),
            destination_path.as_ptr(),
            MOVEFILE_REPLACE_EXISTING,
        ) != 0
        {
            match source {
                TempfileOrTemppath::Tempfile(file) => file.disable_cleanup(true),
                TempfileOrTemppath::Temppath(path) => path.disable_cleanup(true),
            }
            return Ok(());
        }

        let err = std::io::Error::last_os_error();
        let _ = SetFileAttributesW(source_path.as_ptr(), source_attributes);
        if destination_was_readonly {
            let _ = SetFileAttributesW(destination_path.as_ptr(), destination_attributes);
        }
        Err(err)
    }
}

enum TempfileOrTemppath {
    Tempfile(NamedTempFile),
    Temppath(TempPath),
}

pub(crate) struct ForksafeTempfile {
    inner: TempfileOrTemppath,
    cleanup: AutoRemove,
    pub owning_process_id: u32,
}

impl ForksafeTempfile {
    pub fn new(tempfile: NamedTempFile, cleanup: AutoRemove, mode: handle::Mode) -> Self {
        use handle::Mode::*;
        ForksafeTempfile {
            inner: match mode {
                Closed => TempfileOrTemppath::Temppath(tempfile.into_temp_path()),
                Writable => TempfileOrTemppath::Tempfile(tempfile),
            },
            cleanup,
            owning_process_id: std::process::id(),
        }
    }
}

impl ForksafeTempfile {
    pub fn as_mut_tempfile(&mut self) -> Option<&mut NamedTempFile> {
        match &mut self.inner {
            TempfileOrTemppath::Tempfile(file) => Some(file),
            TempfileOrTemppath::Temppath(_) => None,
        }
    }
    pub fn close(self) -> Self {
        if let TempfileOrTemppath::Tempfile(file) = self.inner {
            ForksafeTempfile {
                inner: TempfileOrTemppath::Temppath(file.into_temp_path()),
                cleanup: self.cleanup,
                owning_process_id: self.owning_process_id,
            }
        } else {
            self
        }
    }
    pub fn persist(self, path: impl AsRef<Path>) -> Result<Option<std::fs::File>, (std::io::Error, Self)> {
        self.persist_inner(path.as_ref())
    }

    #[cfg(windows)]
    fn persist_inner(mut self, path: &Path) -> Result<Option<std::fs::File>, (std::io::Error, Self)> {
        /// Maximum number of attempts for Windows file locking issues.
        /// Matches libgit2's default retry count.
        const MAX_ATTEMPTS: usize = 10;
        /// Delay between retry attempts in milliseconds.
        /// Matches libgit2's retry delay.
        const RETRY_DELAY_MS: u64 = 5;

        fn should_retry(err: &std::io::Error) -> bool {
            use std::io::ErrorKind;
            // Access denied (ERROR_ACCESS_DENIED = 5) or sharing violation (ERROR_SHARING_VIOLATION = 32)
            // are the common errors when external processes like antivirus or file watchers hold the file.
            matches!(err.kind(), ErrorKind::PermissionDenied) || err.raw_os_error() == Some(32)
            // ERROR_SHARING_VIOLATION
        }

        for attempt in 0..MAX_ATTEMPTS {
            match persist_windows(&mut self.inner, path) {
                Ok(()) => {
                    return Ok(match self.inner {
                        TempfileOrTemppath::Tempfile(file) => Some(file.into_file()),
                        TempfileOrTemppath::Temppath(_) => None,
                    });
                }
                Err(err) if attempt + 1 < MAX_ATTEMPTS && should_retry(&err) => {
                    std::thread::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS));
                }
                Err(err) => return Err((err, self)),
            }
        }
        unreachable!("loop always returns")
    }

    #[cfg(not(windows))]
    fn persist_inner(mut self, path: &Path) -> Result<Option<std::fs::File>, (std::io::Error, Self)> {
        match self.inner {
            TempfileOrTemppath::Tempfile(file) => match file.persist(path) {
                Ok(file) => Ok(Some(file)),
                Err(err) => Err((err.error, {
                    self.inner = TempfileOrTemppath::Tempfile(err.file);
                    self
                })),
            },
            TempfileOrTemppath::Temppath(temppath) => match temppath.persist(path) {
                Ok(_) => Ok(None),
                Err(err) => Err((err.error, {
                    self.inner = TempfileOrTemppath::Temppath(err.path);
                    self
                })),
            },
        }
    }

    pub fn into_temppath(self) -> TempPath {
        match self.inner {
            TempfileOrTemppath::Tempfile(file) => file.into_temp_path(),
            TempfileOrTemppath::Temppath(path) => path,
        }
    }
    pub fn into_tempfile(self) -> Option<NamedTempFile> {
        match self.inner {
            TempfileOrTemppath::Tempfile(file) => Some(file),
            TempfileOrTemppath::Temppath(_) => None,
        }
    }
    pub fn drop_impl(self) {
        let file_path = match self.inner {
            TempfileOrTemppath::Tempfile(file) => file.path().to_owned(),
            TempfileOrTemppath::Temppath(path) => path.to_path_buf(),
        };
        let parent_directory = file_path.parent().expect("every tempfile has a parent directory");
        self.cleanup.execute_best_effort(parent_directory);
    }

    pub fn drop_without_deallocation(self) {
        use std::io::Write;
        let temppath = match self.inner {
            TempfileOrTemppath::Tempfile(file) => {
                let (mut file, temppath) = file.into_parts();
                file.flush().ok();
                temppath
            }
            TempfileOrTemppath::Temppath(path) => path,
        };
        std::fs::remove_file(&temppath).ok();
        std::mem::forget(
            self.cleanup
                .execute_best_effort(temppath.parent().expect("every file has a directory")),
        );
        std::mem::forget(temppath); // leak memory to prevent deallocation
    }
}
