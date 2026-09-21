//! Mount-table changes are silent to inotify, so fences must verify their coverage separately.

use std::{
    collections::BTreeMap,
    ffi::OsString,
    io::{self, Read},
    os::unix::ffi::OsStringExt,
    path::{Path, PathBuf},
};

use gix_error::{ErrorExt, Result, ResultExt, message};

#[derive(Debug, Eq, PartialEq)]
pub(super) struct Snapshot {
    pub(super) supported: bool,
    records: Vec<Vec<u8>>,
}

/// Capture mounts containing a watched path or covered by a watched directory.
///
/// Keep complete records, including mount IDs, so replacement by another mount of the same
/// filesystem still invalidates a fence. Unknown filesystems cannot certify native delivery.
pub(super) fn snapshot(watches: &BTreeMap<PathBuf, bool>) -> Result<Snapshot> {
    let mut data = Vec::new();
    const MAX_BYTES: u64 = 8 * 1024 * 1024;
    std::fs::File::open("/proc/self/mountinfo")
        .and_then(|file| file.take(MAX_BYTES + 1).read_to_end(&mut data))
        .or_raise(|| message("could not read mount table for filesystem notification coverage"))?;
    if data.len() as u64 > MAX_BYTES {
        return Err(message("mount table exceeds the certification budget").raise());
    }
    from_bytes(&data, watches).or_raise(|| message("could not parse mount table for filesystem notification coverage"))
}

fn from_bytes(data: &[u8], watches: &BTreeMap<PathBuf, bool>) -> io::Result<Snapshot> {
    if data.is_empty() || !data.ends_with(b"\n") {
        return Err(invalid("empty or truncated mount table"));
    }
    let mut snapshot = Snapshot {
        supported: true,
        records: Vec::new(),
    };
    for record in data[..data.len() - 1].split(|byte| *byte == b'\n') {
        let (path, filesystem) = parse_record(record)?;
        if watches
            .iter()
            .any(|(watch, recursive)| intersects(watch, *recursive, &path))
        {
            snapshot.supported &= supports_notifications(filesystem);
            snapshot.records.push(record.to_vec());
        }
    }
    snapshot.records.sort_unstable();
    if !watches.is_empty() && snapshot.records.is_empty() {
        return Err(invalid("mount table does not describe watched paths"));
    }
    Ok(snapshot)
}

fn parse_record(record: &[u8]) -> io::Result<(PathBuf, &[u8])> {
    let fields: Vec<_> = record.split(|byte| *byte == b' ').collect();
    let separator = fields.iter().position(|field| *field == b"-");
    let separator = separator
        .filter(|index| *index >= 6 && *index + 4 == fields.len())
        .ok_or_else(|| invalid("invalid mountinfo field layout"))?;
    if fields.iter().any(|field| field.is_empty() || field.contains(&0)) {
        return Err(invalid("empty or NUL-containing mountinfo field"));
    }
    for index in [0, 1] {
        if !fields[index].iter().all(u8::is_ascii_digit) {
            return Err(invalid("invalid mountinfo identity"));
        }
    }
    let path = decode_path(fields[4])?;
    // The filesystem-specific root field can be opaque, such as `mnt:[4026532313]`
    // for an nsfs mount. Only the namespace's mount point must be an absolute path.
    if !path.is_absolute() {
        return Err(invalid("mountinfo mount point must be absolute"));
    }
    Ok((path, fields[separator + 1]))
}

fn decode_path(field: &[u8]) -> io::Result<PathBuf> {
    let mut bytes = Vec::with_capacity(field.len());
    let mut remaining = field;
    while let Some((&byte, tail)) = remaining.split_first() {
        if byte == b'\\' {
            let decoded = match tail.get(..3) {
                Some(b"040") => b' ',
                Some(b"011") => b'\t',
                Some(b"012") => b'\n',
                Some(b"134") => b'\\',
                _ => return Err(invalid("invalid mountinfo path escape")),
            };
            bytes.push(decoded);
            remaining = &tail[3..];
        } else {
            bytes.push(byte);
            remaining = tail;
        }
    }
    Ok(PathBuf::from(OsString::from_vec(bytes)))
}

fn intersects(watch: &Path, recursive: bool, mount: &Path) -> bool {
    watch.starts_with(mount)
        || if recursive {
            mount.starts_with(watch)
        } else {
            mount.parent() == Some(watch)
        }
}

