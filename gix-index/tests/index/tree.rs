use bstr::ByteSlice;
use gix_index::{
    State,
    entry::{Flags, Mode, Stage},
};
use gix_testtools::TestResult;

#[test]
fn write_tree_matches_git_with_sparse_directories_and_all_entry_kinds() -> TestResult {
    for fixture in [
        "make_index/v2_all_file_kinds.sh",
        "make_index/v2_deeper_tree.sh",
        "make_index/v3_sparse_index.sh",
    ] {
        let fixture = gix_testtools::scripted_fixture_writable(fixture)?;
        let workdir = fixture.path();
        let index_path = workdir.join(".git/index");
        let expected = gix_testtools::git(workdir, "write-tree")?;
        let before = std::fs::read(&index_path)?;
        let index = gix_index::File::at(&index_path, gix_testtools::object_hash(), false, Default::default())?;
        let objects = crate::odb_at(workdir.join(".git/objects"))?;
        let actual = index.write_tree(&objects, Default::default())?;
        assert_eq!(
            actual.to_string(),
            expected.trim(),
            "tree serialization agrees with Git for {fixture:?}"
        );
        assert_eq!(
            std::fs::read(&index_path)?,
            before,
            "writing trees does not change the index"
        );
    }
    Ok(())
}

#[test]
fn write_tree_orders_directories_and_skips_intent_to_add_and_removals() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_writable("make_index/v2.sh")?;
    let workdir = fixture.path();
    let objects = crate::odb_at(workdir.join(".git/objects"))?;
    let blob_id = gix_object::Write::write_buf(&objects, gix_object::Kind::Blob, b"content\n")?;
    let mut state = State::new(gix_testtools::object_hash());
    assert_eq!(
        state.write_tree(&objects, Default::default())?,
        gix_hash::ObjectId::empty_tree(gix_testtools::object_hash()),
        "an empty index produces the empty tree"
    );
    for (path, flags) in [
        ("a/x", Flags::empty()),
        ("a-", Flags::empty()),
        ("a0", Flags::empty()),
        ("not-yet/added", Flags::INTENT_TO_ADD | Flags::EXTENDED),
        ("removed", Flags::REMOVE),
    ] {
        state.dangerously_push_entry(Default::default(), blob_id, flags, Mode::FILE, path.into());
    }
    state.sort_entries();
    let actual = state.write_tree(&objects, Default::default())?;
    let mut index = gix_index::File::from_state(state, workdir.join(".git/index"));
    index.write(Default::default())?;
    let expected = gix_testtools::git(workdir, "write-tree")?;
    assert_eq!(
        actual.to_string(),
        expected.trim(),
        "index ordering differs from tree ordering at directories"
    );
    Ok(())
}

#[test]
fn write_tree_rejects_conflicts_and_invalid_index_shapes() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_writable("make_index/v2.sh")?;
    let objects = crate::odb_at(fixture.path().join(".git/objects"))?;
    let blob_id = gix_object::Write::write_buf(&objects, gix_object::Kind::Blob, b"content\n")?;
    for (name, entries) in [
        ("conflict", vec![("a", Mode::FILE, Flags::from_stage(Stage::Ours))]),
        (
            "duplicate",
            vec![("a", Mode::FILE, Flags::empty()), ("a", Mode::FILE, Flags::empty())],
        ),
        (
            "not sorted",
            vec![("z", Mode::FILE, Flags::empty()), ("a", Mode::FILE, Flags::empty())],
        ),
        (
            "file and directory",
            vec![
                ("a", Mode::FILE, Flags::empty()),
                ("a-", Mode::FILE, Flags::empty()),
                ("a/x", Mode::FILE, Flags::empty()),
            ],
        ),
        (
            "sparse directory and child",
            vec![
                ("a/", Mode::DIR, Flags::SKIP_WORKTREE),
                ("a/x", Mode::FILE, Flags::empty()),
            ],
        ),
        ("invalid mode", vec![("a", Mode::empty(), Flags::empty())]),
        ("absolute path", vec![("/a", Mode::FILE, Flags::empty())]),
        ("empty component", vec![("a//b", Mode::FILE, Flags::empty())]),
        ("metadata", vec![(".git/config", Mode::FILE, Flags::empty())]),
        ("parent component", vec![("../a", Mode::FILE, Flags::empty())]),
        ("invalid sparse path", vec![("a", Mode::DIR, Flags::SKIP_WORKTREE)]),
        ("unmarked sparse directory", vec![("a/", Mode::DIR, Flags::empty())]),
    ] {
        let mut state = State::new(gix_testtools::object_hash());
        for (path, mode, flags) in entries {
            state.dangerously_push_entry(Default::default(), blob_id, flags, mode, path.into());
        }
        assert!(state.write_tree(&objects, Default::default()).is_err(), "reject {name}");
    }
    Ok(())
}

