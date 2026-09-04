//! Read Git notes from notes trees.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

use gix_error::{CorruptionError, ErrorExt, ResultExt, ValidationError, message};
use gix_hash::{ObjectId, Prefix, oid};
use gix_hashtable::{HashMap, HashSet};
use gix_object::{
    Find, FindExt, Tree, Write,
    bstr::{BStr, BString, ByteSlice},
    tree::{Editor, EntryKind, EntryMode},
};

/// The type-erased error returned by note operations.
pub type Error = gix_error::Exn;

/// The result of changing one note mapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Edit {
    /// The root tree containing the changed notes.
    pub tree: ObjectId,
    /// The object ID of the note previously associated with the annotated object.
    ///
    /// This is `Some` when [`replace()`] replaced an existing note or [`remove()`]
    /// removed one. It is `None` when adding a new mapping or when removal
    /// found no matching note. Note IDs are expected to reference blobs, but
    /// their object kind is not verified.
    pub previous: Option<ObjectId>,
}

/// Return the note associated with `object` in the notes tree at `root`.
///
/// Git notes are expected to reference blobs. This function verifies that the
/// notes-tree entry has blob mode, but does not load the referenced object to
/// verify its actual kind.
///
/// Trees are loaded lazily along the progressive two-hex-digit fanout path.
/// Entries that do not conform to Git's notes layout are ignored.
///
/// For repeated lookups, `objects` should have a built-in object cache to
/// accelerate tree retrieval.
pub fn get(root_tree_id: ObjectId, annotated_object_id: &oid, objects: &impl Find) -> Result<Option<ObjectId>, Error> {
    let prefix = annotated_object_id.to_prefix(0..annotated_object_id.as_bytes().len());
    let mut hex = gix_hash::Kind::hex_buf();
    let mut remaining = prefix.hex_to_buf(&mut hex).as_bytes().as_bstr();
    let mut tree_id = root_tree_id;
    let mut buf = Vec::new();

    loop {
        let tree = objects
            .find_tree(&tree_id, &mut buf)
            .or_raise_erased(|| message!("Could not load notes tree {tree_id}"))?;
        if let Some(entry) = tree.bisect_entry(remaining, false).filter(|entry| entry.mode.is_blob()) {
            return Ok(Some(entry.oid.to_owned()));
        }
        let Some(component) = remaining.get(..2).filter(|_| remaining.len() > 2) else {
            return Ok(None);
        };
        let Some(subtree) = tree
            .bisect_entry(component.into(), true)
            .filter(|entry| entry.mode.is_tree())
        else {
            return Ok(None);
        };
        tree_id = subtree.oid.to_owned();
        remaining = remaining[2..].as_bstr();
    }
}

/// Replace the note for `object`, or add it if absent, returning the new root
/// tree and any previous note.
///
/// The notes tree is rewritten with the same progressive fanout heuristic as
/// Git while retaining entries that are not notes. `note` is expected to
/// reference a blob, but its actual object kind is not verified; the mapping is
/// always written as a blob-mode tree entry. For repeated edits, `objects`
/// should have a built-in object cache to accelerate tree retrieval.
pub fn replace(
    root_tree_id: ObjectId,
    annotated_object_id: ObjectId,
    note_blob_id: ObjectId,
    objects: &(impl Find + Write),
) -> Result<Edit, Error> {
    if annotated_object_id.kind() != root_tree_id.kind() || note_blob_id.kind() != root_tree_id.kind() {
        return Err(
            ValidationError::from("Notes, annotated objects, and their root tree must use the same hash kind")
                .raise_erased(),
        );
    }
    edit(root_tree_id, annotated_object_id, Some(note_blob_id), objects)
}

/// Remove the note for `object`, returning the new root tree and removed note.
///
/// If there is no such note, the root is returned unchanged. For repeated
/// edits, `objects` should have a built-in object cache to accelerate tree
/// retrieval.
pub fn remove(
    root_tree_id: ObjectId,
    annotated_object_id: ObjectId,
    objects: &(impl Find + Write),
) -> Result<Edit, Error> {
    if annotated_object_id.kind() != root_tree_id.kind() {
        return Err(
            ValidationError::from("The annotated object and notes root tree must use the same hash kind")
                .raise_erased(),
        );
    }
    edit(root_tree_id, annotated_object_id, None, objects)
}

