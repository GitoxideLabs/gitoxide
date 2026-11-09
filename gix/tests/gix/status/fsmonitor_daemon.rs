use std::path::{Path, PathBuf};

use gix_testtools::TestResult;

fn paths(repo: &gix::Repository, write_back: bool) -> TestResult<Vec<String>> {
    let mut iter = repo.status(gix::progress::Discard)?.into_index_worktree_iter(None)?;
    let mut paths = iter
        .by_ref()
        .map(|item| item.map(|item| item.rela_path().to_string()))
        .collect::<gix::Result<Vec<_>>>()?;
    if write_back {
        iter.into_outcome()
            .expect("a fully consumed iterator has an outcome")
            .write_changes()
            .transpose()?;
    }
    paths.sort();
    Ok(paths)
}

fn repository(root: &Path) -> gix::Result<gix::Repository> {
    gix::open_opts(
        root,
        gix::open::Options::isolated()
            .config_overrides(["core.fsmonitor=true", "gitoxide.core.externalCommandStderr=false"]),
    )
}

fn move_into_deep_directory(root: &Path) -> TestResult<PathBuf> {
    let entries = std::fs::read_dir(root)?.collect::<std::io::Result<Vec<_>>>()?;
    let nested = root.join("long-parent-directory-name-".repeat(4)).join("repo");
    std::fs::create_dir_all(&nested)?;
    for entry in entries {
        std::fs::rename(entry.path(), nested.join(entry.file_name()))?;
    }
    Ok(nested)
}

#[test]
#[cfg(any(target_os = "macos", windows))]
fn git_daemon_starts_in_deep_repositories_and_observes_changes_after_writeback() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_writable("make_status_fsmonitor.sh")?;
    let root = move_into_deep_directory(fixture.path())?;
    let root = root.as_path();
    let current_dir = std::env::current_dir()?;
    let probe = gix_testtools::git_command(root)
        .args(["fsmonitor--daemon", "status"])
        .output()?;
    if String::from_utf8_lossy(&probe.stderr).contains("not supported") {
        return Ok(());
    }

    struct StopDaemon<'a>(&'a Path);
    impl Drop for StopDaemon<'_> {
        fn drop(&mut self) {
            if let Err(err) = gix_testtools::git_command(self.0)
                .args(["fsmonitor--daemon", "stop"])
                .output()
            {
                eprintln!("Could not stop the fixture's filesystem monitor: {err}");
            }
        }
    }
    let _stop = StopDaemon(root);
    gix_testtools::git(root, "config core.fsmonitor true")?;
    let repo = repository(root)?;
    assert_eq!(
        paths(&repo, true)?,
        ["dir/a", "directory", "dirty", "reported", "unreported", "untracked"],
        "the first daemon query invalidates the old hook token"
    );
    assert!(
        gix_testtools::git_command(root)
            .args(["fsmonitor--daemon", "status"])
            .output()?
            .status
            .success(),
        "status starts the daemon when it is not already running"
    );
    let index = gix::index::File::at(root.join(".git/index"), repo.object_hash(), false, Default::default())?;
    assert!(
        matches!(index.fs_monitor().map(gix::index::extension::FsMonitor::token), Some(
            gix::index::extension::fs_monitor::Token::V2 { token }
        ) if token.starts_with(b"builtin:")),
        "writeback saves the daemon's token in a Git-readable extension"
    );

    std::fs::write(root.join("dir/b"), "changed after the daemon's first query\n")?;
    std::fs::remove_file(root.join("reported"))?;
    std::fs::write(root.join("new-untracked"), "new file\n")?;
    let baseline = gix_testtools::git_command(root)
        .args(["-c", "core.fsmonitor=false", "status", "--porcelain"])
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()?;
    assert!(
        baseline.status.success(),
        "Git's full scan supplies the reference status"
    );
    let mut expected: Vec<_> = std::str::from_utf8(&baseline.stdout)?
        .lines()
        .map(|line| line[3..].to_owned())
        .collect();
    expected.sort();
    assert_eq!(
        paths(&repo, true)?,
        expected,
        "daemon events expose tracked modifications and deletions alongside untracked files"
    );
    gix_testtools::git(root, "fsmonitor--daemon stop")?;
    assert!(
        !gix_testtools::git_command(root)
            .args(["fsmonitor--daemon", "status"])
            .output()?
            .status
            .success(),
        "the fixture's daemon is stopped before cleanup"
    );
    assert_eq!(
        std::env::current_dir()?,
        current_dir,
        "status must preserve the process working directory"
    );
    Ok(())
}

