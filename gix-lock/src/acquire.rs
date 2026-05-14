use std::{
    fmt,
    path::{Path, PathBuf},
    time::Duration,
};

use gix_tempfile::{AutoRemove, ContainingDirectory};

use crate::{DOT_LOCK_SUFFIX, File, Marker, backoff};

/// Describe what to do if a lock cannot be obtained as it's already held elsewhere.
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Fail {
    /// Fail after the first unsuccessful attempt of obtaining a lock.
    #[default]
    Immediately,
    /// Retry after failure with quadratically longer sleep times to block the current thread.
    /// Fail once the given duration is exceeded, similar to [Fail::Immediately]
    AfterDurationWithBackoff(Duration),
}

impl fmt::Display for Fail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fail::Immediately => f.write_str("immediately"),
            Fail::AfterDurationWithBackoff(duration) => {
                write!(f, "after {:.02}s", duration.as_secs_f32())
            }
        }
    }
}

impl From<Duration> for Fail {
    fn from(value: Duration) -> Self {
        if value.is_zero() {
            Fail::Immediately
        } else {
            Fail::AfterDurationWithBackoff(value)
        }
    }
}

/// The error returned when acquiring a [`File`] or [`Marker`].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Another IO error occurred while obtaining the lock")]
    Io(#[from] std::io::Error),
    #[error(
        "The lock for resource '{resource_path}' could not be obtained {mode} after {attempts} attempt(s). The lockfile at '{resource_path}{}' might need manual deletion.",
        super::DOT_LOCK_SUFFIX
    )]
    PermanentlyLocked {
        resource_path: PathBuf,
        mode: Fail,
        attempts: usize,
    },
}

impl File {
    /// Create a writable lock file with failure `mode` whose content will eventually overwrite the given resource `at_path`.
    ///
    /// If `boundary_directory` is given, non-existing directories will be created automatically and removed in the case of
    /// a rollback. Otherwise the containing directory is expected to exist, even though the resource doesn't have to.
    ///
    /// Note that permissions will be set to `0o666`, which usually results in `0o644` after passing a default umask, on Unix systems.
    ///
    /// ### Warning of potential resource leak
    ///
    /// Please note that the underlying file will remain if destructors don't run, as is the case when interrupting the application.
    /// This results in the resource being locked permanently unless the lock file is removed by other means.
    /// See [the crate documentation](crate) for more information.
    pub fn acquire_to_update_resource(
        at_path: impl AsRef<Path>,
        mode: Fail,
        boundary_directory: Option<PathBuf>,
    ) -> Result<File, Error> {
        let (lock_path, handle) = lock_with_mode(at_path.as_ref(), mode, boundary_directory, &|p, d, c| {
            if let Some(permissions) = default_permissions() {
                gix_tempfile::writable_at_with_permissions(p, d, c, permissions)
            } else {
                gix_tempfile::writable_at(p, d, c)
            }
        })?;
        Ok(File {
            inner: handle,
            lock_path,
        })
    }

    /// Like [`acquire_to_update_resource()`](File::acquire_to_update_resource), but allows to set filesystem permissions using `make_permissions`.
    pub fn acquire_to_update_resource_with_permissions(
        at_path: impl AsRef<Path>,
        mode: Fail,
        boundary_directory: Option<PathBuf>,
        make_permissions: impl Fn() -> std::fs::Permissions,
    ) -> Result<File, Error> {
        let (lock_path, handle) = lock_with_mode(at_path.as_ref(), mode, boundary_directory, &|p, d, c| {
            gix_tempfile::writable_at_with_permissions(p, d, c, make_permissions())
        })?;
        Ok(File {
            inner: handle,
            lock_path,
        })
    }
}

