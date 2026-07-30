//! Read Git notes from notes trees.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::cmp::Ordering;

use gix_error::{ResultExt, message};
use gix_hash::{ObjectId, oid};
use gix_object::{
    Find, FindExt, TreeRef,
    bstr::{BStr, ByteSlice},
};

/// The error returned by note operations.
pub type Error = gix_error::Exn<gix_error::Message>;

/// Return the blob associated with `object` in the notes tree at `root`.
///
/// Trees are loaded lazily along the progressive two-hex-digit fanout path.
/// Entries that do not conform to Git's notes layout are ignored.
///
/// For repeated lookups, `objects` should have a built-in object cache to
/// accelerate tree retrieval.
pub fn get(root: ObjectId, object: &oid, objects: &impl Find) -> Result<Option<ObjectId>, Error> {
    let hex = object.to_hex().to_string();
    let mut remaining = hex.as_bytes().as_bstr();
    let mut tree_id = root;
    let mut buf = Vec::new();

    loop {
        let tree = objects
            .find_tree(&tree_id, &mut buf)
            .or_raise(|| message!("Could not load notes tree {tree_id}"))?;
        if let Some(entry) = entry(&tree, remaining, false).filter(|entry| entry.mode.is_blob()) {
            return Ok(Some(entry.oid.to_owned()));
        }
        let Some(component) = remaining.get(..2).filter(|_| remaining.len() > 2) else {
            return Ok(None);
        };
        let Some(subtree) = entry(&tree, BStr::new(component), true).filter(|entry| entry.mode.is_tree()) else {
            return Ok(None);
        };
        tree_id = subtree.oid.to_owned();
        remaining = remaining[2..].as_bstr();
    }
}

fn entry<'tree, 'data>(
    tree: &'tree TreeRef<'data>,
    name: &BStr,
    is_tree: bool,
) -> Option<&'tree gix_object::tree::EntryRef<'data>> {
    tree.entries
        .binary_search_by(|candidate| cmp_entry_with_name(candidate, name, is_tree))
        .ok()
        .map(|index| &tree.entries[index])
}

fn cmp_entry_with_name(entry: &gix_object::tree::EntryRef<'_>, name: &BStr, is_tree: bool) -> Ordering {
    gix_object::tree::name_order(entry.filename, entry.mode.is_tree(), name, is_tree)
}
