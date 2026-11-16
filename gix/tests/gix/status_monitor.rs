use std::{path::Path, sync::atomic::AtomicBool};

use gix::{
    bstr::BString,
    notify::{Changes, PathChange, Scope},
    status::{Item, Monitor, index_worktree::Item as WorktreeItem, monitor::Update},
};
use gix_error::TestResult;

fn fixture() -> TestResult<gix_testtools::tempfile::TempDir> {
    Ok(gix_testtools::scripted_fixture_writable(
        "make_status_monitor_index.sh",
    )?)
}

fn open(root: &Path) -> gix::Result<gix::Repository> {
    gix::open_opts(root, gix::open::Options::isolated())
}

fn monitor(root: &Path) -> TestResult<Monitor> {
    Ok(Monitor::new(&open(root)?, Default::default())?)
}

fn refresh(monitor: &mut Monitor, root: &Path) -> TestResult<Update> {
    Ok(monitor.refresh(&open(root)?, &AtomicBool::new(false))?)
}

fn git(root: &Path, args: &[&str]) -> TestResult {
    let output = gix_testtools::git_command(root).args(args).output()?;
    assert!(
        output.status.success(),
        "Git changes the disposable fixture with {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn invalidate_index(monitor: &mut Monitor) {
    monitor.invalidate_changes(&Changes {
        index: true,
        ..Default::default()
    });
}

fn scope(path: &str) -> Option<Scope> {
    Some(Scope::Paths(vec![PathChange {
        path: path.into(),
        kind: gix::notify::plumbing::EventKind::Any,
        path_kind: gix::notify::plumbing::PathKind::Any,
    }]))
}

fn assert_matches_fresh_status(monitor: &Monitor, root: &Path) -> TestResult {
    let mut fresh = open(root)?
        .status(gix::progress::Discard)?
        .index_worktree_options_mut(|options| {
            options.sorting = Some(gix::status::plumbing::index_as_worktree_with_renames::Sorting::ByPathCaseSensitive);
        })
        .into_vec(None::<BString>, &AtomicBool::new(false))?;
    let rank = |item: &Item| match item {
        Item::TreeIndex(_) => 0,
        Item::IndexWorktree(_) => 1,
    };
    fresh.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| a.location().cmp(b.location())));
    assert_eq!(
        monitor.snapshot(),
        Some(fresh.as_slice()),
        "the cached snapshot includes the current index positions and entry backing"
    );
    Ok(())
}

#[test]
fn bare_status_monitor_collects_staged_changes() -> TestResult {
    let (root, ()) = gix_testtools::scripted_fixture_read_only_with_post("make_status_monitor_index.sh", 1, |state| {
        if let gix_testtools::FixtureState::Uninitialized(path) = state {
            gix_testtools::git(path, "update-index --force-remove -- a")?;
            gix_testtools::git(path, "config core.bare true")?;
        }
        Ok(())
    })?;
    let repo = open(&root.join(".git"))?;
    assert!(repo.is_bare(), "the staged index belongs to a bare repository");
    let mut monitor = Monitor::new(&repo, Default::default())?;
    let update = monitor.refresh(&repo, &AtomicBool::new(false))?;
    assert!(
        matches!(monitor.snapshot(), Some([item @ Item::TreeIndex(_)]) if item.location() == "a"),
        "the staged deletion is visible without a worktree"
    );
    assert!(
        update.staged && update.unstaged.is_none(),
        "only the staged domain was refreshed"
    );
    Ok(())
}

