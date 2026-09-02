use std::{cell::Cell, collections::BTreeSet, io::Read};

use gix_hash::{Kind, ObjectId, oid};
use gix_object::{
    FindExt, Tree, Write,
    bstr::{BString, ByteSlice},
    tree::{Entry, EntryKind},
};

type ObjectDb = gix_odb::memory::Proxy<gix_object::find::Never>;

struct CountingObjectDb {
    inner: ObjectDb,
    reads: Cell<usize>,
    writes: Cell<usize>,
}

impl gix_object::Find for CountingObjectDb {
    fn try_find<'a>(
        &self,
        id: &gix_hash::oid,
        buffer: &'a mut Vec<u8>,
    ) -> Result<Option<gix_object::Data<'a>>, gix_object::find::Error> {
        self.reads.set(self.reads.get() + 1);
        self.inner.try_find(id, buffer)
    }
}

impl gix_object::Write for CountingObjectDb {
    fn write_buf_with_known_id(
        &self,
        kind: gix_object::Kind,
        from: &[u8],
        id: ObjectId,
    ) -> Result<ObjectId, gix_object::write::Error> {
        self.writes.set(self.writes.get() + 1);
        self.inner.write_buf_with_known_id(kind, from, id)
    }

    fn write_stream(
        &self,
        kind: gix_object::Kind,
        size: u64,
        from: &mut dyn Read,
    ) -> Result<ObjectId, gix_object::write::Error> {
        self.writes.set(self.writes.get() + 1);
        self.inner.write_stream(kind, size, from)
    }

    fn write_stream_with_known_id(
        &self,
        kind: gix_object::Kind,
        size: u64,
        from: &mut dyn Read,
        id: ObjectId,
    ) -> Result<ObjectId, gix_object::write::Error> {
        self.writes.set(self.writes.get() + 1);
        self.inner.write_stream_with_known_id(kind, size, from, id)
    }
}

#[test]
fn reads_notes_without_fanout() -> gix_testtools::Result {
    assert_note_at_fanout(0)
}

#[test]
fn reads_notes_with_one_fanout_level() -> gix_testtools::Result {
    assert_note_at_fanout(1)
}

#[test]
fn reads_notes_with_two_fanout_levels() -> gix_testtools::Result {
    assert_note_at_fanout(2)
}

#[test]
fn reads_notes_with_three_fanout_levels() -> gix_testtools::Result {
    assert_note_at_fanout(3)
}

#[test]
fn replacing_a_note_does_not_read_untouched_fanout_subtrees() -> gix_testtools::Result {
    let kind = gix_testtools::object_hash();
    let objects = CountingObjectDb {
        inner: ObjectDb::new(gix_object::find::Never, kind),
        reads: Cell::new(0),
        writes: Cell::new(0),
    };
    let note = objects.write_buf(gix_object::Kind::Blob, b"note")?;
    let replacement = objects.write_buf(gix_object::Kind::Blob, b"replacement")?;
    let mut annotated = Vec::new();
    let mut root_entries = Vec::new();
    for high_nibble in b"0123456789abcdef" {
        for low_nibble in b"01" {
            let object = object_id_with_nibbles(kind, &[(0, *high_nibble), (1, *low_nibble)])?;
            let hex = object.to_hex().to_string();
            let subtree = objects.write(&Tree {
                entries: vec![Entry {
                    mode: EntryKind::Blob.into(),
                    filename: hex[2..].into(),
                    oid: note,
                }],
            })?;
            annotated.push(object);
            root_entries.push(Entry {
                mode: EntryKind::Tree.into(),
                filename: hex[..2].into(),
                oid: subtree,
            });
        }
    }
    let root = objects.write(&Tree {
        entries: root_entries.clone(),
    })?;
    let target_hex = annotated[0].to_hex().to_string();
    let replacement_subtree = objects.write(&Tree {
        entries: vec![Entry {
            mode: EntryKind::Blob.into(),
            filename: target_hex[2..].into(),
            oid: replacement,
        }],
    })?;
    root_entries[0].oid = replacement_subtree;
    let expected_root = objects.write(&Tree { entries: root_entries })?;
    objects.writes.set(0);

    let outcome = gix_note::replace(root, annotated[0], replacement, &objects)?;
    assert_eq!(outcome.previous, Some(note), "replacement returns the previous note");
    assert_eq!(
        outcome.tree, expected_root,
        "the rewritten tree matches a path-local edit"
    );
    assert_eq!(
        objects.reads.get(),
        2,
        "only the root and the changed fanout subtree are read"
    );
    assert_eq!(
        objects.writes.get(),
        2,
        "only the changed fanout subtree and root are written"
    );
    assert_eq!(
        gix_note::get(outcome.tree, &annotated[0], &objects)?,
        Some(replacement),
        "the replacement is visible"
    );
    Ok(())
}

