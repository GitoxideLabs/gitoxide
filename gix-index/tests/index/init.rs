use gix_testtools::TestResult;
use std::path::Path;

use crate::{odb_at, scripted_fixture_read_only};
use gix_index::State;

#[test]
fn from_tree() -> gix_testtools::TestResult {
    let fixtures = [
        "make_index/v2.sh",
        "make_index/v2_more_files.sh",
        "make_index/v2_all_file_kinds.sh",
        "make_index/v4_more_files_IEOT.sh",
    ];

    for fixture in fixtures {
        let worktree_dir = scripted_fixture_read_only(fixture)?;

        let tree_id = tree_id(&worktree_dir);

        let git_dir = worktree_dir.join(".git");
        let expected_state = gix_index::File::at(
            git_dir.join("index"),
            gix_testtools::object_hash(),
            false,
            Default::default(),
        )?;
        let odb = odb_at(git_dir.join("objects"))?;
        let actual_state = State::from_tree(&tree_id, &odb, Default::default())?;

        compare_states(&actual_state, &expected_state, fixture);
        let cached_state =
            State::from_tree_with_cache(&tree_id, gix_object::find::Never, Default::default(), &expected_state)?;
        compare_states(&cached_state, &actual_state, fixture);
        assert!(
            cached_state.tree().is_none(),
            "tree construction doesn't copy extensions"
        );
        assert!(
            cached_state
                .entries()
                .iter()
                .all(|entry| entry.stat == Default::default()),
            "cached entries receive the same empty stat information as traversed entries"
        );
    }
    Ok(())
}

#[test]
fn cached_trees_fall_back_for_ineligible_entries_and_counts() -> TestResult {
    use gix_index::entry::{Flags, Mode};

    let root = scripted_fixture_read_only("make_index/v2.sh")?;
    let tree_id = tree_id(&root);
    let index = gix_index::File::at(
        root.join(".git/index"),
        gix_testtools::object_hash(),
        false,
        Default::default(),
    )?;
    let tree = index.tree().cloned();
    let odb = odb_at(root.join(".git/objects"))?;
    let expected = State::from_tree(&tree_id, &odb, Default::default())?;
    let assert_fallback = |index: &State| -> TestResult {
        assert!(
            State::from_tree_with_cache(&tree_id, gix_object::find::Never, Default::default(), index).is_err(),
            "an unusable cache must read the tree"
        );
        let actual = State::from_tree_with_cache(&tree_id, &odb, Default::default(), index)?;
        compare_states(&actual, &expected, "fallback");
        Ok(())
    };

    for flags in [Flags::STAGE_MASK, Flags::INTENT_TO_ADD, Flags::REMOVE] {
        let mut index = index.clone();
        index.entries_mut()[0].flags = flags;
        index.set_tree(tree.clone());
        assert_fallback(&index)?;
    }
    for mode in [Mode::DIR, Mode::empty()] {
        let mut index = index.clone();
        index.entries_mut()[0].mode = mode;
        index.set_tree(tree.clone());
        assert_fallback(&index)?;
    }
    for count in [None, Some(0), Some(2)] {
        let mut index = index.clone();
        index.tree_mut().expect("Git wrote a cache-tree").num_entries = count;
        assert_fallback(&index)?;
    }
    for path in ["/a", "a//b", "a/"] {
        let mut index = index.clone();
        let entry = index.remove_entry_at_index(0);
        index.dangerously_push_entry(entry.stat, entry.id, entry.flags, entry.mode, path.into());
        index.set_tree(tree.clone());
        assert_fallback(&index)?;
    }
    let mut index = index.clone();
    index.remove_tree();
    assert_fallback(&index)?;
    Ok(())
}

#[test]
fn cached_entries_discard_worktree_flags_and_validate_paths() -> TestResult {
    use gix_index::{
        entry::{Flags, Mode},
        validate::path::component::Error,
    };

    let root = scripted_fixture_read_only("make_index/v2.sh")?;
    let tree_id = tree_id(&root);
    let mut index = gix_index::File::at(
        root.join(".git/index"),
        gix_testtools::object_hash(),
        false,
        Default::default(),
    )?;
    let tree = index.tree().cloned();
    index.entries_mut()[0].flags = Flags::ASSUME_VALID | Flags::FSMONITOR_VALID | Flags::SKIP_WORKTREE;
    index.set_tree(tree.clone());
    let actual = State::from_tree_with_cache(&tree_id, gix_object::find::Never, Default::default(), &index)?;
    assert!(
        actual.entries()[0].flags.is_empty(),
        "tree entries have no worktree validity flags"
    );

    for (path, mode, symlink) in [(".gitmodules", Mode::SYMLINK, true), (".git/a", Mode::FILE, false)] {
        let mut index = index.clone();
        let entry = index.remove_entry_at_index(0);
        index.dangerously_push_entry(entry.stat, entry.id, Flags::empty(), mode, path.into());
        index.set_tree(tree.clone());
        let err = State::from_tree_with_cache(&tree_id, gix_object::find::Never, Default::default(), &index)
            .expect_err("cached paths must receive mode-specific component validation");
        assert!(err.is_validation(), "invalid cached paths remain validation errors");
        assert!(
            matches!(err.downcast_any_ref::<Error>(), Some(Error::SymlinkedGitModules)) == symlink,
            "symlink leaves receive their additional protection"
        );
        assert!(
            matches!(err.downcast_any_ref::<Error>(), Some(Error::DotGitDir)) != symlink,
            "directory components receive ordinary path protection"
        );
    }
    Ok(())
}

