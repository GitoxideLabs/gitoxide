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
            .map(|value| gix::path::from_bstr(value).map(std::borrow::Cow::into_owned))
            .transpose()?;
        let path = remote(worktree, configured.as_deref())?;
        let home = gix::path::env::home_dir();
        return Ok(
            gix::config::Path::from(gix::path::into_bstr(path)?.into_owned()).interpolate(
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
    hash.update(gix::path::to_unix_separators_on_windows(gix::path::into_bstr(worktree)?).as_ref());
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
}