impl Marker {
    /// Like [`acquire_to_update_resource()`](File::acquire_to_update_resource()) but _without_ the possibility to make changes
    /// and commit them.
    ///
    /// If `boundary_directory` is given, non-existing directories will be created automatically and removed in the case of
    /// a rollback.
    ///
    /// Note that permissions will be set to `0o666`, which usually results in `0o644` after passing a default umask, on Unix systems.
    ///
    /// ### Warning of potential resource leak
    ///
    /// Please note that the underlying file will remain if destructors don't run, as is the case when interrupting the application.
    /// This results in the resource being locked permanently unless the lock file is removed by other means.
    /// See [the crate documentation](crate) for more information.
    pub fn acquire_to_hold_resource(
        at_path: impl AsRef<Path>,
        mode: Fail,
        boundary_directory: Option<PathBuf>,
    ) -> Result<Marker, Error> {
        let (lock_path, handle) = lock_with_mode(at_path.as_ref(), mode, boundary_directory, &|p, d, c| {
            if let Some(permissions) = default_permissions() {
                gix_tempfile::mark_at_with_permissions(p, d, c, permissions)
            } else {
                gix_tempfile::mark_at(p, d, c)
            }
        })?;
        Ok(Marker {
            created_from_file: false,
            inner: handle,
            lock_path,
        })
    }

    /// Like [`acquire_to_hold_resource()`](Marker::acquire_to_hold_resource), but allows to set filesystem permissions using `make_permissions`.
    pub fn acquire_to_hold_resource_with_permissions(
        at_path: impl AsRef<Path>,
        mode: Fail,
        boundary_directory: Option<PathBuf>,
        make_permissions: impl Fn() -> std::fs::Permissions,
    ) -> Result<Marker, Error> {
        let (lock_path, handle) = lock_with_mode(at_path.as_ref(), mode, boundary_directory, &|p, d, c| {
            gix_tempfile::mark_at_with_permissions(p, d, c, make_permissions())
        })?;
        Ok(Marker {
            created_from_file: false,
            inner: handle,
            lock_path,
        })
    }
}

fn dir_cleanup(boundary: Option<PathBuf>) -> (ContainingDirectory, AutoRemove) {
    match boundary {
        None => (ContainingDirectory::Exists, AutoRemove::Tempfile),
        Some(boundary_directory) => (
            ContainingDirectory::CreateAllRaceProof(Default::default()),
            AutoRemove::TempfileAndEmptyParentDirectoriesUntil { boundary_directory },
        ),
    }
}

fn lock_with_mode<T>(
    resource: &Path,
    mode: Fail,
    boundary_directory: Option<PathBuf>,
    try_lock: &dyn Fn(&Path, ContainingDirectory, AutoRemove) -> std::io::Result<T>,
) -> Result<(PathBuf, T), Error> {
    use std::io::ErrorKind::*;
    let (directory, cleanup) = dir_cleanup(boundary_directory);
    let lock_path = add_lock_suffix(resource);
    let mut attempts = 1;
    match mode {
        Fail::Immediately => try_lock(&lock_path, directory, cleanup),
        Fail::AfterDurationWithBackoff(time) => {
            for wait in backoff::Quadratic::default_with_random().until_no_remaining(time) {
                attempts += 1;
                match try_lock(&lock_path, directory, cleanup.clone()) {
                    Ok(v) => return Ok((lock_path, v)),
                    #[cfg(windows)]
                    Err(err) if err.kind() == AlreadyExists || err.kind() == PermissionDenied => {
                        std::thread::sleep(wait);
                        continue;
                    }
                    #[cfg(not(windows))]
                    Err(err) if err.kind() == AlreadyExists => {
                        std::thread::sleep(wait);
                        continue;
                    }
                    Err(err) => return Err(Error::from(err)),
                }
            }
            try_lock(&lock_path, directory, cleanup)
        }
    }
    .map(|v| (lock_path, v))
    .map_err(|err| match err.kind() {
        AlreadyExists => Error::PermanentlyLocked {
            resource_path: resource.into(),
            mode,
            attempts,
        },
        _ => Error::Io(err),
    })
}

fn add_lock_suffix(resource_path: &Path) -> PathBuf {
    resource_path.with_extension(resource_path.extension().map_or_else(
        || DOT_LOCK_SUFFIX.chars().skip(1).collect(),
        |ext| format!("{}{}", ext.to_string_lossy(), DOT_LOCK_SUFFIX),
    ))
}

fn default_permissions() -> Option<std::fs::Permissions> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        Some(std::fs::Permissions::from_mode(0o666))
    }
    #[cfg(not(unix))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_lock_suffix_to_file_with_extension() {
        assert_eq!(add_lock_suffix(Path::new("hello.ext")), Path::new("hello.ext.lock"));
    }

    #[test]
    fn add_lock_suffix_to_file_without_extension() {
        assert_eq!(add_lock_suffix(Path::new("hello")), Path::new("hello.lock"));
    }
}
