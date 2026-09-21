use super::*;
use std::{fmt::Write, path::Path};

fn git(root: &Path, args: &[&str]) -> gix_testtools::Result {
    let output = gix_testtools::git_command(root).args(args).output()?;
    assert!(
        output.status.success(),
        "git {args:?} succeeds in the disposable fixture: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn initialize(root: &Path) -> gix_testtools::Result {
    std::fs::create_dir_all(root)?;
    git(root, &["init", "--quiet", "--initial-branch=main"])?;
    std::fs::write(root.join("tracked"), "initial\n")?;
    git(root, &["add", "tracked"])?;
    git(root, &["commit", "--quiet", "-m", "initial"])
}

fn fixture() -> gix_testtools::Result<gix_testtools::tempfile::TempDir> {
    let root = gix_testtools::tempfile::tempdir()?;
    initialize(root.path())?;
    Ok(root)
}

fn open(root: &Path) -> Result<Repository, crate::open::Error> {
    crate::open_opts(root, crate::open::Options::isolated())
}

fn monitor(root: &Path) -> gix_testtools::Result<Monitor> {
    Ok(Monitor::new(
        &open(root)?,
        Options {
            repository: crate::notify::Options {
                safety_interval: None,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .map_err(gix_error::Exn::into_error)?)
}

fn refresh(monitor: &mut Monitor, root: &Path) -> gix_testtools::Result<Update> {
    Ok(monitor
        .refresh(&open(root)?, &AtomicBool::new(false))
        .map_err(gix_error::Exn::into_error)?)
}

fn changed_paths(monitor: &Monitor) -> Vec<BString> {
    monitor
        .snapshot()
        .expect("a successful refresh created a snapshot")
        .iter()
        .map(|item| item.location().to_owned())
        .collect()
}

fn invalidate_path(monitor: &mut Monitor, path: impl Into<BString>) {
    monitor.invalidate_changes(&Changes {
        worktree: Scope::Paths(vec![path.into()]),
        ..Default::default()
    });
}

#[test]
fn full_and_scoped_snapshots_match_fresh_status() -> gix_testtools::Result {
    let fixture = fixture()?;
    let root = fixture.path();
    let mut monitor = monitor(root)?;
    assert!(
        monitor.snapshot().is_none() && monitor.is_dirty(),
        "the initial baseline is pending"
    );
    let initial = refresh(&mut monitor, root)?;
    assert!(
        initial.changed && initial.staged && initial.unstaged == Scope::All,
        "the first refresh establishes complete coverage"
    );
    assert!(
        !monitor.is_dirty(),
        "successful collection clears only completed invalidations"
    );
    std::fs::write(root.join("tracked"), "modified\n")?;
    invalidate_path(&mut monitor, "tracked");
    let update = refresh(&mut monitor, root)?;
    assert_eq!(
        update.unstaged,
        Scope::Paths(vec!["tracked".into()]),
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
fn equal_items_still_report_content_refresh_coverage() -> gix_testtools::Result {
    let fixture = fixture()?;
    let root = fixture.path();
    let mut monitor = monitor(root)?;
    std::fs::write(root.join("tracked"), "change one\n")?;
    refresh(&mut monitor, root)?;
    std::fs::write(root.join("tracked"), "change two\n")?;
    invalidate_path(&mut monitor, "tracked");
    let update = refresh(&mut monitor, root)?;
    assert_eq!(
        update.unstaged,
        Scope::Paths(vec!["tracked".into()]),
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
fn collapsed_untracked_scopes_expand_to_their_top_level_ancestor() -> gix_testtools::Result {
    let fixture = fixture()?;
    let root = fixture.path();
    std::fs::create_dir_all(root.join("new/deep"))?;
    std::fs::write(root.join("new/deep/file"), "new\n")?;
    let mut monitor = monitor(root)?;
    refresh(&mut monitor, root)?;
    std::fs::remove_file(root.join("new/deep/file"))?;
    invalidate_path(&mut monitor, "new/deep/file");
    let update = refresh(&mut monitor, root)?;
    assert_eq!(
        update.unstaged,
        Scope::Paths(vec!["new".into()]),
        "a child event can change whether its untracked ancestor collapses"
    );
    assert!(
        changed_paths(&monitor).is_empty(),
        "an emptied untracked directory disappears from the cached snapshot"
    );
    Ok(())
}

#[test]
fn ignored_nested_targets_remain_ignored_during_scoped_refresh() -> gix_testtools::Result {
    let fixture = fixture()?;
    let root = fixture.path();
    std::fs::create_dir_all(root.join("project/fuzz/target/deep/out"))?;
    std::fs::write(root.join(".gitignore"), "target/\n")?;
    std::fs::write(root.join("project/.gitignore"), "!out/\n")?;
    git(root, &["add", ".gitignore", "project/.gitignore"])?;
    git(root, &["commit", "--quiet", "-m", "ignore targets"])?;
    let mut monitor = monitor(root)?;
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
fn unrelated_refs_do_not_recompute_staged_or_worktree_status() -> gix_testtools::Result {
    let started = Instant::now();
    let phase = |phase| eprintln!("unrelated_refs ({:?}): {phase}", started.elapsed());
    phase("create fixture");
    let fixture = fixture()?;
    let root = fixture.path();
    phase("create monitor");
    let mut monitor = monitor(root)?;
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
fn failures_and_interruptions_preserve_snapshot_and_pending_work() -> gix_testtools::Result {
    let fixture = fixture()?;
    let root = fixture.path();
    let mut monitor = monitor(root)?;
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
    assert!(monitor.is_dirty(), "cancelled scopes remain pending");
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
    assert!(monitor.is_dirty(), "failed refreshes are retryable");
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
fn invalidation_bounds_and_literal_scopes_are_preserved() -> gix_testtools::Result {
    let fixture = fixture()?;
    let root = fixture.path();
    std::fs::write(root.join("literal[glob]"), "old\n")?;
    git(root, &["add", "literal[glob]"])?;
    git(root, &["commit", "--quiet", "-m", "literal"])?;
    let mut monitor = monitor(root)?;
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
    assert_eq!(
        monitor
            .effective_scope(&open(root)?)
            .map_err(gix_error::Exn::into_error)?,
        Scope::All,
        "invalid relative scopes fall back to a root scan"
    );
    monitor.unstaged = Scope::None;
    for number in 0..5000 {
        invalidate_path(&mut monitor, format!("file-{number}"));
    }
    assert_eq!(
        monitor.unstaged,
        Scope::All,
        "a busy application cannot accumulate unbounded scope storage"
    );
    Ok(())
}

fn add_submodule(parent: &Path, name: &str) -> gix_testtools::Result {
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
fn initialized_submodules_are_live_recursive_and_obey_ignore_modes() -> gix_testtools::Result {
    let fixture = fixture()?;
    let root = fixture.path();
    add_submodule(root, "child")?;
    add_submodule(&root.join("child"), "grandchild")?;
    git(root, &["add", "child"])?;
    git(root, &["commit", "--quiet", "-m", "update child"])?;
    let mut monitor = monitor(root)?;
    refresh(&mut monitor, root)?;
    assert_eq!(
        monitor.children.len(),
        2,
        "initialized nested modules each have a detached monitor"
    );
    assert_eq!(
        monitor
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
        monitor.children.len(),
        1,
        "ignore=dirty watches child HEAD but needs no descendant worktree watches"
    );
    assert!(
        !monitor
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
        monitor.children.is_empty(),
        "ignore=all releases the child's native resources"
    );
    Ok(())
}

#[test]
fn submodule_initialization_and_removal_update_the_inventory() -> gix_testtools::Result {
    let fixture = fixture()?;
    let root = fixture.path();
    add_submodule(root, "child")?;
    // Toggle the checkout's backlink while keeping native watches live. Moving a directory
    // containing active metadata watches can be denied on Windows.
    git(root, &["submodule", "absorbgitdirs", "child"])?;
    let git_file = root.join("child/.git");
    let backlink = std::fs::read(&git_file)?;
    std::fs::remove_file(&git_file)?;
    let mut monitor = monitor(root)?;
    refresh(&mut monitor, root)?;
    assert!(
        monitor.children.is_empty(),
        "uninitialized modules do not create failing worktree watchers"
    );
    std::fs::write(&git_file, backlink)?;
    invalidate_path(&mut monitor, "child");
    refresh(&mut monitor, root)?;
    assert_eq!(
        monitor.children.len(),
        1,
        "a newly initialized configured module gets live coverage"
    );
    std::fs::remove_file(&git_file)?;
    invalidate_path(&mut monitor, "child");
    refresh(&mut monitor, root)?;
    assert!(
        monitor.children.is_empty(),
        "deinitialization removes stale native coverage"
    );
    Ok(())
}

#[test]
fn disable_reenable_and_reconfigure_keep_snapshot_ownership_explicit() -> gix_testtools::Result {
    let started = Instant::now();
    let phase = |phase| eprintln!("disable_reenable_and_reconfigure ({:?}): {phase}", started.elapsed());
    phase("create first fixture");
    let fixture = fixture()?;
    phase("create second fixture");
    let other = self::fixture()?;
    let root = fixture.path();
    phase("add submodule");
    add_submodule(root, "child")?;
    phase("create monitor");
    let mut monitor = monitor(root)?;
    phase("initial refresh");
    refresh(&mut monitor, root)?;
    phase("disable worktree monitoring");
    monitor.set_worktree_enabled(false);
    assert!(
        monitor.children.is_empty(),
        "disabling worktree monitoring releases descendants immediately"
    );
    phase("reenable worktree monitoring");
    monitor.set_worktree_enabled(true);
    phase("refresh after reenabling");
    refresh(&mut monitor, root)?;
    assert_eq!(
        monitor.children.len(),
        1,
        "reenabling rediscovers initialized descendants"
    );
    phase("reconfigure for second fixture");
    monitor
        .reconfigure(&open(other.path())?)
        .map_err(gix_error::Exn::into_error)?;
    assert!(
        monitor.snapshot().is_none() && monitor.children.is_empty(),
        "switching repositories cannot expose the old snapshot"
    );
    phase("refresh second fixture");
    refresh(&mut monitor, other.path())?;
    assert!(
        !monitor.is_dirty(),
        "the replacement repository obtains its own baseline"
    );
    phase("drop monitor");
    drop(monitor);
    phase("close second fixture");
    if let Err(err) = other.close() {
        eprintln!("disable_reenable_and_reconfigure: second fixture cleanup failed: {err}");
    }
    phase("close first fixture");
    if let Err(err) = fixture.close() {
        eprintln!("disable_reenable_and_reconfigure: first fixture cleanup failed: {err}");
    }
    phase("finished");
    Ok(())
}

#[test]
fn metadata_only_monitoring_does_not_schedule_unconsumed_status() -> gix_testtools::Result {
    let fixture = fixture()?;
    let root = fixture.path();
    let mut monitor = Monitor::new(
        &open(root)?,
        Options {
            repository: crate::notify::Options {
                worktree: false,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .map_err(gix_error::Exn::into_error)?;
    let now = Instant::now();
    let outcome = monitor.service(now, || open(root).or_erased());
    assert!(outcome.errors.is_empty(), "metadata coverage starts successfully");
    let outcome = monitor.service(now, || open(root).or_erased());
    assert!(
        outcome.errors.is_empty(),
        "the installed inventory verifies successfully"
    );
    assert!(monitor.is_dirty(), "the unused status baseline remains pending");
    assert_eq!(
        monitor.next_timeout(now),
        monitor.repository_monitor().next_timeout(now),
        "pending status adds no deadline while worktree monitoring is disabled"
    );
    assert!(
        monitor.next_timeout(now).is_some_and(|timeout| !timeout.is_zero()),
        "metadata polling continues without spinning on unconsumed status"
    );
    monitor.retry = Some(now);
    assert_eq!(
        monitor.next_timeout(now),
        monitor.repository_monitor().next_timeout(now),
        "a status retry likewise stays suspended"
    );
    let outcome = monitor.service(now + Duration::from_secs(61), || open(root).or_erased());
    assert!(outcome.changes.references, "the metadata safety refresh remains active");
    monitor.set_worktree_enabled(true);
    assert_eq!(
        monitor.next_timeout(now + Duration::from_secs(61)),
        Some(Duration::ZERO),
        "reenabling worktree monitoring resumes the pending baseline immediately"
    );
    let now = now + Duration::from_secs(61);
    monitor.service(now, || open(root).or_erased());
    monitor.service(now, || open(root).or_erased());
    assert_eq!(
        monitor.next_service_timeout(now),
        monitor.repository_monitor().next_timeout(now),
        "a temporarily unavailable status consumer can schedule notification service independently"
    );
    assert!(
        monitor
            .next_service_timeout(now)
            .is_some_and(|timeout| !timeout.is_zero()),
        "pending status does not make service-only scheduling spin"
    );
    Ok(())
}

#[test]
fn monitoring_recovery_requires_successful_retry_and_inventory_verification() -> gix_testtools::Result {
    let fixture = fixture()?;
    let root = fixture.path();
    let mut monitor = monitor(root)?;
    assert!(!monitor.is_healthy(), "uninstalled subscriptions are not healthy");
    refresh(&mut monitor, root)?;
    monitor.service(Instant::now(), || open(root).or_erased());
    refresh(&mut monitor, root)?;
    assert!(
        monitor.is_healthy(),
        "the baseline and subscription inventory are verified"
    );

    monitor.rescan();
    let now = Instant::now();
    let failed = monitor.service(now, || {
        Err(message("injected repository reopen failure").raise().erased())
    });
    assert!(!failed.errors.is_empty(), "the registration retry reports its failure");
    assert!(!monitor.is_healthy(), "failure remains visible while retry is pending");
    let quiet = monitor.service(now + Duration::from_millis(1), || {
        panic!("a quiet service call must wait for the retry deadline")
    });
    assert!(
        quiet.errors.is_empty(),
        "pending retries need not repeat their diagnostic"
    );
    assert!(!monitor.is_healthy(), "silence is not a positive recovery signal");
    let later = now + Duration::from_secs(6);
    let recovered = monitor.service(later, || open(root).or_erased());
    assert!(
        recovered.errors.is_empty(),
        "the repository can be reopened after repair"
    );
    assert!(
        !monitor.is_healthy(),
        "replacement subscriptions still need their registration verification and submodule reconciliation"
    );
    monitor.service(later, || open(root).or_erased());
    refresh(&mut monitor, root)?;
    assert!(
        monitor.is_healthy(),
        "successful verification and reconciliation certify recovery"
    );
    Ok(())
}

#[test]
fn descendant_scope_mapping_is_component_aware() {
    let modules = vec![BString::from("child/nested"), BString::from("childish/other")];
    assert!(
        scope_touches_modules_at(&Scope::Paths(vec!["nested".into()]), &"child".into(), &modules),
        "nested topology refreshes discovery"
    );
    assert!(
        !scope_touches_modules_at(&Scope::Paths(vec!["source/file".into()]), &"child".into(), &modules),
        "ordinary child files do not rediscover the submodule graph"
    );
    assert!(
        !scope_touches_modules(&Scope::Paths(vec!["childishness".into()]), &modules),
        "byte prefixes do not conflate distinct path components"
    );
}

#[test]
fn child_service_is_fair_bounded_and_does_not_postpone_an_idle_deadline() -> gix_testtools::Result {
    let fixture = fixture()?;
    let root = fixture.path();
    for number in 0..CHILDREN_PER_SERVICE + 2 {
        add_submodule(root, &format!("child-{number}"))?;
    }
    let mut monitor = monitor(root)?;
    refresh(&mut monitor, root)?;
    let now = Instant::now();
    monitor.service(now, || open(root).or_erased());
    assert_eq!(
        monitor.child_round_remaining, 2,
        "one service call visits at most its fixed child budget"
    );
    assert_eq!(
        monitor.next_timeout(now),
        Some(Duration::ZERO),
        "unvisited children promptly continue the current round"
    );
    monitor.service(now, || open(root).or_erased());
    assert_eq!(
        monitor.child_round_remaining, 0,
        "the next call fairly visits the remainder"
    );
    let deadline = monitor.child_poll.expect("initialized children retain a poll deadline");
    let cursor = monitor.child_cursor.clone();
    if deadline > now {
        monitor.service(now, || open(root).or_erased());
        assert_eq!(
            monitor.child_poll,
            Some(deadline),
            "unrelated application activity cannot keep postponing child polling"
        );
        assert_eq!(
            monitor.child_cursor, cursor,
            "a completed round does not immediately spin over idle children"
        );
    }
    for child in monitor.children.values_mut() {
        std::fs::remove_file(child.monitor.layout().git_dir.join("HEAD"))?;
        child.monitor.rescan();
    }
    monitor.child_round_remaining = monitor.children.len();
    let failures = monitor.service(Instant::now(), || open(root).or_erased());
    assert_eq!(
        failures.errors.len(),
        MAX_ERRORS,
        "simultaneous child failures retain bounded representative diagnostics plus an omission summary"
    );
    assert!(
        !failures.changes.worktree.is_none(),
        "truncating diagnostics never hides invalidations"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn physical_submodule_aliases_share_coverage_and_request_full_root_invalidation() -> gix_testtools::Result {
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
    let discovered = submodules::discover(&open(root)?, &Options::default(), &AtomicBool::new(false))
        .map_err(gix_error::Exn::into_error)?;
    assert_eq!(
        discovered.children.len(),
        1,
        "aliases of one physical repository do not duplicate native streams"
    );
    assert!(
        discovered.aliased,
        "shared coverage conservatively invalidates every root gitlink"
    );
    let mut monitor = monitor(root)?;
    refresh(&mut monitor, root)?;
    let removed = monitor
        .children
        .values()
        .next()
        .expect("one physical child is monitored")
        .top_level
        .to_string();
    let retained = if removed == "child" { "alias" } else { "child" };
    std::fs::write(
        root.join(".gitmodules"),
        format!("[submodule \"{retained}\"]\npath = {retained}\nurl = ./unused\n"),
    )?;
    git(root, &["update-index", "--force-remove", &removed])?;
    monitor.invalidate_changes(&Changes {
        index: true,
        attributes: true,
        ..Default::default()
    });
    refresh(&mut monitor, root)?;
    let child = monitor.children.values().next().expect("the survivor retains coverage");
    assert_eq!(
        child.mount.as_bstr(),
        retained.as_bytes().as_bstr(),
        "shared physical coverage adopts the surviving logical mount"
    );
    assert_eq!(
        child.top_level.as_bstr(),
        retained.as_bytes().as_bstr(),
        "future child events invalidate the surviving root gitlink"
    );
    assert!(
        !monitor.aliased_children,
        "single ownership restores scoped invalidation"
    );
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn submodule_identity_uses_os_spelling_even_when_configuration_names_a_unicode_alias() -> gix_testtools::Result {
    let fixture = fixture()?;
    let root = fixture.path();
    add_submodule(root, "Café")?;
    std::fs::rename(root.join("Café"), root.join("temporary-child"))?;
    std::fs::rename(root.join("temporary-child"), root.join("Cafe\u{301}"))?;
    let repo = crate::open_opts(
        root,
        crate::open::Options::isolated().config_overrides(["core.precomposeUnicode=false"]),
    )?;
    let discovered = submodules::discover(&repo, &Options::default(), &AtomicBool::new(false))
        .map_err(gix_error::Exn::into_error)?;
    assert_eq!(
        discovered.children.keys().collect::<Vec<_>>(),
        vec![&root.join("Cafe\u{301}/.git").canonicalize()?],
        "cycle detection and shared-repository identity use the canonical physical spelling"
    );
    Ok(())
}
