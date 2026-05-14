use gix_object::{
    TreeRefIter,
    bstr::ByteSlice,
    tree::{self, EntryRef},
};
use pretty_assertions::assert_eq;

use crate::{fixture_hash_kind, fixture_oid, tree_fixture};

#[test]
fn empty() {
    assert_eq!(
        TreeRefIter::from_bytes(&[], gix_testtools::object_hash()).count(),
        0,
        "empty trees are definitely ok"
    );
}

#[test]
fn error_handling() {
    let data = tree_fixture("everything.tree").expect("fixture is valid");
    let iter = TreeRefIter::from_bytes(&data[..data.len() / 2], fixture_hash_kind());
    let entries = iter.collect::<Vec<_>>();
    assert!(
        entries.last().expect("at least one token").is_err(),
        "errors are propagated and none is returned from that point on"
    );
}

#[test]
fn offset_to_next_entry() {
    let hash_kind = fixture_hash_kind();
    let buf = tree_fixture("everything.tree").expect("fixture is valid");
    let mut iter = TreeRefIter::from_bytes(&buf, hash_kind);
    assert_eq!(iter.offset_to_next_entry(&buf), 0, "first entry is always at 0");
    iter.next();

    let actual = iter.offset_to_next_entry(&buf);
    assert_eq!(actual, 11 + hash_kind.len_in_bytes(), "now the offset increases");
    assert_eq!(
        TreeRefIter::from_bytes(&buf[actual..], hash_kind)
            .next()
            .map(|e| e.unwrap().filename),
        iter.next().map(|e| e.unwrap().filename),
        "One can now start the iteration at a certain entry"
    );
}

#[test]
fn everything() -> crate::Result {
    assert_eq!(
        TreeRefIter::from_bytes(&tree_fixture("everything.tree")?, fixture_hash_kind())
            .collect::<Result<Vec<_>, _>>()?,
        vec![
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
    );
    Ok(())
}

#[test]
fn leading_space_in_tree_name() -> crate::Result {
    let oid = fixture_oid("4d5fcadc293a348e88f777dc0920f11e7d71441c");
    let mut buf = b"40000  leading space\0".to_vec();
    buf.extend_from_slice(oid.as_bytes());

    assert_eq!(
        TreeRefIter::from_bytes(&buf, fixture_hash_kind()).collect::<Result<Vec<_>, _>>()?,
        vec![EntryRef {
            mode: tree::EntryKind::Tree.into(),
            filename: b" leading space".as_bstr(),
            oid: oid.as_ref(),
        }]
    );
    Ok(())
}

mod lookup_entry {
    use gix_object::tree::EntryKind;
    use utils::entry;

    use crate::fixture_hash_kind;

    #[test]
    fn top_level_directory() -> crate::Result {
        assert_eq!(
            utils::lookup_entry_by_path("bin")?,
            entry("bin", EntryKind::Blob, fixture_hash_kind().empty_blob())
        );
        Ok(())
    }

    #[test]
    fn nested_file() -> crate::Result {
        assert_eq!(
            utils::lookup_entry_by_path("file/a")?,
            entry("a", EntryKind::Blob, fixture_hash_kind().empty_blob())
        );
        Ok(())
    }

    #[test]
    fn non_existing_nested_file() -> crate::Result {
        for path in ["file/does-not-exist", "non-existing", "file/a/through-file"] {
            let actual = utils::lookup_entry_by_path(path)?;
            assert_eq!(actual, None);
        }
        Ok(())
    }

    mod utils {
        use gix_object::{FindExt, tree};

        use crate::generated_tree_root_id;

        pub(super) fn entry(filename: &str, mode: tree::EntryKind, oid: gix_hash::ObjectId) -> Option<tree::Entry> {
            Some(tree::Entry {
                mode: mode.into(),
                filename: filename.into(),
                oid,
            })
        }

        pub(super) fn tree_odb() -> gix_testtools::Result<gix_odb::Handle> {
            let root = gix_testtools::scripted_fixture_read_only("make_trees.sh")?;
            Ok(gix_odb::at_opts(
                root.join(".git/objects"),
                Vec::new(),
                gix_odb::store::init::Options {
                    object_hash: crate::fixture_hash_kind(),
                    ..Default::default()
                },
            )?)
        }

        pub(super) fn lookup_entry_by_path(path: &str) -> gix_testtools::Result<Option<gix_object::tree::Entry>> {
            let odb = tree_odb()?;
            let root_tree_id = generated_tree_root_id()?;

            let mut buf = Vec::new();
            let root_tree = odb.find_tree_iter(&root_tree_id, &mut buf)?;

            let mut buf = Vec::new();
            root_tree.lookup_entry_by_path(&odb, &mut buf, path)
        }
    }
}
