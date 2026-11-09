use std::time::{Duration, Instant};

use gix::notify::{
    Changes, DependencyKind, Layout, Options, Outcome, RepositoryMonitor, Scope, WorktreeChange, WorktreeCoverage,
};
use gix_error::TestResult;

fn worktree_only(coverage: WorktreeCoverage) -> Options {
    Options {
        metadata: false,
        worktree_coverage: Some(coverage),
        metadata_debounce: Duration::ZERO,
        ..Options::default()
    }
}

fn subtree_watch_count() -> usize {
    // These fixtures expose a subtree with its root and one subdirectory.
    if gix_notify::Watcher::supports_native_recursive_watches() {
        1
    } else {
        2
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
fn manual_monitor_refresh_invalidates_every_enabled_cache() -> TestResult {
    use gix::error::{ErrorExt, message};

    let repo = crate::basic_repo()?;
    let root = repo.workdir().expect("the read-only fixture has a worktree");
    let opener = || gix::open_opts(root, crate::restricted());
    for worktree_coverage in [None, Some(WorktreeCoverage::All), Some(WorktreeCoverage::IgnoreAware)] {
        let mut monitor = repo.monitor(Options {
            worktree_coverage,
            ..Options::default()
        })?;
        for _ in 0..2 {
            let outcome = monitor.service(Instant::now(), opener);
            assert!(outcome.errors.is_empty(), "startup succeeds: {:?}", outcome.errors);
        }
        monitor.rescan();
        let mut opens = 0;
        let refreshed = monitor.service(Instant::now(), || {
            opens += 1;
            opener()
        });
        assert!(
            refreshed.errors.is_empty(),
            "manual refresh succeeds: {:?}",
            refreshed.errors
        );
        assert_eq!(
            opens, 1,
            "manual refresh rediscovers the inventory using a fresh repository"
        );
        assert_eq!(
            refreshed.changes,
            Changes {
                references: true,
                index: worktree_coverage.is_some(),
                configuration: true,
                ignores: true,
                attributes: true,
                operations: true,
                layout: true,
                worktree: worktree_coverage.map(|_| Scope::All),
            },
            "an explicit refresh invalidates every enabled cache even when the watch set is unchanged"
        );
        let verified = monitor.service(Instant::now(), opener);
        assert!(
            verified.errors.is_empty(),
            "refreshed watches are verified successfully"
        );
        assert_eq!(
            (verified.statistics.added, verified.statistics.removed),
            (0, 0),
            "verification retains the established subscriptions despite asynchronous notifications"
        );

        monitor.rescan();
        let failed = monitor.service(Instant::now(), || Err(message("deliberate refresh failure").raise()));
        assert_eq!(
            failed.errors.len(),
            1,
            "failed rediscovery remains visible to the caller"
        );
        assert_eq!(
            failed.changes, refreshed.changes,
            "manual refresh invalidates every enabled cache even when rediscovery fails"
        );
    }
    Ok(())
}

fn assert_options(actual: &Options, expected: &Options) {
    assert_eq!(
        (
            actual.metadata,
            actual.all_worktrees,
            actual.worktree_coverage,
            actual.metadata_debounce,
            actual.max_latency,
            actual.retry_interval,
        ),
        (
            expected.metadata,
            expected.all_worktrees,
            expected.worktree_coverage,
            expected.metadata_debounce,
            expected.max_latency,
            expected.retry_interval,
        ),
        "the options accessor reports every effective coverage and scheduling choice"
    );
}

#[test]
fn monitor_defers_worktree_discovery_until_service() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_notify_worktrees.sh")?;
    let repo = gix::open_opts(fixture.join("main"), crate::restricted())?;
    let options = Options {
        metadata: true,
        all_worktrees: true,
        worktree_coverage: Some(WorktreeCoverage::IgnoreAware),
        metadata_debounce: Duration::from_millis(19),
        max_latency: Duration::from_millis(43),
        retry_interval: Duration::from_millis(89),
    };
    let mut monitor = repo.monitor(options.clone())?;
    assert_options(monitor.options(), &options);
    assert_eq!(
        monitor.other_worktrees().count(),
        0,
        "construction leaves discovery to the first service call"
    );
    let mut opens = 0;
    let out = monitor.service(Instant::now(), || {
        opens += 1;
        gix::open_opts(repo.git_dir(), crate::restricted())
    });
    assert!(out.errors.is_empty(), "initial discovery succeeds: {:?}", out.errors);
    assert_eq!(opens, 1, "the first service opens the repository for discovery");
    assert_eq!(
        out.other_worktrees.len(),
        2,
        "enabling all_worktrees discovers the fixture's two linked checkouts"
    );
    assert!(
        out.other_worktrees
            .iter()
            .all(|change| change.kind == WorktreeChange::Added),
        "newly enabled checkouts receive initial baselines rather than incremental invalidations"
    );
    Ok(())
}

#[test]
fn monitor_rejects_invalid_options() -> TestResult {
    let repo = crate::basic_repo()?;
    for (invalid, diagnostic) in [
        (
            Options {
                all_worktrees: true,
                ..worktree_only(WorktreeCoverage::All)
            },
            "all-worktree monitoring requires metadata",
        ),
        (
            Options {
                retry_interval: Duration::ZERO,
                ..worktree_only(WorktreeCoverage::All)
            },
            "retry interval must be positive",
        ),
    ] {
        let error = repo
            .monitor(invalid)
            .err()
            .expect("invalid options must be rejected during construction");
        assert!(
            format!("{error:?}").contains(diagnostic),
            "construction identifies the invalid option: {error:?}"
        );
    }
    Ok(())
}

fn pattern_options() -> gix::open::Options {
    let mut options = crate::restricted();
    options.permissions.env.xdg_config_home = gix::sec::Permission::Allow;
    options.permissions.attributes.git = true;
    options
}

fn has_dependency(layout: &Layout, path: &std::path::Path, kind: DependencyKind) -> bool {
    layout
        .dependencies
        .iter()
        .any(|dependency| dependency.path == path && dependency.kind == kind)
}

#[test]
fn pattern_dependencies_include_optional_candidates_and_selected_fallbacks() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let root = gix_testtools::scripted_fixture_read_only_with_args("make_notify_repo.sh", ["pattern-dependencies"])?
        .canonicalize()?;
    let _cwd = gix_testtools::set_current_dir(&root)?;
    let _env = gix_testtools::Env::new().set(
        "XDG_CONFIG_HOME",
        root.join("xdg").to_str().expect("UTF-8 fixture path"),
    );
    let mut repo = gix::open_opts(&root, pattern_options())?;
    let _elsewhere = gix_testtools::set_current_dir(root.join("xdg"))?;
    assert!(
        !root.join(".git/patterns").exists(),
        "even the optional candidates' ancestor is missing"
    );
    for (metadata, worktree_coverage) in [
        (true, None),
        (true, Some(WorktreeCoverage::All)),
        (false, Some(WorktreeCoverage::IgnoreAware)),
        (false, Some(WorktreeCoverage::All)),
        (false, None),
    ] {
        let monitor = repo.monitor(Options {
            metadata,
            worktree_coverage,
            ..Options::default()
        })?;
        for (name, kind) in [
            ("ignore", DependencyKind::Ignore),
            ("attributes", DependencyKind::Attributes),
        ] {
            for path in [
                root.join(".git/patterns/nested").join(name),
                root.join("xdg/git").join(name),
            ] {
                assert_eq!(
                    has_dependency(monitor.layout(), &path, kind),
                    metadata
                        || kind == DependencyKind::Ignore && worktree_coverage == Some(WorktreeCoverage::IgnoreAware),
                    "missing optional candidates and selected fallbacks are dependencies only when needed: {} ({kind:?})",
                    path.display()
                );
            }
        }
    }

    repo.config_snapshot_mut()
        .set_raw_value("core.excludesFile", "xdg/git/ignore")?;
    let index = repo.index_or_empty()?;
    assert!(
        repo.excludes(
            &index,
            None,
            gix_worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
        )?
        .at_entry("ignored/deep/file", Some(gix_index::entry::Mode::FILE))?
        .is_excluded(),
        "pattern loading uses the same captured path base as monitoring after the process CWD changes"
    );
    for key in ["core.excludesFile", "core.attributesFile"] {
        repo.config_snapshot_mut().set_raw_value(key, ":(optional)this")?;
    }
    let monitor = repo.monitor(Options::default())?;
    for (name, kind) in [
        ("ignore", DependencyKind::Ignore),
        ("attributes", DependencyKind::Attributes),
    ] {
        assert!(
            has_dependency(monitor.layout(), &root.join("this"), kind)
                && !has_dependency(monitor.layout(), &root.join("xdg/git").join(name), kind),
            "existing optional candidates use the repository's captured CWD and suppress fallback selection: {kind:?}"
        );
    }
    Ok(())
}

