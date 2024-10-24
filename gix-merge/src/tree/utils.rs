use crate::blob::ResourceKind;
use crate::tree::{Conflict, ConflictMapping, Error, Options, Resolution, ResolutionFailure};
use bstr::ByteSlice;
use bstr::{BStr, BString, ByteVec};
use gix_diff::tree_with_rewrites::Change;
use gix_hash::ObjectId;
use gix_object::tree;
use gix_object::tree::EntryMode;
use std::borrow::Cow;
use std::collections::HashMap;

/// Assuming that `their_location` is the destination of *their* rewrite, check if *it* passes
/// over a directory rewrite in *our* tree. If so, rewrite it so that we get the path
/// it would have had if it had been renamed along with *our* directory.
pub fn possibly_rewritten_location<'a>(
    check_tree: &mut TreeNodes,
    their_location: &'a BStr,
    our_changes: &[Change],
) -> Cow<'a, BStr> {
    check_tree
        .check_conflict(their_location, CheckConflict::DoNotChangePassedNodes)
        .and_then(|pc| match pc {
            PossibleConflict::PassedRewrittenDirectory { change_idx } => {
                let passed_change = &our_changes[change_idx];
                rewrite_location_with_renamed_directory(their_location, passed_change).map(Cow::Owned)
            }
            _ => None,
        })
        .unwrap_or(Cow::Borrowed(their_location))
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

/// Produce a unique path within the directory that contains the file at `file_path` (like `a/b`, using `editor`
/// and `tree` to assure unique names, to obtain the tree at `a/` and `side_name` to more clearly signal
/// where the file is coming from.
pub fn unique_path_in_tree(
    file_path: &BStr,
    editor: &gix_object::tree::Editor<'_>,
    tree: &mut TreeNodes,
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
    while editor.get(to_components_bstring_ref(&buf)).is_some()
        || tree
            .check_conflict(buf.as_bstr(), CheckConflict::DoNotChangePassedNodes)
            .is_some()
    {
        buf.truncate(base_len);
        buf.push_str(format!("_{suffix}",));
        suffix += 1;
    }
    Ok(buf)
}

/// Perform a merge between two blobs and return the result of its object id.
#[allow(clippy::too_many_arguments)]
pub fn perform_blob_merge<E>(
    labels: crate::blob::builtin_driver::text::Labels<'_>,
    objects: &impl gix_object::FindObjectOrHeader,
    blob_merge: &mut crate::blob::Platform,
    buf: &mut Vec<u8>,
    write_blob_to_odb: &mut impl FnMut(&[u8]) -> Result<ObjectId, E>,
    (our_location, our_id, our_mode): (&BString, ObjectId, EntryMode),
    (their_location, their_id, their_mode): (&BString, ObjectId, EntryMode),
    (previous_location, previous_id, previous_mode): (&BString, ObjectId, EntryMode),
    extra_markers: u8,
    options: &Options,
) -> Result<(gix_hash::ObjectId, crate::blob::Resolution), Error>
where
    E: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
{
    blob_merge.set_resource(
        our_id,
        our_mode.kind(),
        our_location.as_bstr(),
        ResourceKind::CurrentOrOurs,
        objects,
    )?;
    blob_merge.set_resource(
        their_id,
        their_mode.kind(),
        their_location.as_bstr(),
        ResourceKind::OtherOrTheirs,
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

    let (ancestor, current, other);
    let labels = if our_location == their_location {
        labels
    } else {
        ancestor = previous_location.as_bstr();
        current = labels.current.map(|side| combined(side, our_location));
        other = labels.other.map(|side| combined(side, their_location));
        crate::blob::builtin_driver::text::Labels {
            ancestor: Some(ancestor),
            current: current.as_ref().map(|n| n.as_bstr()),
            other: other.as_ref().map(|n| n.as_bstr()),
        }
    };
    let prep = blob_merge.prepare_merge(objects, with_extra_markers(options, extra_markers))?;
    let (pick, resolution) = prep.merge(buf, labels, &options.blob_merge_command_ctx)?;
    // TODO: properly handle binary/other buffers. This API is troublesome, fix it
    let merged_content = prep.buffer_by_pick(pick).unwrap_or(buf);
    let merged_blob_id = write_blob_to_odb(merged_content).map_err(|err| Error::WriteBlobToOdb(err.into()))?;
    Ok((merged_blob_id, resolution))
}

fn with_extra_markers(opts: &Options, extra_makers: u8) -> crate::blob::platform::merge::Options {
    let mut out = opts.blob_merge;
    if let crate::blob::builtin_driver::text::Conflict::Keep { marker_size, .. } = &mut out.text.conflict {
        *marker_size = marker_size.saturating_add(extra_makers.saturating_add(opts.call_depth.saturating_mul(2)));
    }
    out
}

/// Only keep leaf nodes, or trees that are the renamed, pushing `change` on `changes`.
/// Doing so makes it easy to track renamed or rewritten or copied directories, and properly
/// handle *their* changes that fall within them.
/// Note that it also rewrites `change` if it is a copy, turning it into an addition so copies don't have an effect
/// on the merge algorthm.
pub fn track(change: gix_diff::tree_with_rewrites::ChangeRef<'_>, changes: &mut Vec<Change>) {
    if !change.entry_mode().is_tree() || matches!(change.relation(), Some(gix_diff::tree::visit::Relation::Parent(_))) {
        changes.push(match change.into_owned() {
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
        });
    }
}

/// Unconditionally apply `change` to `editor`.
pub fn apply_change(editor: &mut tree::Editor<'_>, change: &Change) -> Result<(), gix_object::tree::editor::Error> {
    use to_components_bstring_ref as to_components;
    // TODO(performance): we could apply tree changes if they are renames-by-identity, and then save the
    //                    work needed to set all the children recursively. This would require more elaborate
    //                    tracking though.
    if change.entry_mode().is_tree() {
        return Ok(());
    }

    match change {
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
        } => editor.upsert(to_components(location), entry_mode.kind(), *id)?,
        Change::Deletion { location, .. } => editor.remove(to_components(location))?,
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
            editor.upsert(to_components(location), entry_mode.kind(), *id)?
        }
    };
    Ok(())
}