#[test]
fn write_tree_requires_objects_except_gitlinks() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_writable("make_index/v2.sh")?;
    let workdir = fixture.path();
    let objects = crate::odb_at(workdir.join(".git/objects"))?;
    let missing_id = gix_object::compute_hash(gix_testtools::object_hash(), gix_object::Kind::Blob, b"missing\n")?;
    for mode in [Mode::FILE, Mode::SYMLINK, Mode::COMMIT] {
        let mut state = State::new(gix_testtools::object_hash());
        state.dangerously_push_entry(Default::default(), missing_id, Flags::empty(), mode, "missing".into());
        let actual = state.write_tree(&objects, Default::default());
        let mut index = gix_index::File::from_state(state, workdir.join(".git/index"));
        index.write(Default::default())?;
        let git_succeeds = gix_testtools::run_git(workdir, &["write-tree"])?.success();
        assert_eq!(
            actual.is_ok(),
            git_succeeds,
            "Git and gitoxide agree about absent {mode:?} objects"
        );
    }
    let mut state = State::new(gix_testtools::object_hash());
    state.dangerously_push_entry(
        Default::default(),
        gix_hash::ObjectId::null(gix_testtools::object_hash()),
        Flags::empty(),
        Mode::COMMIT,
        "null".into(),
    );
    assert!(
        state.write_tree(&objects, Default::default()).is_err(),
        "even gitlinks must have a non-null object ID"
    );
    let other_hash = match gix_testtools::object_hash() {
        gix_hash::Kind::Sha1 => gix_hash::Kind::Sha256,
        _ => gix_hash::Kind::Sha1,
    };
    let other_id = gix_object::compute_hash(other_hash, gix_object::Kind::Blob, b"other hash\n")?;
    state.entries_mut()[0].id = other_id;
    assert!(
        state.write_tree(&objects, Default::default()).is_err(),
        "entries must use the index's object hash"
    );
    let mut other_state = State::new(other_hash);
    other_state.dangerously_push_entry(
        Default::default(),
        other_id,
        Flags::empty(),
        Mode::COMMIT,
        "gitlink".into(),
    );
    assert!(
        other_state.write_tree(&objects, Default::default()).is_err(),
        "the object store must use the index's hash kind"
    );
    Ok(())
}

#[test]
fn write_tree_accepts_non_utf8_paths() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_writable("make_index/v2.sh")?;
    let objects = crate::odb_at(fixture.path().join(".git/objects"))?;
    let blob_id = gix_object::Write::write_buf(&objects, gix_object::Kind::Blob, b"content\n")?;
    let mut state = State::new(gix_testtools::object_hash());
    state.dangerously_push_entry(
        Default::default(),
        blob_id,
        Flags::empty(),
        Mode::FILE,
        b"dir/\xff".as_bstr(),
    );
    let tree_id = state.write_tree(&objects, Default::default())?;
    let roundtrip = State::from_tree(&tree_id, &objects, Default::default())?;
    assert_eq!(
        roundtrip.entries()[0].path(&roundtrip),
        b"dir/\xff".as_bstr(),
        "tree writing preserves path bytes"
    );
    Ok(())
}
