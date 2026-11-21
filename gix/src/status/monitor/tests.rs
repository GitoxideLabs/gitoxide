use gix_testtools::Result as TestResult;

use super::*;
use std::{fmt::Write, path::Path};

fn git(root: &Path, args: &[&str]) -> TestResult {
    let output = gix_testtools::git_command(root).args(args).output()?;
    assert!(
        output.status.success(),
        "git {args:?} succeeds in the disposable fixture: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn initialize(root: &Path) -> TestResult {
    std::fs::create_dir_all(root)?;
    git(root, &["init", "--quiet", "--initial-branch=main"])?;
    std::fs::write(root.join("tracked"), "initial\n")?;
    git(root, &["add", "tracked"])?;
    git(root, &["commit", "--quiet", "-m", "initial"])
}

fn fixture() -> TestResult<gix_testtools::tempfile::TempDir> {
    let root = gix_testtools::tempfile::tempdir()?;
    initialize(root.path())?;
    Ok(root)
}

fn open(root: &Path) -> Result<Repository> {
    crate::open_opts(root, crate::open::Options::isolated())
}

fn monitor(root: &Path) -> TestResult<(RepositoryMonitor, WorktreeStatusHandle)> {
    let repo = open(root)?;
    let project = repo.monitor(Default::default())?;
    let status = project.worktree_status(
        &repo,
        Options {
            thread_limit: Some(1),
            ..Default::default()
        },
    )?;
    Ok((project, status))
}

fn refresh(monitor: &mut WorktreeStatusHandle, root: &Path) -> TestResult<Update> {
    Ok(monitor.refresh(&open(root)?, &AtomicBool::new(false))?)
}

fn changed_paths(monitor: &WorktreeStatusHandle) -> Vec<BString> {
    monitor
        .snapshot()
        .expect("a successful refresh created a snapshot")
        .iter()
        .map(|item| item.location().to_owned())
        .collect()
}

fn invalidate_path(monitor: &mut WorktreeStatusHandle, path: impl Into<BString>) {
    monitor.invalidate_changes(&Changes {
        worktree: paths(vec![path.into()]),
        ..Default::default()
    });
}

#[test]
fn full_and_scoped_snapshots_match_fresh_status() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    let (_project, mut monitor) = monitor(root)?;
    assert!(
        monitor.snapshot().is_none() && monitor.needs_refresh(),
        "the initial baseline is pending"
    );
    let initial = refresh(&mut monitor, root)?;
    assert!(
        initial.changed && initial.staged && initial.unstaged == Some(Scope::All),
        "the first refresh establishes complete coverage"
    );
    assert!(
        !monitor.needs_refresh(),
        "successful collection clears only completed invalidations"
    );
    std::fs::write(root.join("tracked"), "modified\n")?;
    invalidate_path(&mut monitor, "tracked");
    let update = refresh(&mut monitor, root)?;
    assert_eq!(
        update.unstaged,
        paths(vec!["tracked".into()]),
        "a tracked file event stays local"
    );
    assert!(!update.staged, "unrelated staged state is retained");
    let incremental = monitor.snapshot().expect("snapshot exists").to_vec();
    monitor.invalidate();
    refresh(&mut monitor, root)?;
    assert_eq!(
        monitor.snapshot(),
        Some(incremental.as_slice()),
        "incremental and full status agree"
    );
    std::fs::write(root.join("tracked"), "initial\n")?;
    invalidate_path(&mut monitor, "tracked");
    refresh(&mut monitor, root)?;
    assert!(
        changed_paths(&monitor).is_empty(),
        "scoped replacement removes entries that became clean"
    );
    Ok(())
}

#[test]
fn equal_items_still_report_content_refresh_coverage() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    let (_project, mut monitor) = monitor(root)?;
    std::fs::write(root.join("tracked"), "change one\n")?;
    refresh(&mut monitor, root)?;
    std::fs::write(root.join("tracked"), "change two\n")?;
    invalidate_path(&mut monitor, "tracked");
    let update = refresh(&mut monitor, root)?;
    assert_eq!(
        update.unstaged,
        paths(vec!["tracked".into()]),
        "line counts and other content caches must refresh even for equal status classifications"
    );
    assert!(
        !update.changed,
        "same-sized content edits can retain identical owned status classifications"
    );
    assert!(!update.staged, "only the observed domain was refreshed");
    Ok(())
}

#[test]
fn collapsed_untracked_scopes_expand_to_their_top_level_ancestor() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    std::fs::create_dir_all(root.join("new/deep"))?;
    std::fs::write(root.join("new/deep/file"), "new\n")?;
    let (_project, mut monitor) = monitor(root)?;
    refresh(&mut monitor, root)?;
    std::fs::remove_file(root.join("new/deep/file"))?;
    invalidate_path(&mut monitor, "new/deep/file");
    let update = refresh(&mut monitor, root)?;
    assert_eq!(
        update.unstaged,
        paths(vec!["new".into()]),
        "a child event can change whether its untracked ancestor collapses"
    );
    assert!(
        changed_paths(&monitor).is_empty(),
        "an emptied untracked directory disappears from the cached snapshot"
    );
    Ok(())
}