/// A potential conflict that needs to be checked. It comes in several varieties and always happens
/// if paths overlap in some way between *theirs* and *ours*.
#[derive(Debug)]
// TODO: remove this when all fields are used.
#[allow(dead_code)]
pub enum PossibleConflict {
    /// *our* changes have a tree here, but *they* place a non-tree or edit an existing item (that we removed).
    TreeToNonTree {
        /// The node at the end of *their* location, i.e. the node at `c` of path `a/b/c`, and there is `a/b/c/d`
        /// present in the tree (or more children).
        /// This always happens if `c` was a directory and turned into a non-directory, but can also happen if
        /// *their* change is a directory change.
        /// This also means `our_node` has children.
        our_node_idx: usize,
        /// The possibly available change at this node.
        change_idx: Option<usize>,
    },
    /// A non-tree in *our* tree turned into a tree in *theirs* - this can be done with additions in *theirs*,
    /// or if we added a blob, while they added a directory.
    NonTreeToTree {
        /// The last seen node at the end of the *our* portion of *their* path, i.e. the node at `a/b` when *their*
        /// path is `a/b/c`.
        our_leaf_node_idx: usize,
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
    fn change_idx(&self) -> Option<usize> {
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
pub struct TreeNodes(Vec<TreeNode>);

/// Trees lead to other trees, or leafs (without children), and it can be represented by a renamed directory.
#[derive(Debug, Default, Clone)]
struct TreeNode {
    /// A mapping of path components to their children to quickly see if `theirs` in some way is potentially
    /// conflicting with `ours`.
    children: HashMap<BString, usize>,
    /// The index to a change, which is always set if this is a leaf node (with no children), and if there are children and this
    /// is a rewritten tree.
    change_idx: Option<usize>,
    /// If `true`, this means we were part of conflict resolution which always means that this (*our*) change
    /// should be written by the one handling the conflict.
    /// If `false`, it will be applied to the tree with [`TreeNodes::apply_nonconflicting_changes()`].
    skip_when_writing: bool,
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
        TreeNodes(vec![TreeNode::default()])
    }

    /// Write out all `changes` that don't have a conflict marker. Assumed to be the array backing the changes we were initialized with.
    pub fn apply_nonconflicting_changes(
        &self,
        changes: &[Change],
        editor: &mut tree::Editor<'_>,
    ) -> Result<(), tree::editor::Error> {
        for change_idx in self.0.iter().filter_map(|n| {
            n.change_idx
                .filter(|_| !n.skip_when_writing && matches!(n.location, ChangeLocation::CurrentLocation))
        }) {
            apply_change(editor, &changes[change_idx])?;
        }
        Ok(())
    }

    /// Insert our `change` at `change_idx`, into a linked-tree, assuring that each `change` is non-conflicting
    /// with this tree structure, i.e. reach path is only seen once.
    pub fn track_ours_exclusive(&mut self, change: &Change, change_idx: usize) {
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
            let mut cursor = &mut self.0[0];
            while let Some(component) = components.next() {
                match cursor.children.get(component).copied() {
                    None => {
                        let new_node = TreeNode {
                            children: Default::default(),
                            change_idx: components.peek().is_none().then_some(change_idx),
                            skip_when_writing: false,
                            location: location_hint,
                        };
                        cursor.children.insert(component.to_owned(), next_index);
                        self.0.push(new_node);
                        cursor = &mut self.0[next_index];
                        next_index += 1;
                    }
                    Some(index) => {
                        cursor = &mut self.0[index];
                    }
                }
            }
        }
    }

    /// Search the tree with `our` changes for `theirs` by [`source_location()`](Change::source_location())).
    /// If there is an entry but both are the same, or if there is no entry, return `None`.
    pub fn check_conflict(&mut self, theirs_location: &BStr, mode: CheckConflict) -> Option<PossibleConflict> {
        let components = to_components(theirs_location);
        let mut cursor = &mut self.0[0];
        let mut cursor_idx = 0;
        let mut intermediate_change = None;
        let maybe_change = |cursor: &mut TreeNode| {
            // TODO: remove this condition - it's not required.
            if matches!(mode, CheckConflict::PassedNodesDoNotWrite) {
                cursor.skip_when_writing = true;
            }
        };
        for component in components {
            if cursor.change_idx.is_some() {
                intermediate_change = cursor.change_idx.map(|change_idx| (change_idx, cursor_idx));
            }
            match cursor.children.get(component).copied() {
                // *their* change is outside *our* tree
                None => {
                    let res = if cursor.is_leaf_node() {
                        maybe_change(cursor);
                        Some(PossibleConflict::NonTreeToTree {
                            our_leaf_node_idx: cursor_idx,
                            change_idx: cursor.change_idx,
                        })
                    } else {
                        // a change somewhere else, i.e. `a/c` and we know `a/b` only.
                        intermediate_change.and_then(|(change, cursor_idx)| {
                            let cursor = &mut self.0[cursor_idx];
                            // If this is a destination location of a rename, then the `their_location`
                            // is already at the right spot, and we can just ignore it.
                            if matches!(cursor.location, ChangeLocation::CurrentLocation) {
                                maybe_change(cursor);
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
                    cursor = &mut self.0[cursor_idx];
                }
            }
        }

        maybe_change(cursor);
        if cursor.is_leaf_node() {
            PossibleConflict::Match {
                change_idx: cursor.change_idx.expect("leaf nodes always have a change"),
            }
        } else {
            PossibleConflict::TreeToNonTree {
                our_node_idx: cursor_idx,
                change_idx: cursor.change_idx,
            }
        }
        .into()
    }

    /// Compare both changes and return `true` if they are *not* exactly the same.
    /// One two changes are the same, they will have the same effect.
    /// Since this is called after [`Self::check_conflict`], *our* change will not be applied,
    /// only theirs, which naturally avoids double-application
    /// (which shouldn't have side-effects, but let's not risk it)
    pub fn is_not_same_change_in_possible_conflict(
        &self,
        theirs: &Change,
        conflict: &PossibleConflict,
        our_changes: &[Change],
    ) -> bool {
        conflict.change_idx().map_or(true, |idx| &our_changes[idx] != theirs)
    }

    pub fn remove_existing_leaf(&mut self, location: &BStr) {
        let mut components = to_components(location).peekable();
        let mut cursor = &mut self.0[0];
        while let Some(component) = components.next() {
            match cursor.children.get(component).copied() {
                None => unreachable!("didn't find {} for removal", location),
                Some(existing_idx) => {
                    let is_last_component = components.peek().is_none();
                    if is_last_component {
                        cursor.children.remove(component);
                        cursor = &mut self.0[existing_idx];
                        cursor.change_idx = None;
                    } else {
                        cursor = &mut self.0[existing_idx];
                    }
                }
            }
        }
    }

    /// Insert `new_change` which affects this tree into it and put it into `storage` to obtain the index.
    /// Panic if that change already exists as it must be made so that it definitely doesn't overlap with this tree.
    pub fn insert(&mut self, new_change: Change, storage: &mut Vec<Change>) {
        let mut next_index = self.0.len();
        let mut cursor = &mut self.0[0];
        for component in to_components(new_change.location()) {
            match cursor.children.get(component).copied() {
                None => {
                    cursor.children.insert(component.to_owned(), next_index);
                    self.0.push(TreeNode::default());
                    cursor = &mut self.0[next_index];
                    next_index += 1;
                }
                Some(existing_idx) => {
                    cursor = &mut self.0[existing_idx];
                }
            }
        }

        // Overwrite what might be there - this can be the destination of a rename
        // which was linked to its `Rewrite` change. Now we wire it up so that it
        // will be written.
        let new_change_idx = storage.len();
        // TODO: This overwrites nodes, possibly rewrites, is that OK or can there be issues later?
        assert!(
            !matches!(new_change, Change::Rewrite { .. }),
            "BUG: we thought we wouldn't do that current.location is related?"
        );
        storage.push(new_change);
        cursor.change_idx = Some(new_change_idx);
        cursor.location = ChangeLocation::CurrentLocation;
        cursor.skip_when_writing = false;
    }
}

pub enum CheckConflict {
    /// We assume that the changes in the nodes we pass over and return ultimately are handled by the caller,
    /// so they won't apply anymore later.
    PassedNodesDoNotWrite,
    /// Don't make any change, just perform the lookup.
    DoNotChangePassedNodes,
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
        changes: (&Change, &Change, ConflictMapping),
    ) -> Self {
        Conflict::maybe_resolved(Err(resolution), changes)
    }
    pub(super) fn with_resolution(resolution: Resolution, changes: (&Change, &Change, ConflictMapping)) -> Self {
        Conflict::maybe_resolved(Ok(resolution), changes)
    }
    pub(super) fn maybe_resolved(
        resolution: Result<Resolution, ResolutionFailure>,
        (ours, theirs, map): (&Change, &Change, ConflictMapping),
    ) -> Self {
        Conflict {
            resolution,
            ours: ours.clone(),
            theirs: theirs.clone(),
            map,
        }
    }
}
