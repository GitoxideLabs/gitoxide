//! ## About `debug_assert!()
//!
//! The idea is to have code that won't panic in production. Thus, if in production that assertion would fail,
//! we will rather let the code run and hope it will either be correct enough or fail in more graceful ways later.
//!
//! Once such a case becomes a bug and is reproduced in testing, the debug-assertion will kick in and hopefully
//! contribute to finding a fix faster.
use std::collections::HashMap;

use bstr::{BStr, BString, ByteSlice, ByteVec};
use gix_diff::tree_with_rewrites::{Change, ChangeRef};
use gix_hash::ObjectId;
use gix_object::{
    tree,
    tree::{EntryKind, EntryMode},
};

use crate::{
    blob::{builtin_driver::binary::Pick, ResourceKind},
    tree::{
        Conflict, ConflictIndexEntry, ConflictIndexEntryPathHint, ConflictMapping, Error, Options, Resolution,
        ResolutionFailure,
    },
};

/// Assuming that `their_location` is the destination of *their* rewrite, check if *it* passes
/// over a directory rewrite in *our* tree. If so, rewrite it so that we get the path
/// it would have had if it had been renamed along with *our* directory.
pub fn possibly_rewritten_location(
    check_tree: &TreeNodes,
    their_location: &BStr,
    our_changes: &ChangeListRef,
) -> Option<BString> {
    check_tree.check_conflict(their_location).and_then(|pc| match pc {
        PossibleConflict::PassedRewrittenDirectory { change_idx } => {
            let passed_change = &our_changes[change_idx];
            rewrite_location_with_renamed_directory(their_location, &passed_change.inner)
        }
        _ => None,
    })
}

pub fn rewrite_location_with_renamed_directory(their_location: &BStr, passed_change: &Change) -> Option<BString> {
    match passed_change {
        Change::Rewrite {
            source_location,
            location,
            ..
        } if passed_change.entry_mode().is_tree() => {
            // This is safe even without dealing with slashes as we found this rewrite
            // by walking each component, and we know it's a tree for added safety.
            let suffix = their_location.strip_prefix(source_location.as_bytes())?;
            let mut rewritten = location.to_owned();
            rewritten.push_str(suffix);
            Some(rewritten)
        }
        _ => None,
    }
}

/// Produce a unique path within the directory that contains the file at `file_path` like `a/b`, using `editor`
/// and `tree` to assure unique names, to obtain the tree at `a/` and `side_name` to more clearly signal
/// where the file is coming from.
pub fn unique_path_in_tree(
    file_path: &BStr,
    editor: &tree::Editor<'_>,
    tree: &TreeNodes,
    side_name: &BStr,
) -> Result<BString, Error> {
    let mut buf = file_path.to_owned();
    buf.push(b'~');
    buf.extend(
        side_name
            .as_bytes()
            .iter()
            .copied()
            .map(|b| if b == b'/' { b'_' } else { b }),
    );

    // We could use a cursor here, but clashes are so unlikely that this wouldn't be meaningful for performance.
    let base_len = buf.len();
    let mut suffix = 0;
    while editor.get(to_components_bstring_ref(&buf)).is_some() || tree.check_conflict(buf.as_bstr()).is_some() {
        buf.truncate(base_len);
        buf.push_str(format!("_{suffix}"));
        suffix += 1;
    }
    Ok(buf)
}

