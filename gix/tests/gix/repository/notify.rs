use std::time::{Duration, Instant};

use gix::notify::{Budget, Changes, Options, Outcome, RepositoryMonitor, Scope, Watch, WorktreeCoverage};
use gix_error::TestResult;

fn worktree_only(coverage: WorktreeCoverage) -> Options {
    Options {
        metadata: false,
        worktree_coverage: coverage,
        metadata_debounce: Duration::ZERO,
        ..Options::default()
    }
}

fn assert_worktree_only(changes: &Changes) {
    assert!(
        !changes.references
            && !changes.index
            && !changes.configuration
            && !changes.ignores
            && !changes.attributes
            && !changes.operations
            && !changes.layout,
        "metadata=false must suppress every metadata flag, including on refresh or loss: {changes:?}"
    );
}

fn service(monitor: &mut RepositoryMonitor, repo: &gix::Repository) -> Outcome {
    let out = monitor.service(Instant::now(), || gix::open_opts(repo.git_dir(), crate::restricted()));
    assert!(
        out.errors.is_empty(),
        "watch inventory maintenance succeeds: {:?}",
        out.errors
    );
    assert_worktree_only(&out.changes);
    out
}

#[test]
fn worktree_stream_requires_worktree_only_all_coverage_and_an_unstarted_nonbare_monitor() -> TestResult {
    let repo = crate::basic_repo()?;
    let valid = worktree_only(WorktreeCoverage::All);
    for options in [
        Options {
            metadata: true,
            ..valid.clone()
        },
        Options {
            worktree: false,
            ..valid.clone()
        },
        worktree_only(WorktreeCoverage::IgnoreAware),
    ] {
        let error = repo
            .monitor(options)?
            .into_worktree_stream()
            .err()
            .expect("invalid stream options are rejected");
        assert!(
            error.is_validation(),
            "invalid options retain validation classification: {error:?}"
        );
    }
    let bare = crate::named_subrepo_opts("make_basic_repo.sh", "bare.git", crate::restricted())?;
    let error = bare
        .monitor(valid.clone())?
        .into_worktree_stream()
        .err()
        .expect("a bare repository has no worktree stream");
    assert!(
        error.is_validation(),
        "bare repositories are rejected with a validation error"
    );

    let mut stream = repo.monitor(valid.clone())?.into_worktree_stream()?;
    let batch = stream.drain(Budget::default());
    assert!(
        batch.events.is_empty() && batch.loss.is_none() && batch.error.is_none(),
        "construction and drain install no subscriptions: {batch:?}"
    );
    let error = stream
        .register(&[Watch {
            path: "relative".into(),
            recursive: false,
        }])
        .expect_err("auxiliary paths must be absolute");
    assert!(
        error.is_validation(),
        "invalid auxiliary paths retain validation classification"
    );

    let mut monitor = repo.monitor(valid)?;
    let out = monitor.service(Instant::now(), || {
        panic!("All worktree-only coverage never opens a repository")
    });
    assert!(
        out.errors.is_empty(),
        "starting the monitor installs coverage: {:?}",
        out.errors
    );
    let error = monitor
        .into_worktree_stream()
        .err()
        .expect("conversion must not discard a started monitor's queued events");
    assert!(
        error.is_validation(),
        "started monitors are rejected with a validation error"
    );
    Ok(())
}

#[test]
fn ignore_aware_worktree_only_coverage_skips_ignored_subtrees() -> TestResult {
    assert_eq!(
        Options::default().worktree_coverage,
        WorktreeCoverage::IgnoreAware,
        "Git-aware directory selection remains the default"
    );
    let registrations = |name| -> TestResult<usize> {
        let repo = crate::named_subrepo_opts("make_notify_repo.sh", name, crate::restricted())?;
        let mut monitor = repo.monitor(worktree_only(WorktreeCoverage::IgnoreAware))?;
        let out = service(&mut monitor, &repo);
        assert_eq!(
            out.changes.worktree,
            Scope::All,
            "initial registration requests a worktree baseline"
        );
        assert!(
            out.statistics.added > 0,
            "worktree and dependency watches were installed"
        );
        Ok(out.statistics.added)
    };
    let ignored = registrations("ignored")?;
    let unignored = registrations("unignored")?;
    assert_eq!(
        unignored,
        ignored + 2,
        "only the unignored fixture watches ignored/ and ignored/deep/"
    );
    Ok(())
}

