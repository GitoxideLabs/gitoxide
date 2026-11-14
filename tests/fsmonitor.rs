#![cfg(feature = "pretty-cli")]

use std::{
    path::Path,
    process::{Command, Output},
};

use gix_testtools::TestResult;

fn command(repository: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_gix"));
    gix_testtools::configure_git_environment(&mut command, repository);
    command.current_dir(repository).arg("-r").arg(repository);
    command
}

fn successful(output: Output) -> TestResult<Vec<u8>> {
    assert!(
        output.status.success(),
        "command succeeds without corrupting its output: {}; stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output.stdout)
}

#[cfg(any(target_os = "macos", target_os = "linux", windows))]
mod native {
    use super::*;
    use gix_testtools::tempfile::TempDir;
    use std::{ffi::OsStr, path::PathBuf, time::Duration};

    fn fsmonitor(repository: &Path) -> Command {
        let mut command = command(repository);
        command.arg("fsmonitor");
        command
    }

    struct Fixture {
        directory: TempDir,
        repository: PathBuf,
    }

    impl Fixture {
        fn new() -> TestResult<Self> {
            let directory = gix_testtools::tempfile::tempdir()?;
            let repository = directory.path().join("repository with 'quotes'");
            std::fs::create_dir(&repository)?;
            gix_testtools::git(&repository, "init --initial-branch=main")?;
            std::fs::write(repository.join("tracked"), "before\n")?;
            gix_testtools::git(&repository, "add tracked")?;
            gix_testtools::git(&repository, "commit -m baseline")?;
            Ok(Self { directory, repository })
        }
    }

    struct Cleanup(PathBuf);

    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fsmonitor(&self.0).arg("stop").output();
        }
    }

    fn token(response: &[u8]) -> TestResult<&OsStr> {
        let end = response
            .iter()
            .position(|byte| *byte == 0)
            .ok_or("filesystem monitor response has no token separator")?;
        Ok(OsStr::new(std::str::from_utf8(&response[..end])?))
    }

    fn query(repository: &Path, previous: Option<&OsStr>) -> TestResult<Vec<u8>> {
        let mut command = fsmonitor(repository);
        command.arg("query");
        if let Some(previous) = previous {
            command.arg(previous);
        }
        successful(command.output()?)
    }

    fn assert_changed(response: &[u8], path: &[u8], context: &str) {
        assert!(
            response.split(|byte| *byte == 0).any(|candidate| candidate == path),
            "{context}: same-stream synchronization includes writes preceding the query: {response:?}"
        );
    }

    #[test]
    fn self_spawn_preserves_native_tokens_and_control_commands() -> TestResult {
        let fixture = Fixture::new()?;
        let repository = &fixture.repository;
        let _cleanup = Cleanup(repository.clone());
        assert!(
            !fsmonitor(repository).arg("status").output()?.status.success(),
            "a missing daemon fails status"
        );
        successful(fsmonitor(repository).args(["start", "--idle-timeout", "30"]).output()?)?;
        successful(fsmonitor(repository).arg("status").output()?)?;
        assert!(
            !fsmonitor(repository)
                .args(["start", "--idle-timeout", "30"])
                .output()?
                .status
                .success(),
            "a second start cannot take over an occupied endpoint"
        );
        let baseline = query(repository, None)?;
        assert!(
            baseline.ends_with(b"\0/\0"),
            "an unknown token requests a full baseline"
        );
        std::fs::write(repository.join("tracked"), "after\n")?;
        assert_changed(
            &query(repository, Some(token(&baseline)?))?,
            b"tracked",
            "main worktree",
        );
        let flushed = successful(fsmonitor(repository).arg("flush").output()?)?;
        assert!(flushed.ends_with(b"\0/\0"), "flush returns a fresh baseline");
        assert!(
            query(repository, Some(token(&baseline)?))?.ends_with(b"\0/\0"),
            "flush retires earlier tokens"
        );
        assert!(
            successful(fsmonitor(repository).arg("stop").output()?)?.is_empty(),
            "stop emits no hook payload"
        );
        assert!(
            !fsmonitor(repository).arg("status").output()?.status.success(),
            "stop waits for endpoint release"
        );
        Ok(())
    }

    fn git_status(repository: &Path, provider: &str) -> TestResult<Vec<u8>> {
        successful(
            gix_testtools::git_command(repository)
                .args([
                    "-c",
                    &format!("core.fsmonitor={provider}"),
                    "-c",
                    "core.fsmonitorHookVersion=2",
                    "status",
                    "--porcelain=v2",
                    "-z",
                    "--untracked-files=all",
                ])
                .output()?,
        )
    }

    #[test]
    fn git_hook_v2_self_starts_the_current_binary_and_preserves_status() -> TestResult {
        let fixture = Fixture::new()?;
        let repository = &fixture.repository;
        let _cleanup = Cleanup(repository.clone());
        let provider = format!(
            "'{}' fsmonitor hook --idle-timeout 30",
            env!("CARGO_BIN_EXE_gix").replace('\\', "/").replace('\'', "'\\''")
        );
        assert_eq!(
            git_status(repository, &provider)?,
            git_status(repository, "false")?,
            "hook autostart emits only Git's binary response"
        );
        successful(fsmonitor(repository).arg("status").output()?)?;
        std::fs::write(repository.join("tracked"), "after\n")?;
        std::fs::write(repository.join("untracked"), "new\n")?;
        assert_eq!(
            git_status(repository, &provider)?,
            git_status(repository, "false")?,
            "Git's hook and full verification agree after immediate tracked and untracked writes"
        );
        Ok(())
    }

    #[test]
    fn self_spawn_keeps_linked_worktree_identity_and_command_line_configuration() -> TestResult {
        let fixture = Fixture::new()?;
        let repository = &fixture.repository;
        let linked = fixture.directory.path().join("linked checkout");
        successful(
            gix_testtools::git_command(repository)
                .args(["worktree", "add", "-b", "linked"])
                .arg(&linked)
                .output()?,
        )?;
        {
            let _cleanup = Cleanup(linked.clone());
            successful(fsmonitor(&linked).args(["start", "--idle-timeout", "30"]).output()?)?;
            assert!(
                !fsmonitor(repository).arg("status").output()?.status.success(),
                "a linked worktree owns its private endpoint rather than the main worktree's endpoint"
            );
            let baseline = query(&linked, None)?;
            std::fs::write(linked.join("tracked"), "linked\n")?;
            assert_changed(&query(&linked, Some(token(&baseline)?))?, b"tracked", "linked worktree");
        }
        // Persist the opposite setting so the reply proves that the child received the CLI override.
        gix_testtools::git(repository, "config core.precomposeUnicode false")?;
        let _cleanup = Cleanup(repository.clone());
        successful(
            command(repository)
                .arg("-c")
                .arg("core.precomposeUnicode=true")
                .args(["fsmonitor", "start", "--idle-timeout", "30"])
                .output()?,
        )?;
        let baseline = query(repository, None)?;
        std::fs::write(repository.join("a\u{308}"), "configured path\n")?;
        assert_changed(
            &query(repository, Some(token(&baseline)?))?,
            "ä".as_bytes(),
            "command-line Unicode configuration",
        );
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn self_spawn_preserves_conditional_configuration_through_symlinks() -> TestResult {
        let fixture = Fixture::new()?;
        let repository = &fixture.repository;
        let alias = fixture.directory.path().join("symlink alias");
        std::os::unix::fs::symlink(repository, &alias)?;
        gix_testtools::git(repository, "config core.precomposeUnicode false")?;
        let included = fixture.directory.path().join("conditional config");
        std::fs::write(
            &included,
            "[core]
            precomposeUnicode = true
        ",
        )?;
        successful(
            gix_testtools::git_command(repository)
                .args(["config", "--add"])
                .arg(format!("includeIf.gitdir:{}.path", alias.join(".git").display()))
                .arg(&included)
                .output()?,
        )?;
        // Only the logical Git directory matches the condition; canonicalizing it loses the setting.
        let _cleanup = Cleanup(alias.clone());
        successful(fsmonitor(&alias).args(["start", "--idle-timeout", "30"]).output()?)?;
        let baseline = query(&alias, None)?;
        std::fs::write(repository.join("a\u{308}"), "conditionally configured path\n")?;
        assert_changed(
            &query(&alias, Some(token(&baseline)?))?,
            "ä".as_bytes(),
            "conditional configuration through a symlink",
        );
        Ok(())
    }

    #[test]
    fn detached_child_receives_the_idle_deadline() -> TestResult {
        let fixture = Fixture::new()?;
        let repository = &fixture.repository;
        let _cleanup = Cleanup(repository.clone());
        successful(fsmonitor(repository).args(["start", "--idle-timeout", "1"]).output()?)?;
        // Probes extend the idle window, so leave the daemon entirely idle before checking once.
        std::thread::sleep(Duration::from_millis(1500));
        assert!(
            !fsmonitor(repository).arg("status").output()?.status.success(),
            "idle expiry is forwarded to the self-spawned child"
        );
        Ok(())
    }
}

#[test]
fn monitor_commands_validate_options_without_opening_a_repository() -> TestResult {
    let directory = gix_testtools::tempfile::tempdir()?;
    let help = successful(command(directory.path()).args(["watch", "--help"]).output()?)?;
    assert!(
        help.windows(b"--all-worktrees".len())
            .any(|part| part == b"--all-worktrees"),
        "watch exposes linked-worktree monitoring"
    );
    for args in [
        vec!["watch", "--no-metadata", "--all-worktrees"],
        vec!["watch", "--poll-ms", "0"],
        vec!["watch", "--retry-ms", "0"],
        vec!["watch", "--worktree-coverage", "invalid"],
        vec!["fsmonitor", "hook", "1", "token"],
    ] {
        assert!(
            command(directory.path()).args(args).output()?.status.code() == Some(2),
            "invalid monitoring choices fail during CLI parsing"
        );
    }
    Ok(())
}