/// Perform a merge between two blobs and return the result of its object id.
#[allow(clippy::too_many_arguments)]
pub fn perform_blob_merge<E>(
    mut labels: crate::blob::builtin_driver::text::Labels<'_>,
    objects: &impl gix_object::FindObjectOrHeader,
    blob_merge: &mut crate::blob::Platform,
    buf: &mut Vec<u8>,
    write_blob_to_odb: &mut impl FnMut(&[u8]) -> Result<ObjectId, E>,
    (our_location, our_id, our_mode): (&BString, ObjectId, EntryMode),
    (their_location, their_id, their_mode): (&BString, ObjectId, EntryMode),
    (previous_location, previous_id, previous_mode): (&BString, ObjectId, EntryMode),
    (extra_markers, outer_side): (u8, ConflictMapping),
    options: &Options,
) -> Result<(ObjectId, crate::blob::Resolution), Error>
where
    E: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
{
    if our_id == their_id {
        // This can happen if the merge modes are different.
        debug_assert_ne!(
            our_mode, their_mode,
            "BUG: we must think anything has to be merged if the modes and the ids are the same"
        );
        return Ok((their_id, crate::blob::Resolution::Complete));
    }
    if matches!(our_mode.kind(), EntryKind::Link) && matches!(their_mode.kind(), EntryKind::Link) {
        let (pick, resolution) = crate::blob::builtin_driver::binary(options.symlink_conflicts);
        let (our_id, their_id) = match outer_side {
            ConflictMapping::Original => (our_id, their_id),
            ConflictMapping::Swapped => (their_id, our_id),
        };
        let id = match pick {
            Pick::Ancestor => previous_id,
            Pick::Ours => our_id,
            Pick::Theirs => their_id,
        };
        return Ok((id, resolution));
    }
    let (our_kind, their_kind) = match outer_side {
        ConflictMapping::Original => (ResourceKind::CurrentOrOurs, ResourceKind::OtherOrTheirs),
        ConflictMapping::Swapped => (ResourceKind::OtherOrTheirs, ResourceKind::CurrentOrOurs),
    };
    blob_merge.set_resource(our_id, our_mode.kind(), our_location.as_bstr(), our_kind, objects)?;
    blob_merge.set_resource(
        their_id,
        their_mode.kind(),
        their_location.as_bstr(),
        their_kind,
        objects,
    )?;
    blob_merge.set_resource(
        previous_id,
        previous_mode.kind(),
        previous_location.as_bstr(),
        ResourceKind::CommonAncestorOrBase,
        objects,
    )?;

    fn combined(side: &BStr, location: &BString) -> BString {
        let mut buf = side.to_owned();
        buf.push_byte(b':');
        buf.push_str(location);
        buf
    }

    if outer_side.is_swapped() {
        (labels.current, labels.other) = (labels.other, labels.current);
    }

    let (ancestor, current, other);
    let labels = if our_location == their_location {
        labels
    } else {
        ancestor = labels.ancestor.map(|side| combined(side, previous_location));
        current = labels.current.map(|side| combined(side, our_location));
        other = labels.other.map(|side| combined(side, their_location));
        crate::blob::builtin_driver::text::Labels {
            ancestor: ancestor.as_ref().map(|n| n.as_bstr()),
            current: current.as_ref().map(|n| n.as_bstr()),
            other: other.as_ref().map(|n| n.as_bstr()),
        }
    };
    let prep = blob_merge.prepare_merge(objects, with_extra_markers(options, extra_markers))?;
    let (pick, resolution) = prep.merge(buf, labels, &options.blob_merge_command_ctx)?;

    let merged_blob_id = prep
        .id_by_pick(pick, buf, write_blob_to_odb)
        .map_err(|err| Error::WriteBlobToOdb(err.into()))?
        .ok_or(Error::MergeResourceNotFound)?;
    Ok((merged_blob_id, resolution))
}

fn with_extra_markers(opts: &Options, extra_makers: u8) -> crate::blob::platform::merge::Options {
    let mut out = opts.blob_merge;
    if let crate::blob::builtin_driver::text::Conflict::Keep { marker_size, .. } = &mut out.text.conflict {
        *marker_size =
            marker_size.saturating_add(extra_makers.saturating_add(opts.marker_size_multiplier.saturating_mul(2)));
    }
    out
}