fn assert_note_at_fanout(fanout: usize) -> gix_testtools::Result {
    let kind = gix_testtools::object_hash();
    let objects = ObjectDb::new(gix_object::find::Never, kind);
    let annotated = gix_object::compute_hash(kind, gix_object::Kind::Blob, b"annotated")?;
    let note = objects.write_buf(gix_object::Kind::Blob, b"note")?;
    let root = notes_tree(&objects, &annotated, note, fanout)?;

    assert_eq!(
        gix_note::get(root, &annotated, &objects)?,
        Some(note),
        "the note is found with {fanout} fanout levels"
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
    let kind = gix_testtools::object_hash();
    let objects = gix_odb::memory::Proxy::new(gix_object::find::Never, kind);
    let annotated = gix_object::compute_hash(kind, gix_object::Kind::Blob, b"annotated")?;
    let hex = annotated.to_hex().to_string();
    let root = objects.write(&Tree {
        entries: vec![Entry {
            mode: EntryKind::Tree.into(),
            filename: BString::from(hex),
            oid: kind.empty_tree(),
        }],
    })?;

    assert_eq!(
        gix_note::get(root, &annotated, &objects)?,
        None,
        "the canonical empty-tree ID in a tree-mode entry at the full object-ID path is not a blob note"
    );
    Ok(())
}

#[test]
fn mutations_rebalance_when_each_nibble_bucket_has_multiple_notes_and_preserve_non_notes() -> gix_testtools::Result {
    let kind = gix_testtools::object_hash();
    let objects = ObjectDb::new(gix_object::find::Never, kind);
    let unrelated = objects.write_buf(gix_object::Kind::Blob, b"keep")?;
    let note = objects.write_buf(gix_object::Kind::Blob, b"note")?;
    let mut root = objects.write(&Tree {
        entries: vec![Entry {
            mode: EntryKind::Blob.into(),
            filename: "README".into(),
            oid: unrelated,
        }],
    })?;
    let mut annotated = Vec::new();
    for nibble in b"0123456789abcdef" {
        let object = object_id_with_nibble(kind, 0, *nibble)?;
        annotated.push(object);
        let outcome = gix_note::replace(root, object, note, &objects)?;
        assert_eq!(outcome.previous, None, "each distinct object receives a new note");
        root = outcome.tree;
    }

    let mut buf = Vec::new();
    let tree = objects.find_tree(&root, &mut buf)?;
    assert_eq!(
        tree.entries.iter().filter(|entry| entry.mode.is_tree()).count(),
        0,
        "one note in every leading-nibble bucket remains flat, like Git"
    );
    assert_eq!(
        tree.entries.iter().filter(|entry| entry.mode.is_blob()).count(),
        17,
        "the flat tree contains sixteen notes and README"
    );

    for nibble in b"0123456789abcde" {
        let object = object_id_with_nibbles(kind, &[(0, *nibble), (2, b'1')])?;
        annotated.push(object);
        let outcome = gix_note::replace(root, object, note, &objects)?;
        assert_eq!(outcome.previous, None, "each bucket receives a second note");
        root = outcome.tree;
    }
    let mut buf = Vec::new();
    let tree = objects.find_tree(&root, &mut buf)?;
    assert_eq!(
        tree.entries.iter().filter(|entry| entry.mode.is_tree()).count(),
        0,
        "a bucket with only one note keeps all thirty-one notes flat"
    );

    let last = object_id_with_nibbles(kind, &[(0, b'f'), (2, b'1')])?;
    annotated.push(last);

    let outcome = gix_note::replace(root, last, note, &objects)?;
    assert_eq!(outcome.previous, None, "the final bucket receives its second note");
    root = outcome.tree;

    let mut buf = Vec::new();
    let tree = objects.find_tree(&root, &mut buf)?;
    assert_eq!(
        tree.entries.iter().filter(|entry| entry.mode.is_tree()).count(),
        16,
        "two notes in every first-nibble bucket cause Git's first fanout level"
    );
    assert!(
        tree.entries
            .iter()
            .any(|entry| entry.filename == "README" && entry.oid == unrelated),
        "non-note entries survive rebalancing"
    );

    for object in &annotated {
        assert_eq!(
            gix_note::get(root, object, &objects)?,
            Some(note),
            "every note remains readable after fanout"
        );
    }

    let replacement = objects.write_buf(gix_object::Kind::Blob, b"replacement")?;
    let outcome = gix_note::replace(root, annotated[0], replacement, &objects)?;
    assert_eq!(outcome.previous, Some(note), "replacement returns the previous note");
    assert_eq!(
        gix_note::get(outcome.tree, &annotated[0], &objects)?,
        Some(replacement),
        "replacement is visible through lookup"
    );
    let outcome = gix_note::remove(outcome.tree, annotated[0], &objects)?;
    assert_eq!(outcome.previous, Some(replacement), "removal returns the removed note");
    assert_eq!(
        gix_note::get(outcome.tree, &annotated[0], &objects)?,
        None,
        "the removed mapping is no longer visible"
    );
    let mut buf = Vec::new();
    let tree = objects.find_tree(&outcome.tree, &mut buf)?;
    assert_eq!(
        tree.entries.iter().filter(|entry| entry.mode.is_tree()).count(),
        0,
        "Git collapses fanout when the edited bucket materializes as a single note"
    );

    let outcome = gix_note::remove(outcome.tree, annotated[16], &objects)?;
    assert_eq!(outcome.previous, Some(note), "the second removal empties the bucket");
    let mut buf = Vec::new();
    let tree = objects.find_tree(&outcome.tree, &mut buf)?;
    assert_eq!(
        tree.entries.iter().filter(|entry| entry.mode.is_blob()).count(),
        31,
        "emptying one leading-nibble bucket collapses the thirty remaining notes beside README"
    );
    Ok(())
}

#[test]
fn mutation_fanout_matches_git_at_bucket_count_boundaries() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_read_only("make_notes_fanout_repo.sh")?;
    let baseline = std::fs::read_to_string(fixture.join("fanout.baseline"))?;
    let mut lines = baseline.lines();

    let mut note_fields = lines
        .next()
        .expect("the baseline starts with the shared note ID")
        .split(' ');
    assert_eq!(
        note_fields.next(),
        Some("note"),
        "the first baseline record identifies the note"
    );
    let note = ObjectId::from_hex(note_fields.next().expect("the note record has an object ID").as_bytes())?;
    assert_eq!(note_fields.next(), None, "the note record has exactly two fields");

    let kind = gix_testtools::object_hash();
    let objects = ObjectDb::new(gix_object::find::Never, kind);
    assert_eq!(
        objects.write_buf(gix_object::Kind::Blob, b"note")?,
        note,
        "Git and gix-object hash the shared note blob identically"
    );

    let mut root = kind.empty_tree();
    let mut additions = 0;
    let mut removals = 0;
    let mut checkpoints = 0;
    for (index, line) in lines.enumerate() {
        let mut fields = line.split(' ');
        let operation = fields.next().expect("each baseline record has an operation");
        match operation {
            "add" | "remove" => {
                let annotated_object_id = ObjectId::from_hex(
                    fields
                        .next()
                        .expect("each mutation record has an annotated-object ID")
                        .as_bytes(),
                )?;
                assert_eq!(
                    fields.next(),
                    None,
                    "mutation record {} has exactly two fields",
                    index + 2
                );
                if operation == "add" {
                    additions += 1;
                    let outcome = gix_note::replace(root, annotated_object_id, note, &objects)?;
                    assert_eq!(outcome.previous, None, "baseline additions introduce distinct notes");
                    root = outcome.tree;
                } else {
                    removals += 1;
                    let outcome = gix_note::remove(root, annotated_object_id, &objects)?;
                    assert_eq!(outcome.previous, Some(note), "baseline removals remove the shared note");
                    root = outcome.tree;
                }
            }
            "tree" => {
                checkpoints += 1;
                let name = fields.next().expect("each tree checkpoint has a descriptive name");
                let expected_tree_id = ObjectId::from_hex(
                    fields
                        .next()
                        .expect("each tree checkpoint has Git's resulting tree ID")
                        .as_bytes(),
                )?;
                assert_eq!(
                    fields.next(),
                    None,
                    "tree checkpoint record {} has exactly three fields",
                    index + 2
                );
                assert_eq!(root, expected_tree_id, "the {name} notes tree matches Git");
            }
            other => panic!("unknown baseline operation {other:?}"),
        }
    }
    assert_eq!(
        additions, 32,
        "the fixture adds two notes to every leading-nibble bucket"
    );
    assert_eq!(removals, 2, "the fixture empties one leading-nibble bucket");
    assert_eq!(checkpoints, 5, "all interesting fanout boundaries are compared");
    Ok(())
}