#[test]
fn full_status_monitor_requires_metadata_notifications() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_status_monitor_index.sh")?;
    let repo = open(&root)?;
    for worktree_coverage in [
        None,
        Some(gix::notify::WorktreeCoverage::IgnoreAware),
        Some(gix::notify::WorktreeCoverage::All),
    ] {
        let error = Monitor::new(
            &repo,
            gix::status::monitor::Options {
                repository: gix::notify::Options {
                    metadata: false,
                    worktree_coverage,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .err()
        .expect("a full status snapshot needs index and reference notifications");
        assert!(
            error.to_string().contains("metadata"),
            "the error identifies the unsupported notification setting"
        );
    }
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn unicode_worktree_aliases_refresh_tracked_files_and_parent_directories() -> TestResult {
    for (indexed_directory, native_directory, precompose) in
        [("Café", "Cafe\u{301}", "true"), ("Cafe\u{301}", "Café", "false")]
    {
        let fixture = fixture()?;
        let root = fixture.path();
        let indexed_path = format!("{indexed_directory}/file");
        let native_path = format!("{native_directory}/file");
        git(root, &["config", "core.precomposeUnicode", precompose])?;
        std::fs::create_dir(root.join(indexed_directory))?;
        std::fs::write(root.join(&indexed_path), "initial\n")?;
        git(root, &["add", "--", &indexed_path])?;
        git(root, &["commit", "-qm", "add Unicode path"])?;
        // Rename before installing watches to make the disk spelling differ from the index.
        std::fs::rename(root.join(indexed_directory), root.join("temporary-directory"))?;
        std::fs::rename(root.join("temporary-directory"), root.join(native_directory))?;
        git(root, &["config", "core.precomposeUnicode", "false"])?;
        assert!(
            open(root)?
                .index()?
                .entry_by_path(indexed_path.as_str().into())
                .is_some(),
            "the fixture retains the requested index spelling"
        );

        let has_tracked_change = |monitor: &Monitor| {
            monitor
                .snapshot()
                .expect("a successful refresh provides a snapshot")
                .iter()
                .any(|item| {
                    matches!(
                        item,
                        Item::IndexWorktree(WorktreeItem::Modification { rela_path, .. })
                            if rela_path.as_slice() == indexed_path.as_bytes()
                    )
                })
        };
        let mut monitor = monitor(root)?;
        refresh(&mut monitor, root)?;
        assert!(!has_tracked_change(&monitor), "the indexed Unicode file starts clean");

        std::fs::write(root.join(&native_path), "changed\n")?;
        monitor.invalidate_changes(&Changes {
            worktree: scope(&native_path),
            ..Default::default()
        });
        let update = refresh(&mut monitor, root)?;
        assert!(
            has_tracked_change(&monitor),
            "the native file event refreshes the tracked row in its index spelling"
        );
        assert_eq!(
            update.unstaged,
            Some(Scope::All),
            "Unicode aliases require full coverage"
        );
        assert!(!update.staged, "native file edits do not change staged status");
        assert_matches_fresh_status(&monitor, root)?;

        std::fs::write(root.join(&native_path), "initial\n")?;
        monitor.invalidate_changes(&Changes {
            worktree: scope(native_directory),
            ..Default::default()
        });
        let update = refresh(&mut monitor, root)?;
        assert!(
            !has_tracked_change(&monitor),
            "the native parent event removes the restored file's cached modification"
        );
        assert_eq!(update.unstaged, Some(Scope::All), "Unicode parent scopes cover aliases");
        assert_matches_fresh_status(&monitor, root)?;

        std::fs::write(root.join("tracked"), "changed\n")?;
        monitor.invalidate_changes(&Changes {
            worktree: scope("tracked"),
            ..Default::default()
        });
        assert_eq!(
            refresh(&mut monitor, root)?.unstaged,
            scope("tracked"),
            "ASCII invalidations retain incremental coverage beside Unicode paths"
        );
        assert_matches_fresh_status(&monitor, root)?;
    }
    Ok(())
}

#[test]
fn staging_one_path_keeps_unrelated_worktree_rows() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    std::fs::write(root.join("tracked"), "changed\n")?;
    std::fs::write(root.join("z"), "still changed\n")?;
    let mut monitor = monitor(root)?;
    refresh(&mut monitor, root)?;

    git(root, &["add", "tracked"])?;
    invalidate_index(&mut monitor);
    let update = refresh(&mut monitor, root)?;
    assert!(update.staged, "staging changes the comparison with HEAD");
    assert_eq!(
        update.unstaged,
        scope("tracked"),
        "only the staged path needs a worktree recheck"
    );
    assert_matches_fresh_status(&monitor, root)
}

#[test]
fn earlier_index_insertions_and_removals_rebind_retained_rows() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    std::fs::write(root.join("z"), "changed\n")?;
    let mut monitor = monitor(root)?;
    refresh(&mut monitor, root)?;

    std::fs::write(root.join("0"), "earlier\n")?;
    git(root, &["add", "0"])?;
    invalidate_index(&mut monitor);
    assert_eq!(
        refresh(&mut monitor, root)?.unstaged,
        scope("0"),
        "an earlier insertion does not recheck the retained dirty path"
    );
    assert_matches_fresh_status(&monitor, root)?;

    git(root, &["rm", "--quiet", "--force", "0"])?;
    invalidate_index(&mut monitor);
    assert_eq!(
        refresh(&mut monitor, root)?.unstaged,
        scope("0"),
        "an earlier removal also keeps the worktree scan local"
    );
    assert_matches_fresh_status(&monitor, root)
}

#[test]
fn trackedness_changes_replace_collapsed_untracked_directories() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    std::fs::create_dir(root.join("dir"))?;
    std::fs::write(root.join("dir/file"), "new\n")?;
    std::fs::write(root.join("dir/other"), "also new\n")?;
    let mut monitor = monitor(root)?;
    refresh(&mut monitor, root)?;
    assert_eq!(
        monitor.snapshot().expect("the baseline exists")[0].location(),
        "dir",
        "the untracked subtree initially collapses into one directory row"
    );

    git(root, &["add", "dir/file"])?;
    invalidate_index(&mut monitor);
    assert_eq!(
        refresh(&mut monitor, root)?.unstaged,
        scope("dir"),
        "staging a descendant refreshes the collapsed ancestor"
    );
    assert_matches_fresh_status(&monitor, root)?;

    git(root, &["rm", "--cached", "--quiet", "dir/file"])?;
    invalidate_index(&mut monitor);
    assert_eq!(
        refresh(&mut monitor, root)?.unstaged,
        scope("dir"),
        "removing the tracked descendant can collapse the ancestor again"
    );
    assert_eq!(
        monitor.snapshot().expect("the refreshed snapshot exists")[0].location(),
        "dir",
        "both retained worktree files collapse back into one untracked directory"
    );
    assert_matches_fresh_status(&monitor, root)
}

