//! This module contains a `Metadata` implementation that must be used instead of `std::fs::Metadata` to assure
//! that the `ctime` information is populated exactly like the one in `git`, which wouldn't be the case on unix.
#![allow(clippy::useless_conversion)] // on some MacOOS conversions are required, but on linux usually not.
#![allow(clippy::unnecessary_cast)]

// it's allowed for good measure, in case there are systems that use different types for that.
use std::{path::Path, time::SystemTime};

/// A structure to partially mirror [`std::fs::Metadata`].
#[cfg(not(windows))]
pub struct Metadata(rustix::fs::Stat);

#[cfg(windows)]
/// A structure to partially mirror [`std::fs::Metadata`].
pub struct Metadata(std::fs::Metadata);

/// Lifecycle
impl Metadata {
    /// Obtain the metadata at `path` without following symlinks.
    pub fn from_path_no_follow(path: &Path) -> Result<Self, std::io::Error> {
        #[cfg(not(windows))]
        {
            rustix::fs::lstat(path).map(Metadata).map_err(Into::into)
        }
        #[cfg(windows)]
        path.symlink_metadata().map(Metadata)
    }

    /// Obtain the metadata at `path` without following symlinks.
    pub fn from_file(file: &std::fs::File) -> Result<Self, std::io::Error> {
        #[cfg(not(windows))]
        {
            rustix::fs::fstat(file).map(Metadata).map_err(Into::into)
        }
        #[cfg(windows)]
        file.metadata().map(Metadata)
    }
}

/// Access
#[allow(clippy::len_without_is_empty)]
impl Metadata {
    /// Return true if the metadata belongs to a directory
    pub fn is_dir(&self) -> bool {
        #[cfg(not(windows))]
        {
            (self.0.st_mode as u32 & libc::S_IFMT as u32) == libc::S_IFDIR as u32
        }
        #[cfg(windows)]
        self.0.is_dir()
    }

    /// Return the time at which the underlying file was modified.
    pub fn modified(&self) -> Option<SystemTime> {
        #[cfg(not(windows))]
        {
            #[cfg(not(any(target_os = "aix", target_os = "hurd")))]
            let seconds = self.0.st_mtime;
            #[cfg(any(target_os = "aix", target_os = "hurd"))]
            let seconds = self.0.st_mtim.tv_sec;

            #[cfg(not(any(target_os = "aix", target_os = "hurd")))]
            let nanoseconds = self.0.st_mtime_nsec;
            #[cfg(any(target_os = "aix", target_os = "hurd"))]
            let nanoseconds = self.0.st_mtim.tv_nsec;

            // All operating systems treat the seconds as offset from unix epoch, hence it must
            // be signed in order to deal with dates before epoch.
            // Rustix seems to think this value is u64, but we fix it here for now.
            let seconds = seconds as i64;
            system_time_from_secs_nanos(seconds, nanoseconds.try_into().ok()?)
        }
        #[cfg(windows)]
        self.0.modified().ok()
    }

    /// Return the time at which the underlying file was created.
    ///
    /// Note that this differs from [`std::fs::Metadata::created()`] which would return
    /// the inode birth time, which is notably different to what `git` does.
    pub fn created(&self) -> Option<SystemTime> {
        #[cfg(not(windows))]
        {
            #[cfg(not(any(target_os = "aix", target_os = "hurd")))]
            let seconds = self.0.st_ctime;
            #[cfg(any(target_os = "aix", target_os = "hurd"))]
            let seconds = self.0.st_ctim.tv_sec;

            #[cfg(not(any(target_os = "aix", target_os = "hurd")))]
            let nanoseconds = self.0.st_ctime_nsec;
            #[cfg(any(target_os = "aix", target_os = "hurd"))]
            let nanoseconds = self.0.st_ctim.tv_nsec;

            // All operating systems treat the seconds as offset from unix epoch, hence it must
            // be signed in order to deal with dates before epoch.
            // Rustix seems to think this value is u64, but we fix it here for now.
            let seconds = seconds as i64;
            system_time_from_secs_nanos(seconds, nanoseconds.try_into().ok()?)
        }
        #[cfg(windows)]
        self.0.created().ok()
    }