#[test]
fn ignored_nested_targets_remain_ignored_during_scoped_refresh() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    std::fs::create_dir_all(root.join("project/fuzz/target/deep/out"))?;
    std::fs::write(root.join(".gitignore"), "target/\n")?;
    std::fs::write(root.join("project/.gitignore"), "!out/\n")?;
    git(root, &["add", ".gitignore", "project/.gitignore"])?;
    git(root, &["commit", "--quiet", "-m", "ignore targets"])?;
    let (_project, mut monitor) = monitor(root)?;
    refresh(&mut monitor, root)?;
    std::fs::write(root.join("project/fuzz/target/deep/out/artifact"), "artifact\n")?;
    invalidate_path(&mut monitor, "project/fuzz/target/deep/out/artifact");
    refresh(&mut monitor, root)?;
    assert!(
        changed_paths(&monitor).is_empty(),
        "nested negation cannot re-include files beneath an ignored ancestor"
    );
    Ok(())
}

#[test]
fn unrelated_refs_do_not_recompute_staged_or_worktree_status() -> TestResult {
    let started = Instant::now();
    let phase = |phase| eprintln!("unrelated_refs ({:?}): {phase}", started.elapsed());
    phase("create fixture");
    let fixture = fixture()?;
    let root = fixture.path();
    phase("create monitor");
    let (_project, mut monitor) = monitor(root)?;
    phase("initial refresh");
    refresh(&mut monitor, root)?;
    phase("write unobserved file");
    std::fs::write(root.join("unobserved"), "new\n")?;
    phase("git branch other");
    git(root, &["branch", "other"])?;
    monitor.invalidate_changes(&Changes {
        references: true,
        ..Default::default()
    });
    phase("refresh unrelated references");
    let update = refresh(&mut monitor, root)?;
    assert_eq!(
        update,
        Update::default(),
        "checking unchanged HEAD avoids both status scans"
    );
    assert!(
        changed_paths(&monitor).is_empty(),
        "an unrelated ref does not accidentally refresh the worktree"
    );
    phase("git symbolic-ref HEAD refs/heads/other");
    git(root, &["symbolic-ref", "HEAD", "refs/heads/other"])?;
    monitor.invalidate_changes(&Changes {
        references: true,
        ..Default::default()
    });
    phase("refresh changed HEAD");
    let update = refresh(&mut monitor, root)?;
    assert!(
        update.staged && update.unstaged.is_none(),
        "HEAD attachment changes refresh staged status only"
    );
    assert!(
        changed_paths(&monitor).is_empty(),
        "HEAD-only refresh preserves cached unstaged entries"
    );
    phase("drop monitor");
    drop(monitor);
    phase("close fixture");
    if let Err(err) = fixture.close() {
        eprintln!("unrelated_refs: fixture cleanup failed: {err}");
    }
    phase("finished");
    Ok(())
}

#[test]
fn captured_head_tree_is_stable_after_reference_moves() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    let repo = open(root)?;
    let captured = Head::read(&repo)?;
    let original_tree_id = repo.head_tree_id()?.detach();

    std::fs::write(root.join("tracked"), "new HEAD contents\n")?;
    git(root, &["add", "tracked"])?;
    git(root, &["commit", "--quiet", "-m", "move HEAD"])?;
    let repo = open(root)?;
    assert_ne!(
        repo.head_tree_id()?.detach(),
        original_tree_id,
        "the reference move changes the tree that an unpinned status would read"
    );
    let pinned_tree_id = captured.tree_id(&repo)?;
    assert_eq!(
        pinned_tree_id, original_tree_id,
        "the captured commit determines the comparison tree"
    );
    let items = repo
        .status(gix_utils::progress::Discard)?
        .head_tree(pinned_tree_id)
        .collect_internal(Vec::new(), true, false, &AtomicBool::new(false), None, false)?;
    assert!(
        matches!(
            items.as_slice(),
            [Item::TreeIndex(gix_diff::index::Change::Modification { location, .. })]
                if location.as_ref() == "tracked"
        ),
        "staged collection compares the current index against the captured commit after HEAD moves"
    );

    git(root, &["symbolic-ref", "HEAD", "refs/heads/unborn"])?;
    let unborn = Head::read(&open(root)?)?;
    assert_eq!(unborn.commit_id, None, "the new branch has no commit baseline");
    git(root, &["symbolic-ref", "HEAD", "refs/heads/main"])?;
    let repo = open(root)?;
    let items = repo
        .status(gix_utils::progress::Discard)?
        .head_tree(unborn.tree_id(&repo)?)
        .collect_internal(Vec::new(), true, false, &AtomicBool::new(false), None, false)?;
    assert!(
        matches!(
            items.as_slice(),
            [Item::TreeIndex(gix_diff::index::Change::Addition { location, .. })]
                if location.as_ref() == "tracked"
        ),
        "a captured unborn HEAD keeps the empty-tree comparison after HEAD becomes born"
    );
    Ok(())
}