#[test]
fn status_flags_refresh_the_affected_path() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    std::fs::write(root.join("tracked"), "changed\n")?;
    let mut monitor = monitor(root)?;
    refresh(&mut monitor, root)?;

    for flag in [
        "--assume-unchanged",
        "--no-assume-unchanged",
        "--skip-worktree",
        "--no-skip-worktree",
    ] {
        git(root, &["update-index", flag, "tracked"])?;
        invalidate_index(&mut monitor);
        assert_eq!(
            refresh(&mut monitor, root)?.unstaged,
            scope("tracked"),
            "{flag} affects status without changing unrelated file contents"
        );
        assert_matches_fresh_status(&monitor, root)?;
    }
    Ok(())
}

#[test]
fn intent_to_add_transitions_refresh_the_affected_path() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    std::fs::write(root.join("intent"), "new\n")?;
    let mut monitor = monitor(root)?;
    refresh(&mut monitor, root)?;

    git(root, &["add", "--intent-to-add", "intent"])?;
    invalidate_index(&mut monitor);
    assert_eq!(
        refresh(&mut monitor, root)?.unstaged,
        scope("intent"),
        "intent-to-add replaces the untracked row"
    );
    assert!(
        monitor
            .snapshot()
            .expect("the baseline exists")
            .iter()
            .any(|item| matches!(
                item,
                Item::IndexWorktree(WorktreeItem::Modification {
                    status: gix::status::plumbing::index_as_worktree::EntryStatus::IntentToAdd,
                    ..
                })
            )),
        "the intent-to-add entry remains visible in worktree status"
    );
    assert_matches_fresh_status(&monitor, root)?;

    git(root, &["add", "intent"])?;
    invalidate_index(&mut monitor);
    assert_eq!(
        refresh(&mut monitor, root)?.unstaged,
        scope("intent"),
        "fully staging the path replaces its intent-to-add row"
    );
    assert_matches_fresh_status(&monitor, root)
}

#[test]
fn conflict_stage_changes_preserve_current_entries() -> TestResult {
    use gix::index::entry::{Flags, Mode, Stage};

    let fixture = fixture()?;
    let root = fixture.path();
    std::fs::write(root.join("z"), "changed\n")?;
    let mut monitor = monitor(root)?;
    refresh(&mut monitor, root)?;

    let repo = open(root)?;
    let mut index = repo.open_index()?;
    index.remove_entries(|_, path, _| path == "tracked");
    for (stage, content) in [
        (Stage::Base, "base\n"),
        (Stage::Ours, "ours\n"),
        (Stage::Theirs, "theirs\n"),
    ] {
        let blob_id = repo.write_blob(content)?;
        index.dangerously_push_entry(
            Default::default(),
            blob_id.detach(),
            Flags::from_stage(stage),
            Mode::FILE,
            "tracked".into(),
        );
    }
    index.sort_entries();
    index.write(Default::default(), repo.refs.shared_repository_permissions)?;
    invalidate_index(&mut monitor);
    assert_eq!(
        refresh(&mut monitor, root)?.unstaged,
        scope("tracked"),
        "conflict stages form one affected path"
    );
    assert_matches_fresh_status(&monitor, root)?;

    let mut index = repo.open_index()?;
    index
        .entry_mut_by_path_and_stage("tracked".into(), Stage::Ours)
        .expect("the fixture has an ours stage")
        .flags |= Flags::EXTENDED;
    index.write(Default::default(), repo.refs.shared_repository_permissions)?;
    invalidate_index(&mut monitor);
    let update = refresh(&mut monitor, root)?;
    assert!(
        !update.staged && update.unstaged.is_none(),
        "conflict storage flags do not require status scans"
    );
    assert!(update.changed, "embedded conflict metadata follows the rewritten index");
    assert_matches_fresh_status(&monitor, root)?;

    let mut index = repo.open_index()?;
    index.remove_entries(|_, path, entry| path == "tracked" && entry.stage() == Stage::Base);
    index.write(Default::default(), repo.refs.shared_repository_permissions)?;
    invalidate_index(&mut monitor);
    assert_eq!(
        refresh(&mut monitor, root)?.unstaged,
        scope("tracked"),
        "removing one stage changes the same conflict row"
    );
    assert_matches_fresh_status(&monitor, root)
}

