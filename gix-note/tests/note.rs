use gix_hash::{Kind, ObjectId, oid};
use gix_object::{
    Tree, Write,
    bstr::BString,
    tree::{Entry, EntryKind},
};

type ObjectDb = gix_odb::memory::Proxy<gix_object::find::Never>;

#[test]
fn reads_notes_without_fanout() -> gix_testtools::Result {
    assert_note_at_fanout(0, "40")
}

#[test]
fn reads_notes_with_one_fanout_level() -> gix_testtools::Result {
    assert_note_at_fanout(1, "2/38")
}

#[test]
fn reads_notes_with_two_fanout_levels() -> gix_testtools::Result {
    assert_note_at_fanout(2, "2/2/36")
}

#[test]
fn reads_notes_with_three_fanout_levels() -> gix_testtools::Result {
    assert_note_at_fanout(3, "2/2/2/34")
}

fn assert_note_at_fanout(fanout: usize, layout: &str) -> gix_testtools::Result {
    let objects = ObjectDb::new(gix_object::find::Never, Kind::Sha1);
    let annotated = gix_object::compute_hash(Kind::Sha1, gix_object::Kind::Blob, b"annotated")?;
    let note = objects.write_buf(gix_object::Kind::Blob, b"note")?;
    let root = notes_tree(&objects, &annotated, note, fanout)?;

    assert_eq!(
        gix_note::get(root, &annotated, &objects).map_err(gix_error::Exn::into_error)?,
        Some(note),
        "the note is found in the {layout} layout"
    );
    Ok(())
}

fn notes_tree(objects: &ObjectDb, annotated: &oid, note: ObjectId, fanout: usize) -> gix_testtools::Result<ObjectId> {
    let hex = annotated.to_hex().to_string();
    let mut tree = objects.write(&Tree {
        entries: vec![Entry {
            mode: EntryKind::Blob.into(),
            filename: BString::from(&hex[fanout * 2..]),
            oid: note,
        }],
    })?;

    for level in (0..fanout).rev() {
        tree = objects.write(&Tree {
            entries: vec![Entry {
                mode: EntryKind::Tree.into(),
                filename: BString::from(&hex[level * 2..][..2]),
                oid: tree,
            }],
        })?;
    }
    Ok(tree)
}

#[test]
fn ignores_entries_that_are_not_notes() -> gix_testtools::Result {
    let objects = gix_odb::memory::Proxy::new(gix_object::find::Never, Kind::Sha1);
    let annotated = gix_object::compute_hash(Kind::Sha1, gix_object::Kind::Blob, b"annotated")?;
    let hex = annotated.to_hex().to_string();
    let root = objects.write(&Tree {
        entries: vec![Entry {
            mode: EntryKind::Tree.into(),
            filename: BString::from(hex),
            oid: Kind::Sha1.empty_tree(),
        }],
    })?;

    assert_eq!(
        gix_note::get(root, &annotated, &objects).map_err(gix_error::Exn::into_error)?,
        None,
        "the canonical empty-tree ID in a tree-mode entry at the full object-ID path is not a blob note"
    );
    Ok(())
}
