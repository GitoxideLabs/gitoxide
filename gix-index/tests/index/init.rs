use std::{error::Error, path::Path};

use crate::scripted_fixture_read_only;
use gix_index::State;

#[test]
fn from_tree() -> crate::Result {
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
        let expected_state =
            gix_index::File::at(git_dir.join("index"), gix_hash::Kind::Sha1, false, Default::default())?;
        let odb = gix_odb::at(git_dir.join("objects"))?;
        let actual_state = State::from_tree(&tree_id, &odb, Default::default())?;

        compare_states(&actual_state, &expected_state, fixture);
    }
    Ok(())
}

#[test]
fn from_tree_validation() -> crate::Result {
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
        let odb = gix_odb::at(git_dir.join("objects"))?;

        let err = State::from_tree(&tree_id, &odb, Default::default()).unwrap_err();
        assert_eq!(
            err.source().expect("inner").to_string(),
            r"Path separators like / or \ are not allowed",
            r"Note that this effectively tests what would happen on Windows, where \ also isn't allowed"
        );
    }
    Ok(())
}

#[test]
fn from_tree_returns_file_directory_conflicts_until_fixed() -> crate::Result {
    let worktree_dir = scripted_fixture_read_only("make_symlink_prefix_reuse_advisory.sh")?;
    let tree_id = tree_id(&worktree_dir);
    let odb = gix_odb::at(worktree_dir.join(".git").join("objects"))?;

    let actual_state = State::from_tree(&tree_id, &odb, Default::default())?;
    actual_state
        .verify_entries()
        .expect("valid, even though invariants aren't met");

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