/// A way to attach metadata to each change.
#[derive(Debug)]
pub struct TrackedChange {
    /// The actual change
    pub inner: Change,
    /// If `true`, this change counts as written to the tree using a [`tree::Editor`].
    pub was_written: bool,
    /// If `Some(ours_idx_to_ignore)`, this change must be placed into the tree before handling it.
    /// This makes sure that new changes aren't visible too early, which would mean the algorithm
    /// knows things too early which can be misleading.
    /// The `ours_idx_to_ignore` assures that the same rewrite won't be used as matching side, which
    /// would lead to strange effects. Only set if it's a rewrite though.
    pub needs_tree_insertion: Option<Option<usize>>,
    /// A new `(location, change_idx)` pair for the change that can happen if the location is touching a rewrite in a parent
    /// directory, but otherwise doesn't have a match. This means we shall redo the operation but with
    /// the changed path.
    /// The second tuple entry `change_idx` is the change-idx we passed over, which refers to the other side that interfered.
    pub rewritten_location: Option<(BString, usize)>,
}

pub type ChangeList = Vec<TrackedChange>;
pub type ChangeListRef = [TrackedChange];

/// Only keep leaf nodes, or trees that are the renamed, pushing `change` on `changes`.
/// Doing so makes it easy to track renamed or rewritten or copied directories, and properly
/// handle *their* changes that fall within them.
/// Note that it also rewrites `change` if it is a copy, turning it into an addition so copies don't have an effect
/// on the merge algorithm.
pub fn track(change: ChangeRef<'_>, changes: &mut ChangeList) {
    if change.entry_mode().is_tree() && matches!(change, ChangeRef::Modification { .. }) {
        return;
    }
    let is_tree = change.entry_mode().is_tree();
    changes.push(TrackedChange {
        inner: match change.into_owned() {
            Change::Rewrite {
                id,
                entry_mode,
                location,
                relation,
                copy,
                ..
            } if copy => Change::Addition {
                location,
                relation,
                entry_mode,
                id,
            },
            other => other,
        },
        was_written: is_tree,
        needs_tree_insertion: None,
        rewritten_location: None,
    });
}

/// Unconditionally apply `change` to `editor`.
pub fn apply_change(
    editor: &mut tree::Editor<'_>,
    change: &Change,
    alternative_location: Option<&BString>,
) -> Result<(), tree::editor::Error> {
    use to_components_bstring_ref as to_components;
    if change.entry_mode().is_tree() {
        return Ok(());
    }

    let (location, mode, id) = match change {
        Change::Addition {
            location,
            entry_mode,
            id,
            ..
        }
        | Change::Modification {
            location,
            entry_mode,
            id,
            ..
        } => (location, entry_mode, id),
        Change::Deletion { location, .. } => {
            editor.remove(to_components(alternative_location.unwrap_or(location)))?;
            return Ok(());
        }
        Change::Rewrite {
            source_location,
            entry_mode,
            id,
            location,
            copy,
            ..
        } => {
            if !*copy {
                editor.remove(to_components(source_location))?;
            }
            (location, entry_mode, id)
        }
    };

    editor.upsert(
        to_components(alternative_location.unwrap_or(location)),
        mode.kind(),
        *id,
    )?;
    Ok(())
}

/// A potential conflict that needs to be checked. It comes in several varieties and always happens
/// if paths overlap in some way between *theirs* and *ours*.
#[derive(Debug)]
pub enum PossibleConflict {
    /// *our* changes have a tree here, but *they* place a non-tree or edit an existing item (that we removed).
    TreeToNonTree {
        /// The possibly available change at this node.
        change_idx: Option<usize>,
    },
    /// A non-tree in *our* tree turned into a tree in *theirs* - this can be done with additions in *theirs*,
    /// or if we added a blob, while they added a directory.
    NonTreeToTree {
        /// The possibly available change at this node.
        change_idx: Option<usize>,
    },
    /// A perfect match, i.e. *our* change at `a/b/c` corresponds to *their* change at the same path.
    Match {
        /// The index to *our* change at *their* path.
        change_idx: usize,
    },
    /// *their* change at `a/b/c` passed `a/b` which is an index to *our* change indicating a directory that was rewritten,
    /// with all its contents being renamed. However, *theirs* has been added *into* that renamed directory.
    PassedRewrittenDirectory { change_idx: usize },
}