#[test]
fn stat_only_index_changes_do_not_recompute_staged_status() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    std::fs::write(root.join("tracked"), "changed\n")?;
    let mut monitor = monitor(root)?;
    refresh(&mut monitor, root)?;

    let repo = open(root)?;
    let mut index = repo.open_index()?;
    index
        .entry_mut_by_path_and_stage("tracked".into(), gix::index::entry::Stage::Unconflicted)
        .expect("the fixture tracks this path")
        .stat
        .ctime
        .secs ^= 1;
    index.write(Default::default(), repo.refs.shared_repository_permissions)?;
    invalidate_index(&mut monitor);
    let update = refresh(&mut monitor, root)?;
    assert!(!update.staged, "stat caches do not change staged contents");
    assert_eq!(
        update.unstaged,
        scope("tracked"),
        "only the entry with changed stat data is rechecked"
    );
    assert_matches_fresh_status(&monitor, root)
}

#[test]
fn unchanged_index_invalidations_do_not_recompute_status() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_status_monitor_index.sh")?;
    let root = fixture.as_path();
    let mut monitor = monitor(root)?;
    refresh(&mut monitor, root)?;

    invalidate_index(&mut monitor);
    assert_eq!(
        refresh(&mut monitor, root)?,
        Update::default(),
        "an unchanged index leaves both status domains and the snapshot unchanged"
    );
    assert!(
        !monitor.is_dirty(),
        "the verified index invalidation is consumed without a status scan"
    );
    assert_matches_fresh_status(&monitor, root)
}

#[test]
fn cancellation_preserves_the_previous_index_baseline_for_retry() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    std::fs::write(root.join("tracked"), "changed\n")?;
    let mut monitor = monitor(root)?;
    refresh(&mut monitor, root)?;
    let previous = monitor.snapshot().expect("the baseline exists").to_vec();

    git(root, &["add", "tracked"])?;
    invalidate_index(&mut monitor);
    assert!(
        monitor.refresh(&open(root)?, &AtomicBool::new(true)).is_err(),
        "cancelled index refreshes cannot publish partial state"
    );
    assert_eq!(
        monitor.snapshot(),
        Some(previous.as_slice()),
        "cancellation retains the previous status snapshot"
    );
    assert!(monitor.is_dirty(), "the cancelled index change remains pending");
    assert_eq!(
        refresh(&mut monitor, root)?.unstaged,
        scope("tracked"),
        "retry still compares against the old index baseline"
    );
    assert_matches_fresh_status(&monitor, root)
}

#[test]
fn staged_rename_tracking_recomputes_cross_path_pairs() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    git(root, &["rm", "--quiet", "a"])?;
    let mut monitor = monitor(root)?;
    refresh(&mut monitor, root)?;

    std::fs::write(root.join("renamed"), "early\n")?;
    git(root, &["add", "renamed"])?;
    invalidate_index(&mut monitor);
    let update = refresh(&mut monitor, root)?;
    assert!(
        update.staged,
        "the new staged path can pair with an existing deletion elsewhere"
    );
    assert_eq!(
        update.unstaged,
        scope("renamed"),
        "staged rename tracking does not broaden worktree checking"
    );
    assert!(
        monitor
            .snapshot()
            .expect("the baseline exists")
            .iter()
            .any(|item| matches!(
                item,
                Item::TreeIndex(gix::diff::index::Change::Rewrite { source_location, location, .. })
                    if source_location.as_ref() == "a" && location.as_ref() == "renamed"
            )),
        "the previous deletion becomes a staged rename"
    );
    assert_matches_fresh_status(&monitor, root)
}

#[test]
fn staged_rename_tracking_can_override_repository_configuration() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    git(root, &["config", "status.renames", "false"])?;
    git(root, &["mv", "a", "renamed"])?;
    let repo = open(root)?;
    let mut monitor = Monitor::new(
        &repo,
        gix::status::monitor::Options {
            tree_index_track_renames: gix::status::tree_index::TrackRenames::Given(Default::default()),
            ..Default::default()
        },
    )?;
    monitor.refresh(&repo, &AtomicBool::new(false))?;
    assert!(
        monitor
            .snapshot()
            .expect("the configured snapshot exists")
            .iter()
            .any(|item| matches!(
                item,
                Item::TreeIndex(gix::diff::index::Change::Rewrite { source_location, location, .. })
                    if source_location.as_ref() == "a" && location.as_ref() == "renamed"
            )),
        "the explicit staged rename policy overrides status.renames=false"
    );
    Ok(())
}

