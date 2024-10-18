use bstr::BString;
use gix_diff::tree_with_rewrites::Change;
use gix_diff::Rewrites;

/// The error returned by [`tree()`](crate::tree()).
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Could not find ancestor, our or their tree to get started")]
    FindTree(#[from] gix_object::find::existing_object::Error),
    #[error("Could not find ancestor, our or their tree iterator to get started")]
    FindTreeIter(#[from] gix_object::find::existing_iter::Error),
    #[error("Failed to diff our side or their side")]
    DiffTree(#[from] gix_diff::tree_with_rewrites::Error),
    #[error("Could not apply merge result to base tree")]
    TreeEdit(#[from] gix_object::tree::editor::Error),
    #[error("Failed to load resource to prepare for blob merge")]
    BlobMergeSetResource(#[from] crate::blob::platform::set_resource::Error),
    #[error(transparent)]
    BlobMergePrepare(#[from] crate::blob::platform::prepare_merge::Error),
    #[error(transparent)]
    BlobMerge(#[from] crate::blob::platform::merge::Error),
    #[error("Failed to write merged blob content as blob to the object database")]
    WriteBlobToOdb(Box<dyn std::error::Error + Send + Sync + 'static>),
}

/// The outcome produced by [`tree()`](crate::tree()).
#[derive(Clone)]
pub struct Outcome<'a> {
    /// The ready-made (but unwritten) which is the *base* tree, including all non-conflicting changes, and the changes that had
    /// conflicts which could be resolved automatically.
    ///
    /// This means, if all of their changes were conflicting, this will be equivalent to the *base* tree.
    pub tree: gix_object::tree::Editor<'a>,
    /// The set of conflicts we encountered. Can be empty to indicate there was no conflict.
    /// Note that conflicts might have been auto-resolved, but they are listed here for completeness.
    /// Use [`has_unresolved_conflicts()`](Outcome::has_unresolved_conflicts()) to see if any action is needed
    /// before using [`tree`](Outcome::tree).
    pub conflicts: Vec<Conflict>,
    /// `true` if `conflicts` contains only a single *unresolved* conflict in the last slot, but possibly more resolved ones.
    /// This also makes this outcome a very partial merge that cannot be completed.
    /// Only set if [`fail_on_conflict`](Options::fail_on_conflict) is `true`.
    pub failed_on_first_unresolved_conflict: bool,
}

impl Outcome<'_> {
    /// Return `true` if there is any conflict that would still need to be resolved.
    pub fn has_unresolved_conflicts(&self) -> bool {
        self.conflicts.iter().any(|c| c.resolution.is_err())
    }
}

/// A description of a conflict (i.e. merge issue without an auto-resolution) as seen during a [tree-merge](crate::tree()).
/// They may have a resolution that was applied automatically, or be left for the caller to resolved.
#[derive(Debug, Clone)]
pub struct Conflict {
    /// A record on how the conflict resolution succeeded or failed.
    /// On failure, one can examine `ours` and `theirs` to potentially find a custom solution.
    /// Note that the descriptions of resolutions or resolution failures may be swapped compared
    /// to the actual changes. This is due to changes like `modification|deletion` being treated the
    /// same as `deletion|modification`, i.e. *ours* is not more privileged than theirs.
    /// To compensate for that, use [`changes_in_resolution()`](Conflict::changes_in_resolution()).
    pub resolution: Result<Resolution, ResolutionFailure>,
    /// The change representing *our* side.
    pub ours: Change,
    /// The change representing *their* side.
    pub theirs: Change,
    map: ConflictMapping,
}

impl Conflict {
    /// Returns the changes of fields `ours` and `theirs` so they match their description in the
    /// [`Resolution`] or [`ResolutionFailure`] respectively.
    /// Without this, the sides may appear swapped as `ours|theirs` is treated the same as `theirs/ours`
    /// if both types are different, like `modification|deletion`.
    pub fn changes_in_resolution(&self) -> (&Change, &Change) {
        match self.map {
            ConflictMapping::Original => (&self.ours, &self.theirs),
            ConflictMapping::Swapped => (&self.theirs, &self.ours),
        }
    }

    /// Similar to [`changes_in_resolution()`](Self::changes_in_resolution()), but returns the parts
    /// of the structure so the caller can take ownership. This can be useful when applying your own
    /// resolutions for resolution failures.
    pub fn into_parts_by_resolution(self) -> (Result<Resolution, ResolutionFailure>, Change, Change) {
        match self.map {
            ConflictMapping::Original => (self.resolution, self.ours, self.theirs),
            ConflictMapping::Swapped => (self.resolution, self.theirs, self.ours),
        }
    }
}

/// A utility to help define which side is what.
#[derive(Debug, Clone)]
enum ConflictMapping {
    Original,
    Swapped,
}

/// Describes of a conflict involving *our* change and *their* change was specifically resolved.
///
/// Note that all resolutions are side-agnostic, so *ours* could also have been *theirs* and vice-versa.
/// This is done so that no side is inherently more valuable
#[derive(Debug, Clone, Copy)]
pub enum Resolution {
    /// *ours* was a modified blob and *theirs* renamed that blob, but didn't change it.
    /// We moved the changed blob from *ours* to its new location.
    /// If this is a `copy`, the source of the copy was set to be the changed blob as well so both match.
    OursModifiedTheirsRenamedAndUnchangedRewriteOurs {
        /// If `true`, a copy was made, instead of moving the blob.
        copy: bool,
    },
    /// *ours* and *theirs* was auto-merged successfully, without leaving any conflicting hunks.
    OursModifiedTheirsModifiedBlobMerge(
        /// The fully merged blob.
        gix_hash::ObjectId,
    ),
}

/// Describes of a conflict involving *our* change and *their* failed to be resolved.
#[derive(Debug, Clone)]
pub enum ResolutionFailure {
    /// *ours* and *theirs* was merged by blob-content, but left conflict markers due to conflicts.
    OursModifiedTheirsModifiedBlobMergeConflict(
        /// The fully merged blob, with conflict markers.
        gix_hash::ObjectId,
    ),
    /// *ours* was modified, but *theirs* was turned into a directory, so *ours* was renamed to a non-conflicting path.
    OursModifiedTheirsDirectoryOursRenamed {
        /// The path at which `ours` can be found in the tree - it's in the same directory that it was in before.
        renamed_path_to_modified_blob: BString,
    },
}

/// A way to configure [`tree()`](crate::tree()).
#[derive(Default, Debug, Clone)]
pub struct Options {
    /// If *not* `None`, rename tracking will be performed when determining the changes of each side of the merge.
    pub rewrites: Option<Rewrites>,
    /// Decide how blob-merges should be done. This relates to if conflicts can be resolved or not.
    pub blob_merge: crate::blob::platform::merge::Options,
    /// The context to use when invoking merge-drivers.
    pub blob_merge_command_ctx: gix_command::Context,
    /// If `true`, the first conflict will cause the entire
    pub fail_on_conflict: bool,
    // TODO(Perf) add a flag to allow parallelizing the tree-diff itself.
}

pub(super) mod function {
    use crate::blob::ResourceKind;
    use crate::tree::{Conflict, ConflictMapping, Error, Options, Outcome, Resolution, ResolutionFailure};
    use bstr::{BStr, BString, ByteSlice, ByteVec};
    use gix_diff::tree::recorder::Location;
    use gix_diff::tree::visit::Relation;
    use gix_diff::tree_with_rewrites::Change;
    use gix_hash::ObjectId;
    use gix_object::tree::EntryMode;
    use gix_object::{tree, FindExt};
    use std::collections::HashMap;
    use std::convert::Infallible;

    /// Perform a merge between `our_tree` and `their_tree`, using `base_tree` as merge-base.
    /// Note that `base_tree` can be an empty tree to indicate 'no common ancestor between the two sides'.
    ///
    /// * `labels` are relevant for text-merges and will be shown in conflicts.
    /// * `objects` provides access to trees when diffing them.
    /// * `write_blob_to_odb(content) -> Result<ObjectId, E>` writes newly merged content into the odb to obtain an id
    ///    that will be used in merged trees.
    /// * `diff_state` is state used for diffing trees.
    /// * `diff_resource_cache` is used for similarity checks.
    /// * `blob_merge` is a pre-configured platform to merge any content.
    ///     - Note that it shouldn't be allowed to read from the worktree, given that this is a tree-merge.
    /// * `options` are used to affect how the merge is performed.
    ///
    /// ### Performance
    ///
    /// Note that `objects` *should* have an object cache to greatly accelerate tree-retrieval.
    #[allow(clippy::too_many_arguments)]
    pub fn tree<'objects, E>(
        base_tree: &gix_hash::oid,
        our_tree: &gix_hash::oid,
        their_tree: &gix_hash::oid,
        labels: crate::blob::builtin_driver::text::Labels<'_>,
        objects: &'objects impl gix_object::FindObjectOrHeader,
        mut write_blob_to_odb: impl FnMut(&[u8]) -> Result<ObjectId, E>,
        diff_state: &mut gix_diff::tree::State,
        diff_resource_cache: &mut gix_diff::blob::Platform,
        blob_merge: &mut crate::blob::Platform,
        options: Options,
    ) -> Result<Outcome<'objects>, Error>
    where
        E: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    {
        let (mut base_buf, mut side_buf) = (Vec::new(), Vec::new());
        let ancestor_tree = objects.find_tree(base_tree, &mut base_buf)?;
        let our_tree = objects.find_tree_iter(our_tree, &mut side_buf)?;

        let mut editor = tree::Editor::new(ancestor_tree.to_owned(), objects, base_tree.kind());
        let ancestor_tree = gix_object::TreeRefIter::from_bytes(&base_buf);

        let mut our_changes = Vec::new();
        gix_diff::tree_with_rewrites(
            ancestor_tree,
            our_tree,
            diff_resource_cache,
            diff_state,
            objects,
            |change| -> Result<_, Infallible> {
                if may_track(change) {
                    our_changes.push(change.into_owned());
                }
                Ok(gix_diff::tree_with_rewrites::Action::Continue)
            },
            gix_diff::tree_with_rewrites::Options {
                location: Some(Location::Path),
                rewrites: options.rewrites,
            },
        )?;

        // NOTE(borrowchk): This is for holding additions to the original `conflict_check_ours` tree
        // which we can't edit due to referencing madness.
        // Funnily enough, it must be done like this, doing everything with the Owned type also wasn't possible.
        // TODO: test that we can even trigger a conflict with an evasion - should be possible. If not,
        // remove this complexity of testing for newly added changes.
        let mut our_changes_tree = TreeNodes::new();
        for (idx, change) in our_changes.iter().enumerate() {
            our_changes_tree.track_ours_exclusive(change, idx);
        }

        let their_tree = objects.find_tree_iter(their_tree, &mut side_buf)?;
        let mut their_changes = Vec::new();
        gix_diff::tree_with_rewrites(
            ancestor_tree,
            their_tree,
            diff_resource_cache,
            diff_state,
            objects,
            |change| -> Result<_, Infallible> {
                if may_track(change) {
                    their_changes.push(change.into_owned());
                }
                Ok(gix_diff::tree_with_rewrites::Action::Continue)
            },
            gix_diff::tree_with_rewrites::Options {
                location: Some(Location::Path),
                rewrites: options.rewrites,
            },
        )?;

        dbg!(&our_changes, &their_changes);

        let mut conflicts = Vec::new();
        let mut failed_on_first_conflict = false;
        let mut should_fail_on_unresolved_conflict = |conflict: Conflict| -> bool {
            if options.fail_on_conflict && conflict.resolution.is_err() {
                failed_on_first_conflict = true;
            }
            conflicts.push(conflict);
            failed_on_first_conflict
        };
        let mut their_changes = their_changes.into_iter().peekable();

        while let Some(theirs) = their_changes.next() {
            // `their` can be a tree, and it could be used to efficiently prune child-changes as these
            // trees are always rewrites with parent ids (of course we validate), so child-changes could be handled
            // quickly. However, for now the benefit of having these trees is to have them as part of the match-tree
            // on *our* side so that it's clear that we passed a renamed directory (by identity).
            if theirs.entry_mode().is_tree() {
                continue;
            }

            match our_changes_tree
                .check_conflict(&theirs)
                .filter(|ours| our_changes_tree.is_not_same_change_in_possible_conflict(&theirs, ours, &our_changes))
            {
                None => {
                    apply_change(&mut editor, &theirs)?;
                }
                Some(candidate) => {
                    use to_components_bstring_ref as to_components;
                    let ours_idx = match candidate {
                        PossibleConflict::PassedRewrittenDirectory { .. } => {
                            todo!("rewritten directory changes the destination directory of their change by prefix")
                        }
                        PossibleConflict::TreeToNonTree {
                            our_node_idx: _,
                            change_idx,
                        } if change_idx.map_or(false, |idx| matches!(our_changes[idx], Change::Deletion { .. })) => {
                            dbg!(&theirs, change_idx.map(|idx| &our_changes[idx]));
                            change_idx
                        }
                        PossibleConflict::NonTreeToTree { .. } => {
                            todo!("NonTreeToTree: This can never be reconciled unless we are told which tree to pick (also todo)")
                        }
                        PossibleConflict::Match { change_idx: ours_idx } => Some(ours_idx),
                        _ => None,
                    };

                    let Some(ours_idx) = ours_idx else {
                        todo!(
                            "this should also be a conflict, unless we know how to deal with it better: {candidate:?}"
                        )
                    };

                    let ours = &our_changes[ours_idx];
                    match (ours, &theirs) {
                        (
                            Change::Modification {
                                previous_id,
                                id: our_id,
                                location: our_location,
                                entry_mode: our_mode,
                                ..
                            },
                            Change::Rewrite {
                                source_id: their_source_id,
                                id: their_id,
                                location: their_location,
                                entry_mode: their_mode,
                                copy,
                                ..
                            },
                        )
                        | (
                            Change::Rewrite {
                                source_id: their_source_id,
                                id: their_id,
                                location: their_location,
                                entry_mode: their_mode,
                                copy,
                                ..
                            },
                            Change::Modification {
                                previous_id,
                                id: our_id,
                                location: our_location,
                                entry_mode: our_mode,
                                ..
                            },
                        ) if our_mode.kind() == their_mode.kind() => {
                            assert_eq!(
                                previous_id, their_source_id,
                                "both refer to the same base, so should always match"
                            );
                            let renamed_without_change = their_source_id == their_id;
                            if renamed_without_change {
                                if *copy {
                                    // TODO: test this branch, does git do the same?
                                    editor.upsert(to_components(our_location), our_mode.kind(), *our_id)?;
                                } else {
                                    editor.remove(to_components(our_location))?;
                                    our_changes_tree.remove_existing_leaf(our_location.as_bstr());
                                }
                                editor.upsert(to_components(their_location), their_mode.kind(), *our_id)?;
                                let new_change = Change::Addition {
                                    location: their_location.to_owned(),
                                    relation: None,
                                    entry_mode: *their_mode,
                                    id: *our_id,
                                };
                                let should_break = should_fail_on_unresolved_conflict(Conflict::with_resolution(
                                    Resolution::OursModifiedTheirsRenamedAndUnchangedRewriteOurs { copy: *copy },
                                    ours,
                                    theirs,
                                    if matches!(ours, Change::Modification { .. }) {
                                        ConflictMapping::Original
                                    } else {
                                        ConflictMapping::Swapped
                                    },
                                ));
                                let new_change_idx = our_changes.len();
                                our_changes.push(new_change);
                                our_changes_tree.insert(our_changes.last().expect("just pushed"), new_change_idx);
                                if should_break {
                                    break;
                                }
                            } else {
                                todo!("needs blob merge")
                            }
                        }
                        (
                            Change::Modification {
                                location,
                                previous_id,
                                previous_entry_mode,
                                entry_mode: our_mode,
                                id: our_id,
                                ..
                            },
                            Change::Modification {
                                entry_mode: their_mode,
                                id: their_id,
                                ..
                            },
                        ) if our_mode.kind() == their_mode.kind() && our_id != their_id => {
                            let (merged_blob_id, resolution) = perform_blob_merge(
                                labels,
                                objects,
                                blob_merge,
                                &mut diff_state.buf1,
                                &mut write_blob_to_odb,
                                location,
                                *our_id,
                                *our_mode,
                                *their_id,
                                *their_mode,
                                *previous_id,
                                *previous_entry_mode,
                                &options,
                            )?;
                            editor.upsert(to_components(location), our_mode.kind(), merged_blob_id)?;
                            if should_fail_on_unresolved_conflict(Conflict::maybe_resolved(
                                match resolution {
                                    crate::blob::Resolution::Complete => {
                                        Ok(Resolution::OursModifiedTheirsModifiedBlobMerge(merged_blob_id))
                                    }
                                    crate::blob::Resolution::Conflict => Err(
                                        ResolutionFailure::OursModifiedTheirsModifiedBlobMergeConflict(merged_blob_id),
                                    ),
                                },
                                ours,
                                theirs,
                                ConflictMapping::Original,
                            )) {
                                break;
                            };
                        }
                        (
                            Change::Modification {
                                location,
                                entry_mode,
                                id,
                                ..
                            },
                            Change::Deletion { .. },
                        )
                        | (
                            Change::Deletion { .. },
                            Change::Modification {
                                location,
                                entry_mode,
                                id,
                                ..
                            },
                        ) => {
                            let (label_of_side_to_be_moved, conflict_map) =
                                if matches!(ours, Change::Modification { .. }) {
                                    (labels.current.unwrap_or_default(), ConflictMapping::Original)
                                } else {
                                    (labels.other.unwrap_or_default(), ConflictMapping::Swapped)
                                };
                            let deletion_prefaces_addition_of_directory = {
                                let change_on_right = match conflict_map {
                                    ConflictMapping::Original => their_changes.peek(),
                                    ConflictMapping::Swapped => our_changes.get(ours_idx + 1),
                                };
                                change_on_right
                                    .map(|change| change.entry_mode().is_tree() && change.location() == location)
                                    .unwrap_or_default()
                            };

                            if deletion_prefaces_addition_of_directory {
                                let renamed_path =
                                    unique_path_in_tree(location.as_bstr(), &editor, label_of_side_to_be_moved)?;
                                editor.upsert(to_components(&renamed_path), entry_mode.kind(), *id)?;
                                editor.remove(to_components(location))?;
                                our_changes_tree.remove_existing_leaf(location.as_bstr());

                                let new_change = Change::Addition {
                                    location: renamed_path.clone(),
                                    relation: None,
                                    entry_mode: *entry_mode,
                                    id: *id,
                                };
                                let should_break = should_fail_on_unresolved_conflict(Conflict::without_resolution(
                                    ResolutionFailure::OursModifiedTheirsDirectoryOursRenamed {
                                        renamed_path_to_modified_blob: renamed_path,
                                    },
                                    ours,
                                    theirs,
                                    conflict_map,
                                ));
                                let new_change_idx = our_changes.len();
                                our_changes.push(new_change);
                                our_changes_tree.insert(our_changes.last().expect("just pushed"), new_change_idx);

                                if should_break {
                                    break;
                                };
                            } else {
                                todo!("ordinary mod/del")
                            }
                        }
                        unknown => {
                            todo!("all other cases we can test, then default this to be a conflict: {unknown:?}")
                        }
                    }
                }
            }
        }

        our_changes_tree.apply_nonconflicting_changes(&our_changes, &mut editor)?;

        Ok(Outcome {
            tree: editor,
            conflicts,
            failed_on_first_unresolved_conflict: failed_on_first_conflict,
        })
    }

    /// Produce a unique path within the directory that contains the file at `file_path` (like `a/b`, using `editor` to
    /// obtain the tree at `a/` and `side_name` to more clearly signal where the file is coming from.
    fn unique_path_in_tree(
        file_path: &BStr,
        editor: &gix_object::tree::Editor<'_>,
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
        while editor.get(to_components_bstring_ref(&buf)).is_some() {
            buf.truncate(base_len);
            buf.push_str(format!("_{suffix}",));
            suffix += 1;
        }
        Ok(buf)
    }

    /// Perform a merge between two blobs and return the result of its object id.
    #[allow(clippy::too_many_arguments)]
    fn perform_blob_merge<E>(
        labels: crate::blob::builtin_driver::text::Labels<'_>,
        objects: &impl gix_object::FindObjectOrHeader,
        blob_merge: &mut crate::blob::Platform,
        buf: &mut Vec<u8>,
        write_blob_to_odb: &mut impl FnMut(&[u8]) -> Result<ObjectId, E>,
        location: &BString,
        our_id: ObjectId,
        our_mode: EntryMode,
        their_id: ObjectId,
        their_mode: EntryMode,
        previous_id: ObjectId,
        previous_mode: EntryMode,
        options: &Options,
    ) -> Result<(gix_hash::ObjectId, crate::blob::Resolution), Error>
    where
        E: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    {
        blob_merge.set_resource(
            our_id,
            our_mode.kind(),
            location.as_bstr(),
            ResourceKind::CurrentOrOurs,
            objects,
        )?;
        blob_merge.set_resource(
            their_id,
            their_mode.kind(),
            location.as_bstr(),
            ResourceKind::OtherOrTheirs,
            objects,
        )?;
        blob_merge.set_resource(
            previous_id,
            previous_mode.kind(),
            location.as_bstr(),
            ResourceKind::CommonAncestorOrBase,
            objects,
        )?;
        let prep = blob_merge.prepare_merge(objects, options.blob_merge)?;
        let (pick, resolution) = prep.merge(buf, labels, &options.blob_merge_command_ctx)?;
        let merged_content = prep.buffer_by_pick(pick).unwrap_or(buf);
        let merged_blob_id = write_blob_to_odb(merged_content).map_err(|err| Error::WriteBlobToOdb(err.into()))?;
        Ok((merged_blob_id, resolution))
    }

    /// Only keep leaf nodes, or trees that are the renamed.
    /// Doing so makes it easy to track renamed or rewritten or copied directories, and properly
    /// handle *their* changes that fall within them.
    fn may_track(change: gix_diff::tree_with_rewrites::ChangeRef<'_>) -> bool {
        !change.entry_mode().is_tree() || matches!(change.relation(), Some(Relation::Parent(_)))
    }

    /// Unconditionally apply `change` to `editor`.
    fn apply_change(editor: &mut tree::Editor<'_>, change: &Change) -> Result<(), gix_object::tree::editor::Error> {
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
    enum PossibleConflict {
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
                PossibleConflict::TreeToNonTree { change_idx, .. }
                | PossibleConflict::NonTreeToTree { change_idx, .. } => *change_idx,
                PossibleConflict::Match { change_idx, .. }
                | PossibleConflict::PassedRewrittenDirectory { change_idx, .. } => Some(*change_idx),
            }
        }
    }

    /// The flat list of all tree-nodes so we can avoid having a linked-tree using pointers
    /// which is useful for traversal and initial setup as that can then trivially be non-recursive.
    struct TreeNodes(Vec<TreeNode>);

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
        skip_when_writing: bool,
    }

    impl TreeNode {
        fn is_leaf_node(&self) -> bool {
            self.children.is_empty()
        }
    }

    impl TreeNodes {
        fn new() -> Self {
            TreeNodes(vec![TreeNode::default()])
        }

        /// Write out all `changes` that don't have a conflict marker. Assumed to be the array backing the changes we were initialized with.
        fn apply_nonconflicting_changes(
            &self,
            changes: &[Change],
            editor: &mut tree::Editor<'_>,
        ) -> Result<(), tree::editor::Error> {
            for change_idx in self
                .0
                .iter()
                .filter_map(|n| n.change_idx.filter(|_| !n.skip_when_writing))
            {
                apply_change(editor, &changes[change_idx])?;
            }
            Ok(())
        }

        /// Insert our `change` at `change_idx`, into a linked-tree, assuring that each `change` is non-conflicting
        /// with this tree structure, i.e. reach path is only seen once.
        fn track_ours_exclusive(&mut self, change: &Change, change_idx: usize) {
            let mut components = to_components(change.source_location()).peekable();
            let mut next_index = self.0.len();
            let mut cursor = &mut self.0[0];
            while let Some(component) = components.next() {
                match cursor.children.get(component).copied() {
                    None => {
                        let new_node = TreeNode {
                            children: Default::default(),
                            change_idx: components.peek().is_none().then_some(change_idx),
                            skip_when_writing: false,
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

        /// Search the tree with `our` changes for `theirs` by [`source_location()`](Change::source_location())).
        /// If there is an entry but both are the same, or if there is no entry, return `None`.
        fn check_conflict(&mut self, theirs: &Change) -> Option<PossibleConflict> {
            let components = to_components(theirs.source_location());
            let mut cursor = &mut self.0[0];
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
                            cursor.skip_when_writing = true;
                            Some(PossibleConflict::NonTreeToTree {
                                our_leaf_node_idx: cursor_idx,
                                change_idx: cursor.change_idx,
                            })
                        } else {
                            // a change somewhere else, i.e. `a/c` and we know `a/b` only.
                            intermediate_change.map(|(change, cursor_idx)| {
                                self.0[cursor_idx].skip_when_writing = true;
                                PossibleConflict::PassedRewrittenDirectory { change_idx: change }
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

            cursor.skip_when_writing = true;
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
        fn is_not_same_change_in_possible_conflict(
            &self,
            theirs: &Change,
            conflict: &PossibleConflict,
            our_changes: &[Change],
        ) -> bool {
            conflict.change_idx().map_or(true, |idx| &our_changes[idx] != theirs)
        }

        fn remove_existing_leaf(&mut self, location: &BStr) {
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

        /// After making a modification to the original tree that affects *our* side, update
        /// this tree as well so it reflects the actual tree for future [conflict checks](Self::check_conflict()).
        /// Note that no newly added change will be written to the final tree later, they are assumed to be added already.
        fn insert(&mut self, new_change: &Change, new_change_idx: usize) {
            let mut components = to_components(new_change.location()).peekable();
            let mut next_index = self.0.len();
            let mut cursor = &mut self.0[0];
            while let Some(component) = components.next() {
                match cursor.children.get(component).copied() {
                    None => {
                        let is_last_component = components.peek().is_none();
                        if is_last_component {
                            cursor.children.insert(component.to_owned(), next_index);
                            drop(components);
                            let new_node = TreeNode {
                                children: Default::default(),
                                change_idx: Some(new_change_idx),
                                skip_when_writing: true,
                            };
                            self.0.push(new_node);
                            return;
                        }
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
            drop(components);
            cursor.change_idx = Some(new_change_idx);
        }
    }

    fn to_components_bstring_ref(rela_path: &BString) -> impl Iterator<Item = &BStr> {
        rela_path.split(|b| *b == b'/').map(Into::into)
    }

    fn to_components(rela_path: &BStr) -> impl Iterator<Item = &BStr> {
        rela_path.split(|b| *b == b'/').map(Into::into)
    }

    impl Conflict {
        fn without_resolution(
            resolution: ResolutionFailure,
            ours: &Change,
            theirs: Change,
            map: ConflictMapping,
        ) -> Self {
            Conflict::maybe_resolved(Err(resolution), ours, theirs, map)
        }
        fn with_resolution(resolution: Resolution, ours: &Change, theirs: Change, map: ConflictMapping) -> Self {
            Conflict::maybe_resolved(Ok(resolution), ours, theirs, map)
        }
        fn maybe_resolved(
            resolution: Result<Resolution, ResolutionFailure>,
            ours: &Change,
            theirs: Change,
            map: ConflictMapping,
        ) -> Self {
            Conflict {
                resolution,
                ours: ours.clone(),
                theirs,
                map,
            }
        }
    }
}