    /// Return the size of the file in bytes.
    pub fn len(&self) -> u64 {
        #[cfg(not(windows))]
        {
            self.0.st_size as u64
        }
        #[cfg(windows)]
        self.0.len()
    }

    /// Return the device id on which the file is located, or 0 on windows.
    pub fn dev(&self) -> u64 {
        #[cfg(not(windows))]
        {
            self.0.st_dev as u64
        }
        #[cfg(windows)]
        0
    }

    /// Return the inode id tracking the file, or 0 on windows.
    pub fn ino(&self) -> u64 {
        #[cfg(not(windows))]
        {
            self.0.st_ino as u64
        }
        #[cfg(windows)]
        0
    }

    /// Return the user-id of the file or 0 on windows.
    pub fn uid(&self) -> u32 {
        #[cfg(not(windows))]
        {
            self.0.st_uid as u32
        }
        #[cfg(windows)]
        0
    }

    /// Return the group-id of the file or 0 on windows.
    pub fn gid(&self) -> u32 {
        #[cfg(not(windows))]
        {
            self.0.st_gid as u32
        }
        #[cfg(windows)]
        0
    }

    /// Return `true` if the file's executable bit is set, or `false` on windows.
    pub fn is_executable(&self) -> bool {
        #[cfg(not(windows))]
        {
            (self.0.st_mode as u32 & libc::S_IFMT as u32) == libc::S_IFREG as u32
                && self.0.st_mode as u32 & libc::S_IXUSR as u32 == libc::S_IXUSR as u32
        }
        #[cfg(windows)]
        gix_fs::is_executable(&self.0)
    }

    /// Return `true` if the file's is a symbolic link.
    pub fn is_symlink(&self) -> bool {
        #[cfg(not(windows))]
        {
            (self.0.st_mode as u32 & libc::S_IFMT as u32) == libc::S_IFLNK as u32
        }
        #[cfg(windows)]
        self.0.is_symlink()
    }

    /// Return `true` if this is a regular file, executable or not.
    pub fn is_file(&self) -> bool {
        #[cfg(not(windows))]
        {
            (self.0.st_mode as u32 & libc::S_IFMT as u32) == libc::S_IFREG as u32
        }
        #[cfg(windows)]
        self.0.is_file()
    }
}

#[cfg(not(windows))]
fn system_time_from_secs_nanos(secs: i64, nanos: i32) -> Option<SystemTime> {
    // Copied from https://github.com/rust-lang/rust at a8ece1190bf6b340175bc5b688e52bd29924f483, MIT licensed, and adapted.
    // On Apple OS, dates before epoch are represented differently than on other
    // Unix platforms: e.g. 1/10th of a second before epoch is represented as `seconds=-1`
    // and `nanoseconds=100_000_000` on other platforms, but is `seconds=0` and
    // `nanoseconds=-900_000_000` on Apple OS.
    //
    // To compensate, we first detect this special case by checking if both
    // seconds and nanoseconds are in range, and then correct the value for seconds
    // and nanoseconds to match the common unix representation.
    //
    // Please note that Apple OS nonetheless accepts the standard unix format when
    // setting file times, which makes this compensation round-trippable and generally
    // transparent.
    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "tvos", target_os = "watchos"))]
    let (secs, nanos) = if (secs <= 0 && secs > i64::MIN) && (nanos < 0 && nanos > -1_000_000_000) {
        (secs - 1, nanos + 1_000_000_000)
    } else {
        (secs, nanos)
    };
    let d = std::time::Duration::new(secs.abs_diff(0), nanos.try_into().ok()?);
    Some(if secs < 0 {
        std::time::UNIX_EPOCH - d
    } else {
        std::time::UNIX_EPOCH + d
    })
}