#[test]
fn edit_lifecycle_handles_empty_trees_replacements_and_no_op_removals() -> gix_testtools::Result {
    let kind = gix_testtools::object_hash();
    let objects = ObjectDb::new(gix_object::find::Never, kind);
    let root = objects.write(&Tree { entries: Vec::new() })?;
    let annotated = gix_object::compute_hash(kind, gix_object::Kind::Blob, b"annotated")?;
    let child = objects.write_buf(gix_object::Kind::Blob, b"tree contents")?;
    let tree_note = objects.write(&Tree {
        entries: vec![Entry {
            mode: EntryKind::Blob.into(),
            filename: "file".into(),
            oid: child,
        }],
    })?;

    assert_eq!(
        gix_note::get(root, &annotated, &objects)?,
        None,
        "an empty notes tree has no mapping"
    );
    let absent = gix_note::remove(root, annotated, &objects)?;
    assert_eq!(absent.previous, None, "removing an absent mapping has no previous note");
    assert_eq!(
        absent.tree, root,
        "removing an absent mapping leaves the root unchanged"
    );

    let added = gix_note::replace(root, annotated, tree_note, &objects)?;
    assert_eq!(added.previous, None, "adding the first mapping has no previous note");
    assert_eq!(
        gix_note::get(added.tree, &annotated, &objects)?,
        Some(tree_note),
        "lookup returns a note even when its object is actually a tree"
    );
    assert_note_layout(&objects, added.tree, &annotated, 0, tree_note)?;

    let replacement = objects.write_buf(gix_object::Kind::Blob, b"replacement")?;
    let replaced = gix_note::replace(added.tree, annotated, replacement, &objects)?;
    assert_eq!(
        replaced.previous,
        Some(tree_note),
        "replacement returns the tree-valued note"
    );
    assert_eq!(
        gix_note::get(replaced.tree, &annotated, &objects)?,
        Some(replacement),
        "lookup observes the replacement"
    );

    let removed = gix_note::remove(replaced.tree, annotated, &objects)?;
    assert_eq!(
        removed.previous,
        Some(replacement),
        "removal returns the replacement note"
    );
    assert_eq!(
        removed.tree,
        kind.empty_tree(),
        "removing the last note produces the empty tree"
    );
    let absent = gix_note::remove(removed.tree, annotated, &objects)?;
    assert_eq!(absent.previous, None, "a repeated removal is a no-op");
    assert_eq!(absent.tree, removed.tree, "a repeated removal retains the empty root");
    Ok(())
}

