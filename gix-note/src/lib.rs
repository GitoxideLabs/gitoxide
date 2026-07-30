//! Read Git notes from notes trees.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::{cmp::Ordering, collections::HashMap};

use gix_error::{ResultExt, message};
use gix_hash::{ObjectId, oid};
use gix_object::{
    Find, FindExt, Tree,
    bstr::{BStr, ByteSlice},
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
}
