use gix_object::{
    bstr::ByteSlice,
    tree::{self, EntryRef},
    TreeRefIter,
};
use pretty_assertions::assert_eq;

use crate::{fixture_name, hex_to_id};

#[test]
fn empty() {
    assert_eq!(TreeRefIter::from_bytes(&[]).count(), 0, "empty trees are definitely ok");
}

#[test]
fn error_handling() {
    let data = fixture_name("tree", "everything.tree");
    let iter = TreeRefIter::from_bytes(&data[..data.len() / 2]);
    let entries = iter.collect::<Vec<_>>();
    assert!(
        entries.last().expect("at least one token").is_err(),
        "errors are propagated and none is returned from that point on"
    );
}

#[test]
fn offset_to_next_entry() {
    let buf = fixture_name("tree", "everything.tree");
    let mut iter = TreeRefIter::from_bytes(&buf);
    assert_eq!(iter.offset_to_next_entry(&buf), 0, "first entry is always at 0");
    iter.next();

    let actual = iter.offset_to_next_entry(&buf);
    assert_eq!(actual, 31, "now the offset increases");
    assert_eq!(
        TreeRefIter::from_bytes(&buf[actual..])
            .next()
            .map(|e| e.unwrap().filename),
        iter.next().map(|e| e.unwrap().filename),
        "One can now start the iteration at a certain entry"
    );
}

#[test]
fn everything() -> crate::Result {
    assert_eq!(
        TreeRefIter::from_bytes(&fixture_name("tree", "everything.tree")).collect::<Result<Vec<_>, _>>()?,
        vec![
            EntryRef {
                mode: tree::EntryKind::BlobExecutable.into(),
                filename: b"exe".as_bstr(),
                oid: &hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391")
            },
            EntryRef {
                mode: tree::EntryKind::Blob.into(),
                filename: b"file".as_bstr(),
                oid: &hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391")
            },
            EntryRef {
                mode: tree::EntryKind::Commit.into(),
                filename: b"grit-submodule".as_bstr(),
                oid: &hex_to_id("b2d1b5d684bdfda5f922b466cc13d4ce2d635cf8")
            },
            EntryRef {
                mode: tree::EntryKind::Tree.into(),
                filename: b"subdir".as_bstr(),
                oid: &hex_to_id("4d5fcadc293a348e88f777dc0920f11e7d71441c")
            },
            EntryRef {
                mode: tree::EntryKind::Link.into(),
                filename: b"symlink".as_bstr(),
                oid: &hex_to_id("1a010b1c0f081b2e8901d55307a15c29ff30af0e")
            }
        ]
    );
    Ok(())
}

mod lookup_entry {
    use crate::hex_to_id;
    use gix_object::tree::EntryKind;
    use utils::entry;

    #[test]
    fn top_level_directory() -> crate::Result {
        assert_eq!(
            utils::lookup_entry_by_path("bin")?,
            entry(
                "bin",
                EntryKind::Blob,
                hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391")
            )
        );
        Ok(())
    }

    #[test]
    fn nested_file() -> crate::Result {
        assert_eq!(
            utils::lookup_entry_by_path("file/a")?,
            entry(
                "a",
                EntryKind::Blob,
                hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391")
            )
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
        use crate::hex_to_id;

        use gix_object::{tree, FindExt};

        pub(super) fn entry(filename: &str, mode: tree::EntryKind, oid: gix_hash::ObjectId) -> Option<tree::Entry> {
            Some(tree::Entry {
                mode: mode.into(),
                filename: filename.into(),
                oid,
            })
        }

        pub(super) fn tree_odb() -> gix_testtools::Result<gix_odb::Handle> {
            let root = gix_testtools::scripted_fixture_read_only("make_trees.sh")?;
            Ok(gix_odb::at(root.join(".git/objects"))?)
        }

        pub(super) fn lookup_entry_by_path(path: &str) -> gix_testtools::Result<Option<gix_object::tree::Entry>> {
            let odb = tree_odb()?;
            let root_tree_id = hex_to_id("ff7e7d2aecae1c3fb15054b289a4c58aa65b8646");

            let mut buf = Vec::new();
            let root_tree = odb.find_tree_iter(&root_tree_id, &mut buf)?;

            let mut buf = Vec::new();
            root_tree.lookup_entry_by_path(&odb, &mut buf, path)
        }
    }
}