#[test]
fn mutations_create_and_collapse_mixed_deep_fanout() -> gix_testtools::Result {
    let kind = gix_testtools::object_hash();
    let objects = ObjectDb::new(gix_object::find::Never, kind);
    let note = objects.write_buf(gix_object::Kind::Blob, b"note")?;
    let mut root = objects.write(&Tree { entries: Vec::new() })?;
    let mut annotated = BTreeSet::new();

    for nibble in b"0123456789abcdef" {
        annotated.insert(object_id_with_nibble(kind, 0, *nibble)?);
        annotated.insert(object_id_with_nibbles(kind, &[(0, *nibble), (2, b'1')])?);
    }
    for nibble in b"0123456789abcdef" {
        annotated.insert(object_id_with_nibble(kind, 2, *nibble)?);
        annotated.insert(object_id_with_nibbles(kind, &[(2, *nibble), (4, b'1')])?);
    }
    for nibble in b"0123456789abcdef" {
        annotated.insert(object_id_with_nibble(kind, 4, *nibble)?);
        annotated.insert(object_id_with_nibbles(kind, &[(4, *nibble), (6, b'1')])?);
    }
    for object in &annotated {
        let outcome = gix_note::replace(root, *object, note, &objects)?;
        assert_eq!(outcome.previous, None, "every generated object is unique");
        root = outcome.tree;
    }

    let one_level = object_id_with_nibble(kind, 0, b'f')?;
    let two_levels = object_id_with_nibble(kind, 2, b'f')?;
    let three_levels = object_id_with_nibble(kind, 4, b'f')?;
    let surviving_after_collapse = object_id_with_nibble(kind, 4, b'e')?;
    let last_in_level_three = object_id_with_nibbles(kind, &[(4, b'f'), (6, b'1')])?;
    assert_note_layout(&objects, root, &one_level, 1, note)?;
    assert_note_layout(&objects, root, &two_levels, 2, note)?;
    assert_note_layout(&objects, root, &three_levels, 3, note)?;
    assert_note_layout(&objects, root, &last_in_level_three, 3, note)?;
    for object in &annotated {
        assert_eq!(
            gix_note::get(root, object, &objects)?,
            Some(note),
            "mixed fanout depths remain readable"
        );
    }

    let outcome = gix_note::remove(root, last_in_level_three, &objects)?;
    assert_eq!(outcome.previous, Some(note), "deep removal returns its note");
    assert_eq!(
        gix_note::get(outcome.tree, &last_in_level_three, &objects)?,
        None,
        "the deep mapping is removed"
    );
    assert_note_layout(&objects, outcome.tree, &three_levels, 2, note)?;

    let outcome = gix_note::remove(outcome.tree, three_levels, &objects)?;
    assert_eq!(
        outcome.previous,
        Some(note),
        "the second deep removal empties its bucket"
    );
    // Loading the edited subtree exposes its single remaining note, so Git's lazy heuristic collapses the deepest level.
    assert_note_layout(&objects, outcome.tree, &surviving_after_collapse, 2, note)?;
    assert_note_layout(&objects, outcome.tree, &two_levels, 2, note)?;
    assert_note_layout(&objects, outcome.tree, &one_level, 1, note)?;
    Ok(())
}