#[test]
fn live_subscriptions_preserve_snapshots_pending_refresh_and_failed_updates() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    std::fs::write(root.join("tracked"), "modified\n")?;
    let mut monitor = monitor(root)?;
    refresh(&mut monitor, root)?;
    let previous = monitor.snapshot().expect("the successful snapshot exists").to_vec();
    invalidate_index(&mut monitor);
    assert!(
        monitor.refresh(&open(root)?, &AtomicBool::new(true)).is_err(),
        "interruption leaves pending status and a retry deadline"
    );
    let now = std::time::Instant::now();
    assert!(!monitor.refresh_due(now), "the interrupted refresh is in backoff");
    monitor.update_subscriptions(
        Some(vec![]),
        vec![gix::notify::InvalidationPath {
            path: root.join("application-state"),
            changes: Changes {
                operations: true,
                ..Default::default()
            },
        }],
    )?;
    assert_eq!(
        monitor.snapshot(),
        Some(previous.as_slice()),
        "subscription changes retain owned status"
    );
    assert!(
        !monitor.refresh_due(now),
        "subscription changes retain the pending retry"
    );
    assert_eq!(
        monitor.options().repository.additional_paths,
        monitor.repository_monitor().options().additional_paths,
        "status options retain the resolved subscriptions"
    );
    let before = monitor.options().clone();
    assert!(
        monitor
            .update_subscriptions(
                None,
                vec![gix::notify::InvalidationPath {
                    path: std::path::PathBuf::new(),
                    changes: Changes::default(),
                }]
            )
            .is_err(),
        "an invalid dependency rejects the entire update"
    );
    assert_eq!(
        monitor.options().repository.worktree_filter,
        before.repository.worktree_filter,
        "failed updates retain selection"
    );
    assert_eq!(
        monitor.options().repository.additional_paths,
        before.repository.additional_paths,
        "failed updates retain dependencies"
    );
    assert_eq!(
        monitor.snapshot(),
        Some(previous.as_slice()),
        "failed updates retain status"
    );
    let outcome = monitor.service(std::time::Instant::now(), || open(root));
    assert!(
        outcome.changes.operations,
        "the next service publishes the application invalidation"
    );
    Ok(())
}

#[cfg(feature = "async-notify")]
mod asynchronous {
    use std::time::{Duration, Instant};

    use gix::error::{ErrorExt, message};

    use super::*;

    #[tokio::test(flavor = "current_thread")]
    async fn service_processing_runs_on_one_worker_and_preserves_errors_and_changes() -> TestResult {
        let root = gix_testtools::scripted_fixture_read_only("make_status_monitor_index.sh")?;
        let mut monitor = monitor(&root)?;
        refresh(&mut monitor, &root)?;
        let previous = monitor.snapshot().expect("the baseline exists").to_vec();
        let executor = std::thread::current().id();
        let (monitor, failed) = monitor
            .process_async(|monitor| {
                monitor.update_subscriptions(
                    None,
                    vec![gix::notify::InvalidationPath {
                        path: std::path::PathBuf::new(),
                        changes: Changes::default(),
                    }],
                )
            })
            .await?;
        assert!(failed.is_err(), "recoverable processing failures return the monitor");
        assert_eq!(
            monitor.snapshot(),
            Some(previous.as_slice()),
            "failed processing retains status"
        );
        assert!(
            monitor.options().repository.additional_paths.is_empty(),
            "failed processing retains options"
        );
        let (monitor, ()) = monitor.process_async(Monitor::rescan).await?;
        let (opened, opener_thread) = std::sync::mpsc::channel();
        let (monitor, outcome) = monitor
            .service_async_with(
                Instant::now(),
                move || {
                    opened
                        .send(std::thread::current().id())
                        .expect("the processor receives opener identity");
                    Err(message("deliberate status opener failure").raise())
                },
                move |monitor, mut outcome| {
                    let worker = std::thread::current().id();
                    assert_ne!(worker, executor, "status servicing stays off the executor");
                    assert_eq!(
                        opener_thread.try_recv().expect("the opener ran before processing"),
                        worker,
                        "servicing and processing share one worker"
                    );
                    assert!(
                        !outcome.changes.is_empty() && !outcome.errors.is_empty(),
                        "simultaneous invalidations and errors reach processing"
                    );
                    assert_eq!(
                        outcome.coverage,
                        monitor.coverage_status(),
                        "the direct combined coverage query is available"
                    );
                    outcome.errors.push(message("status processing diagnostic").raise());
                    outcome
                },
            )
            .await?;
        assert!(
            outcome.errors.len() >= 2,
            "both servicing and processing diagnostics are retained"
        );
        assert_eq!(
            monitor.snapshot(),
            Some(previous.as_slice()),
            "failed coverage retains the previous snapshot"
        );
        monitor.shutdown_async().await?;
        Ok(())
    }