#[test]
fn head_tree_index_is_reused_until_tree_or_configuration_changes() -> TestResult {
    let fixture = gix_testtools::tempfile::tempdir()?;
    let root = fixture.path();
    git(root, &["init", "--quiet", "--initial-branch=main"])?;
    std::fs::write(root.join("tracked"), "initial\n")?;
    git(root, &["add", "tracked"])?;
    let (_project, mut monitor) = monitor(root)?;
    refresh(&mut monitor, root)?;
    let empty_tree_id = open(root)?.object_hash().empty_tree();
    assert_eq!(
        monitor.head_tree_index.as_ref().map(|cached| cached.tree_id),
        Some(empty_tree_id),
        "an unborn HEAD caches the empty tree"
    );
    assert_eq!(
        changed_paths(&monitor),
        [BString::from("tracked")],
        "the staged file remains visible against the cached head tree"
    );

    git(root, &["commit", "--quiet", "-m", "initial"])?;
    monitor.invalidate_changes(&Changes {
        references: true,
        ..Default::default()
    });
    refresh(&mut monitor, root)?;
    assert!(
        changed_paths(&monitor).is_empty(),
        "the first commit clears staged changes"
    );
    let (initial_tree_id, initial_entries) = monitor
        .head_tree_index
        .as_ref()
        .map(|cached| (cached.tree_id, cached.index.entries().as_ptr()))
        .expect("refresh caches the committed tree");
    assert_ne!(
        initial_tree_id, empty_tree_id,
        "the committed tree replaces the unborn tree"
    );

    std::fs::write(root.join("tracked"), "changed\n")?;
    git(root, &["add", "tracked"])?;
    monitor.invalidate_changes(&Changes {
        index: true,
        ..Default::default()
    });
    refresh(&mut monitor, root)?;
    let cached = monitor.head_tree_index.as_ref().expect("the head tree remains cached");
    assert_eq!(
        cached.tree_id, initial_tree_id,
        "staging does not change the comparison tree"
    );
    assert_eq!(
        cached.index.entries().as_ptr(),
        initial_entries,
        "staging reuses the existing tree index allocation instead of rebuilding it"
    );
    assert_eq!(
        changed_paths(&monitor),
        [BString::from("tracked")],
        "the staged file remains visible against the cached head tree"
    );

    git(root, &["commit", "--quiet", "-m", "changed tree"])?;
    monitor.invalidate_changes(&Changes {
        references: true,
        ..Default::default()
    });
    refresh(&mut monitor, root)?;
    let (changed_tree_id, changed_entries) = monitor
        .head_tree_index
        .as_ref()
        .map(|cached| (cached.tree_id, cached.index.entries().as_ptr()))
        .expect("refresh caches the changed tree");
    assert_ne!(
        changed_tree_id, initial_tree_id,
        "a new head tree replaces the cached index"
    );
    assert!(
        changed_paths(&monitor).is_empty(),
        "committing clears the staged change"
    );

    git(root, &["commit", "--quiet", "--allow-empty", "-m", "same tree"])?;
    monitor.invalidate_changes(&Changes {
        references: true,
        ..Default::default()
    });
    refresh(&mut monitor, root)?;
    assert_eq!(
        monitor
            .head_tree_index
            .as_ref()
            .map(|cached| cached.index.entries().as_ptr()),
        Some(changed_entries),
        "a different commit with the same tree reuses the cached index"
    );

    monitor.invalidate_changes(&Changes {
        configuration: true,
        ..Default::default()
    });
    assert!(
        monitor.head_tree_index.is_none(),
        "configuration changes must reapply tree path validation"
    );
    refresh(&mut monitor, root)?;
    git(root, &["symbolic-ref", "HEAD", "refs/heads/unborn"])?;
    monitor.invalidate_changes(&Changes {
        references: true,
        ..Default::default()
    });
    refresh(&mut monitor, root)?;
    assert_eq!(
        monitor.head_tree_index.as_ref().map(|cached| cached.tree_id),
        Some(empty_tree_id),
        "returning to an unborn branch replaces the comparison tree"
    );
    assert_eq!(
        changed_paths(&monitor),
        [BString::from("tracked")],
        "the staged file remains visible against the cached head tree"
    );
    Ok(())
}

#[test]
fn cached_head_tree_rechecks_path_protection_from_fresh_repositories() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    let open_with = |overrides| {
        crate::open_opts(
            root,
            crate::open::Options::isolated()
                .strict_config(true)
                .config_overrides(overrides),
        )
    };
    let relaxed = open_with([
        "gitoxide.core.protectWindows=false",
        "core.protectHFS=false",
        "core.protectNTFS=false",
    ])?;
    let (_project, mut monitor) = monitor(root)?;
    monitor.refresh(&relaxed, &AtomicBool::new(false))?;
    let tree_id = monitor
        .head_tree_index
        .as_ref()
        .expect("initial tree is cached")
        .tree_id;
    for overrides in [
        [
            "gitoxide.core.protectWindows=true",
            "core.protectHFS=false",
            "core.protectNTFS=false",
        ],
        [
            "gitoxide.core.protectWindows=true",
            "core.protectHFS=true",
            "core.protectNTFS=false",
        ],
        [
            "gitoxide.core.protectWindows=true",
            "core.protectHFS=true",
            "core.protectNTFS=true",
        ],
    ] {
        // Stat-only notifications intentionally skip staged collection in the newer monitor.
        std::fs::write(root.join("tracked"), format!("staged for {overrides:?}\n"))?;
        git(root, &["add", "tracked"])?;
        let previous_entries = monitor
            .head_tree_index
            .as_ref()
            .expect("the tree remains cached")
            .index
            .entries()
            .as_ptr();
        monitor.invalidate_changes(&Changes {
            index: true,
            ..Default::default()
        });
        monitor.refresh(&open_with(overrides)?, &AtomicBool::new(false))?;
        let cached = monitor
            .head_tree_index
            .as_ref()
            .expect("refresh caches the validated tree");
        assert_eq!(cached.tree_id, tree_id, "the head tree did not change");
        assert_ne!(
            cached.index.entries().as_ptr(),
            previous_entries,
            "each stricter path protection rebuilds the tree index without a configuration notification"
        );
    }
    let before = monitor.snapshot().expect("successful snapshot exists").to_vec();
    std::fs::write(root.join("tracked"), "another staged change\n")?;
    git(root, &["add", "tracked"])?;
    monitor.invalidate_changes(&Changes {
        index: true,
        ..Default::default()
    });
    let invalid = open_with([
        "gitoxide.core.protectWindows=true",
        "core.protectHFS=true",
        "core.protectNTFS=invalid",
    ])?;
    assert!(
        monitor.refresh(&invalid, &AtomicBool::new(false)).is_err(),
        "cache hits still validate path-protection configuration"
    );
    assert_eq!(
        monitor.snapshot(),
        Some(before.as_slice()),
        "invalid configuration preserves the snapshot"
    );
    assert!(monitor.needs_refresh(), "failed validation leaves the refresh pending");
    Ok(())
}