#[test]
fn pattern_dependencies_respect_permissions_and_watch_boundaries() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let root = gix_testtools::scripted_fixture_read_only_with_args("make_notify_repo.sh", ["pattern-dependencies"])?
        .canonicalize()?;
    let _cwd = gix_testtools::set_current_dir(&root)?;
    let _env = gix_testtools::Env::new().set(
        "XDG_CONFIG_HOME",
        root.join("xdg").to_str().expect("UTF-8 fixture path"),
    );
    for allow_xdg in [false, true] {
        let mut options = pattern_options();
        options.permissions.attributes.git = false;
        if !allow_xdg {
            options.permissions.env.xdg_config_home = gix::sec::Permission::Deny;
        }
        let repo = gix::open_opts(&root, options)?;
        let monitor = repo.monitor(Options::default())?;
        assert_eq!(
            has_dependency(monitor.layout(), &root.join("xdg/git/ignore"), DependencyKind::Ignore),
            allow_xdg,
            "ignore fallback selection honors XDG permission"
        );
        assert!(
            !has_dependency(
                monitor.layout(),
                &root.join("xdg/git/attributes"),
                DependencyKind::Attributes
            ) && has_dependency(
                monitor.layout(),
                &root.join(".git/patterns/nested/attributes"),
                DependencyKind::Attributes
            ),
            "disabling Git's default attributes does not disable explicitly configured candidates"
        );
    }

    let mut options = pattern_options();
    options.permissions.env.xdg_config_home = gix::sec::Permission::Forbid;
    let mut repo = gix::open_opts(&root, options)?;
    for options in [Options::default(), worktree_only(WorktreeCoverage::IgnoreAware)] {
        assert!(
            repo.monitor(options).is_err(),
            "forbidden ignore fallback access remains an error when ignore dependencies are needed"
        );
    }
    repo.config_snapshot_mut().set_raw_value("core.excludesFile", "this")?;
    let monitor = repo.monitor(Options::default())?;
    assert!(
        !has_dependency(
            monitor.layout(),
            &root.join("xdg/git/attributes"),
            DependencyKind::Attributes
        ),
        "unlike ignores, attribute fallback lookup tolerates forbidden XDG access"
    );

    let outside = root.parent().expect("the fixture has a parent").join("outside-xdg");
    let _outside = gix_testtools::Env::new().set("XDG_CONFIG_HOME", outside.to_str().expect("UTF-8 fixture path"));
    let repo = gix::open_opts(&root, pattern_options())?;
    let monitor = repo.monitor(Options::default())?;
    assert!(
        monitor
            .layout()
            .dependencies
            .iter()
            .all(|dependency| dependency.path.starts_with(&root) || dependency.kind == DependencyKind::Ignore),
        "only external ignore fallbacks extend coverage: {:?}",
        monitor.layout().dependencies
    );
    Ok(())
}

#[test]
fn pattern_dependencies_report_only_errors_relevant_to_the_selected_coverage() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only_with_args("make_notify_repo.sh", ["pattern-dependencies"])?;
    for key in ["core.attributesFile", "core.excludesFile"] {
        let repo = gix::open_opts(
            &root,
            crate::restricted()
                .strict_config(true)
                .config_overrides([format!("{key}=")]),
        )?;
        for coverage in [None, Some(WorktreeCoverage::All), Some(WorktreeCoverage::IgnoreAware)] {
            let result = repo.monitor(Options {
                worktree_coverage: coverage,
                ..worktree_only(WorktreeCoverage::All)
            });
            if key == "core.excludesFile" && coverage == Some(WorktreeCoverage::IgnoreAware) {
                assert!(
                    result.is_err(),
                    "IgnoreAware must validate its global ignore configuration"
                );
            } else {
                let monitor = result?;
                assert!(
                    monitor
                        .layout()
                        .dependencies
                        .iter()
                        .all(|dependency| dependency.kind != DependencyKind::Attributes),
                    "worktree-only coverage never selects attribute dependencies"
                );
            }
        }
        let error = repo
            .monitor(Options::default())
            .err()
            .expect("metadata requires valid pattern paths");
        assert!(
            format!("{error:?}").contains(key),
            "the invalid pattern setting remains identifiable: {error:?}"
        );
    }
    Ok(())
}

#[cfg(windows)]
#[test]
fn metadata_layout_uses_native_windows_paths_including_missing_dependencies() -> TestResult {
    let repo = crate::basic_repo()?;
    let monitor = repo.monitor(Options::default())?;
    let layout = monitor.layout();
    let git_dir = repo.git_dir().canonicalize()?;
    let common_dir = repo.common_dir().canonicalize()?;
    assert_eq!(
        layout.git_dir, git_dir,
        "metadata paths match native notification prefixes"
    );
    assert_eq!(
        layout.common_dir, common_dir,
        "shared metadata uses the same native spelling"
    );
    assert!(
        layout
            .dependencies
            .iter()
            .any(|dependency| dependency.path == common_dir.join("info/attributes")),
        "missing dependencies retain their suffix beneath the canonical native root"
    );
    Ok(())
}