    #[tokio::test(flavor = "current_thread")]
    async fn service_readiness_suspends_pending_status_without_root_polling() -> TestResult {
        use gix::notify::CoverageStatus;

        let root = gix_testtools::scripted_fixture_read_only("make_status_monitor_index.sh")?;
        let mut monitor = Monitor::new(
            &open(&root)?,
            gix::status::monitor::Options {
                repository: gix::notify::Options {
                    metadata_debounce: Duration::ZERO,
                    ..Default::default()
                },
                ..Default::default()
            },
        )?;
        for _ in 0..8 {
            let open_at = root.clone();
            let (next, outcome) = monitor.service_async(Instant::now(), move || open(&open_at)).await?;
            monitor = next;
            assert!(outcome.errors.is_empty(), "coverage service succeeds");
            if monitor.repository_monitor().coverage_status() == CoverageStatus::Healthy {
                break;
            }
        }
        assert_eq!(
            monitor.repository_monitor().coverage_status(),
            CoverageStatus::Healthy,
            "root coverage finishes verification"
        );
        assert!(monitor.is_dirty(), "the hidden status snapshot remains pending");
        tokio::time::timeout(Duration::from_secs(1), monitor.ready_async()).await?;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            assert!(
                Instant::now() < deadline,
                "startup notifications must settle without spinning or synchronous root polling"
            );
            if tokio::time::timeout(Duration::from_millis(400), monitor.ready_service_async())
                .await
                .is_err()
            {
                break;
            }
            let open_at = root.clone();
            let (next, outcome) = monitor.service_async(Instant::now(), move || open(&open_at)).await?;
            monitor = next;
            assert!(
                outcome.errors.is_empty(),
                "startup notifications are serviced successfully"
            );
        }
        monitor.rescan();
        tokio::time::timeout(Duration::from_secs(1), monitor.ready_service_async()).await?;
        monitor.shutdown_async().await?;
        Ok(())
    }

    #[tokio::test(flavor = "current_thread")]
    async fn cancelled_processing_tears_down_unclaimed_results_on_workers() -> TestResult {
        struct RecordDrop(Option<tokio::sync::oneshot::Sender<std::thread::ThreadId>>);
        impl Drop for RecordDrop {
            fn drop(&mut self) {
                if let Some(record) = self.0.take() {
                    let _ = record.send(std::thread::current().id());
                }
            }
        }
        let root = gix_testtools::scripted_fixture_read_only("make_status_monitor_index.sh")?;
        let monitor = monitor(&root)?;
        let executor = std::thread::current().id();
        let (started, processing) = tokio::sync::oneshot::channel();
        let (release, released) = std::sync::mpsc::channel();
        let (record, dropped) = tokio::sync::oneshot::channel();
        let reply = monitor.process_async(move |_| {
            let _ = started.send(());
            released.recv().expect("the test releases the active processor");
            RecordDrop(Some(record))
        });
        tokio::time::timeout(Duration::from_secs(2), processing).await??;
        drop(reply);
        release.send(())?;
        assert_ne!(
            tokio::time::timeout(Duration::from_secs(2), dropped).await??,
            executor,
            "cancelled processing leaves result teardown on a worker"
        );
        Ok(())
    }
}

#[test]
fn cached_snapshot_retains_its_committed_head_until_successful_refresh() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    let mut monitor = monitor(root)?;
    assert_eq!(
        monitor.snapshot_head_id(),
        None,
        "an unread snapshot has no committed HEAD baseline"
    );
    refresh(&mut monitor, root)?;
    let previous_head = open(root)?.head_id()?.detach();
    assert_eq!(
        monitor.snapshot_head_id(),
        Some(previous_head),
        "the successful snapshot records its HEAD baseline"
    );

    std::fs::write(root.join("tracked"), "new HEAD contents\n")?;
    git(root, &["add", "tracked"])?;
    git(root, &["commit", "-qm", "new HEAD"])?;
    let current_head = open(root)?.head_id()?.detach();
    assert_ne!(current_head, previous_head, "the external commit moved HEAD");
    assert!(!monitor.is_dirty(), "native notifications have not yet been serviced");
    assert_eq!(
        refresh(&mut monitor, root)?,
        Update::default(),
        "a cached refresh leaves the old baseline untouched"
    );
    assert_eq!(
        monitor.snapshot_head_id(),
        Some(previous_head),
        "cached status cannot be relabeled with the current repository HEAD"
    );

    monitor.invalidate();
    assert!(
        monitor.refresh(&open(root)?, &AtomicBool::new(true)).is_err(),
        "a failed refresh preserves the committed baseline"
    );
    assert_eq!(
        monitor.snapshot_head_id(),
        Some(previous_head),
        "dirty and failed snapshots retain their original HEAD"
    );
    refresh(&mut monitor, root)?;
    assert_eq!(
        monitor.snapshot_head_id(),
        Some(current_head),
        "a successful refresh commits the new HEAD baseline"
    );
    Ok(())
}

