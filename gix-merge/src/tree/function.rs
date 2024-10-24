use crate::tree::utils::{
    apply_change, perform_blob_merge, possibly_rewritten_location, rewrite_location_with_renamed_directory, track,
    unique_path_in_tree, CheckConflict, PossibleConflict, TreeNodes,
};
use crate::tree::ConflictMapping::{Original, Swapped};
use crate::tree::{Conflict, ConflictMapping, ContentMerge, Error, Options, Outcome, Resolution, ResolutionFailure};
use bstr::ByteSlice;
use gix_diff::tree::recorder::Location;
use gix_diff::tree_with_rewrites::Change;
use gix_hash::ObjectId;
use gix_object::tree::EntryKind;
use gix_object::{tree, FindExt};
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
/// ### Unbiased (Ours x Theirs == Theirs x Ours)
///
/// The algorithm is implemented so that the result is the same no matter how the sides are ordered.
///
/// ### Differences to Merge-ORT
///
/// Merge-ORT (Git) defines the desired outcomes where are merely mimicked here. The algorithms are different, and it's
/// clear that Merge-ORT is significantly more elaborate and general.
///
/// It also writes out trees once it's done with them in a form of reduction process, here an editor is used
/// to keep only the changes, to be written by the caller who receives it as part of the result.
/// This may use more memory in the worst case scenario, but in average *shouldn't* perform much worse due to the
/// natural sparsity of the editor.
///
/// Our rename-tracking also produces copy information, but we discard it and simply treat it like an addition.
///
/// Finally, our algorithm will consider reasonable solutions to merge-conflicts as conflicts that are resolved, leaving
/// only content with conflict markers as unresolved ones.
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
            track(change, &mut our_changes);
            Ok(gix_diff::tree_with_rewrites::Action::Continue)
        },
        gix_diff::tree_with_rewrites::Options {
            location: Some(Location::Path),
            rewrites: options.rewrites,
        },
    )?;

    let mut our_tree = TreeNodes::new();
    for (idx, change) in our_changes.iter().enumerate() {
        our_tree.track_ours_exclusive(change, idx);
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
            track(change, &mut their_changes);
            Ok(gix_diff::tree_with_rewrites::Action::Continue)
        },
        gix_diff::tree_with_rewrites::Options {
            location: Some(Location::Path),
            rewrites: options.rewrites,
        },
    )?;

    let mut their_tree = TreeNodes::new();
    for (idx, change) in their_changes.iter().enumerate() {
        their_tree.track_ours_exclusive(change, idx);
    }

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

    let mut segment_start = 0;
    let mut last_seen_len = their_changes.len();

    while segment_start != last_seen_len {
        for theirs_idx in segment_start..last_seen_len {
            // `their` can be a tree, and it could be used to efficiently prune child-changes as these
            // trees are always rewrites with parent ids (of course we validate), so child-changes could be handled
            // quickly. However, for now the benefit of having these trees is to have them as part of the match-tree
            // on *our* side so that it's clear that we passed a renamed directory (by identity).
            let theirs = &their_changes[theirs_idx];
            if theirs.entry_mode().is_tree() {
                continue;
            }

            match our_tree
                .check_conflict(theirs.source_location(), CheckConflict::PassedNodesDoNotWrite)
                .filter(|ours| our_tree.is_not_same_change_in_possible_conflict(theirs, ours, &our_changes))
            {
                None => {
                    apply_change(&mut editor, theirs)?;
                }
                Some(candidate) => {
                    use crate::tree::utils::to_components_bstring_ref as toc;
                    let ours_idx = match candidate {
                        PossibleConflict::PassedRewrittenDirectory { change_idx } => {
                            let new_location =
                                rewrite_location_with_renamed_directory(theirs.location(), &our_changes[change_idx]);
                            todo!("rewritten directory changes the destination directory of their change by prefix: {new_location:?}");
                        }
                        PossibleConflict::TreeToNonTree {
                            our_node_idx: _,
                            change_idx,
                        } if change_idx.map_or(false, |idx| matches!(our_changes[idx], Change::Deletion { .. })) => {
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
                    match (ours, theirs) {
                        (
                            Change::Modification {
                                previous_id,
                                previous_entry_mode,
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
                                ..
                            },
                        )
                        | (
                            Change::Rewrite {
                                source_id: their_source_id,
                                id: their_id,
                                location: their_location,
                                entry_mode: their_mode,
                                ..
                            },
                            Change::Modification {
                                previous_id,
                                previous_entry_mode,
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
                            let side = if matches!(ours, Change::Modification { .. }) {
                                Original
                            } else {
                                Swapped
                            };
                            let their_rewritten_location = possibly_rewritten_location(
                                pick_our_tree(side, &mut our_tree, &mut their_tree),
                                their_location.as_ref(),
                                pick_our_changes(side, &our_changes, &their_changes),
                            );
                            let renamed_without_change = their_source_id == their_id;
                            let (our_id, resolution) = if renamed_without_change {
                                (*our_id, None)
                            } else {
                                let (our_location, our_id, our_mode, their_location, their_id, their_mode) = match side
                                {
                                    Original => (our_location, our_id, our_mode, their_location, their_id, their_mode),
                                    Swapped => (their_location, their_id, their_mode, our_location, our_id, our_mode),
                                };
                                let (merged_blob_id, resolution) = perform_blob_merge(
                                    labels,
                                    objects,
                                    blob_merge,
                                    &mut diff_state.buf1,
                                    &mut write_blob_to_odb,
                                    (our_location, *our_id, *our_mode),
                                    (their_location, *their_id, *their_mode),
                                    (our_location, *previous_id, *previous_entry_mode),
                                    0,
                                    &options,
                                )?;
                                (merged_blob_id, Some(resolution))
                            };

                            editor.remove(toc(our_location))?;
                            pick_our_tree(side, &mut our_tree, &mut their_tree)
                                .remove_existing_leaf(our_location.as_bstr());
                            let final_location = (their_rewritten_location.as_ref() != their_location)
                                .then(|| their_rewritten_location.clone().into_owned());
                            let new_change = Change::Addition {
                                location: their_rewritten_location.into_owned(),
                                relation: None,
                                entry_mode: *their_mode,
                                id: our_id,
                            };
                            if should_fail_on_unresolved_conflict(Conflict::with_resolution(
                                Resolution::OursModifiedTheirsRenamedAndChangedThenRename {
                                    merged_blob: resolution.map(|resolution| ContentMerge {
                                        resolution,
                                        merged_blob_id: our_id,
                                    }),
                                    final_location,
                                },
                                (ours, theirs, side),
                            )) {
                                break;
                            }

                            // The other side gets the addition, not our side.
                            pick_our_tree(side, &mut their_tree, &mut our_tree).insert(
                                new_change,
                                pick_our_changes_mut(side, &mut their_changes, &mut our_changes),
                            );
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
                                (location, *our_id, *our_mode),
                                (location, *their_id, *their_mode),
                                (location, *previous_id, *previous_entry_mode),
                                0,
                                &options,
                            )?;
                            editor.upsert(toc(location), our_mode.kind(), merged_blob_id)?;
                            if should_fail_on_unresolved_conflict(Conflict::with_resolution(
                                Resolution::OursModifiedTheirsModifiedThenBlobContentMerge {
                                    merged_blob: ContentMerge {
                                        resolution,
                                        merged_blob_id,
                                    },
                                },
                                (ours, theirs, Original),
                            )) {
                                break;
                            };
                        }
                        (
                            Change::Addition {
                                location,
                                entry_mode: our_mode,
                                id: our_id,
                                ..
                            },
                            Change::Addition {
                                entry_mode: their_mode,
                                id: their_id,
                                ..
                            },
                        ) if our_id != their_id => {
                            let conflict = if our_mode.kind() == their_mode.kind() {
                                let (merged_blob_id, resolution) = perform_blob_merge(
                                    labels,
                                    objects,
                                    blob_merge,
                                    &mut diff_state.buf1,
                                    &mut write_blob_to_odb,
                                    (location, *our_id, *our_mode),
                                    (location, *their_id, *their_mode),
                                    (location, their_id.kind().null(), *our_mode),
                                    0,
                                    &options,
                                )?;
                                editor.upsert(toc(location), our_mode.kind(), merged_blob_id)?;
                                Conflict::with_resolution(
                                    Resolution::OursModifiedTheirsModifiedThenBlobContentMerge {
                                        merged_blob: ContentMerge {
                                            resolution,
                                            merged_blob_id,
                                        },
                                    },
                                    (ours, theirs, Original),
                                )
                            } else {
                                // Actually this has a preference, as symlinks are always left in place with the other side renamed.
                                let (side, label_of_side_to_be_moved, (our_mode, our_id), (their_mode, their_id)) =
                                    if matches!(our_mode.kind(), EntryKind::Link) {
                                        (
                                            Original,
                                            labels.other.unwrap_or_default(),
                                            (*our_mode, *our_id),
                                            (*their_mode, *their_id),
                                        )
                                    } else {
                                        (
                                            Swapped,
                                            labels.current.unwrap_or_default(),
                                            (*their_mode, *their_id),
                                            (*our_mode, *our_id),
                                        )
                                    };
                                let renamed_location = unique_path_in_tree(
                                    location.as_bstr(),
                                    &editor,
                                    pick_our_tree(side, &mut their_tree, &mut our_tree),
                                    label_of_side_to_be_moved,
                                )?;
                                editor.upsert(toc(location), our_mode.kind(), our_id)?;
                                let res = Conflict::with_resolution(
                                    Resolution::OursAddedTheirsAddedTypeMismatch {
                                        their_final_location: renamed_location.clone(),
                                    },
                                    (ours, theirs, side),
                                );

                                their_tree.insert(
                                    Change::Addition {
                                        location: renamed_location,
                                        entry_mode: their_mode,
                                        id: their_id,
                                        relation: None,
                                    },
                                    &mut their_changes,
                                );
                                res
                            };

                            if should_fail_on_unresolved_conflict(conflict) {
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
                            let (label_of_side_to_be_moved, side) = if matches!(ours, Change::Modification { .. }) {
                                (labels.current.unwrap_or_default(), Original)
                            } else {
                                (labels.other.unwrap_or_default(), Swapped)
                            };
                            let deletion_prefaces_addition_of_directory = {
                                let change_on_right = match side {
                                    Original => their_changes.get(theirs_idx + 1),
                                    Swapped => our_changes.get(ours_idx + 1),
                                };
                                change_on_right
                                    .map(|change| change.entry_mode().is_tree() && change.location() == location)
                                    .unwrap_or_default()
                            };

                            if deletion_prefaces_addition_of_directory {
                                let renamed_path = unique_path_in_tree(
                                    location.as_bstr(),
                                    &editor,
                                    pick_our_tree(side, &mut our_tree, &mut their_tree),
                                    label_of_side_to_be_moved,
                                )?;
                                editor.remove(toc(location))?;
                                pick_our_tree(side, &mut our_tree, &mut their_tree)
                                    .remove_existing_leaf(location.as_bstr());

                                let new_change = Change::Addition {
                                    location: renamed_path.clone(),
                                    relation: None,
                                    entry_mode: *entry_mode,
                                    id: *id,
                                };
                                let should_break = should_fail_on_unresolved_conflict(Conflict::without_resolution(
                                    ResolutionFailure::OursModifiedTheirsDirectoryThenOursRenamed {
                                        renamed_path_to_modified_blob: renamed_path,
                                    },
                                    (ours, theirs, side),
                                ));

                                // Since we move *our* side, our tree needs to be modified.
                                pick_our_tree(side, &mut our_tree, &mut their_tree).insert(
                                    new_change,
                                    pick_our_changes_mut(side, &mut our_changes, &mut their_changes),
                                );

                                if should_break {
                                    break;
                                };
                            } else {
                                todo!("ordinary mod/del")
                            }
                        }
                        (
                            Change::Rewrite {
                                source_location,
                                source_entry_mode,
                                source_id,
                                entry_mode: our_mode,
                                id: our_id,
                                location: our_location,
                                ..
                            },
                            Change::Rewrite {
                                entry_mode: their_mode,
                                id: their_id,
                                location: their_location,
                                ..
                            },
                        ) => {
                            if our_mode == their_mode {
                                // TODO: test
                                let (merged_blob_id, resolution) = if our_id == their_id {
                                    (*our_id, None)
                                } else {
                                    let (id, resolution) = perform_blob_merge(
                                        labels,
                                        objects,
                                        blob_merge,
                                        &mut diff_state.buf1,
                                        &mut write_blob_to_odb,
                                        (our_location, *our_id, *our_mode),
                                        (their_location, *their_id, *their_mode),
                                        (source_location, *source_id, *source_entry_mode),
                                        1,
                                        &options,
                                    )?;
                                    (id, Some(resolution))
                                };

                                editor.remove(toc(source_location))?;
                                our_tree.remove_existing_leaf(source_location.as_bstr());
                                their_tree.remove_existing_leaf(source_location.as_bstr());

                                let our_addition = Change::Addition {
                                    location: our_location.to_owned(),
                                    relation: None,
                                    entry_mode: *our_mode,
                                    id: merged_blob_id,
                                };
                                let their_addition = Change::Addition {
                                    location: their_location.to_owned(),
                                    relation: None,
                                    entry_mode: *their_mode,
                                    id: merged_blob_id,
                                };
                                if let Some(resolution) = resolution {
                                    if should_fail_on_unresolved_conflict(Conflict::with_resolution(
                                        Resolution::OursModifiedTheirsModifiedThenBlobContentMerge {
                                            merged_blob: ContentMerge {
                                                resolution,
                                                merged_blob_id,
                                            },
                                        },
                                        (ours, theirs, Original),
                                    )) {
                                        break;
                                    };
                                }

                                our_tree.insert(our_addition, &mut our_changes);
                                their_tree.insert(their_addition, &mut their_changes);
                            } else {
                                todo!("different mode, maybe default to conflict (test in Git)")
                            }
                        }
                        (
                            Change::Deletion { .. },
                            Change::Rewrite {
                                source_location,
                                entry_mode,
                                id,
                                location,
                                ..
                            },
                        )
                        | (
                            Change::Rewrite {
                                source_location,
                                entry_mode,
                                id,
                                location,
                                ..
                            },
                            Change::Deletion { .. },
                        ) => {
                            let side = if matches!(ours, Change::Deletion { .. }) {
                                Original
                            } else {
                                Swapped
                            };

                            editor.remove(toc(source_location))?;
                            pick_our_tree(side, &mut our_tree, &mut their_tree)
                                .remove_existing_leaf(source_location.as_bstr());

                            let our_addition = Change::Addition {
                                location: location.to_owned(),
                                relation: None,
                                entry_mode: *entry_mode,
                                id: *id,
                            };

                            if should_fail_on_unresolved_conflict(Conflict::without_resolution(
                                ResolutionFailure::OursDeletedTheirsRenamed,
                                (ours, theirs, side),
                            )) {
                                break;
                            };

                            pick_our_tree(side, &mut their_tree, &mut our_tree).insert(
                                our_addition,
                                pick_our_changes_mut(side, &mut their_changes, &mut our_changes),
                            );
                        }
                        (
                            Change::Rewrite {
                                source_location,
                                source_entry_mode,
                                source_id,
                                entry_mode: our_mode,
                                id: our_id,
                                location,
                                ..
                            },
                            Change::Addition {
                                id: their_id,
                                entry_mode: their_mode,
                                ..
                            },
                        )
                        | (
                            Change::Addition {
                                id: their_id,
                                entry_mode: their_mode,
                                ..
                            },
                            Change::Rewrite {
                                source_location,
                                source_entry_mode,
                                source_id,
                                entry_mode: our_mode,
                                id: our_id,
                                location,
                                ..
                            },
                        ) if our_mode == their_mode => {
                            // TODO: test
                            let (merged_blob_id, resolution) = if our_id == their_id {
                                (*our_id, None)
                            } else {
                                let (id, resolution) = perform_blob_merge(
                                    labels,
                                    objects,
                                    blob_merge,
                                    &mut diff_state.buf1,
                                    &mut write_blob_to_odb,
                                    (location, *our_id, *our_mode),
                                    (location, *their_id, *their_mode),
                                    (source_location, source_id.kind().null(), *source_entry_mode),
                                    0,
                                    &options,
                                )?;
                                (id, Some(resolution))
                            };

                            editor.remove(toc(source_location))?;
                            our_tree.remove_existing_leaf(source_location.as_bstr());
                            their_tree.remove_existing_leaf(source_location.as_bstr());

                            if let Some(resolution) = resolution {
                                if should_fail_on_unresolved_conflict(Conflict::with_resolution(
                                    Resolution::OursModifiedTheirsModifiedThenBlobContentMerge {
                                        merged_blob: ContentMerge {
                                            resolution,
                                            merged_blob_id,
                                        },
                                    },
                                    (ours, theirs, Original),
                                )) {
                                    break;
                                };
                            }

                            // Because this constellation can only be found by the lookup tree, there is
                            // no need to put it as addition, we know it's not going to intersect on the other side.
                            editor.upsert(toc(location), our_mode.kind(), merged_blob_id)?;
                        }
                        unknown => {
                            todo!("all other cases we can test, then default this to be a conflict: {unknown:?}")
                        }
                    }
                }
            }
        }
        segment_start = last_seen_len;
        last_seen_len = their_changes.len();
        dbg!(&their_changes[segment_start..last_seen_len]);
    }

    // TODO: make sure rewrites are properly looked up on `their_tree` (right?)
    our_tree.apply_nonconflicting_changes(&our_changes, &mut editor)?;

    Ok(Outcome {
        tree: editor,
        conflicts,
        failed_on_first_unresolved_conflict: failed_on_first_conflict,
    })
}

fn pick_our_tree<'a>(side: ConflictMapping, ours: &'a mut TreeNodes, theirs: &'a mut TreeNodes) -> &'a mut TreeNodes {
    match side {
        Original => ours,
        Swapped => theirs,
    }
}

fn pick_our_changes<'a>(side: ConflictMapping, ours: &'a [Change], theirs: &'a [Change]) -> &'a [Change] {
    match side {
        Original => ours,
        Swapped => theirs,
    }
}

fn pick_our_changes_mut<'a>(
    side: ConflictMapping,
    ours: &'a mut Vec<Change>,
    theirs: &'a mut Vec<Change>,
) -> &'a mut Vec<Change> {
    match side {
        Original => ours,
        Swapped => theirs,
    }
}