impl PossibleConflict {
    pub(super) fn change_idx(&self) -> Option<usize> {
        match self {
            PossibleConflict::TreeToNonTree { change_idx, .. } | PossibleConflict::NonTreeToTree { change_idx, .. } => {
                *change_idx
            }
            PossibleConflict::Match { change_idx, .. }
            | PossibleConflict::PassedRewrittenDirectory { change_idx, .. } => Some(*change_idx),
        }
    }
}

/// The flat list of all tree-nodes so we can avoid having a linked-tree using pointers
/// which is useful for traversal and initial setup as that can then trivially be non-recursive.
pub struct TreeNodes(NonEmptyVec<TreeNode>);

#[derive(Debug, Clone)]
struct NonEmptyVec<T>(Vec<T>);

impl<T> NonEmptyVec<T> {
    fn new(first: T) -> Self {
        Self(vec![first])
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn push(&mut self, value: T) {
        self.0.push(value);
    }

    fn first(&self) -> &T {
        self.0.first().expect("non-empty by construction")
    }

    fn first_mut(&mut self) -> &mut T {
        self.0.first_mut().expect("non-empty by construction")
    }

    fn get(&self, idx: usize) -> Option<&T> {
        self.0.get(idx)
    }

    fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
        self.0.get_mut(idx)
    }
}

/// Trees lead to other trees, or leafs (without children), and it can be represented by a renamed directory.
#[derive(Debug, Default, Clone)]
struct TreeNode {
    /// A mapping of path components to their children to quickly see if `theirs` in some way is potentially
    /// conflicting with `ours`.
    children: HashMap<BString, usize>,
    /// The index to a change, which is always set if this is a leaf node (with no children), and if there are children and this
    /// is a rewritten tree.
    change_idx: Option<usize>,
    /// Keep track of where the location of this node is derived from.
    location: ChangeLocation,
}

#[derive(Debug, Default, Clone, Copy)]
enum ChangeLocation {
    /// The change is at its current (and only) location, or in the source location of a rename.
    #[default]
    CurrentLocation,
    /// This is always the destination of a rename.
    RenamedLocation,
}

impl TreeNode {
    fn is_leaf_node(&self) -> bool {
        self.children.is_empty()
    }
}

impl TreeNodes {
    pub fn new() -> Self {
        TreeNodes(NonEmptyVec::new(TreeNode::default()))
    }

