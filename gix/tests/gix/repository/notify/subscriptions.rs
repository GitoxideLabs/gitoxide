use super::*;

fn settle(monitor: &mut RepositoryMonitor, repo: &gix::Repository) {
    for _ in 0..10 {
        let outcome = monitor.service(Instant::now(), || gix::open_opts(repo.git_dir(), crate::restricted()));
        assert!(
            outcome.errors.is_empty(),
            "subscription maintenance succeeds: {:?}",
            outcome.errors
        );
        if outcome.coverage == CoverageStatus::Healthy {
            return;
        }
    }
    panic!("unchanged subscriptions must finish verification");
}

#[test]
fn subscription_resolution_is_atomic_and_keeps_the_original_relative_base() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_read_only("make_notify_worktrees.sh")?;
    let root = fixture.join("main");
    let _cwd = gix_testtools::set_current_dir(&root)?;
    let repo = gix::open_opts(&root, crate::restricted())?;
    let mut monitor = repo.monitor(Options::default())?;
    let filter = Some(vec!["./.git/worktrees/linked-a".into()]);
    let checkout_filter = Some(vec!["./.git".into()]);
    let paths = vec![InvalidationPath {
        path: "./tracked/../application/state".into(),
        changes: Changes {
            operations: true,
            ..Changes::default()
        },
    }];
    let expected = repo.monitor(Options {
        worktree_filter: filter.clone(),
        checkout_filter: checkout_filter.clone(),
        additional_paths: paths.clone(),
        ..Options::default()
    })?;
    let _elsewhere = gix_testtools::set_current_dir(fixture.join("linked-b"))?;
    monitor.update_subscriptions(filter, paths)?;
    monitor.update_checkout_filter(checkout_filter)?;
    assert_options(monitor.options(), expected.options());
    let before = monitor.options().clone();
    let error = monitor
        .update_subscriptions(
            None,
            vec![
                InvalidationPath {
                    path: root.join("valid"),
                    changes: Changes::default(),
                },
                InvalidationPath {
                    path: std::path::PathBuf::new(),
                    changes: Changes::default(),
                },
            ],
        )
        .expect_err("an empty path must reject the whole update");
    assert!(
        error.is_validation(),
        "invalid subscription paths produce validation errors: {error:?}"
    );
    assert_options(monitor.options(), &before);

    let additional_paths = monitor.options().additional_paths.clone();
    monitor.update_subscriptions(None, additional_paths)?;
    assert!(
        monitor.options().worktree_filter.is_none(),
        "the accessor reflects the new filter"
    );
    assert_eq!(
        monitor.options().additional_paths,
        before.additional_paths,
        "reusing resolved paths from the accessor preserves the dependencies"
    );
    Ok(())
}

#[test]
fn failed_updates_keep_the_new_selection_and_can_be_corrected_in_place() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_notify_worktrees.sh")?;
    let repo = gix::open_opts(fixture.join("main"), crate::restricted())?;
    let mut monitor = repo.monitor(Options {
        all_worktrees: true,
        ..Options::default()
    })?;
    settle(&mut monitor, &repo);
    monitor.update_subscriptions(
        Some(vec![]),
        vec![InvalidationPath {
            path: fixture.join("main/tracked/file/child"),
            changes: Changes {
                operations: true,
                ..Changes::default()
            },
        }],
    )?;
    let now = Instant::now();
    let failed = monitor.service(now, || gix::open_opts(repo.git_dir(), crate::restricted()));
    assert!(
        !failed.errors.is_empty(),
        "a file cannot contain the requested dependency"
    );
    assert_eq!(
        failed.coverage,
        CoverageStatus::Recovering {
            retry_at: Some(now + monitor.options().retry_interval),
        },
        "failed registration is retried"
    );
    assert!(failed.changes.operations, "failure retains dependency invalidations");
    assert_eq!(
        monitor.other_worktrees().count(),
        0,
        "deselection does not depend on successful registration"
    );
    assert_eq!(
        failed
            .other_worktrees
            .iter()
            .filter(|change| change.kind == WorktreeChange::Removed)
            .count(),
        2,
        "both previous checkouts are retired even on failure"
    );
    let quiet = monitor.service(now, || panic!("failed updates retain retry backoff"));
    assert!(quiet.errors.is_empty(), "quiet backoff does not repeat the error");
    assert_eq!(
        quiet.coverage, failed.coverage,
        "quiet backoff preserves recovery status"
    );
    let worktree_filter = monitor.options().worktree_filter.clone();
    monitor.update_subscriptions(
        worktree_filter,
        vec![InvalidationPath {
            path: fixture.join("main/application-state"),
            changes: Changes {
                operations: true,
                ..Changes::default()
            },
        }],
    )?;
    settle(&mut monitor, &repo);

    assert_eq!(
        monitor.other_worktrees().count(),
        0,
        "recovery uses the updated selection"
    );
    Ok(())
}

