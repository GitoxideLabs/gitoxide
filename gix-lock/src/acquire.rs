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
#[expect(missing_docs)]
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
        Self::acquire_to_update_resource_inner(at_path.as_ref(), mode, boundary_directory, &keep_resource)
    }

    /// Like [`acquire_to_update_resource()`](File::acquire_to_update_resource), but allows to set filesystem permissions using `make_permissions`.
    pub fn acquire_to_update_resource_with_permissions(
        at_path: impl AsRef<Path>,
        mode: Fail,
        boundary_directory: Option<PathBuf>,
        make_permissions: impl Fn() -> std::fs::Permissions,
    ) -> Result<File, Error> {
        let (resource_path, lock_path, handle) = lock_with_mode(
            at_path.as_ref(),
            mode,
            boundary_directory,
            &keep_resource,
            &|p, d, c| gix_tempfile::writable_at_with_permissions(p, d, c, make_permissions()),
        )?;
        Ok(File {
            inner: handle,
            lock_path,
            resource_path,
        })
    }

    /// Like [`acquire_to_update_resource()`](File::acquire_to_update_resource), but follows symlinks at `at_path`
    /// before creating the lock file, matching Git's default lock-file behavior.
    pub fn acquire_to_update_resource_following_symlinks(
        at_path: impl AsRef<Path>,
        mode: Fail,
        boundary_directory: Option<PathBuf>,
    ) -> Result<File, Error> {
        Self::acquire_to_update_resource_inner(at_path.as_ref(), mode, boundary_directory, &resolve_symlink)
    }

    /// Like [`acquire_to_update_resource_following_symlinks()`](File::acquire_to_update_resource_following_symlinks),
    /// but adjusts the permissions that remain after applying the process umask.
    pub fn acquire_to_update_resource_following_symlinks_with_permissions(
        at_path: impl AsRef<Path>,
        mode: Fail,
        boundary_directory: Option<PathBuf>,
        adjust_permissions: impl Fn(std::fs::Permissions) -> std::fs::Permissions,
    ) -> Result<File, Error> {
        let mut lock = Self::acquire_to_update_resource_following_symlinks(at_path, mode, boundary_directory)?;
        lock.with_mut(|file| {
            let permissions = adjust_permissions(file.metadata()?.permissions());
            file.set_permissions(permissions)
        })?;
        Ok(lock)
    }

    fn acquire_to_update_resource_inner(
        at_path: &Path,
        mode: Fail,
        boundary_directory: Option<PathBuf>,
        resolve_resource: &dyn Fn(&Path) -> PathBuf,
    ) -> Result<File, Error> {
        let (resource_path, lock_path, handle) =
            lock_with_mode(at_path, mode, boundary_directory, resolve_resource, &|p, d, c| {
                if let Some(permissions) = default_permissions() {
                    gix_tempfile::writable_at_with_permissions(p, d, c, permissions)
                } else {
                    gix_tempfile::writable_at(p, d, c)
                }
            })?;
        Ok(File {
            inner: handle,
            lock_path,
            resource_path,
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
        let (resource_path, lock_path, handle) = lock_with_mode(
            at_path.as_ref(),
            mode,
            boundary_directory,
            &keep_resource,
            &|p, d, c| {
                if let Some(permissions) = default_permissions() {
                    gix_tempfile::mark_at_with_permissions(p, d, c, permissions)
                } else {
                    gix_tempfile::mark_at(p, d, c)
                }
            },
        )?;
        Ok(Marker {
            created_from_file: false,
            inner: handle,
            lock_path,
            resource_path,
        })
    }

    /// Like [`acquire_to_hold_resource()`](Marker::acquire_to_hold_resource), but allows to set filesystem permissions using `make_permissions`.
    pub fn acquire_to_hold_resource_with_permissions(
        at_path: impl AsRef<Path>,
        mode: Fail,
        boundary_directory: Option<PathBuf>,
        make_permissions: impl Fn() -> std::fs::Permissions,
    ) -> Result<Marker, Error> {
        let (resource_path, lock_path, handle) = lock_with_mode(
            at_path.as_ref(),
            mode,
            boundary_directory,
            &keep_resource,
            &|p, d, c| gix_tempfile::mark_at_with_permissions(p, d, c, make_permissions()),
        )?;
        Ok(Marker {
            created_from_file: false,
            inner: handle,
            lock_path,
            resource_path,
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

fn resolve_symlink(path: &Path) -> PathBuf {
    let mut path = path.to_owned();
    for _ in 0..5 {
        let Ok(destination) = std::fs::read_link(&path) else {
            break;
        };
        path = if destination.is_absolute() {
            destination
        } else {
            path.parent().unwrap_or_else(|| Path::new("")).join(destination)
        };
    }
    path
}

fn keep_resource(path: &Path) -> PathBuf {
    path.to_owned()
}

fn lock_with_mode<T>(
    resource: &Path,
    mode: Fail,
    boundary_directory: Option<PathBuf>,
    resolve_resource: &dyn Fn(&Path) -> PathBuf,
    try_lock: &dyn Fn(&Path, ContainingDirectory, AutoRemove) -> std::io::Result<T>,
) -> Result<(PathBuf, PathBuf, T), Error> {
    use std::io::ErrorKind::*;
    let (directory, cleanup) = dir_cleanup(boundary_directory);
    let try_once = |cleanup| {
        let resource_path = resolve_resource(resource);
        let lock_path = add_lock_suffix(&resource_path);
        match try_lock(&lock_path, directory, cleanup) {
            Ok(value) => Ok((resource_path, lock_path, value)),
            Err(err) => Err((err, resource_path)),
        }
    };
    let mut attempts = 1;
    match mode {
        Fail::Immediately => try_once(cleanup),
        Fail::AfterDurationWithBackoff(time) => {
            for wait in backoff::Quadratic::default_with_random().until_no_remaining(time) {
                attempts += 1;
                match try_once(cleanup.clone()) {
                    Ok(value) => return Ok(value),
                    #[cfg(windows)]
                    Err((err, _)) if err.kind() == AlreadyExists || err.kind() == PermissionDenied => {
                        std::thread::sleep(wait);
                        continue;
                    }
                    #[cfg(not(windows))]
                    Err((err, _)) if err.kind() == AlreadyExists => {
                        std::thread::sleep(wait);
                        continue;
                    }
                    Err((err, _)) => return Err(Error::from(err)),
                }
            }
            try_once(cleanup)
        }
    }
    .map_err(|(err, resource_path)| match err.kind() {
        AlreadyExists => Error::PermanentlyLocked {
            resource_path,
            mode,
            attempts,
        },
        _ => Error::Io(err),
    })
}

fn add_lock_suffix(resource_path: &Path) -> PathBuf {
    let mut lock_path = resource_path.as_os_str().to_owned();
    lock_path.push(DOT_LOCK_SUFFIX);
    lock_path.into()
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

    #[test]
    #[cfg(unix)]
    fn add_lock_suffix_preserves_non_utf8_bytes() {
        use std::os::unix::ffi::OsStrExt;

        let path = Path::new(std::ffi::OsStr::from_bytes(b"hello.\xff"));
        assert_eq!(add_lock_suffix(path).as_os_str().as_bytes(), b"hello.\xff.lock");
    }

    #[test]
    fn resource_is_resolved_on_each_lock_attempt() {
        let resolutions = std::cell::Cell::new(0);
        let resolve = |_: &Path| {
            let current = resolutions.get();
            resolutions.set(current + 1);
            PathBuf::from(if current == 0 { "first" } else { "second" })
        };
        let (resource_path, lock_path, ()) = lock_with_mode(
            Path::new("link"),
            Fail::AfterDurationWithBackoff(Duration::ZERO),
            None,
            &resolve,
            &|path, _, _| {
                if path == Path::new("first.lock") {
                    Err(std::io::ErrorKind::AlreadyExists.into())
                } else {
                    Ok(())
                }
            },
        )
        .expect("the second target can be locked");

        assert_eq!(resolutions.get(), 2, "the resource is resolved before every attempt");
        assert_eq!(resource_path, Path::new("second"), "the locked target is retained");
        assert_eq!(lock_path, Path::new("second.lock"), "the lock follows that target");
    }
}
