use std::{io, path::Path};

/// Classify the filesystem containing `path` independently of remote-monitoring permission.
///
/// macOS uses `MNT_LOCAL`; Linux treats SMB, CIFS, SMB2, NFS, AFS, Coda, and all FUSE filesystems
/// as remote. Windows resolves the native path and volume root, then requires a successful
/// protocol probe for remote drives. Path and classification errors are propagated, including
/// indeterminate Windows drive types. Other targets return [`io::ErrorKind::Unsupported`].
pub(crate) fn is_remote(path: &Path) -> io::Result<bool> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let info = rustix::fs::statfs(path)?;
        #[cfg(target_os = "macos")]
        return Ok(remote_flags(info.f_flags));
        #[cfg(target_os = "linux")]
        return Ok(remote_type(info.f_type as u64));
    }
    #[cfg(windows)]
    {
        windows::is_remote(path)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
    {
        let _ = path;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "filesystem classification is supported on macOS, Linux, and Windows",
        ))
    }
}

#[cfg(target_os = "macos")]
fn remote_flags(flags: u32) -> bool {
    flags & libc::MNT_LOCAL as u32 == 0
}

#[cfg(any(target_os = "linux", test))]
fn remote_type(fs_type: u64) -> bool {
    matches!(
        fs_type & 0xffff_ffff,
        0xff534d42 | 0x517b | 0xfe534d42 | 0x6969 | 0x5346414f | 0x73757245 | 0x65735546
    )
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod windows {
    use std::{
        fs::OpenOptions,
        io,
        mem::size_of,
        os::windows::{ffi::OsStrExt, fs::OpenOptionsExt, io::AsRawHandle},
        path::Path,
    };

    use windows_sys::Win32::{
        Storage::FileSystem::{
            FILE_FLAG_BACKUP_SEMANTICS, FILE_REMOTE_PROTOCOL_INFO, FILE_SHARE_READ, FileRemoteProtocolInfo,
            GetDriveTypeW, GetFileInformationByHandleEx, GetVolumePathNameW,
        },
        System::WindowsProgramming::{
            DRIVE_CDROM, DRIVE_FIXED, DRIVE_NO_ROOT_DIR, DRIVE_RAMDISK, DRIVE_REMOTE, DRIVE_REMOVABLE,
        },
    };

    pub(super) fn is_remote(path: &Path) -> io::Result<bool> {
        if path.as_os_str().is_empty() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "empty filesystem path"));
        }
        if path.as_os_str().encode_wide().any(|unit| unit == 0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "filesystem path contains a NUL byte",
            ));
        }
        // Resolve symlinks, junctions, and SUBST aliases before classifying the containing volume.
        let path = path.canonicalize()?;
        let wide: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let root = volume_root(&wide)?;
        // SAFETY: GetVolumePathNameW returned a NUL-terminated native root with its trailing separator.
        let drive_type = unsafe { GetDriveTypeW(root.as_ptr()) };
        classify_drive(drive_type, || probe_remote(&path))
    }

    fn volume_root(path: &[u16]) -> io::Result<Vec<u16>> {
        // A volume root is no longer than the full native path plus a trailing separator.
        let mut root = vec![0; path.len().max(260) + 1];
        let length = u32::try_from(root.len()).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "filesystem path exceeds Windows API limits",
            )
        })?;
        // SAFETY: The input is NUL-terminated; the output buffer has the advertised number of UTF-16 units.
        if unsafe { GetVolumePathNameW(path.as_ptr(), root.as_mut_ptr(), length) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(root)
    }

    fn classify_drive(drive_type: u32, probe: impl FnOnce() -> io::Result<()>) -> io::Result<bool> {
        match drive_type {
            DRIVE_REMOTE => {
                probe()?;
                Ok(true)
            }
            DRIVE_REMOVABLE | DRIVE_FIXED | DRIVE_CDROM | DRIVE_RAMDISK => Ok(false),
            DRIVE_NO_ROOT_DIR => Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Windows could not find the filesystem drive root",
            )),
            // GetDriveTypeW does not promise to set LastError, so do not report a stale OS error.
            _ => Err(io::Error::other(format!(
                "Windows could not classify the filesystem drive type {drive_type}"
            ))),
        }
    }

    fn probe_remote(path: &Path) -> io::Result<()> {
        let file = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(path)?;
        let mut info = FILE_REMOTE_PROTOCOL_INFO::default();
        // SAFETY: The file owns a live handle, and the information class matches the output structure and size.
        if unsafe {
            GetFileInformationByHandleEx(
                file.as_raw_handle(),
                FileRemoteProtocolInfo,
                (&raw mut info).cast(),
                size_of::<FILE_REMOTE_PROTOCOL_INFO>() as u32,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use windows_sys::Win32::{
            Foundation::{ERROR_ACCESS_DENIED, ERROR_NOT_SUPPORTED},
            System::WindowsProgramming::DRIVE_UNKNOWN,
        };

        #[test]
        fn known_local_drive_types_do_not_probe_remote_protocols() -> gix_testtools::TestResult {
            for drive_type in [DRIVE_REMOVABLE, DRIVE_FIXED, DRIVE_CDROM, DRIVE_RAMDISK] {
                assert!(
                    !classify_drive(drive_type, || panic!("local drives do not have remote protocols"))?,
                    "known local drive types are not remote"
                );
            }
            Ok(())
        }

        #[test]
        fn remote_drives_require_a_successful_protocol_probe() -> gix_testtools::TestResult {
            let mut probed = false;
            assert!(
                classify_drive(DRIVE_REMOTE, || {
                    probed = true;
                    Ok(())
                })?,
                "a remote drive is accepted after its protocol information is available"
            );
            assert!(probed, "remote classification must actually probe the protocol");
            for code in [ERROR_ACCESS_DENIED, ERROR_NOT_SUPPORTED] {
                let error = classify_drive(DRIVE_REMOTE, || Err(io::Error::from_raw_os_error(code as i32)))
                    .expect_err("a failed protocol probe must not classify the drive as local or usable");
                assert_eq!(
                    error.raw_os_error(),
                    Some(code as i32),
                    "protocol probe failures preserve their native error code"
                );
            }
            Ok(())
        }

        #[test]
        fn indeterminate_drive_types_are_errors_without_protocol_probes() -> gix_testtools::TestResult {
            for (drive_type, kind) in [
                (DRIVE_UNKNOWN, io::ErrorKind::Other),
                (DRIVE_NO_ROOT_DIR, io::ErrorKind::NotFound),
                (u32::MAX, io::ErrorKind::Other),
            ] {
                let error = classify_drive(drive_type, || panic!("indeterminate drives must not be probed"))
                    .expect_err("an indeterminate drive type must not be silently treated as local");
                assert_eq!(error.kind(), kind, "invalid drive types retain a useful error category");
            }
            Ok(())
        }

        #[test]
        fn canonical_native_paths_resolve_to_volume_roots() -> gix_testtools::TestResult {
            let fixture = gix_testtools::tempfile::tempdir()?;
            let path = fixture.path().canonicalize()?;
            let wide: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            let root = volume_root(&wide)?;
            let end = root
                .iter()
                .position(|unit| *unit == 0)
                .expect("a volume root is NUL-terminated");
            assert!(end > 0, "a volume root is not empty");
            assert_eq!(
                root[end - 1],
                b'\\' as u16,
                "GetDriveTypeW requires a trailing separator"
            );
            let same_root = volume_root(&root)?;
            assert_eq!(
                root[..=end],
                same_root[..=end],
                "a native volume root resolves to itself"
            );
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(target_os = "macos", target_os = "linux", windows))]
    #[test]
    fn filesystem_classification_reports_path_errors() -> gix_testtools::TestResult {
        for (path, kind) in [
            (Path::new("invalid\0path"), io::ErrorKind::InvalidInput),
            (Path::new(""), io::ErrorKind::NotFound),
        ] {
            assert_eq!(
                is_remote(path)
                    .expect_err("an invalid filesystem path cannot be classified")
                    .kind(),
                kind,
                "path errors are propagated instead of returning a local classification"
            );
        }
        let fixture = gix_testtools::tempfile::tempdir()?;
        assert_eq!(
            is_remote(&fixture.path().join("missing"))
                .expect_err("the disposable fixture contains no missing child")
                .kind(),
            io::ErrorKind::NotFound,
            "filesystem probing propagates missing-path errors"
        );
        Ok(())
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_remote_classification_depends_only_on_mnt_local() -> gix_testtools::TestResult {
        assert!(remote_flags(0), "absence of MNT_LOCAL means remote");
        assert!(
            remote_flags(libc::MNT_RDONLY as u32),
            "unrelated mount flags do not imply local"
        );
        assert!(!remote_flags(libc::MNT_LOCAL as u32), "MNT_LOCAL means local");
        assert!(!remote_flags(u32::MAX), "other flags do not override MNT_LOCAL");
        Ok(())
    }

    #[test]
    fn linux_remote_classification_preserves_git_filesystem_types() -> gix_testtools::TestResult {
        for fs_type in [
            0xff534d42, 0x517b, 0xfe534d42, 0x6969, 0x5346414f, 0x73757245, 0x65735546,
        ] {
            assert!(
                remote_type(fs_type),
                "Git treats this network or FUSE filesystem type as remote"
            );
            assert!(
                remote_type(fs_type | 0xffff_ffff_0000_0000),
                "only the low 32 bits matter, including sign-extended filesystem magic values"
            );
        }
        for fs_type in [0, 0xef53, 0x01021994, 0x9123683e, 0xffff_ffff] {
            assert!(!remote_type(fs_type), "other filesystem types remain local");
        }
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
    #[test]
    fn unsupported_targets_do_not_assume_local_filesystems() -> gix_testtools::TestResult {
        assert_eq!(
            is_remote(Path::new("."))
                .expect_err("this target has no native filesystem classifier")
                .kind(),
            io::ErrorKind::Unsupported,
            "unsupported classification must not silently permit remote monitoring"
        );
        Ok(())
    }
}