#[test]
fn failed_refresh_does_not_publish_a_new_head_tree_index() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    let (_project, mut monitor) = monitor(root)?;
    refresh(&mut monitor, root)?;
    let initial_tree_id = monitor
        .head_tree_index
        .as_ref()
        .expect("initial tree is cached")
        .tree_id;
    let before = monitor.snapshot().expect("initial snapshot exists").to_vec();
    std::fs::write(root.join("tracked"), "changed\n")?;
    git(root, &["commit", "--quiet", "-am", "changed tree"])?;
    monitor.invalidate_changes(&Changes {
        references: true,
        ..Default::default()
    });
    assert!(
        monitor.refresh(&open(root)?, &AtomicBool::new(true)).is_err(),
        "cancellation preserves the last successful cache"
    );
    assert_eq!(
        monitor.head_tree_index.as_ref().map(|cached| cached.tree_id),
        Some(initial_tree_id),
        "interrupted refreshes retain the previously cached tree"
    );
    let invalid = crate::open_opts(
        root,
        crate::open::Options::isolated()
            .strict_config(true)
            .config_overrides(["diff.renames=invalid"]),
    )?;
    assert!(
        monitor.refresh(&invalid, &AtomicBool::new(false)).is_err(),
        "invalid rename configuration fails after expanding the new tree"
    );
    assert_eq!(
        monitor.head_tree_index.as_ref().map(|cached| cached.tree_id),
        Some(initial_tree_id),
        "failed collection does not publish its replacement tree index"
    );
    assert_eq!(
        monitor.snapshot(),
        Some(before.as_slice()),
        "the previous snapshot survives"
    );
    assert!(monitor.needs_refresh(), "the changed head remains pending");
    refresh(&mut monitor, root)?;
    assert_ne!(
        monitor.head_tree_index.as_ref().map(|cached| cached.tree_id),
        Some(initial_tree_id),
        "a successful retry publishes the new tree index"
    );
    assert!(!monitor.needs_refresh(), "the retry completes the pending refresh");
    Ok(())
}

#[test]
fn failures_and_interruptions_preserve_snapshot_and_pending_work() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    let (_project, mut monitor) = monitor(root)?;
    refresh(&mut monitor, root)?;
    let before = monitor.snapshot().expect("snapshot exists").to_vec();
    std::fs::write(root.join("tracked"), "changed\n")?;
    invalidate_path(&mut monitor, "tracked");
    assert!(
        monitor.refresh(&open(root)?, &AtomicBool::new(true)).is_err(),
        "interrupts abort before publishing partial status"
    );
    assert_eq!(
        monitor.snapshot(),
        Some(before.as_slice()),
        "the last successful snapshot survives cancellation"
    );
    assert!(monitor.needs_refresh(), "cancelled scopes remain pending");
    let now = Instant::now();
    assert!(
        !monitor.refresh_due(now),
        "failed refreshes do not spin the application's event loop"
    );
    assert!(
        monitor.refresh_due(now + Duration::from_secs(6)),
        "pending work becomes retryable after the configured delay"
    );
    let index = std::fs::read(root.join(".git/index"))?;
    std::fs::write(root.join(".git/index"), "broken")?;
    monitor.invalidate_changes(&Changes {
        index: true,
        ..Default::default()
    });
    assert!(
        monitor.refresh(&open(root)?, &AtomicBool::new(false)).is_err(),
        "invalid index errors are propagated"
    );
    assert_eq!(
        monitor.snapshot(),
        Some(before.as_slice()),
        "the previous snapshot survives collection failure"
    );
    assert!(monitor.needs_refresh(), "failed refreshes are retryable");
    std::fs::write(root.join(".git/index"), index)?;
    refresh(&mut monitor, root)?;
    assert_eq!(
        changed_paths(&monitor),
        [BString::from("tracked")],
        "repairing the input completes the pending refresh"
    );
    Ok(())
}

