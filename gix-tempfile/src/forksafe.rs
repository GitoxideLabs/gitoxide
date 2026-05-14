use std::path::Path;

use tempfile::{NamedTempFile, TempPath};

use crate::{AutoRemove, handle};

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

        match self.inner {
            TempfileOrTemppath::Tempfile(file) => {
                let mut current_file = file;
                for attempt in 0..MAX_ATTEMPTS {
                    match current_file.persist(path) {
                        Ok(file) => return Ok(Some(file)),
                        Err(err) if attempt + 1 < MAX_ATTEMPTS && should_retry(&err.error) => {
                            std::thread::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS));
                            current_file = err.file;
                        }
                        Err(err) => {
                            return Err((err.error, {
                                self.inner = TempfileOrTemppath::Tempfile(err.file);
                                self
                            }));
                        }
                    }
                }
                unreachable!("loop always returns")
            }
            TempfileOrTemppath::Temppath(temppath) => {
                let mut current_path = temppath;
                for attempt in 0..MAX_ATTEMPTS {
                    match current_path.persist(path) {
                        Ok(_) => return Ok(None),
                        Err(err) if attempt + 1 < MAX_ATTEMPTS && should_retry(&err.error) => {
                            std::thread::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS));
                            current_path = err.path;
                        }
                        Err(err) => {
                            return Err((err.error, {
                                self.inner = TempfileOrTemppath::Temppath(err.path);
                                self
                            }));
                        }
                    }
                }
                unreachable!("loop always returns")
            }
        }
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