#[test]
fn ignore_aware_worktree_only_coverage_skips_ignored_subtrees() -> TestResult {
    assert_eq!(
        Options::default().worktree_coverage,
        Some(WorktreeCoverage::IgnoreAware),
        "Git-aware directory selection remains the default"
    );
    let registrations = |name| -> TestResult<usize> {
        let repo = crate::named_subrepo_opts("make_notify_repo.sh", name, crate::restricted())?;
        let mut monitor = repo.monitor(worktree_only(WorktreeCoverage::IgnoreAware))?;
        let out = service(&mut monitor, &repo);
        assert_eq!(
            out.changes.worktree,
            Some(Scope::All),
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
        ignored + subtree_watch_count(),
        "the unignored fixture adds coverage for ignored/ and its descendants"
    );
    Ok(())
}

#[test]
fn all_coverage_services_a_malformed_index_without_opening_a_repository() -> TestResult {
    let dir = gix_testtools::scripted_fixture_read_only_with_args("make_notify_repo.sh", ["malformed-index"])?;
    let repo = gix::open_opts(dir, crate::restricted())?;
    let enabled_options = Options {
        metadata_debounce: Duration::from_millis(17),
        max_latency: Duration::from_millis(41),
        retry_interval: Duration::from_millis(83),
        ..worktree_only(WorktreeCoverage::All)
    };
    let mut monitor = repo.monitor(enabled_options.clone())?;
    assert_options(monitor.options(), &enabled_options);
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
        Some(Scope::All),
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

    let disabled_options = Options {
        worktree_coverage: None,
        ..monitor.options().clone()
    };
    drop(monitor);
    let mut monitor = repo.monitor(disabled_options)?;
    assert_options(
        monitor.options(),
        &Options {
            worktree_coverage: None,
            ..enabled_options.clone()
        },
    );
    let out = monitor.service(Instant::now(), || {
        panic!("disabling coverage must not open the malformed index")
    });
    assert!(out.errors.is_empty(), "disabling coverage succeeds: {:?}", out.errors);
    assert_worktree_only(&out.changes);
    assert_eq!(out.changes.worktree, None, "disabled coverage publishes no scope");
    assert_eq!(out.statistics.added, 0, "disabled coverage installs no subscriptions");
    assert_eq!(
        monitor.next_timeout(Instant::now()),
        None,
        "a fully disabled replacement schedules no service"
    );

    let options = Options {
        worktree_coverage: Some(WorktreeCoverage::All),
        ..monitor.options().clone()
    };
    drop(monitor);
    let mut monitor = repo.monitor(options)?;
    assert_options(monitor.options(), &enabled_options);
    let out = monitor.service(Instant::now(), || {
        panic!("restoring unfiltered coverage must not open the malformed index")
    });
    assert!(out.errors.is_empty(), "restoring coverage succeeds: {:?}", out.errors);
    assert_worktree_only(&out.changes);
    assert_eq!(
        out.changes.worktree,
        Some(Scope::All),
        "restoring coverage requests a fresh baseline"
    );
    assert_eq!(
        out.statistics.added, 1,
        "restoring coverage reinstalls the recursive root"
    );

    drop(monitor);
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
        Some(Scope::All),
        "failed coverage requires worktree reconciliation"
    );
    assert_worktree_only(&out.changes);
    Ok(())
}

