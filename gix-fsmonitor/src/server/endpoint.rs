use std::path::{Path, PathBuf};

#[cfg(unix)]
use gix_error::message;
use gix_error::{Result, ResultExt};

#[cfg(unix)]
use crate::server::filesystem::is_remote;

/// Select Git's native IPC endpoint without creating a directory or binding the endpoint.
///
/// Canonicalize `git_dir` and, for local filesystems, return its `fsmonitor--daemon.ipc` path
/// without calling `socket_dir`. On supported Unix platforms, a remote Git directory uses a
/// SHA-1-derived filename based on the canonical worktree's pathname bytes, independently of
/// Git object formats. Only then is `socket_dir` called, exactly once, to obtain the caller's
/// trusted, already-interpolated socket directory.
/// Windows uses the Git directory; Unix platforms other than macOS and Linux return an
/// unsupported-filesystem-classification error. Path, classification, callback, and hashing
/// errors are propagated.
pub fn endpoint(git_dir: &Path, worktree: &Path, socket_dir: impl FnOnce() -> Result<PathBuf>) -> Result<PathBuf> {
    #[cfg(unix)]
    {
        endpoint_with_remote(git_dir, worktree, socket_dir, is_remote)
    }
    #[cfg(not(unix))]
    {
        let git_dir = git_dir.canonicalize().or_error()?;
        let _ = (worktree, socket_dir);
        Ok(git_dir.join("fsmonitor--daemon.ipc"))
    }
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
    #[cfg(unix)]
    {
        endpoint_inner(git_dir, || Ok(worktree), socket_dir, is_remote)
    }
    #[cfg(not(unix))]
    {
        let _ = (worktree, socket_dir);
        Ok(git_dir.join("fsmonitor--daemon.ipc"))
    }
}

#[cfg(unix)]
fn endpoint_with_remote(
    git_dir: &Path,
    worktree: &Path,
    socket_dir: impl FnOnce() -> Result<PathBuf>,
    is_remote: impl FnOnce(&Path) -> std::io::Result<bool>,
) -> Result<PathBuf> {
    let git_dir = git_dir.canonicalize().or_error()?;
    endpoint_inner(
        &git_dir,
        || {
            worktree
                .canonicalize()
                .or_raise(|| message("resolving the filesystem monitor worktree"))
        },
        socket_dir,
        is_remote,
    )
}

#[cfg(unix)]
fn endpoint_inner<P: AsRef<Path>>(
    git_dir: &Path,
    worktree: impl FnOnce() -> Result<P>,
    socket_dir: impl FnOnce() -> Result<PathBuf>,
    is_remote: impl FnOnce(&Path) -> std::io::Result<bool>,
) -> Result<PathBuf> {
    if is_remote(git_dir).or_raise(|| message("determining whether the Git directory is on a remote filesystem"))? {
        return remote(worktree()?.as_ref(), &socket_dir()?);
    }
    // Git ignores fsmonitor.socketDir when the administrative directory is local.
    Ok(git_dir.join("fsmonitor--daemon.ipc"))
}

#[cfg(unix)]
fn remote(worktree: &Path, socket_dir: &Path) -> Result<PathBuf> {
    // Native Git endpoint names always use SHA-1, independently of compiled object formats.
    let mut hash = sha1dc::Hasher::new();
    hash.update(gix_path::to_unix_separators_on_windows(gix_path::into_bstr(worktree)?).as_ref());
    let digest = hash
        .finalize()
        .or_raise(|| message("hashing the worktree path for its native IPC endpoint"))?;
    Ok(socket_dir.join(format!(".git-fsmonitor-{digest}")))
}

#[cfg(all(test, unix))]
mod tests {
    use gix_error::ErrorExt;

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

    #[test]
    fn local_endpoint_classifies_canonical_git_directory_and_ignores_socket_directory() -> gix_error::TestResult {
        let fixture = gix_testtools::tempfile::tempdir()?;
        let git_dir = fixture.path().join("git-dir");
        std::fs::create_dir(&git_dir)?;
        let git_dir = git_dir.canonicalize()?;
        let alias = fixture.path().join("git-alias");
        std::os::unix::fs::symlink(&git_dir, &alias)?;
        let endpoint = endpoint_with_remote(
            &alias,
            &fixture.path().join("missing-worktree"),
            || panic!("local Git directories ignore even invalid or untrusted socketDir settings"),
            |path| {
                assert_eq!(path, git_dir, "classify the canonical Git directory, not the worktree");
                Ok(false)
            },
        )?;
        assert_eq!(
            endpoint,
            git_dir.join("fsmonitor--daemon.ipc"),
            "local endpoint selection uses the canonical administrative path"
        );
        assert!(!endpoint.exists(), "selection does not create or bind the endpoint");
        Ok(())
    }