#[test]
fn mutations_preserve_non_notes_at_root_and_below_hex_trees() -> gix_testtools::Result {
    let kind = gix_testtools::object_hash();
    let objects = ObjectDb::new(gix_object::find::Never, kind);
    let payload = objects.write_buf(gix_object::Kind::Blob, b"keep")?;
    let nested = objects.write(&Tree {
        entries: vec![Entry {
            mode: EntryKind::Blob.into(),
            filename: "README".into(),
            oid: payload,
        }],
    })?;
    let existing_object = gix_object::compute_hash(kind, gix_object::Kind::Blob, b"existing")?;
    let full_hex_tree = existing_object.to_hex().to_string();
    let full_non_hex_blob = "g".repeat(kind.len_in_hex());
    let root = objects.write(&Tree {
        entries: vec![
            Entry {
                mode: EntryKind::Tree.into(),
                filename: full_hex_tree.clone().into(),
                oid: kind.empty_tree(),
            },
            Entry {
                mode: EntryKind::Tree.into(),
                filename: "CD".into(),
                oid: nested,
            },
            Entry {
                mode: EntryKind::BlobExecutable.into(),
                filename: "ab".into(),
                oid: payload,
            },
            Entry {
                mode: EntryKind::Blob.into(),
                filename: full_non_hex_blob.clone().into(),
                oid: payload,
            },
            Entry {
                mode: EntryKind::Tree.into(),
                filename: "zz".into(),
                oid: nested,
            },
        ],
    })?;
    let annotated = gix_object::compute_hash(kind, gix_object::Kind::Blob, b"new")?;
    let note = objects.write_buf(gix_object::Kind::Blob, b"note")?;
    let outcome = gix_note::replace(root, annotated, note, &objects)?;

    assert_entry_at_path(&objects, outcome.tree, &["ab"], EntryKind::BlobExecutable, payload)?;
    assert_entry_at_path(&objects, outcome.tree, &["CD", "README"], EntryKind::Blob, payload)?;
    assert_entry_at_path(
        &objects,
        outcome.tree,
        &[full_hex_tree.as_str()],
        EntryKind::Tree,
        kind.empty_tree(),
    )?;
    assert_entry_at_path(
        &objects,
        outcome.tree,
        &[full_non_hex_blob.as_str()],
        EntryKind::Blob,
        payload,
    )?;
    assert_entry_at_path(&objects, outcome.tree, &["zz"], EntryKind::Tree, nested)?;
    assert_eq!(
        gix_note::get(outcome.tree, &annotated, &objects)?,
        Some(note),
        "the new note coexists with all preserved entries"
    );
    Ok(())
}