#[test]
fn invalidation_bounds_and_literal_scopes_are_preserved() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    std::fs::write(root.join("literal[glob]"), "old\n")?;
    git(root, &["add", "literal[glob]"])?;
    git(root, &["commit", "--quiet", "-m", "literal"])?;
    let (_project, mut monitor) = monitor(root)?;
    refresh(&mut monitor, root)?;
    std::fs::write(root.join("literal[glob]"), "new\n")?;
    invalidate_path(&mut monitor, "literal[glob]");
    refresh(&mut monitor, root)?;
    assert_eq!(
        changed_paths(&monitor),
        [BString::from("literal[glob]")],
        "metacharacters are interpreted as literal filenames"
    );
    invalidate_path(&mut monitor, "../outside");
    let repo = open(root)?;
    let index = repo.index_or_empty()?;
    assert_eq!(
        monitor.effective_scope(&repo, &index, monitor.unstaged.clone())?,
        Some(Scope::All),
        "invalid relative scopes fall back to a root scan"
    );
    monitor.unstaged = None;
    for number in 0..5000 {
        invalidate_path(&mut monitor, format!("file-{number}"));
    }
    assert_eq!(
        monitor.unstaged,
        Some(Scope::All),
        "a busy application cannot accumulate unbounded scope storage"
    );
    Ok(())
}

fn add_submodule(parent: &Path, name: &str) -> TestResult {
    let child = parent.join(name);
    initialize(&child)?;
    let modules_path = parent.join(".gitmodules");
    let mut modules = if modules_path.exists() {
        std::fs::read_to_string(&modules_path)?
    } else {
        String::new()
    };
    writeln!(modules, "[submodule \"{name}\"]\npath = {name}\nurl = ./unused")?;
    std::fs::write(modules_path, modules)?;
    git(parent, &["add", ".gitmodules"])?;
    let child_repo = open(&child)?;
    let commit_id = child_repo.head_id()?;
    git(
        parent,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{commit_id},{name}"),
        ],
    )?;
    git(parent, &["commit", "--quiet", "-m", "add submodule"])
}

#[test]
fn submodule_monitor_discovery_respects_ignore_configuration_trust() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_status_monitor_submodules.sh")?;
    for trust in [gix_sec::Trust::Reduced, gix_sec::Trust::Full] {
        let mut repo = open(&fixture.join("root"))?;
        repo.config_snapshot_mut()
            .append(gix_config::File::from_bytes_no_includes(
                b"[diff]
                    ignoreSubmodules = all
                ",
                gix_config::file::Metadata::from(gix_config::Source::Local).with(trust),
                Default::default(),
            )?)?;
        let discovered = submodules::discover(
            &repo,
            Submodule::default(),
            &Default::default(),
            &AtomicBool::new(false),
        )?;
        assert_eq!(
            discovered.children.is_empty(),
            trust == gix_sec::Trust::Full,
            "only trusted configuration can disable monitoring initialized submodules"
        );
    }
    Ok(())
}

#[test]
fn submodule_monitors_do_not_inherit_root_checkout_selection_or_application_dependencies() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_status_monitor_submodules.sh")?;
    let repo = open(&fixture.join("root"))?;
    let options = crate::notify::Options {
        all_worktrees: true,
        worktree_filter: Some(Vec::new()),
        additional_paths: vec![crate::notify::InvalidationPath {
            path: fixture.join("application-state"),
            changes: Changes {
                references: true,
                ..Default::default()
            },
        }],
        worktree_coverage: Some(crate::notify::WorktreeCoverage::All),
        metadata_debounce: Duration::from_millis(17),
        max_latency: Duration::from_millis(31),
        retry_interval: Duration::from_secs(2),
        ..Default::default()
    };
    let discovered = submodules::discover(&repo, Submodule::default(), &options, &AtomicBool::new(false))?;
    assert_eq!(
        discovered.children.len(),
        2,
        "both initialized submodule levels are monitored"
    );
    for child in discovered.children.values() {
        let child = child.child.monitor.options();
        assert!(
            !child.all_worktrees && child.worktree_filter.is_none() && child.additional_paths.is_empty(),
            "root checkout selection and application dependencies belong only to the root monitor"
        );
        assert_eq!(
            (
                child.worktree_coverage,
                child.metadata_debounce,
                child.max_latency,
                child.retry_interval
            ),
            (
                options.worktree_coverage,
                options.metadata_debounce,
                options.max_latency,
                options.retry_interval,
            ),
            "children retain status coverage and scheduling options"
        );
    }
    Ok(())
}

