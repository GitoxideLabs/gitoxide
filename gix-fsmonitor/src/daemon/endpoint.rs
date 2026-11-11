use std::path::{Path, PathBuf};

#[cfg(any(unix, test))]
use gix_error::message;
use gix_error::{Result, ResultExt};

/// Select Git's native IPC endpoint without creating a directory or binding the endpoint.
///
/// Canonicalize `git_dir` and, for local filesystems, return its `fsmonitor--daemon.ipc` path
/// without calling `socket_dir`. On supported Unix platforms, a remote Git directory uses a
/// SHA-1-derived filename based on the canonical worktree's pathname bytes. Only then is
/// `socket_dir` called to obtain the caller's trusted, already-interpolated socket directory.
/// Windows uses the Git directory; Unix platforms other than macOS and Linux return an
/// unsupported-filesystem-classification error. Path, classification, callback, and hashing
/// errors are propagated.
pub fn endpoint(git_dir: &Path, worktree: &Path, socket_dir: impl FnOnce() -> Result<PathBuf>) -> Result<PathBuf> {
    let git_dir = git_dir.canonicalize().or_error()?;
    #[cfg(unix)]
    if is_remote(&git_dir).or_raise(|| message("determining whether the Git directory is on a remote filesystem"))? {
        let worktree = worktree
            .canonicalize()
            .or_raise(|| message("resolving the filesystem monitor worktree"))?;
        return remote(&worktree, &socket_dir()?);
    }
    #[cfg(not(unix))]
    let _ = (worktree, socket_dir);
    // Git ignores fsmonitor.socketDir when the administrative directory is local.
    Ok(git_dir.join("fsmonitor--daemon.ipc"))
}

#[cfg(any(unix, test))]
fn remote(worktree: &Path, socket_dir: &Path) -> Result<PathBuf> {
    // Native Git endpoint names always use SHA-1, independently of compiled object formats.
    let mut hash = sha1dc::Hasher::new();
    hash.update(gix_path::to_unix_separators_on_windows(gix_path::into_bstr(worktree)?).as_ref());
    let digest = hash
        .finalize()
        .or_raise(|| message("hashing the worktree path for its native IPC endpoint"))?;
    Ok(socket_dir.join(format!(".git-fsmonitor-{digest}")))
}

/// Return whether the filesystem containing `path` is classified as remote for IPC endpoint placement.
///
/// On macOS, this means the filesystem lacks `MNT_LOCAL`. On Linux, this means its type is
/// SMB, CIFS, SMB2, NFS, AFS, Coda, or FUSE; FUSE is included even when locally backed.
/// Other Linux filesystem types are classified as local.
///
/// Return an error if `path` contains a NUL byte or `statfs` fails.
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn is_remote(path: &Path) -> std::io::Result<bool> {
    let info = rustix::fs::statfs(path)?;
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

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn filesystem_classification_reports_path_errors() -> gix_testtools::TestResult {
        assert_eq!(
            is_remote(Path::new("invalid\0path"))
                .expect_err("a path containing a NUL byte cannot be passed to statfs")
                .kind(),
            std::io::ErrorKind::InvalidInput,
            "filesystem classification rejects NUL bytes in paths"
        );
        assert_eq!(
            is_remote(Path::new(""))
                .expect_err("an empty path does not name a filesystem")
                .kind(),
            std::io::ErrorKind::NotFound,
            "filesystem classification propagates statfs errors"
        );
        Ok(())
    }

    #[test]
    fn remote_endpoint_hashes_path_with_sha1_independently_of_repository_format() -> gix_testtools::TestResult {
        assert_eq!(
            remote(Path::new("abc"), Path::new("sockets"))?,
            Path::new("sockets/.git-fsmonitor-a9993e364706816aba3e25717850c26c9cd0d89d"),
            "Git hashes pathname bytes with SHA-1, without an object header or trailing NUL"
        );
        Ok(())
    }
}
