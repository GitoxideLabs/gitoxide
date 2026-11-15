use gix_object::{Tree, TreeRef, TreeRefIter, WriteTo, bstr::ByteSlice, tree, tree::EntryRef};

use crate::{fixture_oid, tree_fixture};

#[test]
fn empty() -> gix_testtools::TestResult {
    let tree_ref = TreeRef::from_bytes(&[], gix_testtools::object_hash())?;
    assert_eq!(
        tree_ref,
        TreeRef { entries: vec![] },
        "empty trees are valid despite usually rare in the wild"
    );

    let mut buf = Vec::new();
    tree_ref.write_to(&mut buf)?;
    assert!(buf.is_empty());

    buf.clear();
    Tree::from(tree_ref).write_to(&mut buf)?;
    assert!(buf.is_empty());
    Ok(())
}

#[test]
fn noncanonical_modes_match_git_without_losing_their_encoding() -> gix_testtools::TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_tree_modes.sh")?;
    let hash_kind = crate::fixture_hash_kind();
    for raw_mode in [
        "100700", "100100", "100600", "100010", "100664", "120777", "140000", "140777", "170000", "060000", "40000",
        "040000", "40777", "040777",
    ] {
        let data = std::fs::read(root.join(format!("{raw_mode}.tree")))?;
        let baseline = std::fs::read_to_string(root.join(format!("{raw_mode}.baseline")))?;
        let expected_mode = u16::from_str_radix(&baseline[..6], 8)?;
        let tree = TreeRef::from_bytes(&data, hash_kind)?;
        let iterated = TreeRefIter::from_bytes(&data, hash_kind).entries()?;
        assert_eq!(tree.entries, iterated, "both tree readers preserve the same entries");
        let mode = tree.entries[0].mode;
        assert_eq!(
            mode.normalized().value(),
            expected_mode,
            "{raw_mode}: normalization agrees with Git"
        );
        assert_eq!(
            mode.is_commit(),
            expected_mode == tree::EntryKind::Commit as u16,
            "{raw_mode}: submodule detection agrees with Git"
        );
        assert_eq!(
            mode.value(),
            u16::from_str_radix(raw_mode, 8)?,
            "{raw_mode}: the raw numeric value remains available"
        );
        let mut encoded = Vec::new();
        tree.write_to(&mut encoded)?;
        assert_eq!(encoded, data, "{raw_mode}: re-encoding preserves the object id");
        encoded.clear();
        Tree::from(tree).write_to(&mut encoded)?;
        assert_eq!(encoded, data, "{raw_mode}: owned trees preserve the same encoding");
    }
    Ok(())
}

#[test]
fn everything() -> gix_testtools::TestResult {
    let fixture = tree_fixture("everything.tree")?;
    let hash_kind = crate::fixture_hash_kind();
    let tree_ref = TreeRef::from_bytes(&fixture, hash_kind)?;
    assert_eq!(
        tree_ref,
        TreeRef {
            entries: vec![
                EntryRef {
                    mode: tree::EntryKind::BlobExecutable.into(),
                    filename: b"exe".as_bstr(),
                    oid: &fixture_oid("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391")
                },
                EntryRef {
                    mode: tree::EntryKind::Blob.into(),
                    filename: b"file".as_bstr(),
                    oid: &fixture_oid("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391")
                },
                EntryRef {
                    mode: tree::EntryKind::Commit.into(),
                    filename: b"grit-submodule".as_bstr(),
                    oid: &fixture_oid("b2d1b5d684bdfda5f922b466cc13d4ce2d635cf8")
                },
                EntryRef {
                    mode: tree::EntryKind::Tree.into(),
                    filename: b"subdir".as_bstr(),
                    oid: &fixture_oid("4d5fcadc293a348e88f777dc0920f11e7d71441c")
                },
                EntryRef {
                    mode: tree::EntryKind::Link.into(),
                    filename: b"symlink".as_bstr(),
                    oid: &fixture_oid("1a010b1c0f081b2e8901d55307a15c29ff30af0e")
                }
            ]
        }
    );
    Ok(())
}

#[test]
fn invalid() {
    let fixture = tree_fixture("definitely-special.tree").expect("fixture is valid");
    let partial_tree = &fixture[..fixture.len() / 2];
    let hash_kind = crate::fixture_hash_kind();
    assert!(TreeRef::from_bytes(partial_tree, hash_kind).is_err());
    assert!(
        TreeRefIter::from_bytes(partial_tree, hash_kind)
            .take_while(std::result::Result::is_ok)
            .count()
            > 0,
        "we can decode some entries before failing"
    );
}

#[test]
fn fuzzed() {
    assert!(
        gix_object::TreeRef::from_bytes(b"2", gix_testtools::object_hash()).is_err(),
        "fail, but don't crash"
    );
}

#[test]
fn fuzz_artifact_inputs_can_be_parsed_without_panicking() {
    for path in crate::fuzz_artifact_paths("fuzz_tree") {
        let input = std::fs::read(path).expect("artifact is readable");
        _ = TreeRef::from_bytes(&input, gix_hash::Kind::Sha1);
        _ = TreeRef::from_bytes(&input, gix_hash::Kind::Sha256);
    }
}

#[test]
fn special_trees() -> gix_testtools::TestResult {
    let hash_kind = crate::fixture_hash_kind();
    for (name, expected_entry_count) in [
        ("maybe-special", 160),
        ("definitely-special", 19),
        ("special-1", 5),
        ("special-2", 18),
        ("special-3", 5),
        ("special-4", 18),
        ("special-5", 17),
    ] {
        let fixture = tree_fixture(&format!("{name}.tree"))?;
        let actual = TreeRef::from_bytes(&fixture, hash_kind)?;
        assert_eq!(actual.entries.len(), expected_entry_count, "{name}");
        assert_eq!(
            TreeRefIter::from_bytes(&fixture, hash_kind)
                .map(std::result::Result::unwrap)
                .count(),
            expected_entry_count,
            "{name}"
        );
        // Show we can roundtrip
        let mut buf: Vec<u8> = Default::default();
        actual.write_to(&mut buf)?;
        assert_eq!(buf, fixture);
    }
    Ok(())
}
