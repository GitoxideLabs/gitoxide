//! Read Git notes from notes trees.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap},
};

use gix_error::{ErrorExt, ResultExt, message};
use gix_hash::{ObjectId, oid};
use gix_object::{
    Find, FindExt, Tree, Write,
    bstr::{BStr, BString, ByteSlice},
    tree::{Editor, EntryKind, EntryMode},
};

/// The error returned by note operations.
pub type Error = gix_error::Exn<gix_error::Message>;

/// Decoded trees retained across note lookups.
///
/// The cache is independent of a particular notes root, allowing callers to
/// reuse it for all configured notes references.
#[derive(Default)]
pub struct Cache {
    trees: HashMap<ObjectId, Tree>,
    buf: Vec<u8>,
}

impl Cache {
    fn tree(&mut self, id: ObjectId, objects: &impl Find) -> Result<&Tree, Error> {
        if !self.trees.contains_key(&id) {
            let tree = objects
                .find_tree(&id, &mut self.buf)
                .or_raise(|| message!("Could not load notes tree {id}"))?
                .into_owned();
            self.trees.insert(id, tree);
        }
        Ok(self.trees.get(&id).expect("tree was inserted or already present"))
    }
}

/// The result of changing one note mapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Edit {
    /// The root tree containing the changed notes.
    pub tree: ObjectId,
    /// The note which was replaced or removed.
    pub previous: Option<ObjectId>,
}

/// Return the blob associated with `object` in the notes tree at `root`.
///
/// Trees are loaded lazily along the progressive two-hex-digit fanout path and
/// retained in `cache`. Entries that do not conform to Git's notes layout are
/// ignored.
pub fn get(root: ObjectId, object: &oid, objects: &impl Find, cache: &mut Cache) -> Result<Option<ObjectId>, Error> {
    let hex = object.to_hex().to_string();
    let mut remaining = hex.as_bytes().as_bstr();
    let mut tree_id = root;

    loop {
        let tree = cache.tree(tree_id, objects)?;
        if let Some(entry) = entry(tree, remaining, false).filter(|entry| entry.mode.is_blob()) {
            return Ok(Some(entry.oid));
        }
        let Some(component) = remaining.get(..2).filter(|_| remaining.len() > 2) else {
            return Ok(None);
        };
        let Some(subtree) = entry(tree, BStr::new(component), true).filter(|entry| entry.mode.is_tree()) else {
            return Ok(None);
        };
        tree_id = subtree.oid;
        remaining = remaining[2..].as_bstr();
    }
}

/// Add or replace the note for `object`, returning the new root tree and any
/// previous note.
///
/// The notes tree is rewritten with the same progressive fanout heuristic as
/// Git while retaining entries that are not notes.
pub fn add(
    root: ObjectId,
    object: ObjectId,
    note: ObjectId,
    objects: &(impl Find + Write),
    cache: &mut Cache,
) -> Result<Edit, Error> {
    if object.kind() != root.kind() || note.kind() != root.kind() {
        return Err(message("Notes, annotated objects, and their root tree must use the same hash kind").raise());
    }
    edit(root, object, Some(note), objects, cache)
}

/// Remove the note for `object`, returning the new root tree and removed note.
///
/// If there is no such note, the root is returned unchanged.
pub fn remove(
    root: ObjectId,
    object: ObjectId,
    objects: &(impl Find + Write),
    cache: &mut Cache,
) -> Result<Edit, Error> {
    if object.kind() != root.kind() {
        return Err(message("The annotated object and notes root tree must use the same hash kind").raise());
    }
    edit(root, object, None, objects, cache)
}

fn edit(
    root: ObjectId,
    object: ObjectId,
    note: Option<ObjectId>,
    objects: &(impl Find + Write),
    cache: &mut Cache,
) -> Result<Edit, Error> {
    let mut notes = BTreeMap::new();
    let mut non_notes = Vec::new();
    collect(
        root,
        BString::default(),
        Vec::new(),
        objects,
        cache,
        &mut notes,
        &mut non_notes,
    )?;
    let previous = match note {
        Some(note) => notes.insert(object, note),
        None => notes.remove(&object),
    };
    if note.is_none() && previous.is_none() {
        return Ok(Edit { tree: root, previous });
    }
    let tree = write(notes, non_notes, root.kind(), objects)?;
    Ok(Edit { tree, previous })
}

