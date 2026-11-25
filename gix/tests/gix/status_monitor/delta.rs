use std::{collections::HashMap, path::Path, sync::atomic::AtomicBool};

use gix::{
    bstr::BString,
    notify::{Changes, PathChange, Scope},
    status::{
        Item, WorktreeStatusHandle,
        index_worktree::Item as WorktreeItem,
        monitor::{Change, Update},
    },
};
use gix_error::TestResult;

macro_rules! has_change {
    ($update:expr, $pattern:pat $(if $guard:expr)? $(,)?) => {
        ($update)
            .changes
            .iter()
            .any(|change| matches!(change, $pattern $(if $guard)?))
    };
}

fn fixture_rw() -> TestResult<gix_testtools::tempfile::TempDir> {
    Ok(gix_testtools::scripted_fixture_writable(
        "make_status_monitor_index.sh",
    )?)
}

fn open(root: &Path) -> gix::Result<gix::Repository> {
    gix::open_opts(root, gix::open::Options::isolated())
}

fn refresh(monitor: &mut WorktreeStatusHandle, root: &Path) -> TestResult<Update> {
    Ok(monitor.refresh(&open(root)?, &AtomicBool::new(false))?)
}

fn git(root: &Path, args: &[&str]) -> TestResult {
    let output = gix_testtools::git_command(root).args(args).output()?;
    assert!(
        output.status.success(),
        "Git changes the disposable delta fixture with {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn scope(paths: &[&str]) -> Option<Scope> {
    Some(Scope::Paths(
        paths
            .iter()
            .map(|path| PathChange {
                path: (*path).into(),
                kind: gix::notify::plumbing::EventKind::Any,
                path_kind: gix::notify::plumbing::PathKind::Any,
            })
            .collect(),
    ))
}

fn key(item: &Item) -> (u8, BString, u8) {
    let (side, origin) = match item {
        Item::TreeIndex(_) => (0, 0),
        Item::IndexWorktree(WorktreeItem::Modification { .. }) => (1, 0),
        Item::IndexWorktree(WorktreeItem::DirectoryContents { .. } | WorktreeItem::Rewrite { .. }) => (1, 1),
    };
    (side, item.location().to_owned(), origin)
}

/// Checks that applying `update.changes` to `previous` reproduces the monitor's current snapshot
/// exactly, including index metadata. Pass an empty slice for the first refresh.
///
/// A path can have multiple status items, so each item is identified by its side, path, and origin.
/// Deletions and replacements must match the old items exactly, and new items must not overwrite
/// any item that remains after those removals.
fn assert_delta_reconstructs(previous: &[Item], monitor: &WorktreeStatusHandle, update: &Update) {
    let mut items: HashMap<_, _> = previous.iter().map(|item| (key(item), item.clone())).collect();
    assert_eq!(
        items.len(),
        previous.len(),
        "the fixture has one status item per side, path and origin"
    );
    for change in &update.changes {
        let previous = match change {
            Change::Deletion(previous) | Change::Modification { previous, .. } => previous,
            Change::Addition(_) => continue,
        };
        let removed = items
            .remove(&key(previous))
            .expect("delta removals identify an existing previous item");
        assert_eq!(
            &removed, previous,
            "delta previous values preserve exact owned item metadata"
        );
    }
    for change in &update.changes {
        let current = match change {
            Change::Addition(current) | Change::Modification { current, .. } => current,
            Change::Deletion(_) => continue,
        };
        assert!(
            items.insert(key(current), current.clone()).is_none(),
            "delta additions identify vacant side, path and origin keys after all removals"
        );
    }
    let mut reconstructed: Vec<_> = items.into_values().collect();
    reconstructed.sort_by_key(key);
    assert_eq!(
        monitor.snapshot(),
        Some(reconstructed.as_slice()),
        "applying the delta reconstructs the complete snapshot including index metadata"
    );
}

#[test]
fn initial_snapshot_reports_every_owned_item_as_an_addition() -> TestResult {
    let fixture = fixture_rw()?;
    let root = fixture.path();
    std::fs::write(root.join("tracked"), "changed\n")?;
    std::fs::write(root.join("untracked"), "new\n")?;
    let (_project, mut monitor) = super::monitor(root)?;
    let update = refresh(&mut monitor, root)?;
    assert!(update.changed, "the first snapshot introduces status items");
    assert_eq!(
        update.changes.len(),
        2,
        "the modified and untracked paths are both introduced"
    );
    assert!(
        update
            .changes
            .iter()
            .all(|change| matches!(change, Change::Addition(_))),
        "there are no previous values before the initial snapshot"
    );
    assert_delta_reconstructs(&[], &monitor, &update);
    Ok(())
}

#[test]
fn one_refresh_reports_additions_deletions_and_modifications() -> TestResult {
    let fixture = fixture_rw()?;
    let root = fixture.path();
    std::fs::write(root.join("tracked"), "changed\n")?;
    std::fs::write(root.join("gone"), "untracked\n")?;
    let (_project, mut monitor) = super::monitor(root)?;
    refresh(&mut monitor, root)?;
    let previous = monitor.snapshot().expect("the initial snapshot exists").to_vec();

    std::fs::remove_file(root.join("tracked"))?;
    std::fs::remove_file(root.join("gone"))?;
    std::fs::write(root.join("new"), "untracked\n")?;
    monitor.invalidate_changes(&Changes {
        worktree: scope(&["gone", "new", "tracked"]),
        ..Default::default()
    });
    let update = refresh(&mut monitor, root)?;
    assert_eq!(
        update.changes.len(),
        3,
        "all three observed status transitions appear in this round"
    );
    assert!(
        has_change!(update, Change::Addition(item) if item.location() == "new"),
        "the newly untracked file adds a status item"
    );
    assert!(
        has_change!(update, Change::Deletion(item) if item.location() == "gone"),
        "removing an untracked file deletes its previous status item"
    );
    assert!(
        has_change!(
            update,
            Change::Modification { previous, current }
                if previous.location() == "tracked" && current.location() == "tracked"
        ),
        "the tracked path changes from modified to removed within the same status side"
    );
    assert_delta_reconstructs(&previous, &monitor, &update);
    Ok(())
}

#[test]
fn staging_a_path_deletes_the_worktree_item_and_adds_the_staged_item() -> TestResult {
    let fixture = fixture_rw()?;
    let root = fixture.path();
    std::fs::write(root.join("tracked"), "changed\n")?;
    let (_project, mut monitor) = super::monitor(root)?;
    refresh(&mut monitor, root)?;
    let previous = monitor.snapshot().expect("the initial snapshot exists").to_vec();

    git(root, &["add", "tracked"])?;
    monitor.invalidate_changes(&Changes {
        index: true,
        ..Default::default()
    });
    let update = refresh(&mut monitor, root)?;
    assert_eq!(
        update.changes.len(),
        2,
        "staged and worktree entries have independent keys"
    );
    assert!(
        has_change!(update, Change::Deletion(Item::IndexWorktree(_))),
        "staging deletes the previous worktree status item"
    );
    assert!(
        has_change!(update, Change::Addition(Item::TreeIndex(_))),
        "staging adds a separate HEAD-to-index status item"
    );
    assert_delta_reconstructs(&previous, &monitor, &update);
    Ok(())
}

#[test]
fn file_directory_transitions_preserve_both_worktree_items_at_the_same_path() -> TestResult {
    let fixture = fixture_rw()?;
    let root = fixture.path();
    std::fs::write(root.join("tracked"), "changed\n")?;
    let (_project, mut monitor) = super::monitor(root)?;
    refresh(&mut monitor, root)?;
    let previous = monitor.snapshot().expect("the initial snapshot exists").to_vec();

    std::fs::remove_file(root.join("tracked"))?;
    std::fs::create_dir(root.join("tracked"))?;
    std::fs::write(root.join("tracked/child"), "untracked\n")?;
    monitor.invalidate_changes(&Changes {
        worktree: scope(&["tracked"]),
        ..Default::default()
    });
    let update = refresh(&mut monitor, root)?;
    let snapshot = monitor.snapshot().expect("the directory snapshot exists");
    assert_eq!(
        snapshot.len(),
        2,
        "the removed tracked file and collapsed untracked directory occupy the same path"
    );
    assert!(
        snapshot.iter().all(|item| item.location() == "tracked"),
        "the two distinct worktree origins share their complete path"
    );
    assert!(
        matches!(
            update.changes.as_slice(),
            [
                Change::Modification {
                    previous: Item::IndexWorktree(WorktreeItem::Modification { .. }),
                    current: Item::IndexWorktree(WorktreeItem::Modification { .. }),
                },
                Change::Addition(Item::IndexWorktree(WorktreeItem::DirectoryContents { .. })),
            ]
        ),
        "the tracked origin changes classification while the directory origin is introduced"
    );
    assert_delta_reconstructs(&previous, &monitor, &update);
    let previous = snapshot.to_vec();

    std::fs::remove_dir_all(root.join("tracked"))?;
    std::fs::write(root.join("tracked"), "changed again\n")?;
    monitor.invalidate_changes(&Changes {
        worktree: scope(&["tracked"]),
        ..Default::default()
    });
    let update = refresh(&mut monitor, root)?;
    assert!(
        matches!(
            update.changes.as_slice(),
            [
                Change::Modification {
                    previous: Item::IndexWorktree(WorktreeItem::Modification { .. }),
                    current: Item::IndexWorktree(WorktreeItem::Modification { .. }),
                },
                Change::Deletion(Item::IndexWorktree(WorktreeItem::DirectoryContents { .. })),
            ]
        ),
        "restoring a file deletes only the directory origin and updates the tracked origin"
    );
    assert_eq!(
        monitor.snapshot().expect("the restored file snapshot exists").len(),
        1,
        "only the modified tracked file remains after restoring the file"
    );
    assert_delta_reconstructs(&previous, &monitor, &update);
    Ok(())
}

#[test]
fn stat_metadata_changes_report_exact_previous_and_current_items() -> TestResult {
    let fixture = fixture_rw()?;
    let root = fixture.path();
    std::fs::write(root.join("tracked"), "changed\n")?;
    let (_project, mut monitor) = super::monitor(root)?;
    refresh(&mut monitor, root)?;
    let previous = monitor.snapshot().expect("the initial snapshot exists").to_vec();

    let repo = open(root)?;
    let mut index = repo.open_index()?;
    index
        .entry_mut_by_path_and_stage("tracked".into(), gix::index::entry::Stage::Unconflicted)
        .expect("the fixture tracks this path")
        .stat
        .ctime
        .secs ^= 1;
    index.write(Default::default(), repo.refs.shared_repository_permissions)?;
    monitor.invalidate_changes(&Changes {
        index: true,
        ..Default::default()
    });
    let update = refresh(&mut monitor, root)?;
    assert!(!update.staged, "stat-only changes do not refresh staged contents");
    assert!(update.changed, "owned entry metadata is part of the exact snapshot");
    assert!(
        matches!(
            update.changes.as_slice(),
            [Change::Modification {
                previous: Item::IndexWorktree(WorktreeItem::Modification { entry: before, .. }),
                current: Item::IndexWorktree(WorktreeItem::Modification { entry: after, .. }),
            }] if before.stat != after.stat
        ),
        "the modification retains both distinct entry-stat values"
    );
    assert_delta_reconstructs(&previous, &monitor, &update);
    Ok(())
}

#[test]
fn clean_idle_refresh_has_no_delta() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_status_monitor_index.sh")?;
    let root = fixture.as_path();
    let (_project, mut monitor) = super::monitor(root)?;
    refresh(&mut monitor, root)?;
    let previous = monitor.snapshot().expect("the clean snapshot exists").to_vec();

    let update = refresh(&mut monitor, root)?;
    assert_eq!(
        update,
        Update::default(),
        "an idle refresh has no coverage, changed flag, or delta"
    );
    assert_delta_reconstructs(&previous, &monitor, &update);
    Ok(())
}

#[test]
fn repeated_modified_classifications_keep_coverage_without_an_item_delta() -> TestResult {
    let fixture = fixture_rw()?;
    let root = fixture.path();
    std::fs::write(root.join("tracked"), "change one\n")?;
    let (_project, mut monitor) = super::monitor(root)?;
    refresh(&mut monitor, root)?;
    let previous = monitor.snapshot().expect("the initial snapshot exists").to_vec();

    std::fs::write(root.join("tracked"), "change two\n")?;
    monitor.invalidate_changes(&Changes {
        worktree: scope(&["tracked"]),
        ..Default::default()
    });
    let update = refresh(&mut monitor, root)?;
    assert!(
        !update.changed,
        "same-sized content edits can leave owned status items equal"
    );
    assert!(
        update.changes.is_empty(),
        "equal status items do not produce an exact item delta"
    );
    assert_eq!(
        update.unstaged,
        scope(&["tracked"]),
        "content-derived caches still receive refresh coverage"
    );
    assert_delta_reconstructs(&previous, &monitor, &update);
    Ok(())
}
