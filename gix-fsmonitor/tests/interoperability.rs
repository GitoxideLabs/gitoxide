use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    time::{Duration, Instant},
};

use gix_testtools::tempfile::{TempDir, tempdir};

type Result<T = ()> = gix_testtools::Result<T>;

fn command(repo: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_gix-fsmonitor"));
    gix_testtools::configure_git_environment(&mut command, repo);
    command.current_dir(repo);
    command
}

fn successful(output: Output) -> Result<Vec<u8>> {
    if !output.status.success() {
        return Err(format!(
            "subprocess failed: {}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(output.stdout)
}

fn fixture() -> Result<TempDir> {
    let dir = tempdir()?;
    gix_testtools::git(dir.path(), "init --initial-branch=main")?;
    std::fs::write(dir.path().join(".gitignore"), "ignored/\n")?;
    std::fs::write(dir.path().join("tracked"), "before\n")?;
    std::fs::create_dir(dir.path().join("directory"))?;
    std::fs::write(dir.path().join("directory/nested"), "before\n")?;
    std::fs::create_dir(dir.path().join("ignored"))?;
    std::fs::write(dir.path().join("ignored/forced"), "before\n")?;
    gix_testtools::git(dir.path(), "add .")?;
    gix_testtools::git(dir.path(), "add -f ignored/forced")?;
    successful(
        gix_testtools::git_command(dir.path())
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-m",
                "baseline",
            ])
            .output()?,
    )?;
    Ok(dir)
}

struct Daemon {
    root: PathBuf,
    child: Option<Child>,
}
impl Daemon {
    fn start(root: &Path) -> Result<Self> {
        let child = command(root)
            .arg("run")
            .arg("--repo")
            .arg(root)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;
        let mut daemon = Self {
            root: root.to_owned(),
            child: Some(child),
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = daemon
                .child
                .as_mut()
                .expect("foreground child is retained")
                .try_wait()?
            {
                let output = daemon
                    .child
                    .take()
                    .expect("exited foreground child is retained")
                    .wait_with_output()?;
                return Err(format!(
                    "daemon exited at startup: {status}: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }
            let query = command(root).arg("query").output()?;
            if query.status.success() {
                return Ok(daemon);
            }
            if Instant::now() >= deadline {
                return Err(format!("daemon did not start: {}", String::from_utf8_lossy(&query.stderr)).into());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = command(&self.root).arg("stop").output();
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn hook() -> String {
    format!(
        "'{}' hook",
        env!("CARGO_BIN_EXE_gix-fsmonitor")
            .replace('\\', "/")
            .replace('\'', "'\\''")
    )
}

fn status(root: &Path, provider: &str, index: Option<&Path>) -> Result<Vec<u8>> {
    let mut command = gix_testtools::git_command(root);
    command
        .args([
            "-c",
            &format!("core.fsmonitor={provider}"),
            "-c",
            "core.fsmonitorHookVersion=2",
            "-c",
            "core.untrackedCache=true",
            "status",
            "--porcelain=v2",
            "-z",
            "--untracked-files=all",
        ])
        .env("GIT_OPTIONAL_LOCKS", "1");
    if let Some(index) = index {
        command.env("GIT_INDEX_FILE", index);
    }
    successful(command.output()?)
}

fn token(payload: &[u8]) -> Result<&OsStr> {
    let end = payload
        .iter()
        .position(|byte| *byte == 0)
        .ok_or("response has no token separator")?;
    Ok(OsStr::new(std::str::from_utf8(&payload[..end])?))
}

#[test]
fn hook_v2_and_native_ipc_preserve_git_status_with_ignored_and_alternate_index_paths() -> Result {
    let dir = fixture()?;
    let _daemon = Daemon::start(dir.path())?;
    let hook = hook();
    for provider in [&hook, "true"] {
        #[cfg(not(any(target_os = "macos", windows)))]
        if provider == "true" {
            continue;
        }
        assert_eq!(
            status(dir.path(), provider, None)?,
            status(dir.path(), "false", None)?,
            "a new monitor baseline matches ordinary Git status"
        );
        let _ = status(dir.path(), provider, None)?;
        std::fs::write(dir.path().join("tracked"), "after\n")?;
        std::fs::write(dir.path().join("ignored/forced"), "after\n")?;
        std::fs::write(dir.path().join("ignored/untracked"), "hidden\n")?;
        std::fs::rename(dir.path().join("directory"), dir.path().join("renamed"))?;
        assert_eq!(
            status(dir.path(), provider, None)?,
            status(dir.path(), "false", None)?,
            "an immediate query after file and directory changes agrees with Git's full verification"
        );
        std::fs::rename(dir.path().join("renamed"), dir.path().join("directory"))?;
    }

    let alternate = dir.path().join(".git/alternate-index");
    std::fs::copy(dir.path().join(".git/index"), &alternate)?;
    std::fs::write(dir.path().join("ignored/alternate-only"), "first\n")?;
    successful(
        gix_testtools::git_command(dir.path())
            .env("GIT_INDEX_FILE", &alternate)
            .args(["add", "-f", "ignored/alternate-only"])
            .output()?,
    )?;
    let _ = status(dir.path(), &hook, Some(&alternate))?;
    std::fs::write(dir.path().join("ignored/alternate-only"), "second\n")?;
    assert_eq!(
        status(dir.path(), &hook, Some(&alternate))?,
        status(dir.path(), "false", Some(&alternate))?,
        "provider coverage is independent of the default index and ignore pruning"
    );
    Ok(())
}

#[test]
fn native_control_commands_and_daemon_replacement_keep_token_semantics() -> Result {
    let dir = fixture()?;
    let daemon = Daemon::start(dir.path())?;
    let initial = successful(command(dir.path()).arg("query").output()?)?;
    assert!(initial.ends_with(b"\0/\0"), "a new client requests a full baseline");
    assert!(
        !command(dir.path()).arg("start").output()?.status.success(),
        "explicit start must not steal an occupied endpoint"
    );
    #[cfg(any(target_os = "macos", windows))]
    successful(
        gix_testtools::git_command(dir.path())
            .args(["fsmonitor--daemon", "status"])
            .output()?,
    )?;
    let flushed = successful(command(dir.path()).arg("flush").output()?)?;
    assert!(
        flushed.ends_with(b"\0/\0"),
        "flush creates a full invalidation response"
    );
    assert_ne!(token(&initial)?, token(&flushed)?, "flush changes the token generation");
    #[cfg(any(target_os = "macos", windows))]
    successful(
        gix_testtools::git_command(dir.path())
            .args(["fsmonitor--daemon", "stop"])
            .output()?,
    )?;
    #[cfg(not(any(target_os = "macos", windows)))]
    successful(command(dir.path()).arg("stop").output()?)?;
    drop(daemon);
    let _replacement = Daemon::start(dir.path())?;
    let replaced = successful(command(dir.path()).arg("query").arg(token(&initial)?).output()?)?;
    assert!(
        replaced.ends_with(b"\0/\0"),
        "a daemon restart cannot accept tokens from the previous instance"
    );
    let foreign = successful(command(dir.path()).arg("query").arg("builtin:foreign:1").output()?)?;
    assert!(
        foreign.ends_with(b"\0/\0"),
        "Git daemon tokens establish a fresh baseline"
    );
    Ok(())
}

#[test]
fn hook_autostarts_and_rejects_unsupported_versions() -> Result {
    let dir = fixture()?;
    let _daemon = Daemon {
        root: dir.path().to_owned(),
        child: None,
    };
    let first = successful(command(dir.path()).args(["hook", "2", ""]).output()?)?;
    assert!(
        first.ends_with(b"\0/\0"),
        "hook startup returns Git's full baseline marker"
    );
    assert!(
        !command(dir.path())
            .args(["hook", "1", "123"])
            .output()?
            .status
            .success(),
        "hook v1 is not silently interpreted as v2"
    );
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn native_fence_gives_useful_empty_queries_and_immediate_write_coverage() -> Result {
    let dir = fixture()?;
    let _daemon = Daemon::start(dir.path())?;
    let initial = successful(command(dir.path()).arg("query").output()?)?;
    let warmed = successful(command(dir.path()).arg("query").arg(token(&initial)?).output()?)?;
    assert!(
        !warmed.ends_with(b"\0/\0"),
        "owned FSEvents synchronization must produce useful incremental replies"
    );
    assert_eq!(
        warmed.splitn(2, |byte| *byte == 0).nth(1),
        Some(&b""[..]),
        "no filesystem writes yield an empty change list"
    );
    std::fs::write(dir.path().join("tracked"), "immediate\n")?;
    let changed = successful(command(dir.path()).arg("query").arg(token(&warmed)?).output()?)?;
    assert!(
        !changed.ends_with(b"\0/\0"),
        "ordinary changes should not require full scans"
    );
    assert!(
        changed.split(|byte| *byte == 0).any(|path| path == b"tracked"),
        "the fence includes a write immediately before the query without sleeps"
    );
    Ok(())
}

#[test]
fn linked_worktree_uses_external_cookie_coverage() -> Result {
    let primary = fixture()?;
    let dir = tempdir()?;
    let linked = dir.path().join("linked");
    successful(
        gix_testtools::git_command(primary.path())
            .args([
                OsStr::new("worktree"),
                OsStr::new("add"),
                OsStr::new("-b"),
                OsStr::new("linked"),
                linked.as_os_str(),
            ])
            .output()?,
    )?;
    let _daemon = Daemon::start(&linked)?;
    let initial = successful(command(&linked).arg("query").output()?)?;
    let warmed = successful(command(&linked).arg("query").arg(token(&initial)?).output()?)?;
    #[cfg(target_os = "macos")]
    assert!(
        !warmed.ends_with(b"\0/\0"),
        "the external gitdir cookie shares the worktree's native stream"
    );
    std::fs::write(linked.join("tracked"), "linked worktree write\n")?;
    let changed = successful(command(&linked).arg("query").arg(token(&warmed)?).output()?)?;
    #[cfg(target_os = "macos")]
    assert!(
        changed.split(|byte| *byte == 0).any(|path| path == b"tracked"),
        "the external cookie certifies immediate worktree writes"
    );
    #[cfg(not(target_os = "macos"))]
    assert!(
        changed.ends_with(b"\0/\0"),
        "compatibility backends remain conservative"
    );
    assert_eq!(
        status(&linked, &hook(), None)?,
        status(&linked, "false", None)?,
        "linked worktree status remains identical to full Git verification"
    );
    Ok(())
}

#[cfg(any(target_os = "macos", windows))]
#[test]
fn hook_client_queries_and_stops_gits_native_daemon() -> Result {
    let dir = fixture()?;
    successful(
        gix_testtools::git_command(dir.path())
            .args(["fsmonitor--daemon", "start"])
            .output()?,
    )?;
    let _cleanup = Daemon {
        root: dir.path().to_owned(),
        child: None,
    };
    let initial = successful(command(dir.path()).args(["hook", "2", ""]).output()?)?;
    assert!(
        initial.ends_with(b"\0/\0"),
        "our hook accepts Git's full-baseline response"
    );
    std::fs::write(dir.path().join("tracked"), "native Git daemon\n")?;
    let response = successful(
        command(dir.path())
            .args([OsStr::new("hook"), OsStr::new("2"), token(&initial)?])
            .output()?,
    )?;
    assert!(
        response.split(|byte| *byte == 0).any(|path| path == b"tracked"),
        "our binary framing preserves native Git's incremental response"
    );
    successful(command(dir.path()).arg("stop").output()?)?;
    assert!(
        !gix_testtools::git_command(dir.path())
            .args(["fsmonitor--daemon", "status"])
            .output()?
            .status
            .success(),
        "our quit command stops the native Git daemon"
    );
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn native_queries_normalize_configured_unicode() -> Result {
    let dir = fixture()?;
    gix_testtools::git(dir.path(), "config core.precomposeUnicode true")?;
    let _daemon = Daemon::start(dir.path())?;
    let initial = successful(command(dir.path()).arg("query").output()?)?;
    let warmed = successful(command(dir.path()).arg("query").arg(token(&initial)?).output()?)?;
    std::fs::write(dir.path().join("a\u{308}"), "decomposed path\n")?;
    let changed = successful(command(dir.path()).arg("query").arg(token(&warmed)?).output()?)?;
    assert!(
        changed.split(|byte| *byte == 0).any(|path| path == "ä".as_bytes()),
        "configured Unicode composition matches Git's index spelling"
    );
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn native_queries_preserve_coverage_through_unicode_and_case_root_aliases() -> Result {
    let fixture = fixture()?;
    gix_testtools::git(fixture.path(), "config core.precomposeUnicode true")?;
    let parent = tempdir()?;
    let root = parent.path().join("A\u{308}bc");
    std::fs::rename(fixture.path(), &root)?;
    for alias in [parent.path().join("Äbc"), parent.path().join("äBC")] {
        if !alias.exists() {
            continue; // Case-sensitive volumes need not resolve case aliases.
        }
        let _daemon = Daemon::start(&alias)?;
        let initial = successful(command(&alias).arg("query").output()?)?;
        let warmed = successful(command(&alias).arg("query").arg(token(&initial)?).output()?)?;
        assert!(
            !warmed.ends_with(b"\0/\0"),
            "root aliases retain useful native synchronization"
        );
        std::fs::write(root.join("tracked"), "changed through the physical root\n")?;
        let changed = successful(command(&alias).arg("query").arg(token(&warmed)?).output()?)?;
        assert!(
            changed.split(|byte| *byte == 0).any(|path| path == b"tracked"),
            "native path spelling must match the daemon's worktree root before stripping its prefix"
        );
    }
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn root_replacement_closes_the_obsolete_daemon() -> Result {
    let dir = fixture()?;
    let mut daemon = Daemon::start(dir.path())?;
    let original = dir.path().join(".git");
    let moved = dir.path().join(".git-moved");
    std::fs::rename(&original, &moved)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if daemon
            .child
            .as_mut()
            .expect("foreground daemon retained")
            .try_wait()?
            .is_some()
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "removing .git must stop an obsolete daemon even without client queries"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    std::fs::rename(moved, original)?;
    Ok(())
}

#[cfg(any(target_os = "macos", windows))]
#[test]
fn our_daemon_can_be_replaced_by_gits_native_daemon() -> Result {
    let dir = fixture()?;
    let daemon = Daemon::start(dir.path())?;
    let initial = successful(command(dir.path()).arg("query").output()?)?;
    drop(daemon);
    let _cleanup = Daemon {
        root: dir.path().to_owned(),
        child: None,
    };
    successful(
        gix_testtools::git_command(dir.path())
            .args(["fsmonitor--daemon", "start"])
            .output()?,
    )?;
    let replacement = successful(command(dir.path()).arg("query").arg(token(&initial)?).output()?)?;
    assert!(
        replacement.ends_with(b"\0/\0"),
        "Git can acquire the endpoint after our daemon exits and rejects its old tokens"
    );
    assert!(
        !command(dir.path()).arg("run").output()?.status.success(),
        "our daemon cannot take over an endpoint owned by native Git"
    );
    successful(
        gix_testtools::git_command(dir.path())
            .args(["fsmonitor--daemon", "status"])
            .output()?,
    )?;
    Ok(())
}