#[test]
fn initialized_submodules_are_live_recursive_and_obey_ignore_modes() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    add_submodule(root, "child")?;
    add_submodule(&root.join("child"), "grandchild")?;
    git(root, &["add", "child"])?;
    git(root, &["commit", "--quiet", "-m", "update child"])?;
    let (_project, mut monitor) = monitor(root)?;
    refresh(&mut monitor, root)?;
    assert_eq!(
        monitor.watches.children.len(),
        2,
        "initialized nested modules each have a detached monitor"
    );
    assert_eq!(
        monitor
            .watches
            .children
            .values()
            .find(|child| child.mount.as_slice() == b"child/grandchild")
            .expect("nested child monitor exists")
            .top_level,
        BString::from("child"),
        "nested events invalidate the owning root gitlink"
    );
    std::fs::write(root.join("child/grandchild/tracked"), "nested change\n")?;
    invalidate_path(&mut monitor, "child");
    refresh(&mut monitor, root)?;
    assert_eq!(
        changed_paths(&monitor),
        [BString::from("child")],
        "a deep submodule change is represented at the root gitlink"
    );
    git(root, &["config", "submodule.child.ignore", "dirty"])?;
    monitor.invalidate_changes(&Changes {
        configuration: true,
        ..Default::default()
    });
    refresh(&mut monitor, root)?;
    assert_eq!(
        monitor.watches.children.len(),
        1,
        "ignore=dirty watches child HEAD but needs no descendant worktree watches"
    );
    assert!(
        !monitor
            .watches
            .children
            .values()
            .find(|child| child.mount.as_slice() == b"child")
            .expect("child monitor exists")
            .worktree,
        "worktree notifications are disabled for ignore=dirty"
    );
    assert!(
        changed_paths(&monitor).is_empty(),
        "ignored nested worktree changes are absent from status"
    );
    git(root, &["config", "submodule.child.ignore", "all"])?;
    monitor.invalidate_changes(&Changes {
        configuration: true,
        ..Default::default()
    });
    refresh(&mut monitor, root)?;
    assert!(
        monitor.watches.children.is_empty(),
        "ignore=all releases the child's native resources"
    );
    Ok(())
}

#[test]
fn submodule_initialization_and_removal_update_the_inventory() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    add_submodule(root, "child")?;
    // Toggle the checkout's backlink while keeping native watches live. Moving a directory
    // containing active metadata watches can be denied on Windows.
    git(root, &["submodule", "absorbgitdirs", "child"])?;
    let git_file = root.join("child/.git");
    let backlink = std::fs::read(&git_file)?;
    std::fs::remove_file(&git_file)?;
    let (_project, mut monitor) = monitor(root)?;
    refresh(&mut monitor, root)?;
    assert!(
        monitor.watches.children.is_empty(),
        "uninitialized modules do not create failing worktree watchers"
    );
    std::fs::write(&git_file, backlink)?;
    invalidate_path(&mut monitor, "child");
    refresh(&mut monitor, root)?;
    assert_eq!(
        monitor.watches.children.len(),
        1,
        "a newly initialized configured module gets live coverage"
    );
    std::fs::remove_file(&git_file)?;
    invalidate_path(&mut monitor, "child");
    refresh(&mut monitor, root)?;
    assert!(
        monitor.watches.children.is_empty(),
        "deinitialization removes stale native coverage"
    );
    Ok(())
}

#[test]
fn checkout_coverage_controls_submodule_watches_and_refresh_deadlines() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    add_submodule(root, "child")?;
    let (mut project, mut monitor) = monitor(root)?;
    project.service(Instant::now(), || open(root));
    refresh(&mut monitor, root)?;
    assert_eq!(monitor.watches.children.len(), 1, "the handle owns the child's watch");

    project.update_checkout_filter(Some(Vec::new()))?;
    let now = Instant::now();
    project.service(now, || open(root));
    monitor.service(now);
    assert!(
        monitor.watches.children.is_empty(),
        "disabling checkout coverage releases submodule watches"
    );
    assert!(
        monitor.needs_refresh(),
        "suspended status can still be refreshed on demand"
    );
    assert_eq!(
        monitor.next_timeout(now),
        None,
        "a disabled checkout has no status deadline"
    );
    project.update_checkout_filter(None)?;
    project.service(now, || open(root));
    assert_eq!(
        monitor.next_timeout(now),
        Some(Duration::ZERO),
        "resuming coverage schedules status"
    );
    refresh(&mut monitor, root)?;
    assert_eq!(
        monitor.watches.children.len(),
        1,
        "resuming coverage rediscovers the child"
    );
    Ok(())
}

#[test]
fn monitoring_recovery_requires_successful_retry_and_inventory_verification() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    let (mut project, mut monitor) = monitor(root)?;
    assert!(!monitor.is_healthy(), "uninstalled subscriptions are not healthy");
    refresh(&mut monitor, root)?;
    project.service(Instant::now(), || open(root));
    project.service(Instant::now(), || open(root));
    refresh(&mut monitor, root)?;
    assert!(
        monitor.is_healthy(),
        "the baseline and subscription inventory are verified"
    );

    project.rescan();
    let now = Instant::now();
    let failed = project.service(now, || Err(message("injected repository reopen failure").raise()));
    assert!(!failed.errors.is_empty(), "the registration retry reports its failure");
    assert!(!monitor.is_healthy(), "failure remains visible while retry is pending");
    let quiet = project.service(now + Duration::from_millis(1), || {
        panic!("a quiet service call must wait for the retry deadline")
    });
    assert!(
        quiet.errors.is_empty(),
        "pending retries need not repeat their diagnostic"
    );
    assert!(!monitor.is_healthy(), "silence is not a positive recovery signal");
    let later = now + Duration::from_secs(6);
    let recovered = project.service(later, || open(root));
    assert!(
        recovered.errors.is_empty(),
        "the repository can be reopened after repair"
    );
    assert!(
        !monitor.is_healthy(),
        "replacement subscriptions still need their registration verification and submodule reconciliation"
    );
    project.service(later, || open(root));
    refresh(&mut monitor, root)?;
    assert!(
        monitor.is_healthy(),
        "successful verification and reconciliation certify recovery"
    );
    Ok(())
}