#[test]
fn all_coverage_services_a_malformed_index_without_opening_a_repository() -> TestResult {
    let (repo, _tmp) = crate::basic_rw_repo()?;
    std::fs::write(repo.index_path(), "not a Git index\n")?;
    let mut monitor = repo.monitor(worktree_only(WorktreeCoverage::All))?;
    let mut service_all = || {
        let out = monitor.service(Instant::now(), || {
            panic!("All worktree-only coverage must not inspect repository data")
        });
        assert!(
            out.errors.is_empty(),
            "a malformed index is irrelevant to recursive coverage: {:?}",
            out.errors
        );
        assert_worktree_only(&out.changes);
        out
    };
    let out = service_all();
    assert_eq!(
        out.statistics.added, 1,
        "All needs only one recursive worktree registration"
    );
    assert_eq!(
        out.changes.worktree,
        Scope::All,
        "registration still requests a baseline"
    );
    service_all();
    monitor.rescan();
    let out = monitor.service(Instant::now(), || {
        panic!("explicit rescans also avoid opening a repository")
    });
    assert!(
        out.errors.is_empty(),
        "rescan also ignores the malformed index: {:?}",
        out.errors
    );
    assert_worktree_only(&out.changes);

    let mut ignore_aware = repo.monitor(Options {
        metadata: false,
        ..Options::default()
    })?;
    let mut opened = false;
    let out = ignore_aware.service(Instant::now(), || {
        opened = true;
        gix::open_opts(repo.git_dir(), crate::restricted())
    });
    assert!(opened, "default IgnoreAware coverage needs fresh repository data");
    assert!(
        !out.errors.is_empty(),
        "IgnoreAware cannot select directories from a malformed index"
    );
    assert!(
        out.errors
            .iter()
            .any(|error| format!("{error:?}").contains("could not load index")),
        "the failure identifies the index dependency: {:?}",
        out.errors
    );
    assert_eq!(
        out.changes.worktree,
        Scope::All,
        "failed coverage requires worktree reconciliation"
    );
    assert_worktree_only(&out.changes);

    #[cfg(unix)]
    {
        drop(monitor);
        drop(ignore_aware);
        let index_path = repo.index_path();
        std::fs::remove_file(&index_path)?;
        std::os::unix::fs::symlink("index", &index_path)?;
        for (worktree, coverage) in [
            (true, WorktreeCoverage::All),
            (false, WorktreeCoverage::All),
            (false, WorktreeCoverage::IgnoreAware),
        ] {
            let options = Options {
                worktree,
                ..worktree_only(coverage)
            };
            let mut monitor = repo.monitor(options.clone())?;
            let out = monitor.service(Instant::now(), || {
                panic!("unfiltered or disabled monitoring must not open an index dependency")
            });
            assert!(
                out.errors.is_empty(),
                "an index symlink loop does not prevent unfiltered or disabled monitoring: {:?}",
                out.errors
            );
            assert_worktree_only(&out.changes);
            assert_eq!(
                out.changes.worktree,
                if worktree { Scope::All } else { Scope::None },
                "only active coverage requires a baseline despite the unreadable index path"
            );
            assert_eq!(
                out.statistics.added,
                usize::from(worktree),
                "only active coverage installs its recursive root"
            );
            if worktree {
                let mut stream = repo.monitor(options)?.into_worktree_stream()?;
                stream.register(&[])?;
                let batch = stream.drain(Budget::default());
                assert_eq!(
                    batch.loss,
                    Some(gix::notify::Loss::WatchSetChanged),
                    "a raw stream can register its baseline with an index symlink loop"
                );
                assert!(
                    batch.error.is_none(),
                    "raw draining does not resolve the index: {batch:?}"
                );
            }
        }
        let error = match repo.monitor(worktree_only(WorktreeCoverage::IgnoreAware)) {
            Err(error) => error,
            Ok(mut monitor) => {
                let mut out = monitor.service(Instant::now(), || gix::open_opts(repo.git_dir(), crate::restricted()));
                assert_worktree_only(&out.changes);
                assert!(
                    !out.errors.is_empty(),
                    "IgnoreAware must reject the index symlink loop when loading its inventory"
                );
                out.errors.remove(0)
            }
        };
        let diagnostic = format!("{error:?}");
        assert!(
            diagnostic.contains("symlinks") || diagnostic.contains("could not load index"),
            "IgnoreAware reports the symlink limit or its unreadable index dependency: {diagnostic}"
        );
    }
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod native {
    use std::{path::Path, sync::mpsc};

    use gix::notify::{Event, EventKind, Fence, Loss, PathKind, SynchronizeError, WorktreeStream};

    use super::*;

    const TIMEOUT: Duration = Duration::from_secs(5);

    fn wait_for(
        monitor: &mut RepositoryMonitor,
        repo: &gix::Repository,
        description: &str,
        mut done: impl FnMut(&Outcome) -> bool,
    ) {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let out = service(monitor, repo);
            if done(&out) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {description}; last changes: {:?}",
                out.changes
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn linked_worktree_inside_common_directory_publishes_classified_file_changes() -> TestResult {
        let (repo, _tmp) = crate::basic_rw_repo()?;
        let main = repo.workdir().expect("the fixture has a worktree");
        let bare = main.join("bare.git");
        let output = gix_testtools::git_command(&bare)
            .args(["fetch", "--no-tags"])
            .arg(main)
            .arg("HEAD")
            .output()?;
        assert!(
            output.status.success(),
            "the fixture's empty bare repository receives a tracked file: {output:?}"
        );
        // A common-directory ancestor must not turn the entire linked worktree into metadata.
        let linked = bare.join("checkouts/root");
        let output = gix_testtools::git_command(&bare)
            .args(["worktree", "add", "--detach"])
            .arg(&linked)
            .arg("FETCH_HEAD")
            .output()?;
        assert!(
            output.status.success(),
            "Git creates the linked worktree inside its bare common directory: {output:?}"
        );
        let repo = gix::open_opts(&linked, crate::restricted())?;

        assert!(
            linked.canonicalize()?.starts_with(repo.common_dir().canonicalize()?),
            "the worktree is genuinely beneath its shared administrative directory"
        );
        for coverage in [WorktreeCoverage::All, WorktreeCoverage::IgnoreAware] {
            let mut monitor = repo.monitor(worktree_only(coverage))?;
            service(&mut monitor, &repo);
            service(&mut monitor, &repo);
            // Startup invalidations may absorb a write into All, but later writes must remain path-scoped.
            let deadline = Instant::now() + TIMEOUT;
            let mut next_write = Instant::now();
            let mut writes = 0;
            let mut last_observation = None;
            loop {
                if Instant::now() >= next_write {
                    writes += 1;
                    std::fs::write(
                        linked.join("this"),
                        format!("change {writes} with {coverage:?} coverage\n"),
                    )?;
                    next_write = Instant::now() + Duration::from_millis(100);
                }
                let out = service(&mut monitor, &repo);
                if matches!(&out.changes.worktree, Scope::Paths(paths) if paths.iter().any(|path| path == "this")) {
                    break;
                }
                if !out.changes.is_empty() || out.statistics.received != 0 {
                    last_observation = Some((out.changes.worktree, out.statistics));
                }
                assert!(
                    Instant::now() < deadline,
                    "{coverage:?} must publish the literal tracked-file scope beneath common_dir after {writes} writes; last observation: {last_observation:?}"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        Ok(())
    }

    #[test]
    fn ignore_and_index_events_refresh_coverage_without_publishing_metadata() -> TestResult {
        let (repo, _tmp) = crate::basic_rw_repo()?;
        let root = repo.workdir().expect("the fixture has a worktree");
        for directory in ["ignore-refresh/deep", "index-refresh/deep"] {
            std::fs::create_dir_all(root.join(directory))?;
            std::fs::write(root.join(directory).join("file"), "initial\n")?;
        }
        std::fs::write(
            root.join(".gitignore"),
            "ignore-refresh/
index-refresh/
",
        )?;
        let mut monitor = repo.monitor(worktree_only(WorktreeCoverage::IgnoreAware))?;
        service(&mut monitor, &repo);
        service(&mut monitor, &repo);

        std::fs::write(root.join(".gitignore"), "index-refresh/\n")?;
        let mut added = 0;
        wait_for(
            &mut monitor,
            &repo,
            "the .gitignore event to expand directory coverage",
            |out| {
                added += out.statistics.added;
                added >= 2
            },
        );
        assert_eq!(
            added, 2,
            "unignoring adds both existing directories without an explicit rescan"
        );

        gix_testtools::git(root, "add -f -- index-refresh/deep/file")?;
        let mut added = 0;
        wait_for(
            &mut monitor,
            &repo,
            "the index event to cover the newly tracked ignored path",
            |out| {
                added += out.statistics.added;
                added >= 2
            },
        );
        assert_eq!(
            added, 2,
            "the refreshed index exposes both directories despite their ignore rule"
        );

        for path in ["ignore-refresh/deep/file", "index-refresh/deep/file"] {
            std::fs::write(root.join(path), "changed after coverage expanded\n")?;
            wait_for(&mut monitor, &repo, path, |out| {
                // A late dependency event may broaden the scope, but the file must actually arrive.
                out.statistics.paths.iter().any(|candidate| candidate.ends_with(path))
                    && match &out.changes.worktree {
                        Scope::All => true,
                        Scope::Paths(paths) => paths.iter().any(|candidate| candidate == path),
                        Scope::None => false,
                    }
            });
        }
        Ok(())
    }

    fn drain_through(stream: &mut WorktreeStream, fence: Fence) -> Vec<Event> {
        let mut events = Vec::new();
        loop {
            let batch = stream.drain(Budget {
                max_events: 1,
                max_bytes: 1,
            });
            assert!(
                batch.loss.is_none() && batch.error.is_none(),
                "covered writes preserve native delivery: {batch:?}"
            );
            assert_eq!(
                batch.generation, fence.generation,
                "the fence is consumed in its original coverage generation"
            );
            assert!(
                batch.events.len() <= 1,
                "the event budget is honored even through a fence"
            );
            events.extend(batch.events);
            if batch.sequence >= fence.sequence {
                return events;
            }
            assert!(
                batch.more,
                "all publications preceding the completed fence are ready to drain"
            );
        }
    }

    fn assert_file_event(events: &[Event], path: &Path) {
        assert!(
            events
                .iter()
                .any(|event| event.paths.iter().any(|candidate| candidate == path)
                    && matches!(event.path_kind, PathKind::File | PathKind::Any)
                    && matches!(event.kind, EventKind::Create | EventKind::Modify)),
            "the raw API exposes {path:?}, preserving file or unknown native type hints: {events:?}"
        );
    }

    #[test]
    fn raw_stream_fences_unfiltered_events_and_replaces_auxiliary_registration() -> TestResult {
        let (repo, _tmp) = crate::basic_rw_repo()?;
        let root = repo.workdir().expect("the fixture has a worktree").canonicalize()?;
        std::fs::create_dir_all(root.join("ignored/deep"))?;
        std::fs::write(root.join("ignored/deep/file"), "ignored\n")?;
        std::fs::write(
            root.join(".gitignore"),
            "ignored/
this
",
        )?;
        let cookies_dir = gix_testtools::tempfile::tempdir()?;
        let cookies = cookies_dir.path().canonicalize()?;
        let replacement = gix_testtools::tempfile::tempdir()?;
        let replacement_path = replacement.path().canonicalize()?;
        let auxiliary = Watch {
            path: cookies.clone(),
            recursive: false,
        };
        let mut stream = repo
            .monitor(worktree_only(WorktreeCoverage::All))?
            .into_worktree_stream()?;
        assert_eq!(
            stream.synchronize_at(&cookies, Instant::now() + TIMEOUT),
            Err(SynchronizeError::Unsupported),
            "a fence does not implicitly register the worktree or auxiliary paths"
        );
        stream.register(std::slice::from_ref(&auxiliary))?;
        let initial = stream.drain(Budget {
            max_events: 0,
            max_bytes: 0,
        });
        assert_eq!(
            initial.loss,
            Some(Loss::WatchSetChanged),
            "even a zero budget reports the initial baseline requirement"
        );
        assert!(
            initial.error.is_none() && initial.events.is_empty(),
            "registration succeeds without draining events: {initial:?}"
        );
        let fence = match stream.synchronize_at(&cookies, Instant::now() + TIMEOUT) {
            Ok(fence) => fence,
            Err(SynchronizeError::Unsupported) => {
                eprintln!("skipping native fence assertions on a filesystem without synchronization support");
                return Ok(());
            }
            Err(error) => return Err(error.into()),
        };
        assert_eq!(
            fence.generation, initial.generation,
            "an external cookie fences this stream's existing generation"
        );
        drain_through(&mut stream, fence);

        let (send, wake) = mpsc::sync_channel(1);
        stream.set_waker(move || {
            let _ = send.try_send(());
        });
        while wake.try_recv().is_ok() {}
        let paths = [
            root.join("this"),
            root.join("untracked"),
            root.join("ignored/deep/file"),
            root.join("repo.git/this"),
            root.join("repo.git/.git/raw-notify"),
            root.join(".git/raw-notify"),
            cookies.join("auxiliary"),
        ];
        for path in &paths {
            std::fs::write(path, "raw change\n")?;
        }
        let directory = root.join("new-directory");
        std::fs::create_dir(&directory)?;
        wake.recv_timeout(TIMEOUT)
            .expect("native events wake the caller's event loop within the deadline");
        let fence = stream.synchronize_at(&cookies, Instant::now() + TIMEOUT)?;
        stream.register(&[
            auxiliary.clone(),
            auxiliary,
            Watch {
                path: root.clone(),
                recursive: false,
            },
        ])?;
        let repeated = stream.drain(Budget {
            max_events: 0,
            max_bytes: 0,
        });
        assert_eq!(
            repeated.generation, initial.generation,
            "repeating an equivalent watch set preserves generation"
        );
        assert!(
            repeated.loss.is_none() && repeated.error.is_none(),
            "duplicate and redundant registration does not report loss: {repeated:?}"
        );
        assert!(
            repeated.events.is_empty() && repeated.more,
            "a zero budget retains the fenced notifications for later draining"
        );
        let events = drain_through(&mut stream, fence);
        for path in &paths {
            assert_file_event(&events, path);
        }
        assert!(
            events
                .iter()
                .any(|event| event.paths.contains(&directory) && event.path_kind == PathKind::Directory),
            "raw events distinguish directory changes: {events:?}"
        );

        stream.register(&[Watch {
            path: replacement_path.clone(),
            recursive: false,
        }])?;
        let changed = stream.drain(Budget::default());
        assert_eq!(
            changed.loss,
            Some(Loss::WatchSetChanged),
            "replacing the auxiliary set requests a new baseline"
        );
        assert!(
            changed.generation > fence.generation && changed.error.is_none(),
            "replacement advances generation without restarting history: {changed:?}"
        );
        assert_eq!(
            stream.synchronize_at(&cookies, Instant::now() + TIMEOUT),
            Err(SynchronizeError::Unsupported),
            "the previous auxiliary directory is no longer covered"
        );
        let auxiliary_file = replacement_path.join("replacement");
        std::fs::write(&auxiliary_file, "replacement coverage\n")?;
        std::fs::write(&paths[0], "worktree coverage retained\n")?;
        let fence = stream.synchronize_at(&replacement_path, Instant::now() + TIMEOUT)?;
        let events = drain_through(&mut stream, fence);
        assert_file_event(&events, &auxiliary_file);
        assert_file_event(&events, &paths[0]);

        stream.register(&[])?;
        let removed = stream.drain(Budget::default());
        assert_eq!(
            removed.loss,
            Some(Loss::WatchSetChanged),
            "an empty auxiliary set removes the last additional watch"
        );
        assert!(
            removed.generation > fence.generation && removed.error.is_none(),
            "removing coverage also advances generation: {removed:?}"
        );
        assert_eq!(
            stream.synchronize_at(&replacement_path, Instant::now() + TIMEOUT),
            Err(SynchronizeError::Unsupported),
            "removed auxiliary coverage cannot certify delivery"
        );
        let fence = stream.synchronize_at(&root, Instant::now() + TIMEOUT)?;
        drain_through(&mut stream, fence);
        Ok(())
    }
}
