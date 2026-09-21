use std::{path::Path, sync::atomic::AtomicBool};

use gix::{
    bstr::BString,
    notify::{Changes, PathChange, Scope},
    status::{Item, WorktreeStatusHandle, index_worktree::Item as WorktreeItem, monitor::Update},
};
use gix_error::TestResult;

mod delta;
mod rules;

fn fixture() -> TestResult<gix_testtools::tempfile::TempDir> {
    Ok(gix_testtools::scripted_fixture_writable(
        "make_status_monitor_index.sh",
    )?)
}

fn open(root: &Path) -> gix::Result<gix::Repository> {
    gix::open_opts(root, gix::open::Options::isolated())
}

fn monitor(root: &Path) -> TestResult<(gix::notify::RepositoryMonitor, WorktreeStatusHandle)> {
    let repo = open(root)?;
    let project = repo.monitor(Default::default())?;
    let status = project.worktree_status(&repo, Default::default())?;
    Ok((project, status))
}

fn refresh(monitor: &mut WorktreeStatusHandle, root: &Path) -> TestResult<Update> {
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

fn invalidate_index(monitor: &mut WorktreeStatusHandle) {
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

fn assert_matches_fresh_status(monitor: &WorktreeStatusHandle, root: &Path) -> TestResult {
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
    let project = repo.monitor(Default::default())?;
    let mut monitor = project.worktree_status(&repo, Default::default())?;
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
        let project = repo.monitor(gix::notify::Options {
            metadata: false,
            worktree_coverage,
            ..Default::default()
        })?;
        let error = project
            .worktree_status(&repo, Default::default())
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

        let has_tracked_change = |monitor: &WorktreeStatusHandle| {
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
        let (_project, mut monitor) = monitor(root)?;
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
    let (_project, mut monitor) = monitor(root)?;
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
    let (_project, mut monitor) = monitor(root)?;
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
    let (_project, mut monitor) = monitor(root)?;
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
    let (_project, mut monitor) = monitor(root)?;
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
    let (_project, mut monitor) = monitor(root)?;
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
    let (_project, mut monitor) = monitor(root)?;
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
    let (_project, mut monitor) = monitor(root)?;
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
    let (_project, mut monitor) = monitor(root)?;
    refresh(&mut monitor, root)?;

    invalidate_index(&mut monitor);
    assert_eq!(
        refresh(&mut monitor, root)?,
        Update::default(),
        "an unchanged index leaves both status domains and the snapshot unchanged"
    );
    assert!(
        !monitor.needs_refresh(),
        "the verified index invalidation is consumed without a status scan"
    );
    assert_matches_fresh_status(&monitor, root)
}

#[test]
fn cancellation_preserves_the_previous_index_baseline_for_retry() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    std::fs::write(root.join("tracked"), "changed\n")?;
    let (_project, mut monitor) = monitor(root)?;
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
    assert!(monitor.needs_refresh(), "the cancelled index change remains pending");
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
    let (_project, mut monitor) = monitor(root)?;
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
    let project = repo.monitor(Default::default())?;
    let mut monitor = project.worktree_status(
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
    let (mut project, mut monitor) = monitor(root)?;
    refresh(&mut monitor, root)?;
    let previous = monitor.snapshot().expect("the successful snapshot exists").to_vec();
    invalidate_index(&mut monitor);
    assert!(
        monitor.refresh(&open(root)?, &AtomicBool::new(true)).is_err(),
        "interruption leaves pending status and a retry deadline"
    );
    let now = std::time::Instant::now();
    assert!(!monitor.refresh_due(now), "the interrupted refresh is in backoff");
    project.update_subscriptions(
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
    let before = project.options().clone();
    assert!(
        project
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
        project.options().worktree_filter,
        before.worktree_filter,
        "failed updates retain selection"
    );
    assert_eq!(
        project.options().additional_paths,
        before.additional_paths,
        "failed updates retain dependencies"
    );
    assert_eq!(
        monitor.snapshot(),
        Some(previous.as_slice()),
        "failed updates retain status"
    );
    let outcome = project.service(std::time::Instant::now(), || open(root));
    assert!(
        outcome.changes.operations,
        "the next service publishes the application invalidation"
    );
    Ok(())
}

#[cfg(feature = "async-notify")]
mod asynchronous {
    use std::time::{Duration, Instant};

    use super::*;

    #[tokio::test(flavor = "current_thread")]
    async fn handle_waits_for_its_project_and_processes_status_on_a_worker() -> TestResult {
        let root = gix_testtools::scripted_fixture_read_only("make_status_monitor_index.sh")?;
        let (mut project, mut status) = monitor(&root)?;
        let repo = open(&root)?;
        for _ in 0..3 {
            project.service(Instant::now(), || Ok(repo.clone()));
        }
        status.service(Instant::now());
        tokio::time::timeout(Duration::from_secs(1), status.ready_async()).await?;
        let (status, baseline) = status.process_async(move |status| refresh(status, &root)).await?;
        baseline?;
        let executor = std::thread::current().id();
        let mut waiting = Box::pin(async { tokio::join!(status.ready_service_async(), status.ready_service_async()) });
        assert!(
            tokio::time::timeout(Duration::from_millis(50), &mut waiting)
                .await
                .is_err(),
            "all waiters on a clean handle sleep until the project delivers an invalidation"
        );
        project.rescan();
        project.service(Instant::now(), || Ok(repo.clone()));
        tokio::time::timeout(Duration::from_secs(1), waiting).await?;
        let (status, outcome) = status
            .service_async_with(Instant::now(), move |_, outcome| {
                assert_ne!(
                    std::thread::current().id(),
                    executor,
                    "submodule service and processing run on a worker"
                );
                outcome
            })
            .await?;
        assert!(outcome.changes.references, "the handle received the shared rescan");
        drop(project);
        tokio::time::timeout(Duration::from_secs(1), status.ready_async()).await?;
        assert!(status.is_closed(), "project shutdown wakes and closes its handles");
        status.shutdown_async().await?;
        Ok(())
    }

    #[tokio::test(flavor = "current_thread")]
    async fn settled_submodules_wake_on_native_events_without_periodic_polling() -> TestResult {
        let fixture = gix_testtools::scripted_fixture_writable("make_status_monitor_submodules.sh")?;
        let root = fixture.path().join("root");
        let (mut project, mut handle) = monitor(&root)?;
        let end = Instant::now() + Duration::from_secs(8);
        loop {
            let outcome = project.service_with_status(Instant::now(), || open(&root), [&mut handle]);
            assert!(
                outcome.errors.is_empty(),
                "the project and its submodules are watchable"
            );
            refresh(&mut handle, &root)?;
            if outcome.coverage == gix::notify::CoverageStatus::Healthy
                && tokio::time::timeout(Duration::from_millis(400), project.ready_with_status_async([&handle]))
                    .await
                    .is_err()
            {
                break;
            }
            assert!(
                Instant::now() < end,
                "a hidden view waits past the blocking polling interval"
            );
        }
        handle.invalidate();
        assert!(
            tokio::time::timeout(Duration::from_millis(350), handle.ready_service_async())
                .await
                .is_err(),
            "a deferred snapshot does not wake a hidden status view"
        );
        let (local, combined, commit) = tokio::join!(
            biased;
            tokio::time::timeout_at(end.into(), handle.ready_service_async()),
            tokio::time::timeout_at(end.into(), project.ready_with_status_async([&handle])),
            async { git(&root.join("outer/inner"), &["commit", "--allow-empty", "-qm", "advance a nested HEAD"]) },
        );
        commit?;
        local?;
        combined?;
        drop(handle);
        loop {
            tokio::time::timeout_at(end.into(), project.ready_async()).await?;
            let out = project.service(Instant::now(), || open(&root));
            assert!(out.errors.is_empty(), "retained native registrations can be serviced");
            if matches!(out.changes.worktree, Some(Scope::Paths(paths)) if paths.iter().any(|p| p.path == "outer")) {
                break;
            }
        }
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
        let (_project, monitor) = monitor(&root)?;
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
    let (_project, mut monitor) = monitor(root)?;
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
    assert!(
        !monitor.needs_refresh(),
        "native notifications have not yet been serviced"
    );
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
    let (project, mut monitor) = monitor(&main)?;
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
        project.layout().git_dir,
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
fn handles_track_linked_checkouts_independently_and_release_their_snapshots() -> TestResult {
    use std::time::{Duration, Instant};

    let fixture = gix_testtools::scripted_fixture_writable("make_notify_worktrees.sh")?;
    let main = open(&fixture.path().join("main"))?;
    let mut project = main.monitor(gix::notify::Options {
        all_worktrees: true,
        metadata_debounce: Duration::ZERO,
        ..Default::default()
    })?;
    let outcome = project.service(Instant::now(), || Ok(main.clone()));
    assert!(outcome.errors.is_empty(), "the worktree inventory was installed");
    let mut repos = outcome
        .other_worktrees
        .iter()
        .map(|change| project.open_worktree(&change.git_dir));
    let a = repos.next().expect("first linked checkout")?;
    let b = repos.next().expect("second linked checkout")?;
    let mut a_status = project.worktree_status(&a, Default::default())?;
    let mut b_status = project.worktree_status(&b, Default::default())?;
    let interrupt = AtomicBool::new(false);
    a_status.refresh(&a, &interrupt)?;
    b_status.refresh(&b, &interrupt)?;
    assert_eq!(a_status.snapshot(), Some([].as_slice()), "A starts clean");
    assert_eq!(b_status.snapshot(), Some([].as_slice()), "B starts clean");
    drop(b_status);

    let tracked = a.workdir().expect("A has a checkout").join("tracked/file");
    std::fs::write(&tracked, "dirty\n")?;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let outcome = project.service(Instant::now(), || Ok(main.clone()));
        assert!(outcome.errors.is_empty(), "servicing the shared watches succeeds");
        if a_status.refresh_due(Instant::now()) {
            a_status.refresh(&a, &interrupt)?;
            if a_status.snapshot().is_some_and(|items| !items.is_empty()) {
                break;
            }
        }
        assert!(Instant::now() < deadline, "A receives its checkout notification");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_matches_fresh_status(&a_status, a.workdir().expect("A has a checkout"))?;
    let mut b_status = project.worktree_status(&b, Default::default())?;
    assert!(b_status.snapshot().is_none(), "a new handle has no previous baseline");
    b_status.refresh(&b, &interrupt)?;
    assert_eq!(b_status.snapshot(), Some([].as_slice()), "A's changes do not dirty B");

    std::fs::write(tracked, "tracked\n")?;
    let deadline = Instant::now() + Duration::from_secs(10);
    while a_status.snapshot().is_some_and(|items| !items.is_empty()) {
        project.service(Instant::now(), || Ok(main.clone()));
        a_status.refresh(&a, &interrupt)?;
        assert!(
            Instant::now() < deadline,
            "the retained dirty checkout returns to clean"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    drop(a_status);
    assert_eq!(
        project.other_worktrees().count(),
        2,
        "dropping status preserves discovery"
    );
    assert_eq!(
        b_status.snapshot(),
        Some([].as_slice()),
        "B retains its independent baseline"
    );
    let foreign = gix_testtools::scripted_fixture_read_only("make_status_monitor_index.sh")?;
    assert!(
        project.worktree_status(&open(&foreign)?, Default::default()).is_err(),
        "a foreign repository cannot subscribe to this project"
    );
    Ok(())
}

#[test]
fn dropping_a_clean_status_handle_keeps_submodule_head_changes_observable() -> TestResult {
    use gix::notify::CoverageStatus;
    use std::time::{Duration, Instant};

    let fixture = gix_testtools::scripted_fixture_writable("make_status_monitor_submodules.sh")?;
    let root = fixture.path().join("root");
    let child = root.join("outer");
    git(&child.join("inner"), &["restore", "file"])?;
    let repo = open(&root)?;
    let mut project = repo.monitor(Default::default())?;
    let interrupt = AtomicBool::new(false);
    let mut status = project.worktree_status(&repo, Default::default())?;
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut quiet = Instant::now();
    loop {
        let out = project.service_with_status(Instant::now(), || open(&root), [&mut status]);
        assert!(out.errors.is_empty(), "registration succeeds: {:?}", out.errors);
        status.refresh(&repo, &interrupt)?;
        if out.coverage != CoverageStatus::Healthy || !out.changes.is_empty() {
            quiet = Instant::now();
        }
        if quiet.elapsed() > Duration::from_millis(350) {
            break;
        }
        assert!(Instant::now() < deadline, "initial registrations settle");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        status.snapshot().expect("successful snapshot").is_empty(),
        "the superproject starts clean"
    );
    drop(status);
    git(
        &child,
        &["commit", "--allow-empty", "-qm", "advance only the submodule HEAD"],
    )?;
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        let out = project.service(Instant::now(), || open(&root));
        assert!(out.errors.is_empty(), "submodule service succeeds: {:?}", out.errors);
        if matches!(out.changes.worktree, Some(Scope::Paths(paths)) if paths.iter().any(|change| change.path == "outer"))
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the project reports the changed gitlink after its clean handle was dropped"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let mut status = project.worktree_status(&repo, Default::default())?;
    assert!(
        status.snapshot().is_none(),
        "reacquiring status starts without a cached baseline"
    );
    status.refresh(&repo, &interrupt)?;
    assert_eq!(status.snapshot().expect("new baseline")[0].location(), "outer");
    Ok(())
}

#[test]
fn combined_service_reports_a_linked_checkouts_submodule_at_its_gitlink() -> TestResult {
    use gix::notify::{CoverageStatus, WorktreeChange};
    use std::time::{Duration, Instant};

    let fixture = gix_testtools::scripted_fixture_writable("make_status_monitor_submodules.sh")?;
    let root = fixture.path().join("root");
    git(&root, &["worktree", "add", "-q", "--detach", "../primary"])?;
    let primary = open(&fixture.path().join("primary"))?;
    let repo = open(&root)?;
    let mut project = primary.monitor(gix::notify::Options {
        all_worktrees: true,
        ..Default::default()
    })?;
    let mut handle = project.worktree_status(&repo, Default::default())?;
    let interrupt = AtomicBool::new(false);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let outcome = project.service_with_status(Instant::now(), || Ok(primary.clone()), [&mut handle]);
        assert!(outcome.errors.is_empty(), "registration succeeds: {:?}", outcome.errors);
        handle.refresh(&repo, &interrupt)?;
        if project.coverage_with_status([&handle]) == CoverageStatus::Healthy {
            break;
        }
        assert!(Instant::now() < deadline, "project and submodule coverage settles");
        std::thread::sleep(Duration::from_millis(10));
    }
    let git_dir = handle.layout().git_dir.clone();
    let mut status = Some(handle);
    for with_handle in [true, false] {
        if with_handle {
            std::fs::write(root.join("outer/inner/file"), "another deep edit\n")?;
        } else {
            drop(status.take());
            git(
                &root.join("outer/inner"),
                &["commit", "--allow-empty", "-qm", "nested HEAD"],
            )?;
        }
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let outcome = project.service_with_status(Instant::now(), || Ok(primary.clone()), &mut status);
            assert!(outcome.errors.is_empty(), "servicing nested watches succeeds");
            assert!(outcome.changes.worktree.is_none(), "the primary checkout is untouched");
            if outcome.other_worktrees.iter().any(|other| {
                other.git_dir == git_dir && matches!(&other.kind, WorktreeChange::Changed(changes)
                    if matches!(&changes.worktree, Some(Scope::Paths(paths)) if paths.iter().any(|entry| entry.path == "outer")))
            }) {
                if let Some(handle) = &status {
                    assert!(handle.needs_refresh(), "servicing notifications leaves status demand-driven");
                }
                break;
            }
            assert!(
                Instant::now() < deadline,
                "the project reports the linked gitlink with or without its handle"
            );
            let timeout = project
                .next_timeout_with_status(Instant::now(), &status)
                .expect("active coverage");
            std::thread::sleep(timeout.min(Duration::from_millis(20)));
        }
    }
    Ok(())
}

#[test]
fn bare_project_coordinator_can_select_a_linked_checkout() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_notify_worktrees.sh")?;
    let bare = open(&fixture.join("bare.git"))?;
    let target = open(&fixture.join("bare-a"))?;
    let project = bare.monitor(gix::notify::Options {
        all_worktrees: true,
        ..Default::default()
    })?;
    let mut monitor = project.worktree_status(&target, Default::default())?;
    assert!(
        monitor.refresh_due(std::time::Instant::now()),
        "a selected checkout schedules status even with a bare coordinator"
    );
    monitor.refresh(&target, &AtomicBool::new(false))?;
    assert!(monitor.snapshot().is_some(), "the linked checkout obtains a baseline");
    assert!(project.layout().worktree.is_none(), "the stable project remains bare");
    Ok(())
}

#[test]
fn live_submodule_policy_updates_invalidate_the_same_target() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_status_monitor_index.sh")?;
    let (_project, mut monitor) = monitor(&fixture)?;
    refresh(&mut monitor, &fixture)?;
    let current = monitor.options().submodules;
    monitor.set_submodules(current);
    assert!(!monitor.needs_refresh(), "an identical submodule policy is a no-op");
    monitor.set_submodules(gix::status::Submodule::Given {
        ignore: gix::submodule::config::Ignore::All,
        check_dirty: true,
    });
    assert!(
        monitor.needs_refresh(),
        "changing target policy invalidates cached status"
    );
    assert_eq!(
        refresh(&mut monitor, &fixture)?.unstaged,
        Some(Scope::All),
        "a policy transition recomputes all worktree status"
    );
    Ok(())
}

#[test]
fn the_same_checkout_has_independent_baselines_in_different_namespaces() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_status_monitor_namespaced_worktrees.sh")?;
    let mut coordinator = gix::open_opts(
        fixture.join("linked-b"),
        gix::open::Options::isolated().configure_related_repository(|_, options| {
            Ok(std::ops::ControlFlow::Continue(
                options.config_overrides(["test.worktreeOpen=related"]),
            ))
        }),
    )?;
    coordinator.set_namespace("status")?;
    let ordinary = open(&fixture.join("main"))?;
    let mut namespaced = ordinary.clone();
    namespaced.set_namespace("status")?;
    let mut project = coordinator.monitor(gix::notify::Options {
        all_worktrees: true,
        ..Default::default()
    })?;
    let outcome = project.service(std::time::Instant::now(), || Ok(coordinator.clone()));
    assert!(
        outcome.errors.is_empty(),
        "the namespaced project discovers its checkouts"
    );
    assert_eq!(
        outcome.other_worktrees.len(),
        2,
        "both additional checkouts were discovered"
    );
    for change in outcome.other_worktrees {
        let reopened = project.open_worktree(&change.git_dir)?;
        assert_eq!(
            reopened.namespace(),
            coordinator.namespace(),
            "reopening inherits the project's reference namespace"
        );
        assert_eq!(
            reopened
                .workdir()
                .expect("the reported checkout has a worktree")
                .canonicalize()?,
            change.worktree,
            "both main and linked checkouts resolve"
        );
        assert_eq!(
            reopened.config_snapshot().string("test.worktreeOpen"),
            Some("related".into()),
            "related repository opening policy is applied"
        );
    }
    let mut monitor = project.worktree_status(&ordinary, Default::default())?;
    monitor.refresh(&ordinary, &AtomicBool::new(false))?;
    assert!(
        monitor.snapshot().expect("ordinary status exists").is_empty(),
        "the ordinary checkout is clean"
    );
    assert!(
        monitor.refresh(&namespaced, &AtomicBool::new(false)).is_err(),
        "each handle validates its own namespace"
    );
    let mut namespaced_status = project.worktree_status(&namespaced, Default::default())?;
    assert!(
        namespaced_status.snapshot().is_none(),
        "a new namespace handle starts without a baseline"
    );
    namespaced_status.refresh(&namespaced, &AtomicBool::new(false))?;
    assert_eq!(
        namespaced_status.snapshot_head_id(),
        Some(namespaced.head_id()?.detach()),
        "the selected namespace owns the committed HEAD baseline"
    );
    assert_ne!(
        namespaced_status.snapshot_head_id(),
        Some(ordinary.head_id()?.detach()),
        "the namespace has a distinct HEAD"
    );
    assert!(
        namespaced_status
            .snapshot()
            .expect("namespaced status exists")
            .iter()
            .any(|item| matches!(item, Item::TreeIndex(_))),
        "the namespaced tree comparison detects the differing tracked file"
    );
    assert_eq!(
        project.layout().git_dir,
        coordinator.git_dir().canonicalize()?,
        "namespace selection does not reroot project metadata"
    );
    monitor.refresh(&ordinary, &AtomicBool::new(false))?;
    assert!(
        monitor.snapshot().expect("ordinary status is cached").is_empty(),
        "reading the namespaced status left the ordinary snapshot intact"
    );
    Ok(())
}