fn edit(
    root_tree_id: ObjectId,
    annotated_object_id: ObjectId,
    note_blob_id: Option<ObjectId>,
    objects: &(impl Find + Write),
) -> Result<Edit, Error> {
    let mut notes = HashMap::default();
    let mut non_notes = Vec::new();
    let mut existing_fanout = HashSet::default();
    collect(
        root_tree_id,
        BString::default(),
        Vec::new(),
        objects,
        &mut notes,
        &mut non_notes,
        &mut existing_fanout,
    )?;
    let previous_note_blob_id = match note_blob_id {
        Some(note_blob_id) => notes.insert(annotated_object_id, note_blob_id),
        None => notes.remove(&annotated_object_id),
    };
    if note_blob_id.is_none() && previous_note_blob_id.is_none() {
        return Ok(Edit {
            tree: root_tree_id,
            previous: previous_note_blob_id,
        });
    }
    let root_tree_id = write(notes, non_notes, existing_fanout, root_tree_id.kind(), objects)?;
    Ok(Edit {
        tree: root_tree_id,
        previous: previous_note_blob_id,
    })
}

#[derive(Clone)]
struct NonNote {
    path: Vec<BString>,
    mode: EntryMode,
    object_id: ObjectId,
}

fn collect(
    tree_id: ObjectId,
    hex_prefix: BString,
    path_prefix: Vec<BString>,
    objects: &impl Find,
    notes: &mut HashMap<ObjectId, ObjectId>,
    non_notes: &mut Vec<NonNote>,
    existing_fanout: &mut HashSet<FanoutPrefix>,
) -> Result<(), Error> {
    let mut buf = Vec::new();
    let tree = objects
        .find_tree(&tree_id, &mut buf)
        .or_raise_erased(|| message!("Could not load notes tree {tree_id}"))?;
    let hex_len = tree_id.kind().len_in_hex();
    for entry in tree.entries {
        let mut path = path_prefix.clone();
        path.push(entry.filename.to_owned());
        if entry.mode.is_blob() && entry.filename.len() + hex_prefix.len() == hex_len {
            let mut hex = hex_prefix.clone();
            hex.extend_from_slice(entry.filename);
            if let Ok(annotated_object_id) = ObjectId::from_hex(&hex) {
                for offset in 0..hex_prefix.len() / 2 {
                    existing_fanout.insert(FanoutPrefix(annotated_object_id.to_prefix(0..offset)));
                }
                if notes.insert(annotated_object_id, entry.oid.to_owned()).is_some() {
                    return Err(
                        CorruptionError::from(format!("Multiple notes map to object {annotated_object_id}"))
                            .raise_erased(),
                    );
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
            prefix.extend_from_slice(entry.filename);
            collect(
                entry.oid.to_owned(),
                prefix,
                path,
                objects,
                notes,
                non_notes,
                existing_fanout,
            )?;
        } else {
            non_notes.push(NonNote {
                path,
                mode: entry.mode,
                object_id: entry.oid.to_owned(),
            });
        }
    }
    Ok(())
}

fn write(
    notes: HashMap<ObjectId, ObjectId>,
    non_notes: Vec<NonNote>,
    existing_fanout: HashSet<FanoutPrefix>,
    hash: gix_hash::Kind,
    objects: &(impl Find + Write),
) -> Result<ObjectId, Error> {
    // A notes path contains the full hexadecimal ID plus one slash for each byte consumed as a fanout directory.
    // At least one byte remains for the leaf name, so allowing one slash per hash byte is a simple one-byte overestimate.
    const NOTE_PATH_BUFFER_SIZE: usize =
        gix_hash::Kind::longest().len_in_hex() + gix_hash::Kind::longest().len_in_bytes();

    let mut editor = Editor::new(Tree { entries: Vec::new() }, objects, hash);
    for entry in non_notes {
        editor
            .upsert(entry.path.iter(), entry.mode.kind(), entry.object_id)
            .or_raise_erased(|| message("Could not restore a non-note tree entry"))?;
    }

    let bucket_counts = fanout_bucket_counts(notes.keys());
    let mut path_buf = [0u8; NOTE_PATH_BUFFER_SIZE];
    for (annotated_object_id, note_blob_id) in notes {
        let fanout = fanout(&annotated_object_id, &bucket_counts, &existing_fanout);
        let path = note_path(&annotated_object_id, fanout, &mut path_buf);
        editor
            .upsert(path.split_str("/"), EntryKind::Blob, note_blob_id)
            .or_raise_erased(|| message("Could not add a note tree entry"))?;
    }
    editor
        .write(|tree| objects.write(tree))
        .or_raise_erased(|| message("Could not write the notes tree"))
}

/// Return the next-nibble note counts for every possible two-hex-digit fanout prefix.
///
/// `ids` yields the annotated object IDs that will become note-tree entry names; the IDs of their note blobs do not
/// affect fanout.
///
/// The returned map associates each possible byte-aligned hexadecimal prefix with 16 counters, one for each possible value
/// of the following nibble. Counts saturate at two because Git creates the next fanout level only when all 16 slots at a
/// candidate level represent internal nodes, which requires at least two distinct notes per slot.
fn fanout_bucket_counts<'a>(ids: impl Iterator<Item = &'a ObjectId>) -> HashMap<FanoutPrefix, [u8; 16]> {
    let mut out = HashMap::default();
    for id in ids {
        for offset in 0..id.as_bytes().len().saturating_sub(1) {
            let counts = out.entry(FanoutPrefix(id.to_prefix(0..offset))).or_insert([0u8; 16]);
            let nibble = id.as_bytes()[offset] >> 4;
            let index = usize::from(nibble);
            counts[index] = counts[index].saturating_add(1).min(2);
        }
    }
    out
}

/// Determine how many leading bytes of `id` become two-hex-digit fanout directories.
///
/// `bucket_counts` describes the populated next-nibble buckets below every prefix. `existing_fanout` identifies prefixes
/// that were already directories before the edit. Git creates a level only once every bucket contains at least two notes,
/// but retains an existing level while every bucket remains occupied; this hysteresis avoids tree churn around the creation
/// threshold. The returned number is both the fanout depth and the byte offset at which the remaining hexadecimal ID becomes
/// the note's leaf name.
fn fanout(id: &oid, bucket_counts: &HashMap<FanoutPrefix, [u8; 16]>, existing_fanout: &HashSet<FanoutPrefix>) -> usize {
    let mut fanout = 0;
    while fanout < id.as_bytes().len().saturating_sub(1) {
        let prefix = FanoutPrefix(id.to_prefix(0..fanout));
        let should_fanout = bucket_counts.get(&prefix).is_some_and(|counts| {
            counts.iter().all(|count| *count == 2)
                || (existing_fanout.contains(&prefix) && counts.iter().all(|count| *count >= 1))
        });
        if !should_fanout {
            break;
        }
        fanout += 1;
    }
    fanout
}

/// A prefix key whose hash is compatible with `gix_hashtable`'s object-ID-specialized hasher.
///
/// Equality includes the prefix length through [`Prefix`], while hashing writes only its zero-padded object-ID bytes.
/// Different lengths can therefore collide when their significant bytes are all zero, but equality still distinguishes
/// them and avoids requiring a separate hash table for each prefix depth.
#[derive(Clone, Copy, Eq, PartialEq)]
struct FanoutPrefix(Prefix);

impl std::hash::Hash for FanoutPrefix {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(self.0.as_oid(), state);
    }
}

/// Write the notes-tree path for `id` with `fanout` leading bytes represented as directory components.
///
/// For an ID beginning with `01234567…`, fanout `0` produces `01234567…`, fanout `1` produces `01/234567…`, and
/// fanout `2` produces `01/23/4567…`. The returned path borrows the initialized portion of `out`.
fn note_path<'a>(id: &oid, fanout: usize, out: &'a mut [u8]) -> &'a BStr {
    let mut pos = 0;
    for offset in 0..fanout {
        let component = id.to_prefix(offset..offset + 1);
        pos += component.hex_to_buf(&mut out[pos..]).len();
        out[pos] = b'/';
        pos += 1;
    }
    let remainder = id.to_prefix(fanout..id.as_bytes().len());
    pos += remainder.hex_to_buf(&mut out[pos..]).len();
    BStr::new(&out[..pos])
}
