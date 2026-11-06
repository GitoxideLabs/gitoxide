use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use gix::status::tree_index::{Outcome, TrackRenames};
use gix_diff::index::{Action, Change};
use gix_testtools::TestResult;

struct CountTreeReads(Arc<AtomicUsize>);

impl gix_pack::cache::Object for CountTreeReads {
    fn put(&mut self, _id: gix_hash::ObjectId, kind: gix_object::Kind, _data: &[u8]) {
        if kind == gix_object::Kind::Tree {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn get(&mut self, _id: &gix_hash::ObjectId, _out: &mut Vec<u8>) -> Option<gix_object::Kind> {
        None
    }
}

fn count_tree_reads(repo: &mut gix::Repository) -> Arc<AtomicUsize> {
    let count = Arc::new(AtomicUsize::new(0));
    let cache_count = Arc::clone(&count);
    repo.objects
        .set_object_cache(move || Box::new(CountTreeReads(Arc::clone(&cache_count))));
    count
}

fn assert_same_entries(actual: &gix_index::State, expected: &gix_index::State) {
    assert_eq!(
        actual.entries().len(),
        expected.entries().len(),
        "the entire tree index is retained"
    );
    for (actual_entry, expected_entry) in actual.entries().iter().zip(expected.entries()) {
        assert_eq!(
            actual_entry.path(actual),
            expected_entry.path(expected),
            "entry positions preserve tree paths"
        );
        assert_eq!(
            actual_entry.id, expected_entry.id,
            "entry positions preserve object IDs"
        );
        assert_eq!(actual_entry.mode, expected_entry.mode, "entry modes describe the tree");
        assert_eq!(
            actual_entry.flags, expected_entry.flags,
            "worktree flags do not leak into the tree index"
        );
        assert_eq!(
            actual_entry.stat, expected_entry.stat,
            "worktree stat data does not leak into the tree index"
        );
    }
}

fn changes(
    repo: &gix::Repository,
    tree_id: &gix_hash::oid,
    index: &gix_index::State,
    expected_tree_index: &gix_index::State,
) -> gix::Result<(Vec<Change>, Outcome)> {
    let mut changes = Vec::new();
    let outcome = repo.tree_index_status(
        tree_id,
        index,
        None,
        TrackRenames::Disabled,
        |change, tree_index, worktree_index| {
            assert_same_entries(tree_index, expected_tree_index);
            assert_same_entries(worktree_index, index);
            changes.push(change.into_owned());
            Ok(Action::Continue(()))
        },
    )?;
    assert_same_entries(&outcome.tree_index, expected_tree_index);
    Ok((changes, outcome))
}

#[test]
fn matching_root_cache_tree_avoids_object_reads() -> TestResult {
    let mut repo = super::repo("untracked-only")?;
    let index = repo.open_index()?;
    let tree_id = repo.head_tree_id()?.detach();
    let expected = repo.index_from_tree(&tree_id)?;
    let cached_tree = index.tree().expect("Git commits create a cache tree");
    assert_eq!(cached_tree.id, tree_id, "the entire index matches HEAD");
    assert_eq!(cached_tree.num_entries, Some(index.entries().len() as u32));
    let reads = count_tree_reads(&mut repo);

    let (actual, _) = changes(&repo, &tree_id, &index, &expected)?;
    assert!(
        actual.is_empty(),
        "the tracked index is clean despite untracked worktree files"
    );
    assert_eq!(
        reads.load(Ordering::Relaxed),
        0,
        "a matching root needs no tree object reads"
    );

    let mut uncached = index.clone();
    uncached.remove_tree();
    let (baseline, _) = changes(&repo, &tree_id, &uncached, &expected)?;
    assert_eq!(actual, baseline, "cached and uncached status agree");
    assert_eq!(
        reads.load(Ordering::Relaxed),
        2,
        "without TREE both root and subtree are read"
    );
    Ok(())
}

#[test]
fn invalid_root_still_reuses_matching_subtree() -> TestResult {
    let mut repo = super::repo("index-changed-outside-subdir")?;
    let index = repo.open_index()?;
    let tree_id = repo.head_tree_id()?.detach();
    let expected = repo.index_from_tree(&tree_id)?;
    let cached_tree = index.tree().expect("Git preserves cache trees when staging changes");
    assert_eq!(cached_tree.num_entries, None, "staging invalidates the root");
    assert_eq!(cached_tree.children.len(), 1, "the unchanged directory remains cached");
    assert_eq!(cached_tree.children[0].num_entries, Some(1));
    let reads = count_tree_reads(&mut repo);

    let (actual, _) = changes(&repo, &tree_id, &index, &expected)?;
    assert!(matches!(actual.as_slice(), [Change::Modification { location, .. }] if location.as_ref() == "file"));
    assert_eq!(reads.load(Ordering::Relaxed), 1, "only the changed root tree is read");

    let mut uncached = index.clone();
    uncached.remove_tree();
    reads.store(0, Ordering::Relaxed);
    let (baseline, _) = changes(&repo, &tree_id, &uncached, &expected)?;
    assert_eq!(
        actual, baseline,
        "cached subtrees do not change callback indices or staged changes"
    );
    assert_eq!(
        reads.load(Ordering::Relaxed),
        2,
        "the baseline traverses the unchanged subtree"
    );
    Ok(())
}

#[test]
fn malformed_cache_counts_and_ids_fall_back_to_tree_objects() -> TestResult {
    let mut repo = super::repo("untracked-only")?;
    let index = repo.open_index()?;
    let tree_id = repo.head_tree_id()?.detach();
    let expected = repo.index_from_tree(&tree_id)?;
    let reads = count_tree_reads(&mut repo);
    for invalidate_count in [false, true] {
        let mut index = index.clone();
        let cached_tree = index.tree_mut().expect("the fixture has a root cache tree");
        if invalidate_count {
            cached_tree.num_entries = Some(0);
        } else {
            cached_tree.id = repo.object_hash().null();
        }
        reads.store(0, Ordering::Relaxed);
        let (actual, _) = changes(&repo, &tree_id, &index, &expected)?;
        assert!(actual.is_empty(), "invalid cache metadata cannot invent staged changes");
        assert!(
            reads.load(Ordering::Relaxed) > 0,
            "an unusable root must be read from the object database"
        );
    }
    Ok(())
}

#[test]
fn changing_entries_in_memory_cannot_leave_a_trusted_cache_tree() -> TestResult {
    let repo = super::repo("untracked-only")?;
    let index = repo.open_index()?;
    let tree_id = repo.head_tree_id()?;
    let expected = repo.index_from_tree(&tree_id)?;
    assert_eq!(index.tree().expect("the fixture has a root cache tree").id, tree_id);
    for change_mode in [false, true] {
        let mut index = index.clone();
        let entry = &mut index.entries_mut()[0];
        if change_mode {
            entry.mode = gix_index::entry::Mode::FILE_EXECUTABLE;
        } else {
            entry.id = repo.object_hash().null();
        }
        let (actual, _) = changes(&repo, &tree_id, &index, &expected)?;
        assert!(
            matches!(actual.as_slice(), [Change::Modification { .. }]),
            "mutated entries are compared with HEAD"
        );
    }
    Ok(())
}

#[test]
fn pathspec_and_rename_results_match_uncached_comparisons() -> TestResult {
    for fixture in ["index-changed-outside-subdir", "git-mv"] {
        let repo = super::repo(fixture)?;
        let index = repo.open_index()?;
        let mut uncached = index.clone();
        uncached.remove_tree();
        let tree_id = repo.head_tree_id()?;
        let expected = repo.index_from_tree(&tree_id)?;
        for pattern in [None, Some("subdir"), Some("file")] {
            let mut results = Vec::new();
            for index in [&index, &uncached] {
                let mut pathspec = repo.pathspec(
                    false,
                    pattern,
                    false,
                    index,
                    gix_worktree::stack::state::attributes::Source::IdMapping,
                )?;
                let mut changes = Vec::new();
                let outcome = repo.tree_index_status(
                    &tree_id,
                    index,
                    Some(&mut pathspec),
                    TrackRenames::Given(gix_diff::Rewrites {
                        percentage: None,
                        ..Default::default()
                    }),
                    |change, tree_index, _| {
                        assert_same_entries(tree_index, &expected);
                        changes.push(change.into_owned());
                        Ok(Action::Continue(()))
                    },
                )?;
                assert_same_entries(&outcome.tree_index, &expected);
                results.push(changes);
            }
            assert_eq!(
                results[0], results[1],
                "cache reuse preserves {fixture} with {pattern:?}"
            );
            if fixture == "git-mv" && pattern.is_none() {
                assert!(
                    matches!(results[0].as_slice(), [Change::Rewrite { copy: false, .. }]),
                    "exact renames still combine deletion and addition"
                );
            }
            if pattern == Some("subdir") {
                assert!(
                    results[0].is_empty(),
                    "a clean subtree excludes staged changes elsewhere"
                );
            }
        }
    }
    Ok(())
}