#[test]
fn sparse_cached_trees_fall_back_to_traversal() -> TestResult {
    let root = scripted_fixture_read_only("make_index/v3_sparse_index.sh")?;
    let index = gix_index::File::at(
        root.join(".git/index"),
        gix_testtools::object_hash(),
        false,
        Default::default(),
    )?;
    assert!(index.is_sparse(), "the fixture contains collapsed directory entries");
    let tree_id = index.tree().expect("Git retained the full tree identity").id;
    assert!(
        State::from_tree_with_cache(&tree_id, gix_object::find::Never, Default::default(), &index).is_err(),
        "sparse entries cannot substitute the full tree"
    );
    let odb = odb_at(root.join(".git/objects"))?;
    let expected = State::from_tree(&tree_id, &odb, Default::default())?;
    let actual = State::from_tree_with_cache(&tree_id, &odb, Default::default(), &index)?;
    compare_states(&actual, &expected, "sparse fallback");
    Ok(())
}

#[test]
fn from_tree_validation() -> TestResult {
    let mut error_snapshots = Vec::new();
    use gix_index::validate::path::component::Error;

    let root = scripted_fixture_read_only("make_traverse_literal_separators.sh")?;
    for repo_name in [
        "traverse_dotdot_slashes",
        "traverse_dotgit_slashes",
        "traverse_dotgit_backslashes",
        "traverse_dotdot_backslashes",
    ] {
        let worktree_dir = root.join(repo_name);
        let tree_id = tree_id(&worktree_dir);
        let git_dir = worktree_dir.join(".git");
        let odb = odb_at(git_dir.join("objects"))?;

        let err = State::from_tree(&tree_id, &odb, Default::default())
            .expect_err("tree entries with path separators must fail validation");
        error_snapshots.push(gix_testtools::redact_debug_snapshot(&(err), &[]));
        assert!(err.is_validation(), "invalid path components are validation errors");
        assert!(
            matches!(err.downcast_any_ref::<Error>(), Some(Error::PathSeparator)),
            "the concrete path-separator violation remains available beneath the index context"
        );
        assert!(
            matches!(err.probable_cause().downcast_ref::<Error>(), Some(Error::PathSeparator)),
            "conversion selects the concrete path violation rather than its classification marker"
        );
    }
    insta::assert_debug_snapshot!(error_snapshots, "from tree validation", @r#"
    [
        The path "../outside" is invalid
        
        Caused by:
            0: Path separators like / or \ are not allowed,
        The path ".git/hooks/pre-commit" is invalid
        
        Caused by:
            0: Path separators like / or \ are not allowed,
        The path ".git\hooks\pre-commit" is invalid
        
        Caused by:
            0: Path separators like / or \ are not allowed,
        The path "..\outside" is invalid
        
        Caused by:
            0: Path separators like / or \ are not allowed,
    ]
    "#);
    Ok(())
}

#[test]
fn from_tree_returns_file_directory_conflicts_until_fixed() -> gix_testtools::TestResult {
    let worktree_dir = scripted_fixture_read_only("make_symlink_prefix_reuse_advisory.sh")?;
    let tree_id = tree_id(&worktree_dir);
    let odb = odb_at(worktree_dir.join(".git").join("objects"))?;

    let actual_state = State::from_tree(&tree_id, &odb, Default::default())?;
    actual_state.verify_entries()?;

    let paths: Vec<_> = actual_state
        .entries()
        .iter()
        .map(|entry| entry.path(&actual_state).to_owned())
        .collect();
    assert_eq!(
        paths,
        ["a", "a/post-checkout", "payload"],
        "from_tree currently returns malformed file/directory conflicts; update this expected unfixed state once fixed"
    );
    Ok(())
}

#[test]
fn new() {
    let state = State::new(gix_hash::Kind::Sha1);
    assert_eq!(state.entries().len(), 0);
    assert_eq!(state.version(), gix_index::Version::V2);
    assert_eq!(state.object_hash(), gix_hash::Kind::Sha1);
}

fn compare_states(actual: &State, expected: &State, fixture: &str) {
    actual.verify_entries().expect("valid");
    actual.verify_extensions(false, gix_object::find::Never).expect("valid");

    assert_eq!(
        actual.entries().len(),
        expected.entries().len(),
        "entry count mismatch in {fixture:?}",
    );

    for (a, e) in actual.entries().iter().zip(expected.entries()) {
        assert_eq!(a.id, e.id, "entry id mismatch in {fixture:?}");
        assert_eq!(a.flags, e.flags, "entry flags mismatch in {fixture:?}");
        assert_eq!(a.mode, e.mode, "entry mode mismatch in {fixture:?}");
        assert_eq!(a.path(actual), e.path(expected), "entry path mismatch in {fixture:?}");
    }
}

fn tree_id(root: &Path) -> gix_hash::ObjectId {
    let hex_hash =
        std::fs::read_to_string(root.join("head.tree")).expect("head.tree was created by git rev-parse @^{tree}");
    hex_hash.trim().parse().expect("valid hash")
}