#[test]
fn a_bare_monitor_can_activate_application_dependencies_without_an_opener() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_notify_worktrees.sh")?;
    let repo = gix::open_opts(fixture.join("bare.git"), crate::restricted())?;
    let mut monitor = repo.monitor(Options {
        metadata: false,
        ..Options::default()
    })?;
    monitor.update_subscriptions(
        None,
        vec![InvalidationPath {
            path: fixture.join("application-state"),
            changes: Changes {
                worktree: Some(Scope::All),
                ..Changes::default()
            },
        }],
    )?;
    for _ in 0..2 {
        let outcome = monitor.service(Instant::now(), || {
            panic!("a bare repository has no ignore-aware checkout inventory")
        });
        assert!(
            outcome.errors.is_empty(),
            "application coverage starts without a checkout: {:?}",
            outcome.errors
        );
    }
    assert_eq!(
        monitor.coverage_status(),
        CoverageStatus::Healthy,
        "application coverage finishes verification on a bare repository"
    );
    Ok(())
}

#[test]
fn selection_updates_retire_and_add_checkouts_without_replacing_the_primary() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_notify_worktrees.sh")?;
    let main = gix::open_opts(fixture.join("main"), crate::restricted())?;
    let linked_a = gix::open_opts(fixture.join("linked-a"), crate::restricted())?;
    let linked_b = gix::open_opts(fixture.join("linked-b"), crate::restricted())?;
    selection_updates(&main, linked_a.git_dir(), linked_b.git_dir())?;
    selection_updates(&linked_b, linked_a.git_dir(), linked_b.common_dir())
}

fn selection_updates(repo: &gix::Repository, first: &std::path::Path, second: &std::path::Path) -> TestResult {
    let first = first.canonicalize()?;
    let second = second.canonicalize()?;
    for coverage in [WorktreeCoverage::All, WorktreeCoverage::IgnoreAware] {
        let mut monitor = repo.monitor(Options {
            all_worktrees: true,
            worktree_coverage: Some(coverage),
            ..Options::default()
        })?;
        settle(&mut monitor, repo);
        monitor.update_subscriptions(None, vec![])?;
        let unchanged = monitor.service(Instant::now(), || panic!("equal subscriptions need no repository"));
        assert!(
            unchanged.changes.is_empty() && unchanged.other_worktrees.is_empty(),
            "equal subscriptions preserve the baseline"
        );
        assert_eq!(
            unchanged.statistics.added + unchanged.statistics.removed,
            0,
            "equal subscriptions keep native watches"
        );

        monitor.update_subscriptions(Some(vec![first.clone()]), vec![])?;
        let removed = monitor.service(Instant::now(), || gix::open_opts(repo.git_dir(), crate::restricted()));
        assert!(
            removed.errors.is_empty(),
            "deselection is not an incomplete registration: {:?}",
            removed.errors
        );
        assert!(
            removed
                .other_worktrees
                .iter()
                .any(|change| change.git_dir == second && change.kind == WorktreeChange::Removed),
            "an existing but excluded checkout is retired"
        );
        assert!(
            !removed.other_worktrees.iter().any(|change| change.git_dir == first
                && matches!(change.kind, WorktreeChange::Added | WorktreeChange::Removed)),
            "the retained checkout keeps its stream"
        );
        assert_eq!(
            monitor.other_worktrees().count(),
            1,
            "only the selected checkout retains coverage"
        );
        assert!(removed.statistics.removed > 0, "retired watches are counted");
        settle(&mut monitor, repo);

        monitor.update_subscriptions(Some(vec![second.clone()]), vec![])?;
        let switched = monitor.service(Instant::now(), || gix::open_opts(repo.git_dir(), crate::restricted()));
        assert!(
            switched.errors.is_empty(),
            "selection switch succeeds: {:?}",
            switched.errors
        );
        assert!(
            switched
                .other_worktrees
                .iter()
                .any(|change| change.git_dir == first && change.kind == WorktreeChange::Removed),
            "the previous selection is retired"
        );
        assert!(
            switched
                .other_worktrees
                .iter()
                .any(|change| change.git_dir == second && change.kind == WorktreeChange::Added),
            "reenabling a checkout requests a fresh baseline"
        );
        settle(&mut monitor, repo);
        monitor.update_subscriptions(Some(vec![]), vec![])?;
        let empty = monitor.service(Instant::now(), || gix::open_opts(repo.git_dir(), crate::restricted()));
        assert!(empty.errors.is_empty(), "empty selection succeeds: {:?}", empty.errors);
        assert_eq!(
            monitor.other_worktrees().count(),
            0,
            "an empty selection excludes every additional checkout"
        );
        assert!(
            empty
                .other_worktrees
                .iter()
                .any(|change| change.git_dir == second && change.kind == WorktreeChange::Removed),
            "empty selection retires the final checkout"
        );
        settle(&mut monitor, repo);
    }
    Ok(())
}