#[cfg(unix)]
#[test]
fn all_coverage_services_an_index_symlink_loop_without_opening_a_repository() -> TestResult {
    let dir = gix_testtools::scripted_fixture_read_only_with_args("make_notify_repo.sh", ["index-symlink-loop"])?;
    if !gix_testtools::fixture_has_symlinks(&dir)? {
        return Ok(());
    }
    let repo = gix::open_opts(dir, crate::restricted())?;
    for coverage in [Some(WorktreeCoverage::All), None] {
        let options = Options {
            worktree_coverage: coverage,
            ..worktree_only(WorktreeCoverage::All)
        };
        let mut monitor = repo.monitor(options)?;
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
            if coverage.is_some() { Some(Scope::All) } else { None },
            "only active coverage requires a baseline despite the unreadable index path"
        );
        assert_eq!(
            out.statistics.added,
            usize::from(coverage.is_some()),
            "only active coverage installs its recursive root"
        );
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
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod native {
    use std::{path::Path, sync::mpsc};

    use gix::notify::plumbing::{Budget, Event, EventKind, Fence, Loss, PathKind, SynchronizeError, Watch, Watcher};

    use super::*;

    const TIMEOUT: Duration = Duration::from_secs(5);

    fn writable_fixture(name: &str) -> TestResult<gix_testtools::tempfile::TempDir> {
        Ok(gix_testtools::scripted_fixture_writable_with_args(
            "make_notify_repo.sh",
            [name],
            gix_testtools::Creation::CopyFromReadOnly,
        )?)
    }

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

    fn wait_for_file_change(monitor: &mut RepositoryMonitor, repo: &gix::Repository, relative: &str) -> TestResult {
        let path = repo.workdir().expect("the fixture has a worktree").join(relative);
        let deadline = Instant::now() + TIMEOUT;
        let mut next_write = Instant::now();
        loop {
            // Coverage refresh can absorb a write into All; a later write must retain its path.
            if Instant::now() >= next_write {
                std::fs::write(&path, "changed after coverage expanded\n")?;
                next_write = Instant::now() + Duration::from_millis(100);
            }
            let out = service(monitor, repo);
            if matches!(&out.changes.worktree, Some(Scope::Paths(paths))
                if paths.iter().any(|change| change.path == relative))
            {
                return Ok(());
            }
            assert!(
                Instant::now() < deadline,
                "expected {relative} in the newly covered subtree; last changes: {:?}",
                out.changes
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn linked_worktree_inside_common_directory_publishes_classified_file_changes() -> TestResult {
        let fixture = writable_fixture("linked-in-common")?;
        let linked = fixture.path().join("bare.git/checkouts/root");
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
                if matches!(&out.changes.worktree, Some(Scope::Paths(paths)) if paths.iter().any(|change| change.path == "this"))
                {
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
    fn external_ignore_edits_refresh_directory_coverage() -> TestResult {
        let fixture = writable_fixture("coverage")?;
        let external = gix_testtools::tempfile::tempdir()?;
        let ignore = external.path().join("ignore");
        std::fs::write(&ignore, "ignored/\n")?;
        let root = fixture.path().join("unignored");
        let output = gix_testtools::git_command(&root)
            .args(["config", "core.excludesFile"])
            .arg(&ignore)
            .output()?;
        assert!(
            output.status.success(),
            "the fixture config selects a disposable external ignore file: {output:?}"
        );
        let repo = gix::open_opts(&root, crate::restricted())?;
        let mut monitor = repo.monitor(worktree_only(WorktreeCoverage::IgnoreAware))?;
        service(&mut monitor, &repo);
        service(&mut monitor, &repo);
        assert!(
            has_dependency(monitor.layout(), &ignore.canonicalize()?, DependencyKind::Ignore),
            "global ignores are monitored outside repository roots"
        );
        let replacement = external.path().join("replacement");
        std::fs::write(&replacement, "# Expose the ignored subtree.\n")?;
        std::fs::rename(&replacement, &ignore)?;
        let mut added = 0;
        wait_for(
            &mut monitor,
            &repo,
            "atomic replacement of the global ignore file",
            |out| {
                added += out.statistics.added;
                added == subtree_watch_count()
            },
        );
        wait_for_file_change(&mut monitor, &repo, "ignored/deep/file")?;
        Ok(())
    }

    #[test]
    fn staging_an_ordinary_file_only_invalidates_the_index() -> TestResult {
        let fixture = writable_fixture("ignore-refresh")?;
        let repo = gix::open_opts(fixture.path(), crate::restricted())?;
        let root = repo.workdir().expect("the fixture has a worktree");
        std::fs::write(root.join("new"), "prepared before monitoring\n")?;
        let mut monitor = repo.monitor(Options {
            metadata_debounce: Duration::ZERO,
            ..Options::default()
        })?;
        let mut quiet_since = Instant::now();
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let out = monitor.service(Instant::now(), || gix::open_opts(root, crate::restricted()));
            assert!(out.errors.is_empty(), "startup succeeds: {:?}", out.errors);
            if !out.changes.is_empty() || out.statistics.received != 0 {
                quiet_since = Instant::now();
            }
            if quiet_since.elapsed() >= Duration::from_millis(500) {
                break;
            }
            assert!(Instant::now() < deadline, "startup must settle");
            std::thread::sleep(Duration::from_millis(10));
        }
        gix_testtools::git(root, "add -- new")?;
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let out = monitor.service(Instant::now(), || gix::open_opts(root, crate::restricted()));
            assert!(out.errors.is_empty(), "index refresh succeeds: {:?}", out.errors);
            assert_eq!(
                out.changes.worktree, None,
                "staging alone does not invalidate worktree content"
            );
            assert_eq!(
                (out.statistics.added, out.statistics.removed),
                (0, 0),
                "ordinary staging leaves every native watch intact"
            );
            if out.changes.index {
                break;
            }
            assert!(Instant::now() < deadline, "the index change must be reported");
            std::thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    }

    #[test]
    fn ignore_and_index_events_refresh_coverage_without_publishing_metadata() -> TestResult {
        let fixture = writable_fixture("ignore-refresh")?;
        let repo = gix::open_opts(fixture.path(), crate::restricted())?;
        let root = repo.workdir().expect("the fixture has a worktree");
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
                added >= subtree_watch_count()
            },
        );
        assert_eq!(
            added,
            subtree_watch_count(),
            "unignoring adds subtree coverage without an explicit rescan"
        );

        gix_testtools::git(root, "add -f -- index-refresh/deep/file")?;
        let mut added = 0;
        wait_for(
            &mut monitor,
            &repo,
            "the index event to cover the newly tracked ignored path",
            |out| {
                added += out.statistics.added;
                added >= subtree_watch_count()
            },
        );
        assert_eq!(
            added,
            subtree_watch_count(),
            "the refreshed index adds subtree coverage despite the ignore rule"
        );

        for path in ["ignore-refresh/deep/file", "index-refresh/deep/file"] {
            wait_for_file_change(&mut monitor, &repo, path)?;
        }
        Ok(())
    }

    #[test]
    fn file_events_are_unfiltered_in_both_coverage_modes() -> TestResult {
        let interesting = [
            "this",
            "keep.ignored",
            "tracked.ignored",
            "missing.ignored",
            "only-dir",
            "sub/keep.blocked",
        ];
        let ignored = ["drop.ignored", "sub/drop.blocked"];
        for coverage in [WorktreeCoverage::IgnoreAware, WorktreeCoverage::All] {
            let fixture = writable_fixture("event-filter")?;
            let repo = gix::open_opts(fixture.path(), crate::restricted())?;
            let root = repo.workdir().expect("the fixture has a worktree");
            let mut monitor = repo.monitor(worktree_only(coverage))?;
            let mut quiet_since = Instant::now();
            wait_for(&mut monitor, &repo, "initial coverage to settle", |out| {
                if !out.changes.is_empty() || out.statistics.received != 0 || out.statistics.added != 0 {
                    quiet_since = Instant::now();
                }
                quiet_since.elapsed() >= Duration::from_millis(500)
            });

            for path in ignored.iter().chain(&interesting) {
                std::fs::write(root.join(path), "changed after startup\n")?;
            }
            let mut seen = std::collections::BTreeSet::<gix::bstr::BString>::new();
            wait_for(&mut monitor, &repo, "unfiltered file events", |out| {
                if let Some(scope) = &out.changes.worktree {
                    let Scope::Paths(paths) = scope else {
                        panic!("settled file writes must retain incremental scopes: {coverage:?}");
                    };
                    seen.extend(paths.iter().map(|change| change.path.clone()));
                }
                let contains = |path: &str| seen.contains(gix::bstr::BStr::new(path));
                interesting.iter().all(|path| contains(path)) && ignored.iter().all(|path| contains(path))
            });
            for path in interesting.iter().chain(&ignored) {
                assert!(
                    seen.contains(gix::bstr::BStr::new(path)),
                    "{coverage:?}: callers receive {path} and decide whether to exclude individual files"
                );
            }

            let removed = ["tracked.ignored", "missing.ignored"];
            for path in removed {
                std::fs::remove_file(root.join(path))?;
            }
            let mut seen_removed = [false; 2];
            wait_for(&mut monitor, &repo, "tracked ignored file deletions", |out| {
                if let Some(Scope::Paths(paths)) = &out.changes.worktree {
                    for (path, seen) in removed.iter().zip(&mut seen_removed) {
                        *seen |= paths.iter().any(|change| {
                            change.path == *path && matches!(change.kind, EventKind::Remove | EventKind::Any)
                        });
                    }
                }
                seen_removed.iter().all(|seen| *seen)
            });
            if coverage == WorktreeCoverage::IgnoreAware {
                std::fs::write(root.join(".gitignore"), "*.ignored\n!drop.ignored\n")?;
                wait_for(&mut monitor, &repo, "changed file-only ignore rules", |out| {
                    out.changes.worktree == Some(Scope::All)
                });
                std::fs::write(root.join("drop.ignored"), "newly visible\n")?;
                wait_for(&mut monitor, &repo, "the newly unignored file", |out| {
                    matches!(&out.changes.worktree, Some(Scope::Paths(paths))
                        if paths.iter().any(|change| change.path == "drop.ignored"))
                });
            }
        }
        Ok(())
    }

    #[test]
    fn index_backed_ignore_blob_changes_refresh_coverage() -> TestResult {
        let fixture = writable_fixture("coverage")?;
        let repo = gix::open_opts(fixture.path().join("ignored"), crate::restricted())?;
        let root = repo.workdir().expect("the fixture has a worktree");
        gix_testtools::git(root, "add -- .gitignore")?;
        gix_testtools::git(root, "update-index --skip-worktree -- .gitignore")?;
        std::fs::remove_file(root.join(".gitignore"))?;

        let mut index = repo.open_index()?;
        let ignore_entry = index
            .entry_index_by_path(".gitignore".into())
            .expect("the missing ignore file remains tracked");
        assert!(
            index.entries()[ignore_entry]
                .flags
                .contains(gix::index::entry::Flags::SKIP_WORKTREE),
            "the ignore rules come from the index when the worktree file is missing"
        );
        index.entries_mut()[ignore_entry].id = repo.write_blob(b"")?.detach();
        index.remove_tree();
        let mut updated_index = Vec::new();
        index.write_to(&mut updated_index, Default::default())?;

        let mut monitor = repo.monitor(worktree_only(WorktreeCoverage::IgnoreAware))?;
        let mut quiet_since = Instant::now();
        wait_for(&mut monitor, &repo, "initial coverage to settle", |out| {
            if !out.changes.is_empty() || out.statistics.received != 0 || out.statistics.added != 0 {
                quiet_since = Instant::now();
            }
            quiet_since.elapsed() >= Duration::from_millis(500)
        });

        std::fs::write(repo.git_dir().join("index"), updated_index)?;
        let mut added = 0;
        wait_for(
            &mut monitor,
            &repo,
            "the index-backed ignore blob change to expand coverage",
            |out| {
                added += out.statistics.added;
                added >= subtree_watch_count()
            },
        );
        assert_eq!(
            added,
            subtree_watch_count(),
            "changing only the ignore blob adds subtree coverage without an explicit rescan"
        );
        Ok(())
    }

    fn wait_for_pattern_change(
        monitor: &mut RepositoryMonitor,
        repo: &gix::Repository,
        metadata: bool,
        description: &str,
        mut done: impl FnMut(&Layout, &Outcome) -> bool,
    ) {
        let deadline = Instant::now() + TIMEOUT;
        let mut observed = false;
        loop {
            let out = monitor.service(Instant::now(), || gix::open_opts(repo.git_dir(), pattern_options()));
            assert!(
                out.errors.is_empty(),
                "pattern dependency maintenance succeeds: {:?}",
                out.errors
            );
            if !metadata {
                assert_worktree_only(&out.changes);
            }
            observed |= done(monitor.layout(), &out);
            // Finish queued delivery and watch verification before the next mutation,
            // without imposing a quiet interval after every positively observed change.
            if observed && monitor.next_timeout(Instant::now()) != Some(Duration::ZERO) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {description}; observed={observed}, changes={:?}, statistics={:?}, dependencies={:?}",
                out.changes,
                out.statistics,
                monitor.layout().dependencies
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn pattern_dependencies_detect_creation_and_refresh_ignore_aware_coverage() -> TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        for metadata in [false, true] {
            let fixture = writable_fixture("pattern-dependencies")?;
            let root = fixture.path().canonicalize()?;
            let _cwd = gix_testtools::set_current_dir(&root)?;
            let _env = gix_testtools::Env::new().set(
                "XDG_CONFIG_HOME",
                root.join("xdg").to_str().expect("UTF-8 fixture path"),
            );
            let repo = gix::open_opts(&root, pattern_options())?;
            let mut monitor = repo.monitor(Options {
                metadata,
                metadata_debounce: Duration::ZERO,
                ..Options::default()
            })?;
            let ignore = root.join(".git/patterns/nested/ignore");
            let attributes = root.join(".git/patterns/nested/attributes");
            let fallback_ignore = root.join("xdg/git/ignore");
            let fallback_attributes = root.join("xdg/git/attributes");
            assert!(
                !root.join(".git/patterns").exists()
                    && has_dependency(monitor.layout(), &fallback_ignore, DependencyKind::Ignore),
                "the fallback is initially selected while even the candidate's ancestor is absent"
            );
            let mut registered = false;
            let mut quiet_since = Instant::now();
            wait_for_pattern_change(&mut monitor, &repo, metadata, "initial registration", |_, out| {
                registered |= out.statistics.added != 0;
                if !out.changes.is_empty() || out.statistics.received != 0 {
                    quiet_since = Instant::now();
                }
                registered && quiet_since.elapsed() >= Duration::from_millis(500)
            });

            // Force the intermediate watch which slower, separately delivered directory
            // creation events can install. It must not inflate the final coverage count.
            std::fs::create_dir(root.join(".git/patterns"))?;
            let (mut added, mut removed) = (0, 0);
            wait_for_pattern_change(
                &mut monitor,
                &repo,
                metadata,
                "the intermediate candidate ancestor",
                |_, out| {
                    added += out.statistics.added;
                    removed += out.statistics.removed;
                    added == removed + 1
                },
            );
            std::fs::create_dir(root.join(".git/patterns/nested"))?;
            std::fs::write(&ignore, "# No ignored directories.\n")?;
            std::fs::write(&attributes, "* configured\n")?;
            wait_for_pattern_change(
                &mut monitor,
                &repo,
                metadata,
                "optional dependency selection and expanded coverage",
                |layout, out| {
                    added += out.statistics.added;
                    removed += out.statistics.removed;
                    added == removed + subtree_watch_count()
                        && has_dependency(layout, &ignore, DependencyKind::Ignore)
                        && !has_dependency(layout, &fallback_ignore, DependencyKind::Ignore)
                        && (!metadata
                            || (has_dependency(layout, &attributes, DependencyKind::Attributes)
                                && !has_dependency(layout, &fallback_attributes, DependencyKind::Attributes)))
                },
            );
            assert_eq!(
                added,
                removed + subtree_watch_count(),
                "replacing fallback and intermediate watches adds only ignored/ and ignored/deep/ to net coverage"
            );

            if metadata {
                std::fs::write(&attributes, "* updated\n")?;
                wait_for_pattern_change(
                    &mut monitor,
                    &repo,
                    metadata,
                    "classified configured attribute edit",
                    |_, out| out.changes.attributes,
                );
            }

            std::fs::write(&ignore, "ignored/\n")?;
            let (mut added, mut removed) = (0, 0);
            wait_for_pattern_change(
                &mut monitor,
                &repo,
                metadata,
                "ignoring an existing subtree",
                |_, out| {
                    added += out.statistics.added;
                    removed += out.statistics.removed;
                    removed == added + subtree_watch_count()
                },
            );
            assert_eq!(
                removed,
                added + subtree_watch_count(),
                "editing the selected ignore dependency removes the subtree subscriptions"
            );

            std::fs::write(&ignore, "# No ignored directories.\n")?;
            let (mut added, mut removed) = (0, 0);
            wait_for_pattern_change(
                &mut monitor,
                &repo,
                metadata,
                "unignoring an existing subtree",
                |_, out| {
                    added += out.statistics.added;
                    removed += out.statistics.removed;
                    added == removed + subtree_watch_count()
                },
            );
            assert_eq!(
                added,
                removed + subtree_watch_count(),
                "unignoring restores subtree coverage without changing the directory tree"
            );
            let file = root.join("ignored/deep/file");
            std::fs::write(&file, "covered after unignoring\n")?;
            let mut next_write = Instant::now() + Duration::from_millis(100);
            wait_for_pattern_change(
                &mut monitor,
                &repo,
                metadata,
                "a file event in the newly covered subtree",
                |_, out| {
                    if Instant::now() >= next_write {
                        std::fs::write(&file, "covered after unignoring\n")
                            .expect("the newly covered fixture file remains writable");
                        next_write = Instant::now() + Duration::from_millis(100);
                    }
                    matches!(&out.changes.worktree, Some(Scope::Paths(paths))
                        if paths.iter().any(|change| change.path == "ignored/deep/file"))
                },
            );

            std::fs::remove_file(&ignore)?;
            std::fs::remove_file(&attributes)?;
            let (mut added, mut removed) = (0, 0);
            wait_for_pattern_change(
                &mut monitor,
                &repo,
                metadata,
                "candidate removal restoring fallback dependencies and ignore rules",
                |layout, out| {
                    added += out.statistics.added;
                    removed += out.statistics.removed;
                    // Retain the candidate watch, add the fallback watch, and hide the subtree.
                    removed + 1 == added + subtree_watch_count()
                        && has_dependency(layout, &ignore, DependencyKind::Ignore)
                        && has_dependency(layout, &fallback_ignore, DependencyKind::Ignore)
                        && (!metadata
                            || (has_dependency(layout, &attributes, DependencyKind::Attributes)
                                && has_dependency(layout, &fallback_attributes, DependencyKind::Attributes)))
                },
            );
            assert_eq!(
                removed + 1,
                added + subtree_watch_count(),
                "fallback ignore rules hide the subtree again"
            );
            if metadata {
                std::fs::write(&fallback_attributes, "* resumed\n")?;
                wait_for_pattern_change(
                    &mut monitor,
                    &repo,
                    metadata,
                    "classified attribute edit after fallback resumption",
                    |_, out| out.changes.attributes,
                );
            }
        }
        Ok(())
    }

    fn drain_through(stream: &mut Watcher, fence: Fence) -> Vec<Event> {
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
            "the raw API exposes {}, preserving file or unknown native type hints: {events:?}",
            path.display()
        );
    }

    #[test]
    fn raw_watcher_fences_unfiltered_events_and_replaces_registration() -> TestResult {
        let fixture = writable_fixture("raw-stream")?;
        let repo = gix::open_opts(fixture.path().join("main"), crate::restricted())?;
        let root = repo.workdir().expect("the fixture has a worktree").canonicalize()?;
        let cookies = fixture.path().join("cookies").canonicalize()?;
        let replacement_path = fixture.path().join("replacement").canonicalize()?;
        let auxiliary = Watch {
            path: cookies.clone(),
            recursive: false,
        };
        let root_watch = Watch {
            path: root.clone(),
            recursive: true,
        };
        drop(repo);
        let mut stream = Watcher::new(Default::default())?;
        let batch = stream.drain(Budget::default());
        assert!(
            batch.events.is_empty() && batch.loss.is_none() && batch.error.is_none(),
            "construction and drain install no subscriptions: {batch:?}"
        );
        assert_eq!(
            stream.synchronize_at(&cookies, Instant::now() + TIMEOUT),
            Err(SynchronizeError::Unsupported),
            "a fence does not implicitly register the worktree or auxiliary paths"
        );
        stream.replace([root_watch.clone(), auxiliary.clone()])?;
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
        stream.replace([
            root_watch.clone(),
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

        stream.replace([
            root_watch.clone(),
            Watch {
                path: replacement_path.clone(),
                recursive: false,
            },
        ])?;
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

        stream.replace([root_watch])?;
        let removed = stream.drain(Budget::default());
        assert_eq!(
            removed.loss,
            Some(Loss::WatchSetChanged),
            "retaining only the root removes the last additional watch"
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

mod all_worktrees {
    use super::*;

    const FIXTURE: &str = "make_notify_worktrees.sh";

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn writable_fixture(ref_format: &str) -> TestResult<Option<gix_testtools::tempfile::TempDir>> {
        // Git introduced the reftable backend in 2.45; older versions still run the files cases.
        Ok(gix_testtools::scripted_fixture_writable_with_args_with_git_version(
            FIXTURE,
            [ref_format],
            gix_testtools::Creation::Execute,
            |version| ref_format == "files" || version >= (2, 45, 0),
        )?)
    }

    fn options(coverage: WorktreeCoverage) -> Options {
        Options {
            all_worktrees: true,
            worktree_coverage: Some(coverage),
            metadata_debounce: Duration::ZERO,
            retry_interval: Duration::from_millis(100),
            ..Options::default()
        }
    }

    fn service(monitor: &mut RepositoryMonitor, repo: &gix::Repository) -> Outcome {
        let out = monitor.service(Instant::now(), || gix::open_opts(repo.git_dir(), crate::restricted()));
        assert!(
            out.errors
                .iter()
                .all(|error| out.statistics.rescans != 0 && (error.is_conflict() || error.is_not_found())),
            "moved or removed roots may lose native coverage, but unrelated failures remain errors: {:?}",
            out.errors
        );
        out
    }

    #[test]
    fn manual_refresh_invalidates_every_checkout() -> TestResult {
        use gix::error::{ErrorExt, message};

        let fixture = gix_testtools::scripted_fixture_read_only(FIXTURE)?;
        let repo = gix::open_opts(fixture.join("main"), crate::restricted())?;
        let expected = Changes {
            references: true,
            index: true,
            configuration: true,
            ignores: true,
            attributes: true,
            operations: true,
            layout: true,
            worktree: Some(Scope::All),
        };
        for coverage in [WorktreeCoverage::All, WorktreeCoverage::IgnoreAware] {
            let mut monitor = repo.monitor(options(coverage))?;
            for _ in 0..2 {
                service(&mut monitor, &repo);
            }
            let layouts: Vec<_> = monitor.other_worktrees().cloned().collect();
            assert_eq!(layouts.len(), 2, "both additional checkouts are monitored");
            for fail in [false, true, true] {
                monitor.rescan();
                let out = monitor.service(Instant::now(), || {
                    if fail {
                        Err(message("deliberate all-worktree refresh failure").raise())
                    } else {
                        gix::open_opts(repo.git_dir(), crate::restricted())
                    }
                });
                assert_eq!(
                    out.errors.len(),
                    usize::from(fail),
                    "only failed rediscovery reports an error: {:?}",
                    out.errors
                );
                assert_eq!(
                    out.changes, expected,
                    "manual refresh invalidates the selected checkout"
                );
                assert_eq!(
                    out.other_worktrees.len(),
                    layouts.len(),
                    "every manual refresh invalidates all children, even after a previous maintenance failure"
                );
                for layout in &layouts {
                    let change = out
                        .other_worktrees
                        .iter()
                        .find(|change| change.git_dir == layout.git_dir)
                        .expect("each additional checkout receives a refresh invalidation");
                    assert_eq!(
                        Some(&change.worktree),
                        layout.worktree.as_ref(),
                        "refresh retains each checkout's identity"
                    );
                    assert_eq!(
                        change.kind,
                        WorktreeChange::Changed(expected.clone()),
                        "refresh invalidates all child caches without inventing lifecycle events"
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn startup_is_selected_only_by_default_and_all_worktrees_requires_metadata() -> TestResult {
        assert!(
            !Options::default().all_worktrees,
            "monitoring other checkouts is opt-in"
        );
        for ref_format in ["files", "reftable"] {
            startup_with_ref_format(ref_format)?;
        }
        Ok(())
    }

    fn startup_with_ref_format(ref_format: &str) -> TestResult {
        let Some(fixture) =
            gix_testtools::scripted_fixture_read_only_with_args_with_git_version(FIXTURE, [ref_format], |version| {
                ref_format == "files" || version >= (2, 45, 0)
            })?
        else {
            return Ok(());
        };
        for (selected, others) in [
            ("main", &["linked-a", "linked-b"][..]),
            ("linked-a", &["main", "linked-b"][..]),
            ("bare.git", &["bare-a", "bare-b"][..]),
        ] {
            let repo = gix::open_opts(fixture.join(selected), crate::restricted())?;
            let mut monitor = repo.monitor(Options {
                metadata_debounce: Duration::ZERO,
                ..Options::default()
            })?;
            let baseline = service(&mut monitor, &repo);
            assert!(
                baseline.other_worktrees.is_empty(),
                "default startup includes only {selected}"
            );
            assert_eq!(
                baseline.changes.worktree,
                if repo.is_bare() { None } else { Some(Scope::All) },
                "default startup requests a baseline only for the selected checkout"
            );

            let mut monitor = repo.monitor(options(WorktreeCoverage::IgnoreAware))?;
            let out = service(&mut monitor, &repo);
            assert_eq!(
                out.changes, baseline.changes,
                "the selected repository's baseline is unchanged"
            );
            assert_eq!(
                out.other_worktrees.len(),
                others.len(),
                "each other checkout gets one baseline"
            );
            for name in others {
                let other = gix::open_opts(fixture.join(name), crate::restricted())?;
                let git_dir = other.git_dir().canonicalize()?;
                let worktree = other
                    .workdir()
                    .expect("linked and main checkouts are non-bare")
                    .canonicalize()?;
                let change = out
                    .other_worktrees
                    .iter()
                    .find(|change| change.git_dir == git_dir)
                    .expect("the other checkout is identified by its private Git directory");
                assert_eq!(change.worktree, worktree, "the baseline identifies the checkout root");
                assert_eq!(
                    change.kind,
                    WorktreeChange::Added,
                    "initial discovery requests a full baseline, not an incremental invalidation"
                );
            }
            let error = repo
                .monitor(Options {
                    metadata: false,
                    ..options(WorktreeCoverage::All)
                })
                .err()
                .expect("all-worktree discovery requires metadata monitoring");
            assert!(
                error.is_validation(),
                "incompatible options produce a validation error: {error:?}"
            );
        }
        Ok(())
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    mod native {
        use super::*;

        const TIMEOUT: Duration = Duration::from_secs(5);

        fn wait_for(
            monitor: &mut RepositoryMonitor,
            repo: &gix::Repository,
            description: &str,
            mut done: impl FnMut(&Outcome) -> bool,
        ) -> Outcome {
            let deadline = Instant::now() + TIMEOUT;
            loop {
                let out = service(monitor, repo);
                if done(&out) {
                    return out;
                }
                assert!(
                    Instant::now() < deadline,
                    "timed out waiting for {description}; selected: {:?}, others: {:?}, statistics: {:?}",
                    out.changes,
                    out.other_worktrees,
                    out.statistics
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        }

        fn wait_for_quiet(
            monitor: &mut RepositoryMonitor,
            repo: &gix::Repository,
            description: &str,
            mut done: impl FnMut(&Outcome) -> bool,
        ) {
            let deadline = Instant::now() + TIMEOUT;
            let mut quiet_since = Instant::now();
            let mut observed = false;
            loop {
                let out = service(monitor, repo);
                observed |= done(&out);
                if !out.changes.is_empty()
                    || !out.other_worktrees.is_empty()
                    || out.statistics.received != 0
                    || out.statistics.rescans != 0
                    || out.statistics.added != 0
                    || out.statistics.removed != 0
                {
                    quiet_since = Instant::now();
                }
                if observed && quiet_since.elapsed() >= Duration::from_millis(500) {
                    return;
                }
                assert!(
                    Instant::now() < deadline,
                    "timed out waiting for {description} and quiet delivery (observed={observed}); selected: {:?}, others: {:?}, statistics: {:?}",
                    out.changes,
                    out.other_worktrees,
                    out.statistics
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        }

        fn changed(change: &gix::notify::WorktreeChanges) -> &Changes {
            let WorktreeChange::Changed(changes) = &change.kind else {
                panic!("ordinary invalidations must not publish a lifecycle transition: {change:?}");
            };
            changes
        }

        fn file_change(
            monitor: &mut RepositoryMonitor,
            repo: &gix::Repository,
            other: &gix::Repository,
            relative: &str,
        ) -> TestResult {
            let git_dir = other.git_dir().canonicalize()?;
            let worktree = other
                .workdir()
                .expect("the observed repository has a checkout")
                .canonicalize()?;
            wait_for_quiet(monitor, repo, "previous startup and metadata delivery", |_| true);
            let deadline = Instant::now() + TIMEOUT;
            let mut next_write = Instant::now();
            let mut writes = 0;
            loop {
                // Startup or inventory replacement may initially broaden a file event to All.
                // Repeat the write until native delivery proves the literal checkout-relative scope.
                if Instant::now() >= next_write {
                    writes += 1;
                    std::fs::write(
                        worktree.join(relative),
                        format!("{}: write {writes}\n", repo.git_dir().display()),
                    )?;
                    next_write = Instant::now() + Duration::from_millis(100);
                }
                let out = service(monitor, repo);
                if out.other_worktrees.iter().any(|change| change.git_dir == git_dir
                    && change.worktree == worktree
                    && matches!(&changed(change).worktree, Some(Scope::Paths(paths)) if paths.iter().any(|change| change.path == relative))) {
                    assert_eq!(out.changes.worktree, None, "another checkout's file does not invalidate the selected checkout");
                    return Ok(());
                }
                assert!(
                    Instant::now() < deadline,
                    "expected {relative} relative to {} after {writes} writes; others: {:?}",
                    worktree.display(),
                    out.other_worktrees
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        }

        #[test]
        fn reftable_metadata_only_shared_references_notify_through_native_events() -> TestResult {
            let Some(fixture) = writable_fixture("reftable")? else {
                return Ok(());
            };
            for selected in ["main", "linked-a", "bare.git"] {
                let root = fixture.path().join(selected);
                let repo = gix::open_opts(&root, crate::restricted())?;
                assert!(
                    repo.common_dir().join("reftable/tables.list").is_file(),
                    "the selected repository, including the bare clone, uses a real reftable stack"
                );
                let mut monitor = repo.monitor(Options {
                    worktree_coverage: None,
                    all_worktrees: false,
                    ..options(WorktreeCoverage::All)
                })?;
                wait_for_quiet(&mut monitor, &repo, "metadata-only startup", |_| true);

                // Resolve and mutate references through Git: gix can open reftable repositories,
                // but does not yet implement reading their HEAD or editing their reference store.
                gix_testtools::git(&root, &format!("update-ref refs/heads/notify-{selected} HEAD"))?;
                wait_for_quiet(&mut monitor, &repo, "a shared reftable reference change", |out| {
                    assert_eq!(out.changes.worktree, None, "metadata-only means no checkout scope");
                    assert!(
                        !out.changes.index,
                        "metadata-only does not invalidate an unwatched index"
                    );
                    assert!(out.other_worktrees.is_empty(), "other checkouts were not requested");
                    out.changes.references
                });
            }
            Ok(())
        }

        #[test]
        fn reftable_private_heads_and_shared_references_are_routed_through_native_events() -> TestResult {
            for selected in ["main", "linked-a"] {
                let Some(fixture) = writable_fixture("reftable")? else {
                    return Ok(());
                };
                let repo = gix::open_opts(fixture.path().join(selected), crate::restricted())?;
                let main_git_dir = repo.common_dir().canonicalize()?;
                let mut monitor = repo.monitor(options(WorktreeCoverage::IgnoreAware))?;
                wait_for_quiet(&mut monitor, &repo, "all-worktree startup", |_| true);

                for name in ["main", "linked-a", "linked-b"] {
                    let root = fixture.path().join(name);
                    let other = gix::open_opts(&root, crate::restricted())?;
                    let git_dir = other.git_dir().canonicalize()?;
                    assert!(
                        git_dir.join("reftable/tables.list").is_file(),
                        "each checkout has a native stack for its private HEAD"
                    );
                    gix_testtools::git(&root, "symbolic-ref HEAD refs/heads/notify-head")?;
                    let mut private = (false, false, false);
                    let mut shared = false;
                    wait_for_quiet(&mut monitor, &repo, &format!("{name}'s private reftable HEAD"), |out| {
                        shared |= out.changes.references;
                        if name != selected {
                            assert!(
                                !out.changes.operations,
                                "another checkout's HEAD is not selected operation state"
                            );
                            // Foreign HEAD changes may also invalidate aggregate reference/layout
                            // views; those hints do not claim that selected private state changed.
                        } else {
                            private.0 |= out.changes.references;
                            private.1 |= out.changes.configuration;
                            private.2 |= out.changes.operations;
                        }
                        for change in &out.other_worktrees {
                            let changes = changed(change);
                            if change.git_dir == git_dir {
                                private.0 |= changes.references;
                                private.1 |= changes.configuration;
                                private.2 |= changes.operations;
                            } else {
                                assert!(
                                    !changes.operations,
                                    "private operation flags are not broadcast to sibling checkouts: {change:?}"
                                );
                            }
                        }
                        private == (true, true, true) && (name != "main" || shared)
                    });
                }

                // A linked writer still stores shared refs in COMMON/reftable, which also holds
                // the main checkout's private refs. Only the main may gain operation flags.
                gix_testtools::git(
                    fixture.path().join("linked-b"),
                    "update-ref refs/heads/notify-shared HEAD",
                )?;
                let mut shared = false;
                let mut main_private = (false, false, false);
                wait_for_quiet(&mut monitor, &repo, "shared refs in the main reftable stack", |out| {
                    shared |= out.changes.references;
                    if selected == "main" {
                        main_private.0 |= out.changes.references;
                        main_private.1 |= out.changes.configuration;
                        main_private.2 |= out.changes.operations;
                    } else {
                        assert!(
                            !out.changes.operations,
                            "a shared ref is not linked private operation state"
                        );
                    }
                    for change in &out.other_worktrees {
                        let changes = changed(change);
                        if change.git_dir == main_git_dir {
                            main_private.0 |= changes.references;
                            main_private.1 |= changes.configuration;
                            main_private.2 |= changes.operations;
                        } else {
                            assert!(
                                !changes.operations,
                                "shared reftable events do not broadcast private operation flags: {change:?}"
                            );
                        }
                    }
                    shared && main_private == (true, true, true)
                });
            }
            Ok(())
        }

        #[test]
        fn files_and_private_metadata_are_routed_from_main_and_linked_checkouts() -> TestResult {
            for ref_format in ["files", "reftable"] {
                files_and_private_metadata_with_ref_format(ref_format)?;
            }
            Ok(())
        }

        fn files_and_private_metadata_with_ref_format(ref_format: &str) -> TestResult {
            let Some(fixture) = writable_fixture(ref_format)? else {
                return Ok(());
            };
            for selected in ["main", "linked-a"] {
                let repo = gix::open_opts(fixture.path().join(selected), crate::restricted())?;
                let mut monitor = repo.monitor(options(WorktreeCoverage::IgnoreAware))?;
                service(&mut monitor, &repo);
                service(&mut monitor, &repo);
                for name in ["main", "linked-a", "linked-b"]
                    .into_iter()
                    .filter(|name| *name != selected)
                {
                    let other = gix::open_opts(fixture.path().join(name), crate::restricted())?;
                    let git_dir = other.git_dir().canonicalize()?;
                    file_change(&mut monitor, &repo, &other, "tracked/file")?;

                    let output = gix_testtools::git_command(other.workdir().expect("fixture checkout"))
                        .args(["add", "--", "tracked/file"])
                        .output()?;
                    assert!(
                        output.status.success(),
                        "Git updates only {name}'s private index: {output:?}"
                    );
                    let out = wait_for(&mut monitor, &repo, "the other checkout's index", |out| {
                        out.other_worktrees
                            .iter()
                            .any(|change| change.git_dir == git_dir && changed(change).index)
                    });
                    assert!(!out.changes.index, "the selected index is not another checkout's index");
                    assert!(
                        out.other_worktrees
                            .iter()
                            .filter(|change| change.git_dir != git_dir)
                            .all(|change| !changed(change).index),
                        "index invalidation is not broadcast to sibling checkouts"
                    );

                    std::fs::write(git_dir.join("MERGE_MSG"), "operation state in one checkout\n")?;
                    let out = wait_for(&mut monitor, &repo, "the other checkout's operation state", |out| {
                        out.other_worktrees
                            .iter()
                            .any(|change| change.git_dir == git_dir && changed(change).operations)
                    });
                    assert!(
                        !out.changes.operations,
                        "private operation state does not belong to the selected checkout"
                    );
                    assert!(
                        out.other_worktrees
                            .iter()
                            .filter(|change| change.git_dir != git_dir)
                            .all(|change| !changed(change).operations),
                        "operation invalidation is not broadcast to sibling checkouts"
                    );
                }
            }
            Ok(())
        }

        #[test]
        fn added_moved_and_removed_worktrees_reconcile_through_native_events() -> TestResult {
            for ref_format in ["files", "reftable"] {
                added_moved_and_removed_with_ref_format(ref_format)?;
            }
            Ok(())
        }

        fn added_moved_and_removed_with_ref_format(ref_format: &str) -> TestResult {
            let Some(fixture) = writable_fixture(ref_format)? else {
                return Ok(());
            };
            let main = fixture.path().join("main");
            let repo = gix::open_opts(&main, crate::restricted())?;
            let mut monitor = repo.monitor(options(WorktreeCoverage::All))?;
            service(&mut monitor, &repo);
            service(&mut monitor, &repo);

            let added = fixture.path().join("added");
            let output = gix_testtools::git_command(&main)
                .args(["worktree", "add", "--detach"])
                .arg(&added)
                .arg("HEAD")
                .output()?;
            assert!(
                output.status.success(),
                "Git registers a new sibling checkout: {output:?}"
            );
            let other = gix::open_opts(&added, crate::restricted())?;
            let git_dir = other.git_dir().canonicalize()?;
            let added = added.canonicalize()?;
            wait_for(&mut monitor, &repo, "a new checkout baseline", |out| {
                out.other_worktrees.iter().any(|change| {
                    change.git_dir == git_dir && change.worktree == added && change.kind == WorktreeChange::Added
                })
            });
            file_change(&mut monitor, &repo, &other, "tracked/file")?;

            let moved = fixture.path().join("moved");
            let output = gix_testtools::git_command(&main)
                .args(["worktree", "move"])
                .arg(&added)
                .arg(&moved)
                .output()?;
            assert!(
                output.status.success(),
                "Git moves the checkout without changing its private Git directory: {output:?}"
            );
            let moved = moved.canonicalize()?;
            wait_for(&mut monitor, &repo, "the moved checkout baseline", |out| {
                let mut observed = false;
                for change in out.other_worktrees.iter().filter(|change| change.git_dir == git_dir) {
                    match &change.kind {
                        WorktreeChange::Moved { previous_worktree } => {
                            assert_eq!(
                                previous_worktree, &added,
                                "movement explicitly retires the previous checkout path"
                            );
                            assert_eq!(
                                change.worktree, moved,
                                "movement requires a full baseline at the new checkout path"
                            );
                            observed = true;
                        }
                        WorktreeChange::Changed(_) => {}
                        _ => panic!("moving one registration must not appear as removal or new discovery: {change:?}"),
                    }
                }
                observed
            });
            let other = gix::open_opts(&moved, crate::restricted())?;
            file_change(&mut monitor, &repo, &other, "tracked/file")?;

            let output = gix_testtools::git_command(&main)
                .args(["worktree", "remove", "--force"])
                .arg(&moved)
                .output()?;
            assert!(
                output.status.success(),
                "Git removes the deliberately modified disposable checkout: {output:?}"
            );
            let mut removed = 0;
            let mut tombstone = false;
            wait_for(
                &mut monitor,
                &repo,
                "the removed checkout and its registrations",
                |out| {
                    removed += out.statistics.removed;
                    tombstone |= out.other_worktrees.iter().any(|change| {
                        change.git_dir == git_dir && change.worktree == moved && change.kind == WorktreeChange::Removed
                    });
                    tombstone && removed > 0
                },
            );
            assert!(
                !moved.exists() && !git_dir.exists(),
                "both checkout and private metadata were removed"
            );
            Ok(())
        }

        #[test]
        fn bare_main_covers_other_checkouts_with_all_and_ignore_aware_coverage() -> TestResult {
            for ref_format in ["files", "reftable"] {
                bare_main_with_ref_format(ref_format)?;
            }
            Ok(())
        }

        fn bare_main_with_ref_format(ref_format: &str) -> TestResult {
            let Some(fixture) = writable_fixture(ref_format)? else {
                return Ok(());
            };
            let repo = gix::open_opts(fixture.path().join("bare.git"), crate::restricted())?;
            let other = gix::open_opts(fixture.path().join("bare-a"), crate::restricted())?;
            let git_dir = other.git_dir().canonicalize()?;
            let worktree = other.workdir().expect("the bare main has a linked checkout");
            for coverage in [WorktreeCoverage::All, WorktreeCoverage::IgnoreAware] {
                let mut monitor = repo.monitor(options(coverage))?;
                let baseline = service(&mut monitor, &repo);
                assert_eq!(
                    baseline.changes.worktree, None,
                    "the bare main has no checkout to invalidate"
                );
                assert_eq!(
                    baseline.other_worktrees.len(),
                    2,
                    "both linked checkouts receive coverage"
                );
                service(&mut monitor, &repo);
                if coverage == WorktreeCoverage::IgnoreAware {
                    std::fs::write(worktree.join(".gitignore"), "# Expose the existing ignored subtree.\n")?;
                    let mut added = 0;
                    let mut ignores = false;
                    wait_for(
                        &mut monitor,
                        &repo,
                        "the other checkout's expanded ignore-aware inventory",
                        |out| {
                            added += out.statistics.added;
                            ignores |= out
                                .other_worktrees
                                .iter()
                                .any(|change| change.git_dir == git_dir && changed(change).ignores);
                            ignores && added >= subtree_watch_count()
                        },
                    );
                    assert_eq!(
                        added,
                        subtree_watch_count(),
                        "only this checkout gains subtree coverage for ignored/"
                    );
                }
                // All observes this still-ignored path; IgnoreAware observes it after inventory growth.
                file_change(&mut monitor, &repo, &other, "ignored/deep/file")?;
            }
            Ok(())
        }
    }
}