#[test]
#[cfg(unix)]
fn long_socket_paths_preserve_monitor_acceleration_and_the_working_directory() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_writable("make_status_fsmonitor.sh")?;
    let root = move_into_deep_directory(fixture.path())?;
    let repo = repository(&root)?;
    let current_dir = std::env::current_dir()?;
    assert_eq!(
        mock_status(&repo, &root, b"next-token", b"000dmock-two\x000000", false)?,
        ["dirty", "untracked"],
        "a long canonical repository path must still query the daemon and skip valid entries"
    );
    assert_eq!(
        std::env::current_dir()?,
        current_dir,
        "status must preserve the process working directory"
    );
    Ok(())
}

#[test]
#[cfg(unix)]
fn mock_daemon_refreshes_tokens_skips_unreported_paths_and_falls_back_on_failure() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_writable("make_status_fsmonitor.sh")?;
    let root = fixture.path();
    let repo = repository(root)?;
    let expected = ["dir/a", "dirty", "reported", "untracked"];
    assert_eq!(
        mock_status(
            &repo,
            root,
            b"next-token",
            b"000cmock-one000e\x00reported\x000009dir/\x000000",
            true,
        )?,
        expected,
        "packet boundaries can split a response token or path without affecting status"
    );
    std::fs::write(root.join("dir/b"), "unreported change after the first query\n")?;
    assert_eq!(
        mock_status(&repo, root, b"mock-one", b"000dmock-two\x000000", true)?,
        expected,
        "a token-only response skips valid files but retains previously dirty entries"
    );
    assert_eq!(
        mock_status(&repo, root, b"mock-one", b"0008x", false)?,
        [
            "dir/a",
            "dir/b",
            "directory",
            "dirty",
            "reported",
            "unreported",
            "untracked"
        ],
        "a truncated daemon response triggers a complete scan"
    );
    Ok(())
}

#[cfg(unix)]
fn mock_status(
    repo: &gix::Repository,
    root: &Path,
    expected_token: &[u8],
    response: &[u8],
    write_back: bool,
) -> TestResult<Vec<String>> {
    use std::{
        io::{self, Read, Write},
        os::unix::net::UnixListener,
        time::{Duration, Instant},
    };

    let socket = root.join(".git/fsmonitor--daemon.ipc");
    // Bind through a short parent alias so the server also supports deep repository paths.
    let alias = gix_testtools::tempfile::tempdir_in("/tmp")?;
    std::os::unix::fs::symlink(root.join(".git"), alias.path().join("git"))?;
    let listener = UnixListener::bind(alias.path().join("git/fsmonitor--daemon.ipc"))?;
    listener.set_nonblocking(true)?;
    let mut expected = format!("{:04x}", expected_token.len() + 4).into_bytes();
    expected.extend_from_slice(expected_token);
    expected.extend_from_slice(b"0000");
    let response = response.to_owned();
    let server = std::thread::spawn(move || -> io::Result<()> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(err) if err.kind() == io::ErrorKind::WouldBlock && Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(err) => return Err(err),
            }
        };
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        let mut request = vec![0; expected.len()];
        stream.read_exact(&mut request)?;
        assert_eq!(request, expected, "each query uses the previously persisted token");
        stream.write_all(&response)?;
        Ok(())
    });
    let result = paths(repo, write_back);
    server.join().expect("the mock server must finish without panicking")?;
    std::fs::remove_file(socket)?;
    result
}

#[test]
#[cfg(unix)]
fn configured_response_limit_controls_monitor_fallback() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_writable("make_status_fsmonitor.sh")?;
    let root = fixture.path();
    for limit in [0, 8, 9] {
        let repo = gix::open_opts(
            root,
            gix::open::Options::isolated().config_overrides([
                "core.fsmonitor=true".to_owned(),
                format!("gitoxide.fsmonitor.maxResponseSize={limit}"),
            ]),
        )?;
        let actual = mock_status(&repo, root, b"next-token", b"000dmock-two\x000000", false)?;
        let expected: &[&str] = if limit == 9 {
            &["dirty", "untracked"]
        } else {
            &["dir/a", "directory", "dirty", "reported", "unreported", "untracked"]
        };
        assert_eq!(
            actual, expected,
            "the 9-byte payload must fit the configured limit {limit}"
        );
    }
    Ok(())
}