#[test]
fn application_updates_preserve_invalidations_and_release_the_final_watch() -> TestResult {
    let repo = crate::basic_repo()?;
    let root = repo.workdir().expect("the fixture has a checkout");
    let old_changes = Changes {
        worktree: Some(Scope::Paths(vec![gix::notify::PathChange {
            path: "old".into(),
            kind: gix::notify::plumbing::EventKind::Modify,
            path_kind: gix::notify::plumbing::PathKind::File,
        }])),
        ..Changes::default()
    };
    let new_changes = Changes {
        worktree: Some(Scope::Paths(vec![gix::notify::PathChange {
            path: "new".into(),
            kind: gix::notify::plumbing::EventKind::Modify,
            path_kind: gix::notify::plumbing::PathKind::File,
        }])),
        ..Changes::default()
    };
    let mut monitor = repo.monitor(Options {
        metadata: false,
        worktree_coverage: None,
        additional_paths: vec![InvalidationPath {
            path: root.join("application-old"),
            changes: old_changes,
        }],
        ..Options::default()
    })?;
    let initial = monitor.service(Instant::now(), || panic!("application-only coverage does not reopen"));
    assert!(
        initial.errors.is_empty(),
        "application coverage starts: {:?}",
        initial.errors
    );
    assert_eq!(
        initial.statistics.added, 1,
        "application-only coverage watches the parent without the checkout stream"
    );
    settle(&mut monitor, &repo);
    monitor.update_subscriptions(
        None,
        vec![InvalidationPath {
            path: root.join("application-new"),
            changes: new_changes.clone(),
        }],
    )?;
    let replaced = monitor.service(Instant::now(), || panic!("application updates do not reopen"));
    assert!(
        replaced.errors.is_empty(),
        "application update succeeds: {:?}",
        replaced.errors
    );
    let Some(Scope::Paths(paths)) = replaced.changes.worktree else {
        panic!("both application invalidations remain literal");
    };
    assert_eq!(
        paths.iter().map(|path| path.path.as_slice()).collect::<Vec<_>>(),
        vec![b"new".as_slice(), b"old".as_slice()],
        "updates preserve queued old invalidations alongside the new baseline"
    );
    assert_eq!(
        replaced.statistics.added + replaced.statistics.removed,
        0,
        "changing files in the same parent preserves native registration"
    );
    settle(&mut monitor, &repo);
    monitor.update_subscriptions(None, vec![])?;
    let removed = monitor.service(Instant::now(), || panic!("inactive coverage does not reopen"));
    assert_eq!(
        removed.changes, new_changes,
        "removal publishes the previous application's invalidation"
    );
    assert_eq!(
        removed.statistics.removed, 1,
        "removing the final application path releases its native watcher"
    );
    assert_eq!(
        removed.coverage,
        CoverageStatus::Inactive,
        "the monitor can become inactive in place"
    );
    assert_eq!(
        monitor.next_timeout(Instant::now()),
        None,
        "inactive coverage has no maintenance deadline"
    );
    monitor.update_subscriptions(
        None,
        vec![InvalidationPath {
            path: root.join("application-new"),
            changes: Changes {
                worktree: Some(Scope::All),
                ..Changes::default()
            },
        }],
    )?;
    let restarted = monitor.service(Instant::now(), || panic!("reactivation does not reopen"));
    assert!(
        restarted.errors.is_empty(),
        "application coverage reactivates: {:?}",
        restarted.errors
    );
    assert_eq!(
        restarted.statistics.added, 1,
        "reactivation reinstalls only application coverage"
    );
    assert_eq!(
        restarted.changes.worktree,
        Some(Scope::All),
        "reactivation establishes a baseline"
    );
    Ok(())
}