#[derive(Clone)]
struct NonNote {
    path: Vec<BString>,
    mode: EntryMode,
    oid: ObjectId,
}

fn collect(
    tree_id: ObjectId,
    hex_prefix: BString,
    path_prefix: Vec<BString>,
    objects: &impl Find,
    cache: &mut Cache,
    notes: &mut BTreeMap<ObjectId, ObjectId>,
    non_notes: &mut Vec<NonNote>,
) -> Result<(), Error> {
    let entries = cache.tree(tree_id, objects)?.entries.clone();
    let hex_len = tree_id.kind().len_in_hex();
    for entry in entries {
        let mut path = path_prefix.clone();
        path.push(entry.filename.clone());
        if entry.mode.is_blob() && entry.filename.len() + hex_prefix.len() == hex_len {
            let mut hex = hex_prefix.clone();
            hex.extend_from_slice(&entry.filename);
            if let Ok(object) = ObjectId::from_hex(&hex) {
                if notes.insert(object, entry.oid).is_some() {
                    return Err(message!("Multiple notes map to object {object}").raise());
                }
                continue;
            }
        }
        if entry.mode.is_tree()
            && entry.filename.len() == 2
            && hex_prefix.len() + 2 < hex_len
            && entry.filename.iter().all(u8::is_ascii_hexdigit)
        {
            let mut prefix = hex_prefix.clone();
            prefix.extend_from_slice(&entry.filename);
            collect(entry.oid, prefix, path, objects, cache, notes, non_notes)?;
        } else {
            non_notes.push(NonNote {
                path,
                mode: entry.mode,
                oid: entry.oid,
            });
        }
    }
    Ok(())
}

fn write(
    notes: BTreeMap<ObjectId, ObjectId>,
    non_notes: Vec<NonNote>,
    hash: gix_hash::Kind,
    objects: &(impl Find + Write),
) -> Result<ObjectId, Error> {
    let mut editor = Editor::new(Tree { entries: Vec::new() }, objects, hash);
    for entry in non_notes {
        editor
            .upsert(entry.path.iter(), entry.mode.kind(), entry.oid)
            .or_raise(|| message("Could not restore a non-note tree entry"))?;
    }

    let hexes: Vec<_> = notes.keys().map(|id| id.to_hex().to_string()).collect();
    let masks = fanout_masks(&hexes);
    for ((_, note), hex) in notes.into_iter().zip(hexes) {
        let fanout = fanout(&hex, &masks);
        let path = note_path(&hex, fanout);
        editor
            .upsert(path.split_str("/"), EntryKind::Blob, note)
            .or_raise(|| message("Could not add a note tree entry"))?;
    }
    editor
        .write(|tree| {
            objects
                .write(tree)
                .map_err(|err| message!("Could not write tree object: {err}").raise())
        })
        .or_raise(|| message("Could not write the notes tree"))
}

fn fanout_masks(hexes: &[String]) -> HashMap<BString, u16> {
    let mut out = HashMap::new();
    for hex in hexes {
        let bytes = hex.as_bytes();
        for offset in (0..bytes.len().saturating_sub(2)).step_by(2) {
            let Some(nibble) = hex_nibble(bytes[offset]) else {
                continue;
            };
            *out.entry(BString::from(&bytes[..offset])).or_default() |= 1 << nibble;
        }
    }
    out
}

fn fanout(hex: &str, masks: &HashMap<BString, u16>) -> usize {
    let mut fanout = 0;
    while fanout * 2 < hex.len().saturating_sub(2)
        && masks.get(BStr::new(&hex.as_bytes()[..fanout * 2])) == Some(&u16::MAX)
    {
        fanout += 1;
    }
    fanout
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn note_path(hex: &str, fanout: usize) -> BString {
    let mut out = BString::new(Vec::with_capacity(hex.len() + fanout));
    for component in hex.as_bytes()[..fanout * 2].chunks_exact(2) {
        out.extend_from_slice(component);
        out.push(b'/');
    }
    out.extend_from_slice(&hex.as_bytes()[fanout * 2..]);
    out
}

fn entry<'a>(tree: &'a Tree, name: &BStr, is_tree: bool) -> Option<&'a gix_object::tree::Entry> {
    tree.entries
        .binary_search_by(|candidate| cmp_entry_with_name(candidate, name, is_tree))
        .ok()
        .map(|index| &tree.entries[index])
}