fn supports_notifications(filesystem: &[u8]) -> bool {
    matches!(
        filesystem,
        b"ext2"
            | b"ext3"
            | b"ext4"
            | b"xfs"
            | b"btrfs"
            | b"tmpfs"
            | b"ramfs"
            | b"overlay"
            | b"f2fs"
            | b"jfs"
            | b"reiserfs"
            | b"nilfs2"
            | b"ubifs"
            | b"zfs"
    )
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::ffi::OsStrExt;

    #[test]
    fn escaped_paths_preserve_arbitrary_bytes() -> gix_error::TestResult {
        assert_eq!(
            decode_path(b"/space\\040tab\\011line\\012slash\\134raw-\xff")?
                .as_os_str()
                .as_bytes(),
            b"/space tab\tline\nslash\\raw-\xff",
            "mount paths use kernel escapes without Unicode conversion"
        );
        for invalid in [b"/incomplete\\".as_slice(), b"/short\\04", b"/unknown\\041"] {
            assert!(
                decode_path(invalid).is_err(),
                "malformed escapes cannot certify coverage"
            );
        }
        Ok(())
    }

    #[test]
    fn coverage_intersections_respect_components_and_recursion() {
        assert!(
            intersects(Path::new("/repo/file"), false, Path::new("/")),
            "containing mounts always affect coverage"
        );
        assert!(
            intersects(Path::new("/repo"), true, Path::new("/repo/deep/mount")),
            "recursive watches include all mounted descendants"
        );
        assert!(
            !intersects(Path::new("/repo"), false, Path::new("/repo/deep/mount")),
            "shallow watches exclude deeper mounts"
        );
        assert!(
            intersects(Path::new("/repo"), false, Path::new("/repo/mount")),
            "shallow watches include immediately mounted children"
        );
        assert!(
            !intersects(Path::new("/repo"), true, Path::new("/repository/mount")),
            "a shared string prefix is not an ancestor relationship"
        );
    }

    #[test]
    fn snapshots_reject_remote_coverage_and_detect_replaced_mounts() -> gix_error::TestResult {
        let table = b"1 0 8:1 / / rw - ext4 /dev/root rw
2 1 0:2 / /repo/deep/remote rw shared:3 - nfs server:/export rw
3 1 0:3 / /unrelated rw - fuse.unknown source rw
";
        let mut watches = BTreeMap::from([(PathBuf::from("/repo"), true)]);
        let recursive = from_bytes(table, &watches)?;
        assert!(
            !recursive.supported,
            "nested network mounts prevent delivery certification"
        );
        watches.insert(PathBuf::from("/repo"), false);
        let shallow = from_bytes(table, &watches)?;
        assert!(shallow.supported, "uncovered network mounts do not prevent a fence");
        let replacement = b"4 0 8:1 / / rw - ext4 /dev/root rw
";
        assert_ne!(
            shallow,
            from_bytes(replacement, &watches)?,
            "a new mount ID invalidates watches even when its path and filesystem match"
        );
        let same = b"1 0 8:1 / / rw - ext4 /dev/root rw
";
        assert_eq!(shallow, from_bytes(same, &watches)?, "unrelated mounts are excluded");
        Ok(())
    }

    #[test]
    fn opaque_namespace_roots_do_not_reject_unrelated_coverage() -> gix_error::TestResult {
        let table = b"1 0 8:1 / / rw - ext4 /dev/root rw
709 167 0:5 mnt:[4026532313] /run/snapd/ns/docker.mnt rw - nsfs nsfs rw
";
        let watches = BTreeMap::from([(PathBuf::from("/repo"), true)]);
        assert!(
            from_bytes(table, &watches)?.supported,
            "opaque nsfs roots are valid and do not affect unrelated local watches"
        );
        let watches = BTreeMap::from([(PathBuf::from("/run/snapd/ns"), true)]);
        assert!(
            !from_bytes(table, &watches)?.supported,
            "namespace mounts still cannot certify notifications when covered"
        );
        Ok(())
    }

    #[test]
    fn malformed_tables_are_not_certified() {
        let watches = BTreeMap::from([(PathBuf::from("/repo"), true)]);
        for table in [
            b"".as_slice(),
            b"1 0 8:1 / / rw - ext4 /dev/root rw",
            b"1 0 8:1 / / rw - ext4\n",
            b"1 0 8:1 / relative rw - ext4 /dev/root rw\n",
            b"1 0 8:1 / /bad\\0400\\999 rw - ext4 /dev/root rw\n",
        ] {
            assert!(
                from_bytes(table, &watches).is_err(),
                "malformed mount tables fail closed"
            );
        }
    }
}
