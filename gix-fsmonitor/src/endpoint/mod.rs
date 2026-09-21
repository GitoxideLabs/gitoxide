use std::path::{Path, PathBuf};

#[cfg(any(unix, test))]
use anyhow::Context;

pub(crate) fn for_repository(repo: &gix::Repository, worktree: &Path) -> anyhow::Result<PathBuf> {
    let git_dir = repo.git_dir().canonicalize()?;
    #[cfg(unix)]
    if is_remote(&git_dir).context("determining whether the Git directory is on a remote filesystem")? {
        let configured = repo
            .config_snapshot()
            .string("fsmonitor.socketDir")
            .filter(|value| !value.is_empty())
            .map(|value| gix::path::try_from_bstr(value).map(std::borrow::Cow::into_owned))
            .transpose()?;
        let path = remote(worktree, configured.as_deref())?;
        let home = gix::path::env::home_dir();
        return Ok(
            gix::config::Path::from(gix::path::try_into_bstr(path)?.into_owned()).interpolate(
                gix::config::path::interpolate::Context {
                    home_dir: home.as_deref(),
                    git_install_dir: gix::path::env::installation_config_prefix(),
                    home_for_user: Some(gix::config::path::interpolate::home_for_user),
                },
            )?,
        );
    }
    #[cfg(windows)]
    let _ = worktree;
    // Git ignores fsmonitor.socketDir when the administrative directory is local.
    Ok(git_dir.join("fsmonitor--daemon.ipc"))
}

#[cfg(any(unix, test))]
fn remote(worktree: &Path, socket_dir: Option<&Path>) -> anyhow::Result<PathBuf> {
    let mut hash = gix::hash::hasher(gix::hash::Kind::Sha1);
    hash.update(gix::path::to_unix_separators_on_windows(gix::path::try_into_bstr(worktree)?).as_ref());
    let digest = hash
        .try_finalize()
        .context("hashing the worktree path for its native IPC endpoint")?;
    Ok(socket_dir
        .unwrap_or_else(|| Path::new("~"))
        .join(format!(".git-fsmonitor-{digest}")))
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn is_remote(path: &Path) -> std::io::Result<bool> {
    use std::os::unix::ffi::OsStrExt;
    let path = std::ffi::CString::new(path.as_os_str().as_bytes())?;
    let mut info = std::mem::MaybeUninit::<libc::statfs>::uninit();
    // SAFETY: The path is NUL terminated, and statfs writes the complete platform structure on success.
    if unsafe { libc::statfs(path.as_ptr(), info.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: The successful statfs call initialized info.
    let info = unsafe { info.assume_init() };
    #[cfg(target_os = "macos")]
    return Ok(info.f_flags & libc::MNT_LOCAL as u32 == 0);
    #[cfg(target_os = "linux")]
    return Ok(matches!(
        (info.f_type as u64) & 0xffff_ffff,
        0xff534d42 | 0x517b | 0xfe534d42 | 0x6969 | 0x5346414f | 0x73757245 | 0x65735546
    ));
}

#[cfg(all(unix, not(any(target_os = "macos", target_os = "linux"))))]
fn is_remote(_path: &Path) -> std::io::Result<bool> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "native Git endpoint filesystem classification is supported on macOS and Linux",
    ))
}

#[cfg(any(windows, test))]
pub(crate) fn pipe_name(mut path: Vec<u16>) -> Vec<u16> {
    let verbatim: Vec<_> = "\\\\?\\".encode_utf16().collect();
    let verbatim_unc: Vec<_> = "\\\\?\\UNC\\".encode_utf16().collect();
    if path.starts_with(&verbatim_unc) {
        path.splice(..verbatim_unc.len(), "\\\\".encode_utf16());
    } else if path.starts_with(&verbatim) {
        path.drain(..verbatim.len());
    }
    if path.get(1) == Some(&(b':' as u16)) {
        path[1] = b'_' as u16;
    }
    for unit in &mut path {
        if *unit == b'/' as u16 {
            *unit = b'\\' as u16;
        }
    }
    let mut name: Vec<_> = "\\\\.\\pipe\\".encode_utf16().collect();
    name.extend(path);
    name.push(0);
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_endpoint_hashes_path_with_sha1_independently_of_repository_format() -> anyhow::Result<()> {
        assert_eq!(
            remote(Path::new("abc"), Some(Path::new("sockets")))?,
            Path::new("sockets/.git-fsmonitor-a9993e364706816aba3e25717850c26c9cd0d89d"),
            "Git hashes pathname bytes with SHA-1, without an object header or trailing NUL"
        );
        Ok(())
    }

    #[test]
    fn windows_endpoint_uses_path_without_a_hash() {
        for (input, expected) in [
            (
                "C:/work/.git/fsmonitor--daemon.ipc",
                "\\\\.\\pipe\\C_\\work\\.git\\fsmonitor--daemon.ipc\0",
            ),
            (
                "\\\\?\\C:\\work\\.git\\fsmonitor--daemon.ipc",
                "\\\\.\\pipe\\C_\\work\\.git\\fsmonitor--daemon.ipc\0",
            ),
            (
                "\\\\?\\UNC\\host\\share\\repo\\.git\\fsmonitor--daemon.ipc",
                "\\\\.\\pipe\\\\\\host\\share\\repo\\.git\\fsmonitor--daemon.ipc\0",
            ),
        ] {
            assert_eq!(
                pipe_name(input.encode_utf16().collect()),
                expected.encode_utf16().collect::<Vec<_>>(),
                "Windows naming follows Git's drive-colon and slash conversion"
            );
        }
        let mut units: Vec<_> = "C:/".encode_utf16().collect();
        units.push(0xd800);
        assert!(
            pipe_name(units).contains(&0xd800),
            "native wide path units are preserved"
        );
    }
}