fn cmp_entry_with_name(entry: &gix_object::tree::Entry, name: &BStr, is_tree: bool) -> Ordering {
    let common = entry.filename.len().min(name.len());
    entry.filename[..common].cmp(&name[..common]).then_with(|| {
        let entry = entry
            .filename
            .get(common)
            .or_else(|| entry.mode.is_tree().then_some(&b'/'));
        let name = name.get(common).or_else(|| is_tree.then_some(&b'/'));
        entry.cmp(&name)
    })
}

#[cfg(test)]
mod tests {
    use gix_hash::Kind;
    use gix_object::{
        Write,
        bstr::BString,
        tree::{Entry, EntryKind},
    };

    use super::*;

    #[test]
    fn lazily_reads_fanout_trees_and_reuses_them() -> gix_testtools::Result {
        let objects = gix_odb::memory::Proxy::new(gix_object::find::Never, Kind::Sha1);
        let annotated = gix_object::compute_hash(Kind::Sha1, gix_object::Kind::Blob, b"annotated")?;
        let note = objects.write_buf(gix_object::Kind::Blob, b"note")?;
        let hex = annotated.to_hex().to_string();
        let subtree = objects.write(&Tree {
            entries: vec![Entry {
                mode: EntryKind::Blob.into(),
                filename: BString::from(&hex[2..]),
                oid: note,
            }],
        })?;
        let root = objects.write(&Tree {
            entries: vec![Entry {
                mode: EntryKind::Tree.into(),
                filename: BString::from(&hex[..2]),
                oid: subtree,
            }],
        })?;

        let mut cache = Cache::default();
        assert_eq!(
            get(root, &annotated, &objects, &mut cache).map_err(gix_error::Exn::into_error)?,
            Some(note),
            "the note is found through its fanout path"
        );
        assert_eq!(cache.trees.len(), 2, "only the root and matching subtree were loaded");
        assert_eq!(
            get(root, &annotated, &objects, &mut cache).map_err(gix_error::Exn::into_error)?,
            Some(note),
            "the same lookup remains stable"
        );
        assert_eq!(cache.trees.len(), 2, "repeated lookups reuse decoded trees");
        Ok(())
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
                oid: ObjectId::empty_tree(Kind::Sha1),
            }],
        })?;

        assert_eq!(
            get(root, &annotated, &objects, &mut Cache::default()).map_err(gix_error::Exn::into_error)?,
            None,
            "a tree at a note leaf is not a note"
        );
        Ok(())
    }

    #[test]
    fn mutations_rebalance_like_git_and_preserve_non_notes() -> gix_testtools::Result {
        let objects = gix_odb::memory::Proxy::new(gix_object::find::Never, Kind::Sha1);
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
        let mut cache = Cache::default();
        for nibble in b"0123456789abcdef" {
            let mut hex = vec![b'0'; Kind::Sha1.len_in_hex()];
            hex[0] = *nibble;
            let object = ObjectId::from_hex(&hex)?;
            annotated.push(object);
            root = add(root, object, note, &objects, &mut cache)
                .map_err(gix_error::Exn::into_error)?
                .tree;
        }

        let mut buf = Vec::new();
        let tree = objects.find_tree(&root, &mut buf)?;
        assert_eq!(
            tree.entries.iter().filter(|entry| entry.mode.is_tree()).count(),
            16,
            "covering all first nibbles causes Git's first fanout level"
        );
        assert!(
            tree.entries
                .iter()
                .any(|entry| entry.filename == "README" && entry.oid == unrelated),
            "non-note entries survive rebalancing"
        );

        let replacement = objects.write_buf(gix_object::Kind::Blob, b"replacement")?;
        let outcome = add(root, annotated[0], replacement, &objects, &mut cache).map_err(gix_error::Exn::into_error)?;
        assert_eq!(
            outcome.previous,
            Some(note),
            "overwriting naturally returns the previous note"
        );
        let outcome = remove(outcome.tree, annotated[0], &objects, &mut cache).map_err(gix_error::Exn::into_error)?;
        assert_eq!(outcome.previous, Some(replacement), "removal returns the removed note");
        let mut buf = Vec::new();
        let tree = objects.find_tree(&outcome.tree, &mut buf)?;
        assert_eq!(
            tree.entries.iter().filter(|entry| entry.mode.is_blob()).count(),
            16,
            "dropping one leading nibble collapses the remaining notes to the root beside README"
        );
        Ok(())
    }
}