    /// Insert our `change` at `change_idx`, into a linked-tree, assuring that each `change` is non-conflicting
    /// with this tree structure, i.e. each leaf path is only seen once.
    /// Note that directories can be added in between.
    pub fn track_change(&mut self, change: &Change, change_idx: usize) {
        for (path, location_hint) in [
            Some((change.source_location(), ChangeLocation::CurrentLocation)),
            match change {
                Change::Addition { .. } | Change::Deletion { .. } | Change::Modification { .. } => None,
                Change::Rewrite { location, .. } => Some((location.as_bstr(), ChangeLocation::RenamedLocation)),
            },
        ]
        .into_iter()
        .flatten()
        {
            let mut components = to_components(path).peekable();
            let mut next_index = self.0.len();
            let mut cursor = self.0.first_mut();
            while let Some(component) = components.next() {
                let is_last = components.peek().is_none();
                match cursor.children.get(component).copied() {
                    None => {
                        let new_node = TreeNode {
                            children: Default::default(),
                            change_idx: is_last.then_some(change_idx),
                            location: location_hint,
                        };
                        cursor.children.insert(component.to_owned(), next_index);
                        self.0.push(new_node);
                        cursor = self.0.get_mut(next_index).expect("inserted above");
                        next_index += 1;
                    }
                    Some(index) => {
                        cursor = self.0.get_mut(index).expect("child indices are internal");
                        if is_last && !cursor.is_leaf_node() {
                            // NOTE: we might encounter the same path multiple times in rare conditions.
                            //       At least we avoid overwriting existing intermediate changes, for good measure.
                            if cursor.change_idx.is_none() {
                                cursor.change_idx = Some(change_idx);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Search the tree with `our` changes for `theirs` by [`source_location()`](Change::source_location())).
    /// If there is an entry but both are the same, or if there is no entry, return `None`.
    pub fn check_conflict(&self, theirs_location: &BStr) -> Option<PossibleConflict> {
        if self.0.len() == 1 {
            return None;
        }
        let components = to_components(theirs_location);
        let mut cursor = self.0.first();
        let mut cursor_idx = 0;
        let mut intermediate_change = None;
        for component in components {
            if cursor.change_idx.is_some() {
                intermediate_change = cursor.change_idx.map(|change_idx| (change_idx, cursor_idx));
            }
            match cursor.children.get(component).copied() {
                // *their* change is outside *our* tree
                None => {
                    let res = if cursor.is_leaf_node() {
                        Some(PossibleConflict::NonTreeToTree {
                            change_idx: cursor.change_idx,
                        })
                    } else {
                        // a change somewhere else, i.e. `a/c` and we know `a/b` only.
                        intermediate_change.and_then(|(change, cursor_idx)| {
                            let cursor = self.0.get(cursor_idx).expect("cursor indices are internal");
                            // If this is a destination location of a rename, then the `their_location`
                            // is already at the right spot, and we can just ignore it.
                            if matches!(cursor.location, ChangeLocation::CurrentLocation) {
                                Some(PossibleConflict::PassedRewrittenDirectory { change_idx: change })
                            } else {
                                None
                            }
                        })
                    };
                    return res;
                }
                Some(child_idx) => {
                    cursor_idx = child_idx;
                    cursor = self.0.get(cursor_idx).expect("child indices are internal");
                }
            }
        }

        if cursor.is_leaf_node() {
            PossibleConflict::Match {
                change_idx: cursor.change_idx.expect("leaf nodes always have a change"),
            }
        } else {
            PossibleConflict::TreeToNonTree {
                change_idx: cursor.change_idx,
            }
        }
        .into()
    }

    /// Compare both changes and return `true` if they are *not* exactly the same.
    /// One two changes are the same, they will have the same effect.
    /// Since this is called after [`Self::check_conflict`], *our* change will not be applied,
    /// only theirs, which naturally avoids double-application
    /// (which shouldn't have side effects, but let's not risk it)
    pub fn is_not_same_change_in_possible_conflict(
        &self,
        theirs: &Change,
        conflict: &PossibleConflict,
        our_changes: &ChangeListRef,
    ) -> bool {
        conflict
            .change_idx()
            .is_none_or(|idx| our_changes[idx].inner != *theirs)
    }

    pub fn remove_existing_leaf(&mut self, location: &BStr) {
        self.remove_leaf_inner(location, true);
    }

    pub fn remove_leaf(&mut self, location: &BStr) {
        self.remove_leaf_inner(location, false);
    }

    fn remove_leaf_inner(&mut self, location: &BStr, must_exist: bool) {
        let mut components = to_components(location).peekable();
        let mut cursor = self.0.first_mut();
        while let Some(component) = components.next() {
            match cursor.children.get(component).copied() {
                None => debug_assert!(!must_exist, "didn't find '{location}' for removal"),
                Some(existing_idx) => {
                    let is_last = components.peek().is_none();
                    if is_last {
                        cursor.children.remove(component);
                        cursor = self.0.get_mut(existing_idx).expect("child indices are internal");
                        debug_assert!(
                            cursor.is_leaf_node(),
                            "BUG: we should really only try to remove leaf nodes: {cursor:?}"
                        );
                        cursor.change_idx = None;
                    } else {
                        cursor = self.0.get_mut(existing_idx).expect("child indices are internal");
                    }
                }
            }
        }
    }

    /// Insert `new_change` which affects this tree into it and put it into `storage` to obtain the index.
    /// Panic if that change already exists as it must be made so that it definitely doesn't overlap with this tree.
    pub fn insert(&mut self, new_change: &Change, new_change_idx: usize) {
        let mut next_index = self.0.len();
        let mut cursor = self.0.first_mut();
        for component in to_components(new_change.location()) {
            match cursor.children.get(component).copied() {
                None => {
                    cursor.children.insert(component.to_owned(), next_index);
                    self.0.push(TreeNode::default());
                    cursor = self.0.get_mut(next_index).expect("inserted above");
                    next_index += 1;
                }
                Some(existing_idx) => {
                    cursor = self.0.get_mut(existing_idx).expect("child indices are internal");
                }
            }
        }

        debug_assert!(
            !matches!(new_change, Change::Rewrite { .. }),
            "BUG: we thought we wouldn't do that current.location is related?"
        );
        cursor.change_idx = Some(new_change_idx);
        cursor.location = ChangeLocation::CurrentLocation;
    }
}

pub fn to_components_bstring_ref(rela_path: &BString) -> impl Iterator<Item = &BStr> {
    rela_path.split(|b| *b == b'/').map(Into::into)
}

pub fn to_components(rela_path: &BStr) -> impl Iterator<Item = &BStr> {
    rela_path.split(|b| *b == b'/').map(Into::into)
}

impl Conflict {
    pub(super) fn without_resolution(
        resolution: ResolutionFailure,
        changes: (&Change, &Change, ConflictMapping, ConflictMapping),
        entries: [Option<ConflictIndexEntry>; 3],
    ) -> Self {
        Conflict::maybe_resolved(Err(resolution), changes, entries)
    }

    pub(super) fn with_resolution(
        resolution: Resolution,
        changes: (&Change, &Change, ConflictMapping, ConflictMapping),
        entries: [Option<ConflictIndexEntry>; 3],
    ) -> Self {
        Conflict::maybe_resolved(Ok(resolution), changes, entries)
    }

    fn maybe_resolved(
        resolution: Result<Resolution, ResolutionFailure>,
        (ours, theirs, map, outer_map): (&Change, &Change, ConflictMapping, ConflictMapping),
        entries: [Option<ConflictIndexEntry>; 3],
    ) -> Self {
        Conflict {
            resolution,
            ours: ours.clone(),
            theirs: theirs.clone(),
            entries,
            map: map.to_global(outer_map),
        }
    }

    pub(super) fn unknown(changes: (&Change, &Change, ConflictMapping, ConflictMapping)) -> Self {
        let (source_mode, source_id) = changes.0.source_entry_mode_and_id();
        let (our_mode, our_id) = changes.0.entry_mode_and_id();
        let (their_mode, their_id) = changes.1.entry_mode_and_id();
        let entries = [
            Some(ConflictIndexEntry {
                mode: source_mode,
                id: source_id.into(),
                path_hint: Some(ConflictIndexEntryPathHint::Source),
            }),
            Some(ConflictIndexEntry {
                mode: our_mode,
                id: our_id.into(),
                path_hint: Some(ConflictIndexEntryPathHint::Current),
            }),
            Some(ConflictIndexEntry {
                mode: their_mode,
                id: their_id.into(),
                path_hint: Some(ConflictIndexEntryPathHint::RenamedOrTheirs),
            }),
        ];
        Conflict::maybe_resolved(Err(ResolutionFailure::Unknown), changes, entries)
    }
}