#[test]
fn checkout_coverage_switches_without_retiring_worktree_metadata() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_notify_worktrees.sh")?;
    let repo = gix::open_opts(fixture.join("main"), crate::restricted())?;
    let linked_a = gix::open_opts(fixture.join("linked-a"), crate::restricted())?
        .git_dir()
        .canonicalize()?;
    let linked_b = gix::open_opts(fixture.join("linked-b"), crate::restricted())?
        .git_dir()
        .canonicalize()?;
    for coverage in [WorktreeCoverage::All, WorktreeCoverage::IgnoreAware] {
        let mut monitor = repo.monitor(Options {
            all_worktrees: true,
            checkout_filter: Some(vec![linked_a.clone()]),
            worktree_coverage: Some(coverage),
            ..Options::default()
        })?;
        settle(&mut monitor, &repo);
        assert_eq!(
            monitor.other_worktrees().count(),
            2,
            "both metadata views remain discovered"
        );
        monitor.update_checkout_filter(Some(vec![linked_b.clone()]))?;
        let switched = monitor.service(Instant::now(), || gix::open_opts(repo.git_dir(), crate::restricted()));
        assert!(
            switched.errors.is_empty(),
            "coverage switching succeeds: {:?}",
            switched.errors
        );
        assert_eq!(
            monitor.other_worktrees().count(),
            2,
            "coverage selection keeps membership"
        );
        assert!(
            switched
                .other_worktrees
                .iter()
                .all(|change| matches!(change.kind, WorktreeChange::Changed(_))),
            "coverage changes are not membership additions or removals"
        );
        assert!(
            switched.other_worktrees.iter().any(|change| change.git_dir == linked_b
                && matches!(&change.kind, WorktreeChange::Changed(changes) if changes.worktree == Some(Scope::All))),
            "new checkout coverage publishes a complete baseline"
        );
        settle(&mut monitor, &repo);
        let before = monitor.options().checkout_filter.clone();
        assert!(
            monitor
                .update_checkout_filter(Some(vec![std::path::PathBuf::new()]))
                .is_err(),
            "invalid checkout selection is rejected atomically"
        );
        assert_eq!(
            monitor.options().checkout_filter,
            before,
            "failed selection preserves options"
        );
        monitor.update_checkout_filter(Some(vec![]))?;
        settle(&mut monitor, &repo);
        assert_eq!(
            monitor.other_worktrees().count(),
            2,
            "no checkout coverage still retains metadata views"
        );
        monitor.update_checkout_filter(Some(vec![repo.git_dir().canonicalize()?]))?;
        let primary = monitor.service(Instant::now(), || gix::open_opts(repo.git_dir(), crate::restricted()));
        assert_eq!(
            primary.changes.worktree,
            Some(Scope::All),
            "the coordinator is covered only when selected"
        );
    }
    Ok(())
}

#[test]
fn metadata_only_all_worktrees_retains_private_views_without_checkout_inventory() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_notify_worktrees.sh")?;
    let repo = gix::open_opts(fixture.join("main"), crate::restricted())?;
    let mut monitor = repo.monitor(Options {
        all_worktrees: true,
        worktree_coverage: None,
        ..Options::default()
    })?;
    let initial = monitor.service(Instant::now(), || gix::open_opts(repo.git_dir(), crate::restricted()));
    assert!(
        initial.errors.is_empty(),
        "metadata-only discovery succeeds: {:?}",
        initial.errors
    );
    assert_eq!(
        initial.other_worktrees.len(),
        2,
        "metadata-only discovery still publishes membership baselines"
    );
    assert_eq!(
        monitor.other_worktrees().count(),
        2,
        "linked metadata layouts remain available"
    );
    assert!(
        initial.changes.worktree.is_none(),
        "metadata-only startup does not request a checkout scan"
    );
    settle(&mut monitor, &repo);
    Ok(())
}
