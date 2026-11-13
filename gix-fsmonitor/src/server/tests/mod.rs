mod cookies;
mod source;

use super::*;
use gix_notify::EventKind;

#[test]
fn remote_worktree_permission_does_not_override_probe_errors() -> gix_error::TestResult {
    let worktree = Path::new("worktree");
    for allow_remote in [false, true] {
        check_worktree(worktree, allow_remote, |path| {
            assert_eq!(
                path, worktree,
                "eligibility probes the worktree, not an administrative directory"
            );
            Ok(false)
        })?;
        assert!(
            check_worktree(worktree, allow_remote, |_| Err(io::Error::other("probe failed"))).is_err(),
            "filesystem errors remain failures even when remote monitoring is allowed"
        );
    }
    check_worktree(worktree, true, |_| Ok(true))?;
    let error =
        check_worktree(worktree, false, |_| Ok(true)).expect_err("remote worktrees require explicit permission");
    assert!(
        error.to_string().contains("fsmonitor.allowRemote=true"),
        "remote rejection explains how to opt in: {error}"
    );
    Ok(())
}

#[test]
fn idle_maintenance_preserves_history_until_loss_is_reported() -> gix_error::TestResult {
    let directory = gix_testtools::tempfile::tempdir()?;
    let root = directory.path().canonicalize()?.join("worktree");
    let git_dir = root.join(".git");
    std::fs::create_dir_all(&git_dir)?;
    let options = Options {
        retry_interval: Duration::from_secs(7),
        ..Default::default()
    };
    let mut monitor = Monitor::new(
        Layout {
            administrative: vec![git_dir.clone()],
            endpoint: git_dir.join("socket"),
            cookie_dir: git_dir.join("fsmonitor--daemon/cookies"),
            worktree: root.clone(),
            precompose_unicode: false,
            ignore_case: false,
        },
        Watcher::new(Default::default())?,
        options,
    )?;
    // Control time and loss directly, without racing native callbacks from fixture setup.
    monitor.watcher = Watcher::new(Default::default())?;
    let now = Instant::now();
    monitor.maintenance(now);
    let initial = monitor.journal.response(b"", true, now);
    let token = initial
        .split(|byte| *byte == 0)
        .next()
        .expect("responses begin with a token");
    monitor.journal.record(b"changed".to_vec(), now);
    let later = now + Duration::from_secs(120);
    monitor.maintenance(later);
    assert_eq!(
        monitor
            .journal
            .response(token, true, later)
            .splitn(2, |byte| *byte == 0)
            .nth(1),
        Some(b"changed\0".as_slice()),
        "retained changes remain available to the client while idle"
    );
    assert!(monitor.retry.is_none(), "idle time does not request watch registration");

    std::fs::remove_dir_all(root)?;
    let mut failure = monitor.watcher.drain(Budget::default());
    failure.error = Some(message("native root identity check failed").raise());
    monitor.ingest(failure, later);
    assert!(
        monitor.root_lost && !monitor.healthy,
        "a pathless native error rechecks roots so an obsolete server can stop"
    );
    assert_eq!(
        monitor.retry,
        Some(later + options.retry_interval),
        "native failures schedule recovery with the configured retry interval"
    );
    assert!(
        monitor.response(token, later).ends_with(b"\0/\0"),
        "native failure still requires full verification"
    );
    Ok(())
}

#[test]
fn complete_coverage_keeps_ignored_candidates_and_directory_ambiguity() {
    let root = PathBuf::from(if cfg!(windows) { "C:/work" } else { "/work" });
    let layout = Layout {
        administrative: vec![root.join(".git")],
        endpoint: root.join(".git/socket"),
        cookie_dir: root.join(".git/fsmonitor--daemon/cookies"),
        worktree: root.clone(),
        precompose_unicode: false,
        ignore_case: true,
    };
    let paths = layout.paths(Event {
        paths: vec![root.join("ignored/target/file"), root.join(".git/index")],
        kind: EventKind::Modify,
        path_kind: PathKind::Any,
    });
    assert_eq!(
        paths,
        Some(vec![b"ignored/target/file".to_vec(), b"ignored/target/file/".to_vec()]),
        "provider coverage ignores Git ignore rules and excludes only administrative files"
    );
    #[cfg(windows)]
    assert_eq!(
        layout.paths(Event {
            paths: vec![root.clone(), root.join("created")],
            kind: EventKind::Modify,
            path_kind: PathKind::Any,
        }),
        Some(vec![b"created".to_vec(), b"created/".to_vec()]),
        "Windows root metadata changes preserve the incremental child candidates"
    );
    for kind in [EventKind::Any, EventKind::Modify] {
        assert!(
            layout
                .paths(Event {
                    paths: vec![root.clone()],
                    kind,
                    path_kind: PathKind::Directory
                })
                .is_none(),
            "root-level directory scopes require full invalidation even for vanished Windows names"
        );
    }
}
#[cfg(unix)]
#[test]
fn git_path_conversion_preserves_non_utf8_bytes_and_rejects_unknown_paths() {
    use std::os::unix::ffi::OsStrExt;
    let root = PathBuf::from("/work");
    let layout = Layout {
        administrative: vec![root.join(".git")],
        endpoint: root.join(".git/socket"),
        cookie_dir: root.join(".git/fsmonitor--daemon/cookies"),
        worktree: root.clone(),
        precompose_unicode: true,
        ignore_case: false,
    };
    let path = std::ffi::OsStr::from_bytes(b"raw-\xff");
    assert_eq!(
        layout.paths(Event {
            paths: vec![root.join(path)],
            kind: EventKind::Modify,
            path_kind: PathKind::File
        }),
        Some(vec![b"raw-\xff".to_vec()]),
        "Git's Unix path conversion preserves non-UTF-8 names"
    );
    for paths in [Vec::new(), vec![PathBuf::new()]] {
        assert!(
            layout
                .paths(Event {
                    paths,
                    kind: EventKind::Any,
                    path_kind: PathKind::Any
                })
                .is_none(),
            "an unlocated event cannot certify an empty change list"
        );
    }
    #[cfg(target_os = "macos")]
    {
        let mut layout = layout;
        for (precompose_unicode, expected) in [(true, "ä"), (false, "a\u{308}")] {
            layout.precompose_unicode = precompose_unicode;
            assert_eq!(
                layout.paths(Event {
                    paths: vec![root.join("a\u{308}")],
                    kind: EventKind::Modify,
                    path_kind: PathKind::File
                }),
                Some(vec![expected.as_bytes().to_vec()]),
                "Unicode composition follows repository configuration independently of native event delivery"
            );
        }
    }
}