#[test]
fn cached_refresh_rejects_a_different_worktree() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_notify_worktrees.sh")?;
    let main = fixture.join("main");
    let other = fixture.join("linked-a");
    let mut monitor = monitor(&main)?;
    refresh(&mut monitor, &main)?;
    let before = monitor.snapshot().expect("the initial snapshot exists").to_vec();
    let error = monitor
        .refresh(&open(&other)?, &AtomicBool::new(false))
        .expect_err("even a clean cached refresh must reject another checkout");
    assert!(
        error.is_validation(),
        "target mismatch is a validation error: {error:?}"
    );
    assert_eq!(
        monitor.snapshot(),
        Some(before.as_slice()),
        "a mismatched refresh leaves cached status intact"
    );
    assert_eq!(
        monitor.repository_monitor().layout().git_dir,
        open(&main)?.git_dir().canonicalize()?,
        "refresh cannot silently switch the notification coordinator"
    );
    monitor.invalidate();
    assert!(
        monitor.refresh(&open(&other)?, &AtomicBool::new(false)).is_err(),
        "a dirty foreign refresh also fails"
    );
    assert!(
        monitor.refresh_due(std::time::Instant::now()),
        "foreign requests cannot postpone a valid target's pending refresh"
    );
    Ok(())
}

#[test]
fn status_target_switching_preserves_project_subscriptions_and_rejects_foreign_projects() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_writable("make_notify_worktrees.sh")?;
    let main = fixture.path().join("main");
    let linked_a = fixture.path().join("linked-a");
    let linked_b = fixture.path().join("linked-b");
    std::fs::write(linked_a.join("tracked/file"), "checkout A contents\n")?;
    std::fs::write(linked_b.join("only-b"), "checkout B contents\n")?;
    let main_repo = open(&main)?;
    let mut monitor = Monitor::new(
        &main_repo,
        gix::status::monitor::Options {
            repository: gix::notify::Options {
                all_worktrees: true,
                ..Default::default()
            },
            ..Default::default()
        },
    )?;
    monitor.update_checkout_filter(Some(vec![open(&linked_a)?.git_dir().to_owned()]))?;
    monitor.select_worktree(Some(&open(&linked_a)?))?;
    assert!(
        monitor.snapshot().is_none(),
        "selecting another target discards old status"
    );
    refresh(&mut monitor, &linked_a)?;
    assert_matches_fresh_status(&monitor, &linked_a)?;
    assert_eq!(
        monitor.repository_monitor().layout().git_dir,
        main_repo.git_dir().canonicalize()?,
        "collecting another checkout does not change the project coordinator"
    );
    assert_eq!(
        monitor.repository_monitor().other_worktrees().count(),
        2,
        "both private metadata subscriptions stay active"
    );
    let options = monitor.options().repository.clone();
    let before = monitor.snapshot().expect("A has a snapshot").to_vec();
    monitor.select_worktree(Some(&open(&linked_a)?))?;
    assert_eq!(
        monitor.snapshot(),
        Some(before.as_slice()),
        "reselecting a target preserves its snapshot"
    );
    let foreign = gix_testtools::scripted_fixture_read_only("make_status_monitor_index.sh")?;
    let error = monitor
        .select_worktree(Some(&open(&foreign)?))
        .expect_err("another project's common directory must be rejected");
    assert!(
        error.is_validation(),
        "project mismatch is a validation error: {error:?}"
    );
    assert_eq!(
        monitor.snapshot(),
        Some(before.as_slice()),
        "failed selection preserves its cache"
    );
    assert_eq!(
        monitor.options().repository.checkout_filter,
        options.checkout_filter,
        "failed selection preserves native checkout selection"
    );
    monitor.update_checkout_filter(Some(vec![open(&linked_b)?.git_dir().to_owned()]))?;
    monitor.select_worktree(Some(&open(&linked_b)?))?;
    assert!(
        monitor.snapshot().is_none() && monitor.snapshot_head_id().is_none(),
        "B cannot inherit A's status or HEAD baseline"
    );
    let update = refresh(&mut monitor, &linked_b)?;
    assert_eq!(
        update.unstaged,
        Some(Scope::All),
        "a new target gets a complete baseline"
    );
    assert_matches_fresh_status(&monitor, &linked_b)?;
    assert_eq!(
        monitor.snapshot().expect("B has a snapshot")[0].location(),
        "only-b",
        "the single owned snapshot belongs to the selected checkout"
    );
    monitor.select_worktree(None)?;
    assert!(
        !monitor.refresh_due(std::time::Instant::now()),
        "suspended status must not trigger the synchronous refresh loop"
    );
    assert!(
        monitor.selected_worktree().is_none() && monitor.snapshot().is_none(),
        "suspending status releases target state while retaining metadata monitoring"
    );
    assert_eq!(
        monitor.repository_monitor().other_worktrees().count(),
        2,
        "suspending status retains project membership"
    );
    assert!(
        monitor.refresh(&open(&linked_b)?, &AtomicBool::new(false)).is_err(),
        "status collection requires an explicit selected target"
    );
    Ok(())
}