#[test]
fn pending_submodule_discovery_reports_updating_without_recovery() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_status_monitor_index.sh")?;
    let root = fixture.as_path();
    for enabled in [true, false] {
        let repo = open(root)?;
        let mut project = repo.monitor(crate::notify::Options {
            worktree_coverage: enabled.then_some(Default::default()),
            ..Default::default()
        })?;
        let mut monitor = project.worktree_status(&repo, Default::default())?;
        let now = Instant::now();
        project.service(now, || open(root));
        project.service(now, || open(root));
        let outcome = monitor.service(now);
        assert_eq!(
            project.coverage_status(),
            crate::notify::CoverageStatus::Healthy,
            "the root subscriptions have completed their registration verification"
        );
        assert_eq!(
            outcome.coverage,
            if enabled {
                crate::notify::CoverageStatus::Updating
            } else {
                crate::notify::CoverageStatus::Healthy
            },
            "only enabled submodule discovery keeps combined coverage pending"
        );
    }
    Ok(())
}

#[test]
fn child_recovery_remains_visible_between_bounded_service_rounds() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    add_submodule(root, "child")?;
    let (mut project, mut monitor) = monitor(root)?;
    refresh(&mut monitor, root)?;
    let now = Instant::now();
    project.service(now, || open(root));
    project.service(now, || open(root));
    monitor.service(now);
    refresh(&mut monitor, root)?;
    assert!(monitor.is_healthy(), "root and child coverage start verified");

    let child_key = monitor
        .watches
        .children
        .keys()
        .next()
        .expect("the fixture has one child")
        .clone();
    let child = monitor
        .watches
        .children
        .get_mut(&child_key)
        .expect("the selected child exists");
    let open_options = child.open_options.clone_with_trust();
    // An invalid in-memory override fails reopening without changing watched files.
    child
        .open_options
        .modify(|options| options.config_overrides(["invalid"]));
    child.monitor.rescan();
    let pending = child.monitor.coverage_status();
    let now = Instant::now();
    assert_eq!(
        project.coverage_with_status([&monitor]),
        pending,
        "the child reports pending repair before service"
    );
    assert_eq!(
        project.next_timeout_with_status(now, [&monitor]),
        Some(Duration::ZERO),
        "a pending submodule shortens the project's polling timeout"
    );
    let failed = project.service_with_status(now, || open(root), [&mut monitor]);
    assert!(!failed.errors.is_empty(), "the child's reopen failure is forwarded");
    let retry = now + monitor.notifications.retry_interval;
    assert_eq!(
        failed.coverage,
        crate::notify::CoverageStatus::Recovering { retry_at: Some(retry) },
        "combined coverage reports the child's scheduled repair deadline"
    );
    let quiet = project.service_with_status(now + Duration::from_millis(1), || open(root), [&mut monitor]);
    assert!(
        quiet.errors.is_empty(),
        "quiet retry backoff does not repeat diagnostics"
    );
    assert_eq!(
        quiet.coverage, failed.coverage,
        "quiet backoff retains the child repair deadline"
    );

    monitor
        .watches
        .children
        .get_mut(&child_key)
        .expect("the failed child remains retained")
        .open_options = open_options;
    let repaired = monitor.service(retry);
    assert!(repaired.errors.is_empty(), "restored reopening options allow repair");
    assert_eq!(
        repaired.coverage,
        crate::notify::CoverageStatus::Updating,
        "repaired subscriptions still need verification and submodule reconciliation"
    );
    monitor.service(retry);
    refresh(&mut monitor, root)?;
    let verified = monitor.service(retry);
    assert_eq!(
        verified.coverage,
        crate::notify::CoverageStatus::Healthy,
        "verification and reconciliation certify combined root and child recovery"
    );
    assert!(
        monitor.is_healthy(),
        "the direct health query agrees with the service outcome"
    );
    Ok(())
}

#[test]
fn descendant_scope_mapping_is_component_aware() {
    let modules = vec![BString::from("child/nested"), BString::from("childish/other")];
    assert!(
        scope_touches_modules_at(paths(vec!["nested".into()]).as_ref(), &"child".into(), &modules),
        "nested topology refreshes discovery"
    );
    assert!(
        !scope_touches_modules_at(paths(vec!["source/file".into()]).as_ref(), &"child".into(), &modules),
        "ordinary child files do not rediscover the submodule graph"
    );
    assert!(
        !scope_touches_modules(paths(vec!["childishness".into()]).as_ref(), &modules),
        "byte prefixes do not conflate distinct path components"
    );
    let modules = vec![BString::from("Café/naïve")];
    assert!(
        scope_touches_modules(paths(vec!["Cafe\u{301}/nai\u{308}ve".into()]).as_ref(), &modules),
        "native decomposed paths still invalidate configured submodule topology"
    );
    assert!(
        scope_touches_modules_at(paths(vec!["nai\u{308}ve".into()]).as_ref(), &"Café".into(), &modules),
        "nested native aliases are mapped through the configured parent mount"
    );
    assert!(
        !scope_touches_modules(paths(vec!["Cafe\u{301}-other".into()]).as_ref(), &modules),
        "Unicode composition preserves component boundaries"
    );
}