    #[test]
    fn remote_endpoint_uses_canonical_worktree_bytes_and_calls_socket_directory_once() -> gix_error::TestResult {
        use std::os::unix::ffi::OsStrExt;

        let fixture = gix_testtools::tempfile::tempdir()?;
        let git_dir = fixture.path().join("git-dir");
        let worktree = fixture.path().join("worktree");
        std::fs::create_dir(&git_dir)?;
        std::fs::create_dir(&worktree)?;
        let git_dir = git_dir.canonicalize()?;
        let worktree = worktree.canonicalize()?;
        let git_alias = fixture.path().join("git-alias");
        let worktree_alias = fixture.path().join("worktree-alias");
        std::os::unix::fs::symlink(&git_dir, &git_alias)?;
        std::os::unix::fs::symlink(&worktree, &worktree_alias)?;
        let socket_dir = fixture.path().join("sockets");
        let mut hash = sha1dc::Hasher::new();
        hash.update(worktree.as_os_str().as_bytes());
        let expected = socket_dir.join(format!(".git-fsmonitor-{}", hash.finalize()?));
        for path in [&worktree, &worktree_alias, &worktree_alias.join(".")] {
            let mut calls = 0;
            let resolved_socket_dir = socket_dir.clone();
            let endpoint = endpoint_with_remote(
                &git_alias,
                path,
                || {
                    calls += 1;
                    Ok(resolved_socket_dir)
                },
                |path| {
                    assert_eq!(path, git_dir, "remote classification uses the canonical Git directory");
                    Ok(true)
                },
            )?;
            assert_eq!(
                calls, 1,
                "remote endpoint selection requests the trusted socket directory once"
            );
            assert_eq!(
                endpoint, expected,
                "worktree aliases hash the same canonical pathname bytes, not the Git directory"
            );
        }
        assert!(!socket_dir.exists(), "selection does not create the socket directory");
        Ok(())
    }

    #[test]
    fn remote_endpoint_propagates_socket_directory_errors() -> gix_error::TestResult {
        let fixture = gix_testtools::tempfile::tempdir()?;
        let mut calls = 0;
        let error = endpoint_with_remote(
            fixture.path(),
            fixture.path(),
            || {
                calls += 1;
                Err(message("untrusted socket directory").raise())
            },
            |_| Ok(true),
        )
        .expect_err("remote placement requires a usable trusted socket directory");
        assert_eq!(calls, 1, "a failed callback is not retried");
        assert!(
            error.to_string().contains("untrusted socket directory"),
            "socket directory errors are propagated: {error}"
        );
        Ok(())
    }

    #[test]
    fn classification_errors_do_not_resolve_worktree_or_socket_directory() -> gix_error::TestResult {
        let fixture = gix_testtools::tempfile::tempdir()?;
        for kind in [std::io::ErrorKind::PermissionDenied, std::io::ErrorKind::Unsupported] {
            let error = endpoint_with_remote(
                fixture.path(),
                &fixture.path().join("missing-worktree"),
                || panic!("failed classification must not request a socket directory"),
                |_| Err(std::io::Error::new(kind, "filesystem classification failed")),
            )
            .expect_err("classification failures cannot silently select a local endpoint");
            assert!(
                error.to_string().contains("determining whether the Git directory"),
                "classification failures retain their context: {error}"
            );
            assert_eq!(
                error
                    .downcast_any_ref::<std::io::Error>()
                    .expect("classification retains its native error")
                    .kind(),
                kind,
                "classification preserves the native error instead of resolving the missing worktree"
            );
        }
        Ok(())
    }

    #[test]
    fn remote_endpoint_requires_a_canonical_worktree_before_requesting_socket_directory() -> gix_error::TestResult {
        let fixture = gix_testtools::tempfile::tempdir()?;
        let error = endpoint_with_remote(
            fixture.path(),
            &fixture.path().join("missing-worktree"),
            || panic!("a missing worktree must fail before the socket directory callback"),
            |_| Ok(true),
        )
        .expect_err("remote endpoint hashing requires a canonical worktree");
        assert!(
            error.to_string().contains("resolving the filesystem monitor worktree"),
            "worktree resolution errors retain their context: {error}"
        );
        assert_eq!(
            error
                .downcast_any_ref::<std::io::Error>()
                .expect("resolution retains its native error")
                .kind(),
            std::io::ErrorKind::NotFound,
            "a missing remote worktree is reported instead of hashing its unresolved spelling"
        );
        Ok(())
    }
}
