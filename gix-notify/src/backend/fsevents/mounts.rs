//! A fresh, caller-owned mount snapshot for strict FSEvents certification.

use std::{
    collections::BTreeMap,
    ffi::OsString,
    io,
    mem::{MaybeUninit, size_of},
    os::unix::ffi::OsStringExt,
    path::{Path, PathBuf},
};

/// Reject nonlocal mounts intersecting logical coverage, before reducing native roots.
///
/// `watches` must use physical absolute paths. Each watched path must additionally pass
/// `statfs(MNT_LOCAL)`; this checks mounted descendants that statfs(watch-root) cannot see.
pub(super) fn has_nonlocal_coverage(watches: &BTreeMap<PathBuf, bool>) -> io::Result<bool> {
    for mount in mount_snapshot()? {
        if mount.f_flags & libc::MNT_LOCAL as u32 != 0 {
            continue;
        }
        let end = mount.f_mntonname.iter().position(|byte| *byte == 0).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "unterminated mount point in getfsstat result",
            )
        })?;
        let path = PathBuf::from(OsString::from_vec(
            mount.f_mntonname[..end].iter().map(|byte| *byte as u8).collect(),
        ));
        // Resolve the local mount-point parent, not the remote filesystem itself. In
        // particular, /tmp and /var mount aliases must match /private/tmp and /private/var.
        let physical = match (path.parent(), path.file_name()) {
            (Some(parent), Some(name)) => parent.canonicalize()?.join(name),
            _ => path.clone(),
        };
        if watches
            .iter()
            .any(|(watch, recursive)| intersects(watch, *recursive, &path) || intersects(watch, *recursive, &physical))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn intersects(watch: &Path, recursive: bool, mount: &Path) -> bool {
    watch.starts_with(mount)
        || if recursive {
            mount.starts_with(watch)
        } else {
            mount.parent() == Some(watch)
        }
}

fn mount_snapshot() -> io::Result<Vec<libc::statfs>> {
    // A saturated buffer is ambiguous: the table may have grown between count and read.
    // Retry a few times with spare capacity; never certify an arbitrarily truncated table.
    for _ in 0..3 {
        let count = unsafe {
            // SAFETY: a null buffer asks only for the required entry count.
            libc::getfsstat(std::ptr::null_mut(), 0, libc::MNT_NOWAIT)
        };
        if count < 0 {
            return Err(io::Error::last_os_error());
        }
        let capacity = usize::try_from(count)
            .map_err(io::Error::other)?
            .checked_add(16)
            .filter(|capacity| *capacity <= 65_536)
            .ok_or_else(|| io::Error::other("mount table exceeds the certification budget"))?;
        let bytes = capacity
            .checked_mul(size_of::<libc::statfs>())
            .and_then(|bytes| libc::c_int::try_from(bytes).ok())
            .ok_or_else(|| io::Error::other("mount table buffer size overflows getfsstat"))?;
        let mut buffer: Vec<MaybeUninit<libc::statfs>> = Vec::with_capacity(capacity);
        buffer.resize_with(capacity, MaybeUninit::uninit);
        let filled = unsafe {
            // SAFETY: the buffer provides `bytes` writable bytes with statfs alignment.
            // NOWAIT avoids refreshing filesystem statistics over remote transports.
            libc::getfsstat(buffer.as_mut_ptr().cast(), bytes, libc::MNT_NOWAIT)
        };
        if filled < 0 {
            return Err(io::Error::last_os_error());
        }
        let filled = usize::try_from(filled).map_err(io::Error::other)?;
        if filled >= capacity {
            continue;
        }
        return Ok(buffer
            .into_iter()
            .take(filled)
            .map(|entry| unsafe {
                // SAFETY: getfsstat initialized precisely the returned prefix of the buffer.
                entry.assume_init()
            })
            .collect());
    }
    Err(io::Error::other(
        "mount table kept changing while checking notification coverage",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_intersections_use_path_components_and_recursion() {
        assert!(intersects(Path::new("/repo"), true, Path::new("/repo/deep/remote")));
        assert!(!intersects(Path::new("/repo"), false, Path::new("/repo/deep/remote")));
        assert!(intersects(Path::new("/repo"), false, Path::new("/repo/remote")));
        assert!(intersects(
            Path::new("/repo/remote/file"),
            false,
            Path::new("/repo/remote")
        ));
        assert!(!intersects(Path::new("/repo"), true, Path::new("/repository/remote")));
    }

    #[test]
    fn getfsstat_returns_owned_initialized_entries() -> io::Result<()> {
        let mounts = mount_snapshot()?;
        assert!(!mounts.is_empty(), "a running macOS system has a root mount");
        assert!(
            mounts.iter().all(|mount| mount.f_mntonname.contains(&0)),
            "every initialized entry has a bounded mount-point spelling"
        );
        Ok(())
    }
}