#[test]
fn mutations_reject_mixed_hash_kinds_before_reading_objects() -> gix_testtools::Result {
    let objects = ObjectDb::new(gix_object::find::Never, Kind::Sha1);
    let root = objects.write(&Tree { entries: Vec::new() })?;
    let sha1 = ObjectId::null(Kind::Sha1);
    let sha256 = ObjectId::null(Kind::Sha256);

    let err =
        gix_note::replace(root, sha256, sha1, &objects).expect_err("the annotated object has the wrong hash kind");
    let err = err.into_error();
    assert!(
        err.is_validation(),
        "an annotated-object hash mismatch is a validation error"
    );
    assert_eq!(
        err.probable_cause().to_string(),
        "Notes, annotated objects, and their root tree must use the same hash kind",
        "replace reports an annotated-object hash mismatch"
    );
    let err = gix_note::replace(root, sha1, sha256, &objects).expect_err("the note has the wrong hash kind");
    let err = err.into_error();
    assert!(err.is_validation(), "a note hash mismatch is a validation error");
    assert_eq!(
        err.probable_cause().to_string(),
        "Notes, annotated objects, and their root tree must use the same hash kind",
        "replace reports a note hash mismatch"
    );
    let err = gix_note::remove(root, sha256, &objects).expect_err("the annotated object has the wrong hash kind");
    let err = err.into_error();
    assert!(
        err.is_validation(),
        "an annotated-object hash mismatch is a validation error"
    );
    assert_eq!(
        err.probable_cause().to_string(),
        "The annotated object and notes root tree must use the same hash kind",
        "remove reports an annotated-object hash mismatch"
    );
    Ok(())
}

#[test]
#[cfg(feature = "sha256")]
fn edits_support_sha256_notes_trees() -> gix_testtools::Result {
    let kind = Kind::Sha256;
    let objects = ObjectDb::new(gix_object::find::Never, kind);
    let root = objects.write(&Tree { entries: Vec::new() })?;
    let annotated = gix_object::compute_hash(kind, gix_object::Kind::Blob, b"annotated")?;
    let note = objects.write_buf(gix_object::Kind::Blob, b"note")?;

    let added = gix_note::replace(root, annotated, note, &objects)?;
    assert_eq!(added.previous, None, "the SHA-256 mapping is new");
    assert_eq!(added.tree.kind(), kind, "the rewritten root retains its hash kind");
    assert_eq!(
        gix_note::get(added.tree, &annotated, &objects)?,
        Some(note),
        "SHA-256 notes can be read after insertion"
    );
    let removed = gix_note::remove(added.tree, annotated, &objects)?;
    assert_eq!(removed.previous, Some(note), "SHA-256 removal returns the note");
    assert_eq!(
        removed.tree,
        kind.empty_tree(),
        "removing the last SHA-256 note yields its empty tree"
    );
    Ok(())
}

