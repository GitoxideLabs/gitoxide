use std::{cell::RefCell, collections::HashSet, error::Error, io::Read, path::Path};

use bstr::ByteSlice;
use gix_index::State;
use gix_testtools::scripted_fixture_read_only_standalone;

#[test]
fn from_tree() -> crate::Result {
    let fixtures = [
        "make_index/v2.sh",
        "make_index/v2_more_files.sh",
        "make_index/v2_all_file_kinds.sh",
        "make_index/v4_more_files_IEOT.sh",
    ];

    for fixture in fixtures {
        let worktree_dir = scripted_fixture_read_only_standalone(fixture)?;

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
    let root = scripted_fixture_read_only_standalone("make_traverse_literal_separators.sh")?;
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
    let worktree_dir = scripted_fixture_read_only_standalone("make_symlink_prefix_reuse_advisory.sh")?;
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
fn to_tree_roundtrips_to_fixture_tree() -> crate::Result {
    let fixtures = [
        "make_index/v2.sh",
        "make_index/v2_deeper_tree.sh",
        "make_index/v2_all_file_kinds.sh",
        "make_index/v3_added_files.sh",
        "make_index/v3_sparse_index.sh",
        "make_index/v4_more_files_IEOT.sh",
    ];

    for fixture in fixtures {
        let worktree_dir = scripted_fixture_read_only_standalone(fixture)?;
        let expected_tree_id = tree_id(&worktree_dir);
        let git_dir = worktree_dir.join(".git");
        let mut index = gix_index::File::at(git_dir.join("index"), gix_hash::Kind::Sha1, false, Default::default())?;
        let objects = MemoryDb::exists_all(gix_hash::Kind::Sha1);

        let actual_tree_id = index.to_tree(&objects, Default::default())?;
        assert_eq!(actual_tree_id, expected_tree_id, "tree mismatch in {fixture:?}");
    }
    Ok(())
}

#[test]
fn to_tree_empty_index_is_empty_tree() -> crate::Result {
    let mut state = State::new(gix_hash::Kind::Sha1);
    let objects = MemoryDb::exists_all(gix_hash::Kind::Sha1);

    let actual = state.to_tree(&objects, Default::default())?;

    assert_eq!(actual, gix_hash::ObjectId::empty_tree(gix_hash::Kind::Sha1));
    assert!(state.tree().is_none(), "TREE extension isn't created if absent");
    Ok(())
}

#[test]
fn to_tree_rejects_unmerged_entries() {
    let mut index = super::Fixture::Loose("conflicting-file").open();
    let objects = MemoryDb::exists_all(gix_hash::Kind::Sha1);

    let err = index.to_tree(&objects, Default::default()).unwrap_err();

    assert!(matches!(err, gix_index::init::to_tree::Error::Unmerged { .. }));
}

#[test]
fn to_tree_rejects_file_directory_conflicts() {
    let mut state = state_with_entries(["a", "a.b", "a/b"]);
    let objects = MemoryDb::exists_all(gix_hash::Kind::Sha1);

    let err = state.to_tree(&objects, Default::default()).unwrap_err();

    assert!(matches!(
        err,
        gix_index::init::to_tree::Error::FileDirectoryConflict { .. }
    ));
}

#[test]
fn to_tree_rejects_invalid_components() {
    let mut state = state_with_entries(["a//b"]);
    let objects = MemoryDb::exists_all(gix_hash::Kind::Sha1);

    let err = state.to_tree(&objects, Default::default()).unwrap_err();

    assert!(matches!(err, gix_index::init::to_tree::Error::InvalidComponent { .. }));
}

#[test]
fn to_tree_rejects_missing_objects_unless_allowed() -> crate::Result {
    let mut state = state_with_entries(["file"]);
    let objects = MemoryDb::written_only(gix_hash::Kind::Sha1);

    let err = state.to_tree(&objects, Default::default()).unwrap_err();
    assert!(matches!(err, gix_index::init::to_tree::Error::MissingObject { .. }));

    let options = gix_index::init::to_tree::Options {
        missing_ok: true,
        ..Default::default()
    };
    let actual = state.to_tree(&objects, options)?;
    assert_ne!(actual, gix_hash::Kind::Sha1.null());
    Ok(())
}

#[test]
fn to_tree_refreshes_existing_tree_extension() -> crate::Result {
    let mut index = super::Fixture::Generated("v2").open();
    let original_cached_tree = index.tree().expect("fixture has TREE extension").id;
    index.entries_mut()[0].id = repeated_id(b'b');
    let objects = MemoryDb::exists_all(gix_hash::Kind::Sha1);

    let actual = index.to_tree(&objects, Default::default())?;

    assert_ne!(actual, original_cached_tree);
    let tree = index.tree().expect("TREE extension is preserved and refreshed");
    assert_eq!(tree.id, actual);
    assert_eq!(
        tree.num_entries,
        Some(index.entries().len().try_into().expect("small fixture"))
    );
    tree.verify(false, gix_object::find::Never)?;
    Ok(())
}

#[test]
fn to_tree_reuses_fully_valid_tree_extension() -> crate::Result {
    let mut index = super::Fixture::Generated("v2").open();
    let original_cached_tree = index.tree().expect("fixture has TREE extension").id;
    index.entries_mut_keep_tree_cache()[0].stat.size = 42;
    let objects = MemoryDb::exists_all(gix_hash::Kind::Sha1);

    let actual = index.to_tree(&objects, Default::default())?;

    assert_eq!(actual, original_cached_tree);
    assert!(
        objects.written.borrow().is_empty(),
        "a fully-valid TREE cache can be reused without writing objects"
    );
    Ok(())
}

#[test]
fn to_tree_does_not_create_missing_tree_extension() -> crate::Result {
    let worktree_dir = scripted_fixture_read_only_standalone("make_index/v2.sh")?;
    let odb = gix_odb::at(worktree_dir.join(".git").join("objects"))?;
    let mut state = State::from_tree(&tree_id(&worktree_dir), &odb, Default::default())?;
    assert!(state.tree().is_none());
    let objects = MemoryDb::exists_all(gix_hash::Kind::Sha1);

    state.to_tree(&objects, Default::default())?;

    assert!(state.tree().is_none());
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
    let hex_hash = std::fs::read_to_string(root.join("head.tree")).unwrap_or_else(|_| {
        let mut out = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["rev-parse", "@^{tree}"])
            .output()
            .expect("git can determine the tree id");
        if !out.status.success() {
            out = std::process::Command::new("git")
                .arg("-C")
                .arg(root)
                .arg("write-tree")
                .output()
                .expect("git can write the tree id");
        }
        assert!(
            out.status.success(),
            "git couldn't determine a tree: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).expect("hex tree id is utf8")
    });
    hex_hash.trim().parse().expect("valid hash")
}

fn state_with_entries<const N: usize>(paths: [&str; N]) -> State {
    let mut state = State::new(gix_hash::Kind::Sha1);
    for path in paths {
        state.dangerously_push_entry(
            Default::default(),
            repeated_id(b'a'),
            gix_index::entry::Flags::empty(),
            gix_index::entry::Mode::FILE,
            path.as_bytes().as_bstr(),
        );
    }
    state.sort_entries();
    state
}

fn repeated_id(byte: u8) -> gix_hash::ObjectId {
    gix_hash::ObjectId::from_hex(&vec![byte; gix_hash::Kind::Sha1.len_in_hex()]).expect("valid hex")
}

struct MemoryDb {
    object_hash: gix_hash::Kind,
    exists_all: bool,
    written: RefCell<HashSet<gix_hash::ObjectId>>,
}

impl MemoryDb {
    fn exists_all(object_hash: gix_hash::Kind) -> Self {
        MemoryDb {
            object_hash,
            exists_all: true,
            written: Default::default(),
        }
    }

    fn written_only(object_hash: gix_hash::Kind) -> Self {
        MemoryDb {
            object_hash,
            exists_all: false,
            written: Default::default(),
        }
    }
}

impl gix_object::Exists for MemoryDb {
    fn exists(&self, id: &gix_hash::oid) -> bool {
        self.exists_all || self.written.borrow().contains(id)
    }
}

impl gix_object::Write for MemoryDb {
    fn write_stream(
        &self,
        kind: gix_object::Kind,
        _size: u64,
        from: &mut dyn Read,
    ) -> Result<gix_hash::ObjectId, gix_object::write::Error> {
        let mut buf = Vec::new();
        from.read_to_end(&mut buf)?;
        let id = gix_object::compute_hash(self.object_hash, kind, &buf)?;
        self.written.borrow_mut().insert(id);
        Ok(id)
    }
}