#[test]
fn child_service_is_fair_bounded_and_does_not_postpone_an_idle_deadline() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    for number in 0..CHILDREN_PER_SERVICE + 2 {
        add_submodule(root, &format!("child-{number}"))?;
    }
    let (_project, mut monitor) = monitor(root)?;
    refresh(&mut monitor, root)?;
    let now = Instant::now();
    let keys: Vec<_> = monitor.watches.children.keys().cloned().collect();
    monitor.service(now);
    assert_eq!(
        monitor.watches.cursor.as_ref(),
        Some(&keys[CHILDREN_PER_SERVICE - 1]),
        "one pass verifies at most its fixed child budget"
    );
    assert_eq!(
        monitor.next_service_timeout(now),
        Some(Duration::ZERO),
        "unvisited children remain ready immediately"
    );
    monitor.service(now);
    assert!(
        keys[CHILDREN_PER_SERVICE..].iter().all(
            |key| monitor.watches.children[key].monitor.coverage_status() == crate::notify::CoverageStatus::Healthy
        ),
        "the next pass verifies the remaining children before wrapping"
    );
    for child in monitor.watches.children.values_mut() {
        std::fs::remove_file(child.monitor.layout().git_dir.join("HEAD"))?;
        child.monitor.rescan();
    }
    let failures = monitor.service(Instant::now());
    assert_eq!(
        failures.errors.len(),
        MAX_ERRORS,
        "simultaneous child failures retain bounded representative diagnostics plus an omission summary"
    );
    assert!(
        failures.changes.worktree.is_some(),
        "truncating diagnostics never hides invalidations"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn symbolic_submodule_aliases_are_rejected() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    add_submodule(root, "child")?;
    std::os::unix::fs::symlink("child", root.join("alias"))?;
    let mut modules = std::fs::read_to_string(root.join(".gitmodules"))?;
    modules.push_str("[submodule \"alias\"]\npath = alias\nurl = ./unused\n");
    std::fs::write(root.join(".gitmodules"), modules)?;
    let child = open(&root.join("child"))?;
    let commit_id = child.head_id()?;
    git(root, &["add", ".gitmodules"])?;
    git(
        root,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{commit_id},alias"),
        ],
    )?;
    let error = submodules::discover(
        &open(root)?,
        Submodule::default(),
        &Default::default(),
        &AtomicBool::new(false),
    )
    .err()
    .expect("symbolic submodule aliases are rejected");
    assert!(
        error.is_validation(),
        "submodule discovery preserves path validation errors"
    );
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn submodule_identity_uses_os_spelling_even_when_configuration_names_a_unicode_alias() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    add_submodule(root, "Café")?;
    std::fs::rename(root.join("Café"), root.join("temporary-child"))?;
    std::fs::rename(root.join("temporary-child"), root.join("Cafe\u{301}"))?;
    let repo = crate::open_opts(
        root,
        crate::open::Options::isolated().config_overrides(["core.precomposeUnicode=false"]),
    )?;
    let discovered = submodules::discover(
        &repo,
        Submodule::default(),
        &Default::default(),
        &AtomicBool::new(false),
    )?;
    assert_eq!(
        discovered.children.keys().collect::<Vec<_>>(),
        vec![&root.join("Cafe\u{301}/.git").canonicalize()?],
        "cycle detection and shared-repository identity use the canonical physical spelling"
    );
    drop(discovered);
    git(root, &["submodule", "absorbgitdirs", "Café"])?;
    let git_file = root.join("Cafe\u{301}/.git");
    let backlink = std::fs::read(&git_file)?;
    std::fs::remove_file(&git_file)?;
    std::fs::write(root.join("Cafe\u{301}/tracked"), "changed\n")?;
    let repo = crate::open_opts(
        root,
        crate::open::Options::isolated().config_overrides(["core.precomposeUnicode=false"]),
    )?;
    let project = repo.monitor(Default::default())?;
    let mut monitor = project.worktree_status(&repo, Options::default())?;
    monitor.refresh(&repo, &AtomicBool::new(false))?;
    assert!(
        monitor.watches.children.is_empty(),
        "the initial checkout has no backlink"
    );
    for initialized in [true, false, true] {
        if initialized {
            std::fs::write(&git_file, &backlink)?;
        } else {
            std::fs::remove_file(&git_file)?;
        }
        invalidate_path(&mut monitor, "Cafe\u{301}/.git");
        assert!(monitor.submodules_dirty, "native backlink events rediscover submodules");
        let update = monitor.refresh(&repo, &AtomicBool::new(false))?;
        assert_eq!(
            update.unstaged,
            Some(Scope::All),
            "native aliases require full status to refresh the indexed spelling immediately"
        );
        assert_eq!(
            monitor.watches.children.len(),
            usize::from(initialized),
            "backlink changes reconcile native submodule coverage in the same refresh"
        );
        assert_eq!(
            changed_paths(&monitor).contains(&BString::from("Café")),
            initialized,
            "the indexed gitlink reflects dirty initialized checkouts without another event"
        );
    }
    Ok(())
}

fn paths(paths: impl IntoIterator<Item = BString>) -> Option<Scope> {
    Some(Scope::Paths(paths.into_iter().map(path_change).collect()))
}