#[test]
fn bare_project_coordinator_can_select_a_linked_checkout() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_notify_worktrees.sh")?;
    let bare = open(&fixture.join("bare.git"))?;
    let target = open(&fixture.join("bare-a"))?;
    let mut monitor = Monitor::new(
        &bare,
        gix::status::monitor::Options {
            repository: gix::notify::Options {
                all_worktrees: true,
                ..Default::default()
            },
            ..Default::default()
        },
    )?;
    monitor.update_checkout_filter(Some(vec![target.git_dir().to_owned()]))?;
    monitor.select_worktree(Some(&target))?;
    assert!(
        monitor.refresh_due(std::time::Instant::now()),
        "a selected checkout schedules status even with a bare coordinator"
    );
    monitor.refresh(&target, &AtomicBool::new(false))?;
    assert!(monitor.snapshot().is_some(), "the linked checkout obtains a baseline");
    assert!(
        monitor.repository_monitor().layout().worktree.is_none(),
        "the stable project remains bare"
    );
    Ok(())
}

#[test]
fn live_submodule_policy_updates_invalidate_the_same_target() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_status_monitor_index.sh")?;
    let mut monitor = monitor(&fixture)?;
    refresh(&mut monitor, &fixture)?;
    let current = monitor.options().submodules;
    monitor.set_submodules(current);
    assert!(!monitor.is_dirty(), "an identical submodule policy is a no-op");
    monitor.set_submodules(gix::status::Submodule::Given {
        ignore: gix::submodule::config::Ignore::All,
        check_dirty: true,
    });
    assert!(monitor.is_dirty(), "changing target policy invalidates cached status");
    assert_eq!(
        refresh(&mut monitor, &fixture)?.unstaged,
        Some(Scope::All),
        "a policy transition recomputes all worktree status"
    );
    Ok(())
}

#[test]
fn selecting_the_same_checkout_in_another_namespace_refreshes_its_baseline() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_status_monitor_namespaced_worktrees.sh")?;
    let coordinator = open(&fixture.join("linked-b"))?;
    let ordinary = open(&fixture.join("main"))?;
    let mut namespaced = ordinary.clone();
    namespaced.set_namespace("status")?;
    let mut monitor = Monitor::new(
        &coordinator,
        gix::status::monitor::Options {
            repository: gix::notify::Options {
                all_worktrees: true,
                ..Default::default()
            },
            ..Default::default()
        },
    )?;
    monitor.update_checkout_filter(Some(vec![ordinary.git_dir().to_owned()]))?;
    monitor.select_worktree(Some(&ordinary))?;
    monitor.refresh(&ordinary, &AtomicBool::new(false))?;
    assert!(
        monitor.snapshot().expect("ordinary status exists").is_empty(),
        "the ordinary checkout is clean"
    );
    assert!(
        monitor.refresh(&namespaced, &AtomicBool::new(false)).is_err(),
        "a different namespace must be selected before using its status"
    );
    monitor.select_worktree(Some(&namespaced))?;
    assert!(
        monitor.snapshot().is_none(),
        "switching namespaces discards the ordinary baseline"
    );
    monitor.refresh(&namespaced, &AtomicBool::new(false))?;
    assert_eq!(
        monitor.snapshot_head_id(),
        Some(namespaced.head_id()?.detach()),
        "the selected namespace owns the committed HEAD baseline"
    );
    assert_ne!(
        monitor.snapshot_head_id(),
        Some(ordinary.head_id()?.detach()),
        "the namespace has a distinct HEAD"
    );
    assert!(
        monitor
            .snapshot()
            .expect("namespaced status exists")
            .iter()
            .any(|item| matches!(item, Item::TreeIndex(_))),
        "the namespaced tree comparison detects the differing tracked file"
    );
    assert_eq!(
        monitor.repository_monitor().layout().git_dir,
        coordinator.git_dir().canonicalize()?,
        "namespace selection does not reroot project metadata"
    );
    monitor.select_worktree(Some(&ordinary))?;
    monitor.refresh(&ordinary, &AtomicBool::new(false))?;
    assert!(
        monitor.snapshot().expect("ordinary status is restored").is_empty(),
        "switching back restores ordinary status"
    );
    Ok(())
}
