use std::path::{Path, PathBuf};

use gix_error::{Result, ResultExt, message};

#[cfg(unix)]
use crate::server::filesystem::is_remote;

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
    endpoint_inner(
        &git_dir,
        || {
            worktree
                .canonicalize()
                .or_raise(|| message("resolving the filesystem monitor worktree"))
        },
        socket_dir,
    )
}

/// Select the native IPC endpoint from paths that the caller has already canonicalized.
///
/// This performs the same filesystem classification and lazy `socket_dir` lookup as [`endpoint()`],
/// without resolving `git_dir` or `worktree` again. Their canonical form is not checked; use
/// [`endpoint()`] when accepting unresolved paths.
pub fn endpoint_for_canonical_paths(
    git_dir: &Path,
    worktree: &Path,
    socket_dir: impl FnOnce() -> Result<PathBuf>,
) -> Result<PathBuf> {
    endpoint_inner(git_dir, || Ok(worktree), socket_dir)
}

fn endpoint_inner<P: AsRef<Path>>(
    git_dir: &Path,
    worktree: impl FnOnce() -> Result<P>,
    socket_dir: impl FnOnce() -> Result<PathBuf>,
) -> Result<PathBuf> {
    #[cfg(unix)]
    if is_remote(git_dir).or_raise(|| message("determining whether the Git directory is on a remote filesystem"))? {
        return remote(worktree()?.as_ref(), &socket_dir()?);
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

#[cfg(test)]
mod tests {
    use super::*;

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