#[test]
fn mutations_reject_duplicate_mappings_across_layouts() -> gix_testtools::Result {
    let kind = gix_testtools::object_hash();
    let objects = ObjectDb::new(gix_object::find::Never, kind);
    let annotated = ObjectId::null(kind);
    let hex = annotated.to_hex().to_string();
    let flat_note = objects.write_buf(gix_object::Kind::Blob, b"flat")?;
    let fanout_note = objects.write_buf(gix_object::Kind::Blob, b"fanout")?;
    let subtree = objects.write(&Tree {
        entries: vec![Entry {
            mode: EntryKind::Blob.into(),
            filename: hex[2..].into(),
            oid: fanout_note,
        }],
    })?;
    let root = objects.write(&Tree {
        entries: vec![
            Entry {
                mode: EntryKind::Tree.into(),
                filename: hex[..2].into(),
                oid: subtree,
            },
            Entry {
                mode: EntryKind::Blob.into(),
                filename: hex.into(),
                oid: flat_note,
            },
        ],
    })?;
    let other = gix_object::compute_hash(kind, gix_object::Kind::Blob, b"other")?;

    let err = gix_note::replace(root, other, flat_note, &objects)
        .expect_err("ambiguous existing mappings cannot be rewritten losslessly");
    let err = err.into_error();
    assert!(err.is_corrupted(), "duplicate mappings indicate a corrupt notes tree");
    assert_eq!(
        err.probable_cause().to_string(),
        format!("Multiple notes map to object {annotated}"),
        "mutations diagnose duplicate flat and fanout mappings"
    );
    Ok(())
}

fn object_id_with_nibble(kind: Kind, offset: usize, nibble: u8) -> gix_testtools::Result<ObjectId> {
    object_id_with_nibbles(kind, &[(offset, nibble)])
}

fn object_id_with_nibbles(kind: Kind, nibbles: &[(usize, u8)]) -> gix_testtools::Result<ObjectId> {
    let mut hex = vec![b'0'; kind.len_in_hex()];
    for &(offset, nibble) in nibbles {
        hex[offset] = nibble;
    }
    Ok(ObjectId::from_hex(&hex)?)
}

fn assert_note_layout(
    objects: &ObjectDb,
    root: ObjectId,
    annotated: &oid,
    fanout: usize,
    note: ObjectId,
) -> gix_testtools::Result {
    let hex = annotated.to_hex().to_string();
    let mut path = Vec::with_capacity(fanout + 1);
    for level in 0..fanout {
        path.push(&hex[level * 2..][..2]);
    }
    path.push(&hex[fanout * 2..]);
    assert_entry_at_path(objects, root, &path, EntryKind::Blob, note)
}

fn assert_entry_at_path(
    objects: &ObjectDb,
    root: ObjectId,
    path: &[&str],
    expected_kind: EntryKind,
    expected_oid: ObjectId,
) -> gix_testtools::Result {
    let mut tree_id = root;
    let mut buf = Vec::new();
    for (index, component) in path.iter().enumerate() {
        let is_last = index + 1 == path.len();
        let expected_tree = if is_last {
            expected_kind == EntryKind::Tree
        } else {
            true
        };
        let tree = objects.find_tree(&tree_id, &mut buf)?;
        let entry = tree
            .bisect_entry(component.as_bytes().as_bstr(), expected_tree)
            .unwrap_or_else(|| panic!("entry at {} is present", path[..=index].join("/")));
        if is_last {
            assert_eq!(
                entry.mode.kind(),
                expected_kind,
                "entry at {} retains its mode",
                path.join("/")
            );
            assert_eq!(
                entry.oid,
                expected_oid,
                "entry at {} retains its object ID",
                path.join("/")
            );
        } else {
            assert!(
                entry.mode.is_tree(),
                "{} is an intermediate tree",
                path[..=index].join("/")
            );
            tree_id = entry.oid.to_owned();
        }
    }
    Ok(())
}
