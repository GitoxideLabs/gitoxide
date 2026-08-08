//! Tree-merge scheduling and conflict resolution.
//!
//! See [`tree()`] for the main entrypoint and how it works.

use std::borrow::Cow;

use bstr::{BString, ByteSlice};
use gix_diff::tree_with_rewrites::Change;
use gix_hash::ObjectId;
use gix_object::{
    FindExt, tree,
    tree::{EntryKind, EntryMode},
};

use crate::tree::{
    Conflict, ConflictIndexEntry, ConflictIndexEntryPathHint, ConflictMapping,
    ConflictMapping::{Original, Swapped},
    ContentMerge, Error, Options, Outcome, Resolution, ResolutionFailure, ResolveWith,
    utils::{
        ChangeDisposition, ChangeList, PossibleConflict, TrackedChange, apply_change, perform_blob_merge,
        possibly_rewritten_location, rewrite_location_with_renamed_directory, to_components, unique_path_in_tree,
    },
};

use super::change::{MatchKind, collect as collect_changes, matching as matching_change, pair as pair_candidate};

/// Perform a merge between `our_tree` and `their_tree`, using `base_tree` as merge-base.
/// Note that `base_tree` can be an empty tree to indicate 'no common ancestor between the two sides'.
///
/// * `labels` are relevant for text-merges and will be shown in conflicts.
/// * `objects` provides access to trees when diffing them.
/// * `write_blob_to_odb(content) -> Result<ObjectId, E>` writes newly merged content into the odb to obtain an id
///   that will be used in merged trees.
/// * `diff_state` is state used for diffing trees.
/// * `diff_resource_cache` is used for similarity checks.
/// * `blob_merge` is a pre-configured platform to merge any content.
///     - Note that it shouldn't be allowed to read from the worktree, given that this is a tree-merge.
/// * `options` are used to affect how the merge is performed.
///
/// ### Side handling
///
/// The scheduler swaps the sides to share resolution logic instead of generally privileging one input.
/// Exact merge symmetry is not guaranteed: ambiguous rename candidates and some overlapping structural
/// conflicts can currently produce different trees when ours and theirs are reversed. Conflict-marker
/// content and forced "ours" resolution are directional by definition.
///
/// ### Algorithm
///
/// 1. Diff the ancestor against each side, including rename detection, to obtain two flat lists of tracked changes.
/// 2. Build a path tree for each list. Its nodes point back to list entries and make same-path, tree/non-tree,
///    and renamed-directory interactions discoverable.
/// 3. Start an editor at the ancestor tree and process pending changes from one list against the other list's path tree.
///    A change can be applied directly, paired with another change and merged, consumed only as part of a conflict, or
///    transformed into a deferred change at a rewritten or unique path.
/// 4. Append deferred changes as pending work, then swap the two side-lists and repeat until neither side has pending
///    changes. Swapping roles lets the same scheduling and resolution code process both inputs.
///
/// Each tracked change therefore records both whether it still needs processing and whether its effect is actually
/// represented in the editor. This is why "processed without application" is distinct from "applied": a forced
/// ancestor resolution may consume a deletion while retaining the ancestor entry, and later path conflicts must not
/// behave as if that deletion had removed it.
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
#[expect(clippy::too_many_arguments)]
pub fn tree<'objects, E>(
    base_tree: &gix_hash::oid,
    our_tree: &gix_hash::oid,
    their_tree: &gix_hash::oid,
    mut labels: crate::blob::builtin_driver::text::Labels<'_>,
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
    let _span = gix_trace::coarse!("gix_merge::tree", ?base_tree, ?our_tree, ?their_tree, ?labels);
    let (mut base_buf, mut side_buf) = (Vec::new(), Vec::new());
    let ancestor_tree = objects.find_tree(base_tree, &mut base_buf)?;
    let mut editor = tree::Editor::new(ancestor_tree.to_owned(), objects, base_tree.kind());
    let tree_conflicts = options.tree_conflicts;

    let mut ours = collect_changes(
        base_tree,
        our_tree,
        &base_buf,
        &mut side_buf,
        objects,
        diff_resource_cache,
        diff_state,
        options.rewrites,
    )?;
    let mut theirs = collect_changes(
        base_tree,
        their_tree,
        &base_buf,
        &mut side_buf,
        objects,
        diff_resource_cache,
        diff_state,
        options.rewrites,
    )?;
    let mut conflicts = Vec::new();
    let mut failed_on_first_conflict = false;
    let mut should_fail_on_conflict = |mut conflict: Conflict| -> bool {
        if tree_conflicts.is_some() {
            if let Err(failure) = conflict.resolution {
                conflict.resolution = Ok(Resolution::Forced(failure));
            }
        }
        if let Some(how) = options.fail_on_conflict {
            if conflict.resolution.is_err() || conflict.is_unresolved(how) {
                failed_on_first_conflict = true;
            }
        }
        conflicts.push(conflict);
        failed_on_first_conflict
    };

    // Ambiguous rewrite identities otherwise make the side processed first decide
    // which repeated additions are paired. Give both directions the same schedule.
    let canonicalize_schedule = ours.has_ambiguous_rewrite_sources() || theirs.has_ambiguous_rewrite_sources();
    let swap_sides_for_schedule = canonicalize_schedule && theirs.cmp_for_scheduling(&ours).is_gt();
    let ((mut our_changes, mut our_tree), (mut their_changes, mut their_tree)) = (ours.parts_mut(), theirs.parts_mut());
    let mut outer_side = Original;
    if their_changes.is_empty() || (!our_changes.is_empty() && swap_sides_for_schedule) {
        ((our_changes, our_tree), (their_changes, their_tree)) = ((their_changes, their_tree), (our_changes, our_tree));
        (labels.current, labels.other) = (labels.other, labels.current);
        outer_side = outer_side.swapped();
    }

    'outer: while their_changes.iter().rev().any(TrackedChange::is_pending) {
        let mut segment_start = 0;
        let mut last_seen_len = their_changes.len();

        while segment_start != last_seen_len {
            for theirs_idx in segment_start..last_seen_len {
                // `their` can be a tree, and it could be used to efficiently prune child-changes as these
                // trees are always rewrites with parent ids (of course we validate), so child-changes could be handled
                // quickly. However, for now the benefit of having these trees is to have them as part of the match-tree
                // on *our* side so that it's clear that we passed a renamed directory (by identity).
                let TrackedChange {
                    inner: theirs,
                    needs_tree_insertion,
                    rewritten_location,
                    ..
                } = &their_changes[theirs_idx];
                if theirs.entry_mode().is_tree() || !their_changes[theirs_idx].is_pending() {
                    continue;
                }

                if needs_tree_insertion.is_some() {
                    their_tree.insert(theirs, theirs_idx);
                }

                match matching_change(
                    theirs,
                    *needs_tree_insertion,
                    rewritten_location.as_ref(),
                    our_tree,
                    our_changes,
                ) {
                    None => {
                        if let Some((rewritten_location, ours_idx)) = rewritten_location {
                            // `no_entry` to the index because that's not a conflict at all,
                            // but somewhat advanced rename tracking.
                            if should_fail_on_conflict(Conflict::with_resolution(
                                Resolution::SourceLocationAffectedByRename {
                                    final_location: rewritten_location.to_owned(),
                                },
                                (&our_changes[*ours_idx].inner, theirs, Original, outer_side),
                                [None, None, None],
                            )) {
                                break 'outer;
                            }
                            editor.remove(to_components(theirs.location()))?;
                        }
                        apply_change(&mut editor, theirs, rewritten_location.as_ref().map(|t| &t.0))?;
                        their_changes[theirs_idx].mark_applied();
                    }
                    Some(candidate) => {
                        use crate::tree::utils::to_components_bstring_ref as toc;

                        if let PossibleConflict::PassedRewrittenDirectory { change_idx } = candidate {
                            let ours = &our_changes[change_idx];
                            let location_after_passed_rename =
                                rewrite_location_with_renamed_directory(theirs.location(), &ours.inner);
                            if let Some(new_location) = location_after_passed_rename {
                                // Another structural conflict may already have consumed this scheduling node.
                                their_tree.remove_change(theirs.location());
                                push_deferred_with_rewrite(
                                    (theirs.clone(), Some(change_idx)),
                                    Some((new_location, change_idx)),
                                    their_changes,
                                );
                            } else {
                                apply_change(&mut editor, theirs, None)?;
                                their_changes[theirs_idx].mark_applied();
                            }
                            their_changes[theirs_idx].mark_processed();
                            continue;
                        }

                        let (ours_idx, match_kind) = pair_candidate(&candidate, our_changes);
                        let Some(ours_idx) = ours_idx else {
                            let ours = match candidate {
                                PossibleConflict::TreeToNonTree { change_idx, .. }
                                | PossibleConflict::NonTreeToTree { change_idx, .. } => change_idx,
                                PossibleConflict::Match { change_idx }
                                | PossibleConflict::PassedRewrittenDirectory { change_idx } => Some(change_idx),
                            }
                            .map(|idx| &mut our_changes[idx]);

                            if let Some(ours) = ours {
                                gix_trace::debug!(
                                    "Turning a case we could probably handle into a conflict for now. theirs: {theirs:#?} ours: {ours:#?} kind: {match_kind:?}"
                                );
                                let conflict = Conflict::unknown((&ours.inner, theirs, Original, outer_side));
                                if let Some(ResolveWith::Ours) = tree_conflicts {
                                    apply_our_resolution(&ours.inner, theirs, outer_side, &mut editor)?;
                                    match outer_side {
                                        Original => ours.mark_applied(),
                                        Swapped => their_changes[theirs_idx].mark_applied(),
                                    }
                                }
                                if should_fail_on_conflict(conflict) {
                                    break 'outer;
                                }
                            } else if matches!(candidate, PossibleConflict::TreeToNonTree { .. }) {
                                let (mode, id) = theirs.entry_mode_and_id();
                                let location = theirs.location();
                                if needs_tree_insertion.is_some() {
                                    their_tree.remove_change(location);
                                }
                                let renamed_location = unique_path_in_tree(
                                    location.as_bstr(),
                                    &editor,
                                    their_tree,
                                    labels.other.unwrap_or_default(),
                                )?;
                                match tree_conflicts {
                                    None => {
                                        editor.upsert(toc(&renamed_location), mode.kind(), id.to_owned())?;
                                    }
                                    Some(ResolveWith::Ours) => {
                                        if outer_side.is_swapped() {
                                            editor.upsert(to_components(location), mode.kind(), id.to_owned())?;
                                        }
                                    }
                                    Some(ResolveWith::Ancestor) => {
                                        // we found no matching node of 'ours', so nothing to apply here.
                                    }
                                }

                                let conflict = Conflict::without_resolution(
                                    ResolutionFailure::OursDirectoryTheirsNonDirectoryTheirsRenamed {
                                        renamed_unique_path_of_theirs: renamed_location,
                                    },
                                    (theirs, theirs, Original, outer_side),
                                    [
                                        None,
                                        None,
                                        index_entry_at_path(
                                            &mode.kind().into(),
                                            &id.to_owned(),
                                            ConflictIndexEntryPathHint::RenamedOrTheirs,
                                        ),
                                    ],
                                );
                                their_changes[theirs_idx].mark_processed();
                                if should_fail_on_conflict(conflict) {
                                    break 'outer;
                                }
                            } else if matches!(candidate, PossibleConflict::NonTreeToTree { .. }) {
                                // We are writing on top of what was a file, a conflict we probably already saw and dealt with.
                                let location = theirs.location();
                                let (mode, id) = theirs.entry_mode_and_id();
                                editor.upsert(to_components(location), mode.kind(), id.to_owned())?;
                                their_changes[theirs_idx].mark_applied();
                            } else {
                                gix_trace::debug!(
                                    "Couldn't figure out how to handle {match_kind:?} theirs: {theirs:#?} candidate: {candidate:#?}"
                                );
                            }
                            continue;
                        };

                        let mut ours_disposition = ChangeDisposition::Processed;
                        let mut theirs_disposition = ChangeDisposition::Processed;
                        let ours = &our_changes[ours_idx].inner;
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
                                    source_location,
                                    ..
                                },
                            )
                            | (
                                Change::Rewrite {
                                    source_id: their_source_id,
                                    id: their_id,
                                    location: their_location,
                                    entry_mode: their_mode,
                                    source_location,
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
                            ) => {
                                let side = if matches!(ours, Change::Modification { .. }) {
                                    Original
                                } else {
                                    Swapped
                                };
                                if let Some(merged_mode) = merge_modes(*our_mode, *their_mode) {
                                    debug_assert_eq!(
                                        previous_id, their_source_id,
                                        "both refer to the same base, so should always match"
                                    );
                                    let their_rewritten_location = possibly_rewritten_location(
                                        pick_mut(side, our_tree, their_tree),
                                        their_location.as_ref(),
                                        pick(side, our_changes, their_changes),
                                    );
                                    let renamed_without_change = their_source_id == their_id;
                                    let (merged_blob_id, resolution) = if renamed_without_change {
                                        (*our_id, None)
                                    } else {
                                        let (our_location, our_id, our_mode, their_location, their_id, their_mode) =
                                            match side {
                                                Original => (
                                                    our_location,
                                                    our_id,
                                                    our_mode,
                                                    their_location,
                                                    their_id,
                                                    their_mode,
                                                ),
                                                Swapped => (
                                                    their_location,
                                                    their_id,
                                                    their_mode,
                                                    our_location,
                                                    our_id,
                                                    our_mode,
                                                ),
                                            };
                                        let (merged_blob_id, resolution) = perform_blob_merge(
                                            labels,
                                            objects,
                                            blob_merge,
                                            &mut diff_state.buf1,
                                            &mut write_blob_to_odb,
                                            (our_location, *our_id, *our_mode),
                                            (their_location, *their_id, *their_mode),
                                            (source_location, *previous_id, *previous_entry_mode),
                                            (0, outer_side),
                                            &options,
                                        )?;
                                        (merged_blob_id, Some(resolution))
                                    };

                                    editor.remove(toc(our_location))?;
                                    pick_mut(side, our_tree, their_tree).remove_existing_change(our_location.as_bstr());
                                    let final_location = their_rewritten_location.clone();
                                    let new_change = Change::Addition {
                                        location: their_rewritten_location.unwrap_or_else(|| their_location.to_owned()),
                                        relation: None,
                                        entry_mode: merged_mode,
                                        id: merged_blob_id,
                                    };
                                    if should_fail_on_conflict(Conflict::with_resolution(
                                        Resolution::OursModifiedTheirsRenamedAndChangedThenRename {
                                            merged_mode: (merged_mode != *their_mode).then_some(merged_mode),
                                            merged_blob: resolution.map(|resolution| ContentMerge {
                                                resolution,
                                                merged_blob_id,
                                            }),
                                            final_location,
                                        },
                                        (ours, theirs, side, outer_side),
                                        [
                                            index_entry(previous_entry_mode, previous_id),
                                            index_entry(our_mode, our_id),
                                            index_entry(their_mode, their_id),
                                        ],
                                    )) {
                                        break 'outer;
                                    }

                                    // The other side gets the addition, not our side.
                                    push_deferred((new_change, None), pick_mut(side, their_changes, our_changes));
                                } else {
                                    match tree_conflicts {
                                        None => {
                                            // keep both states - 'our_location' is the previous location as well.
                                            editor.upsert(toc(our_location), our_mode.kind(), *our_id)?;
                                            editor.upsert(toc(their_location), their_mode.kind(), *their_id)?;
                                        }
                                        Some(ResolveWith::Ours) => {
                                            editor.remove(toc(source_location))?;
                                            if side.to_global(outer_side).is_swapped() {
                                                editor.upsert(toc(their_location), their_mode.kind(), *their_id)?;
                                            } else {
                                                editor.upsert(toc(our_location), our_mode.kind(), *our_id)?;
                                            }
                                        }
                                        Some(ResolveWith::Ancestor) => {}
                                    }

                                    if should_fail_on_conflict(Conflict::without_resolution(
                                        ResolutionFailure::OursModifiedTheirsRenamedTypeMismatch,
                                        (ours, theirs, side, outer_side),
                                        [
                                            index_entry_at_path(
                                                previous_entry_mode,
                                                previous_id,
                                                ConflictIndexEntryPathHint::RenamedOrTheirs,
                                            ),
                                            None,
                                            index_entry_at_path(
                                                their_mode,
                                                their_id,
                                                ConflictIndexEntryPathHint::RenamedOrTheirs,
                                            ),
                                        ],
                                    )) {
                                        break 'outer;
                                    }
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
                            ) if !involves_submodule(our_mode, their_mode)
                                && merge_modes(*our_mode, *their_mode).is_some()
                                && our_id != their_id =>
                            {
                                let previous_is_compatible = merge_modes(*our_mode, *previous_entry_mode).is_some()
                                    && merge_modes(*their_mode, *previous_entry_mode).is_some();
                                let merged_mode = if previous_is_compatible {
                                    merge_modes_prev(*our_mode, *their_mode, *previous_entry_mode)
                                } else {
                                    merge_modes(*our_mode, *their_mode)
                                }
                                .expect("the match guard assures compatible current modes");
                                let (merge_base_id, merge_base_mode) = if previous_is_compatible {
                                    (*previous_id, *previous_entry_mode)
                                } else {
                                    (previous_id.kind().null(), merged_mode)
                                };
                                let (merged_blob_id, resolution) = perform_blob_merge(
                                    labels,
                                    objects,
                                    blob_merge,
                                    &mut diff_state.buf1,
                                    &mut write_blob_to_odb,
                                    (location, *our_id, *our_mode),
                                    (location, *their_id, *their_mode),
                                    (location, merge_base_id, merge_base_mode),
                                    (0, outer_side),
                                    &options,
                                )?;

                                editor.upsert(toc(location), merged_mode.kind(), merged_blob_id)?;
                                if should_fail_on_conflict(Conflict::with_resolution(
                                    Resolution::OursModifiedTheirsModifiedThenBlobContentMerge {
                                        merged_blob: ContentMerge {
                                            resolution,
                                            merged_blob_id,
                                        },
                                    },
                                    (ours, theirs, Original, outer_side),
                                    [
                                        index_entry(previous_entry_mode, previous_id),
                                        index_entry(our_mode, our_id),
                                        index_entry(their_mode, their_id),
                                    ],
                                )) {
                                    break 'outer;
                                }
                            }
                            (Change::Deletion { .. }, Change::Addition { .. })
                                if matches!(match_kind, Some(MatchKind::EraseLeaf))
                                    && !our_changes[ours_idx].was_processed_without_application() =>
                            {
                                // Let the shared parent deletion pair with the other side's deletion first.
                                // Applying it after this descendant would remove the newly created directory.
                                push_deferred((theirs.clone(), Some(ours_idx)), their_changes);
                            }
                            (Change::Rewrite { .. }, Change::Addition { relation: Some(_), .. })
                                if matches!(match_kind, Some(MatchKind::EraseLeaf))
                                    && needs_tree_insertion.is_none()
                                    && !matches!(tree_conflicts, Some(ResolveWith::Ancestor)) =>
                            {
                                // Let the replacement's parent deletion resolve the rename first.
                                // The deferred child then ignores this already-handled rewrite.
                                push_deferred((theirs.clone(), Some(ours_idx)), their_changes);
                            }
                            (Change::Rewrite { .. }, Change::Addition { .. })
                                if matches!(match_kind, Some(MatchKind::EraseLeaf)) && rewritten_location.is_some() =>
                            {
                                // An explicit file rename blocks the inferred directory-rename destination.
                                // Keep the explicit rename and apply the addition at its original location.
                                apply_change(&mut editor, ours, None)?;
                                apply_change(&mut editor, theirs, None)?;
                                ours_disposition = ChangeDisposition::Applied;
                                theirs_disposition = ChangeDisposition::Applied;
                            }
                            (
                                Change::Rewrite {
                                    source_location,
                                    entry_mode: blocking_mode,
                                    id: blocking_id,
                                    location: blocking_location,
                                    ..
                                },
                                Change::Addition { .. },
                            ) if matches!(match_kind, Some(MatchKind::EraseLeaf)) => {
                                let renamed_location = unique_path_in_tree(
                                    blocking_location.as_bstr(),
                                    &editor,
                                    our_tree,
                                    labels.current.unwrap_or_default(),
                                )?;
                                let conflict = Conflict::without_resolution(
                                    ResolutionFailure::OursDirectoryTheirsNonDirectoryTheirsRenamed {
                                        renamed_unique_path_of_theirs: renamed_location.clone(),
                                    },
                                    (ours, theirs, Swapped, outer_side),
                                    [
                                        None,
                                        None,
                                        index_entry_at_path(
                                            blocking_mode,
                                            blocking_id,
                                            ConflictIndexEntryPathHint::RenamedOrTheirs,
                                        ),
                                    ],
                                );

                                match tree_conflicts {
                                    None => {
                                        editor.remove(toc(source_location))?;
                                        editor.remove(toc(blocking_location))?;
                                        our_tree.remove_change(blocking_location.as_bstr());
                                        editor.upsert(toc(&renamed_location), blocking_mode.kind(), *blocking_id)?;
                                        apply_change(&mut editor, theirs, None)?;
                                        ours_disposition = ChangeDisposition::Applied;
                                        theirs_disposition = ChangeDisposition::Applied;
                                    }
                                    Some(ResolveWith::Ours) => match outer_side {
                                        Original => {
                                            apply_change(&mut editor, ours, None)?;
                                            ours_disposition = ChangeDisposition::Applied;
                                        }
                                        Swapped => {
                                            apply_change(&mut editor, theirs, None)?;
                                            theirs_disposition = ChangeDisposition::Applied;
                                        }
                                    },
                                    Some(ResolveWith::Ancestor) => {}
                                }

                                if should_fail_on_conflict(conflict) {
                                    break 'outer;
                                }
                            }
                            (
                                Change::Addition {
                                    location: blocking_location,
                                    entry_mode: blocking_mode,
                                    id: blocking_id,
                                    ..
                                },
                                Change::Addition { .. },
                            ) if matches!(match_kind, Some(MatchKind::EraseLeaf)) => {
                                // `ours` is the non-tree prefix of `theirs`, whose parent directories
                                // are represented only by already-applied structural changes. Preserve
                                // the directory at its intended path and move the blocking addition.
                                let renamed_location = unique_path_in_tree(
                                    blocking_location.as_bstr(),
                                    &editor,
                                    our_tree,
                                    labels.current.unwrap_or_default(),
                                )?;
                                let conflict = Conflict::without_resolution(
                                    ResolutionFailure::OursDirectoryTheirsNonDirectoryTheirsRenamed {
                                        renamed_unique_path_of_theirs: renamed_location.clone(),
                                    },
                                    (ours, theirs, Swapped, outer_side),
                                    [
                                        None,
                                        None,
                                        index_entry_at_path(
                                            blocking_mode,
                                            blocking_id,
                                            ConflictIndexEntryPathHint::RenamedOrTheirs,
                                        ),
                                    ],
                                );

                                match tree_conflicts {
                                    None => {
                                        editor.remove(toc(blocking_location))?;
                                        our_tree.remove_change(blocking_location.as_bstr());
                                        editor.upsert(toc(&renamed_location), blocking_mode.kind(), *blocking_id)?;
                                        apply_change(&mut editor, theirs, None)?;
                                        ours_disposition = ChangeDisposition::Applied;
                                        theirs_disposition = ChangeDisposition::Applied;
                                    }
                                    Some(ResolveWith::Ours) => match outer_side {
                                        Original => {
                                            apply_change(&mut editor, ours, None)?;
                                            ours_disposition = ChangeDisposition::Applied;
                                        }
                                        Swapped => {
                                            editor.remove(toc(blocking_location))?;
                                            our_tree.remove_change(blocking_location.as_bstr());
                                            apply_change(&mut editor, theirs, None)?;
                                            theirs_disposition = ChangeDisposition::Applied;
                                        }
                                    },
                                    Some(ResolveWith::Ancestor) => {}
                                }

                                if should_fail_on_conflict(conflict) {
                                    break 'outer;
                                }
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
                            ) if !involves_submodule(our_mode, their_mode) && our_id != their_id => {
                                let conflict = if let Some(merged_mode) = merge_modes(*our_mode, *their_mode) {
                                    let side = if our_mode == their_mode || matches!(our_mode.kind(), EntryKind::Blob) {
                                        outer_side
                                    } else {
                                        outer_side.swapped()
                                    };
                                    let (merged_blob_id, resolution) = perform_blob_merge(
                                        labels,
                                        objects,
                                        blob_merge,
                                        &mut diff_state.buf1,
                                        &mut write_blob_to_odb,
                                        (location, *our_id, merged_mode),
                                        (location, *their_id, merged_mode),
                                        (location, their_id.kind().null(), merged_mode),
                                        (0, side),
                                        &options,
                                    )?;
                                    editor.upsert(toc(location), merged_mode.kind(), merged_blob_id)?;
                                    Conflict::with_resolution(
                                        Resolution::OursModifiedTheirsModifiedThenBlobContentMerge {
                                            merged_blob: ContentMerge {
                                                resolution,
                                                merged_blob_id,
                                            },
                                        },
                                        (ours, theirs, Original, outer_side),
                                        [None, index_entry(our_mode, our_id), index_entry(their_mode, their_id)],
                                    )
                                } else {
                                    // Actually this has a preference, as symlinks are always left in place with the other side renamed.
                                    let (
                                        logical_side,
                                        label_of_side_to_be_moved,
                                        (our_mode, our_id, our_path_hint),
                                        (their_mode, their_id, their_path_hint),
                                    ) = if matches!(our_mode.kind(), EntryKind::Link | EntryKind::Tree) {
                                        (
                                            Original,
                                            labels.other.unwrap_or_default(),
                                            (*our_mode, *our_id, ConflictIndexEntryPathHint::Current),
                                            (*their_mode, *their_id, ConflictIndexEntryPathHint::RenamedOrTheirs),
                                        )
                                    } else {
                                        (
                                            Swapped,
                                            labels.current.unwrap_or_default(),
                                            (*their_mode, *their_id, ConflictIndexEntryPathHint::RenamedOrTheirs),
                                            (*our_mode, *our_id, ConflictIndexEntryPathHint::Current),
                                        )
                                    };
                                    let tree_with_rename = pick_mut(logical_side, their_tree, our_tree);
                                    let renamed_location = unique_path_in_tree(
                                        location.as_bstr(),
                                        &editor,
                                        tree_with_rename,
                                        label_of_side_to_be_moved,
                                    )?;
                                    let mut conflict = Conflict::without_resolution(
                                        ResolutionFailure::OursAddedTheirsAddedTypeMismatch {
                                            their_unique_location: renamed_location.clone(),
                                        },
                                        (ours, theirs, logical_side, outer_side),
                                        [
                                            None,
                                            index_entry_at_path(&our_mode, &our_id, our_path_hint),
                                            index_entry_at_path(&their_mode, &their_id, their_path_hint),
                                        ],
                                    );
                                    match tree_conflicts {
                                        None => {
                                            let new_change = Change::Addition {
                                                location: renamed_location,
                                                entry_mode: their_mode,
                                                id: their_id,
                                                relation: None,
                                            };
                                            editor.upsert(toc(location), our_mode.kind(), our_id)?;
                                            tree_with_rename.remove_change(location.as_bstr());
                                            push_deferred(
                                                (new_change, None),
                                                pick_mut(logical_side, their_changes, our_changes),
                                            );
                                        }
                                        Some(resolve) => {
                                            conflict.entries = Default::default();
                                            match resolve {
                                                ResolveWith::Ours => match outer_side {
                                                    Original => {
                                                        editor.upsert(toc(location), our_mode.kind(), our_id)?;
                                                    }
                                                    Swapped => {
                                                        editor.upsert(toc(location), their_mode.kind(), their_id)?;
                                                    }
                                                },
                                                ResolveWith::Ancestor => {
                                                    // Do nothing - this discards both sides.
                                                    // Note that one of these adds might be the result of a rename, which
                                                    // means we effectively loose the original and can't get it back as that information is degenerated.
                                                }
                                            }
                                        }
                                    }
                                    conflict
                                };

                                if should_fail_on_conflict(conflict) {
                                    break 'outer;
                                }
                            }
                            (
                                Change::Modification {
                                    location,
                                    entry_mode,
                                    id,
                                    previous_entry_mode,
                                    previous_id,
                                },
                                Change::Deletion { .. },
                            )
                            | (
                                Change::Deletion { .. },
                                Change::Modification {
                                    location,
                                    entry_mode,
                                    id,
                                    previous_entry_mode,
                                    previous_id,
                                },
                            ) => {
                                let (label_of_side_to_be_moved, side) = if matches!(ours, Change::Modification { .. }) {
                                    (labels.current.unwrap_or_default(), Original)
                                } else {
                                    (labels.other.unwrap_or_default(), Swapped)
                                };
                                let deletion_replaced_by_directory = {
                                    // The deleted leaf is replaced by a dir added at the same location.
                                    // Rename-tracking sort order shouldn't be dependent on here, but maybe
                                    // could one day once rename tracking caught up with Git.
                                    let changes = match side {
                                        Original => &their_changes,
                                        Swapped => &our_changes,
                                    };
                                    changes.iter().any(|change| {
                                        change.inner.entry_mode().is_tree()
                                            && matches!(change.inner, Change::Addition { .. })
                                            && change.inner.location() == location
                                    })
                                };

                                let should_break = if deletion_replaced_by_directory {
                                    let entries = [
                                        index_entry(previous_entry_mode, previous_id),
                                        index_entry(entry_mode, id),
                                        None,
                                    ];
                                    match tree_conflicts {
                                        None => {
                                            let our_tree = pick_mut(side, our_tree, their_tree);
                                            let renamed_path = unique_path_in_tree(
                                                location.as_bstr(),
                                                &editor,
                                                our_tree,
                                                label_of_side_to_be_moved,
                                            )?;
                                            editor.remove(toc(location))?;
                                            our_tree.remove_existing_change(location.as_bstr());

                                            let new_change = Change::Addition {
                                                location: renamed_path.clone(),
                                                relation: None,
                                                entry_mode: *entry_mode,
                                                id: *id,
                                            };
                                            let should_break = should_fail_on_conflict(Conflict::without_resolution(
                                                ResolutionFailure::OursModifiedTheirsDirectoryThenOursRenamed {
                                                    renamed_unique_path_to_modified_blob: renamed_path,
                                                },
                                                (ours, theirs, side, outer_side),
                                                entries,
                                            ));

                                            // Since we move *our* side, our tree needs to be modified.
                                            push_deferred(
                                                (new_change, None),
                                                pick_mut(side, our_changes, their_changes),
                                            );
                                            should_break
                                        }
                                        Some(ResolveWith::Ours) => {
                                            match side.to_global(outer_side) {
                                                Original => {
                                                    // ours is modification
                                                    editor.upsert(toc(location), entry_mode.kind(), *id)?;
                                                }
                                                Swapped => {
                                                    // ours is deletion
                                                    editor.remove(toc(location))?;
                                                }
                                            }
                                            should_fail_on_conflict(Conflict::without_resolution(
                                                ResolutionFailure::OursModifiedTheirsDeleted,
                                                (ours, theirs, side, outer_side),
                                                entries,
                                            ))
                                        }
                                        Some(ResolveWith::Ancestor) => {
                                            should_fail_on_conflict(Conflict::without_resolution(
                                                ResolutionFailure::OursModifiedTheirsDeleted,
                                                (ours, theirs, side, outer_side),
                                                entries,
                                            ))
                                        }
                                    }
                                } else {
                                    let entries = [
                                        index_entry(previous_entry_mode, previous_id),
                                        index_entry(entry_mode, id),
                                        None,
                                    ];
                                    match tree_conflicts {
                                        None => {
                                            editor.upsert(toc(location), entry_mode.kind(), *id)?;
                                        }
                                        Some(ResolveWith::Ours) => {
                                            let ours = match outer_side {
                                                Original => ours,
                                                Swapped => theirs,
                                            };

                                            match ours {
                                                Change::Modification { .. } => {
                                                    editor.upsert(toc(location), entry_mode.kind(), *id)?;
                                                }
                                                Change::Deletion { .. } => {
                                                    editor.remove(toc(location))?;
                                                }
                                                _ => unreachable!("parent-match assures this"),
                                            }
                                        }
                                        Some(ResolveWith::Ancestor) => {}
                                    }
                                    should_fail_on_conflict(Conflict::without_resolution(
                                        ResolutionFailure::OursModifiedTheirsDeleted,
                                        (ours, theirs, side, outer_side),
                                        entries,
                                    ))
                                };
                                let deletion_was_applied = match tree_conflicts {
                                    None => deletion_replaced_by_directory,
                                    Some(ResolveWith::Ours) => side.to_global(outer_side).is_swapped(),
                                    Some(ResolveWith::Ancestor) => false,
                                };
                                if deletion_was_applied {
                                    match side {
                                        Original => theirs_disposition = ChangeDisposition::Applied,
                                        Swapped => ours_disposition = ChangeDisposition::Applied,
                                    }
                                }
                                if should_break {
                                    break 'outer;
                                }
                            }
                            (
                                Change::Modification { .. },
                                Change::Addition {
                                    location,
                                    entry_mode,
                                    id,
                                    ..
                                },
                            ) if ours.location() != theirs.location() => {
                                match tree_conflicts {
                                    None => {
                                        // A file-to-directory diff can yield the descendant addition
                                        // before the deletion of the blocking base file. Defer it so
                                        // the modification/deletion pair can relocate the modification
                                        // and remove this structural match first.
                                        push_deferred((theirs.clone(), Some(ours_idx)), their_changes);
                                    }
                                    Some(ResolveWith::Ancestor) => {}
                                    Some(ResolveWith::Ours) => {
                                        if outer_side.is_swapped() {
                                            editor.upsert(toc(location), entry_mode.kind(), *id)?;
                                        }
                                        // we have already taken care of the 'root' of this -
                                        // everything that follows can safely be ignored
                                    }
                                }
                            }
                            (
                                Change::Rewrite {
                                    entry_mode: tree_mode,
                                    location: tree_location,
                                    ..
                                },
                                Change::Rewrite {
                                    source_location,
                                    entry_mode,
                                    id,
                                    location,
                                    ..
                                },
                            ) if tree_mode.is_tree() && tree_location == location => {
                                let renamed_location = unique_path_in_tree(
                                    location.as_bstr(),
                                    &editor,
                                    our_tree,
                                    labels.other.unwrap_or_default(),
                                )?;
                                let conflict = Conflict::without_resolution(
                                    ResolutionFailure::OursDirectoryTheirsNonDirectoryTheirsRenamed {
                                        renamed_unique_path_of_theirs: renamed_location.clone(),
                                    },
                                    (ours, theirs, Original, outer_side),
                                    [
                                        None,
                                        None,
                                        index_entry_at_path(
                                            entry_mode,
                                            id,
                                            ConflictIndexEntryPathHint::RenamedOrTheirs,
                                        ),
                                    ],
                                );

                                match tree_conflicts {
                                    None => {
                                        editor.remove(toc(source_location))?;
                                        editor.upsert(toc(&renamed_location), entry_mode.kind(), *id)?;
                                        their_tree.remove_existing_change(location.as_bstr());
                                        ours_disposition = ChangeDisposition::Applied;
                                        theirs_disposition = ChangeDisposition::Applied;
                                    }
                                    Some(ResolveWith::Ours) => {
                                        apply_our_resolution(ours, theirs, outer_side, &mut editor)?;
                                        match outer_side {
                                            Original => {
                                                their_tree.remove_existing_change(location.as_bstr());
                                                ours_disposition = ChangeDisposition::Applied;
                                            }
                                            Swapped => {
                                                our_tree.remove_existing_change(tree_location.as_bstr());
                                                theirs_disposition = ChangeDisposition::Applied;
                                            }
                                        }
                                    }
                                    Some(ResolveWith::Ancestor) => {}
                                }

                                if should_fail_on_conflict(conflict) {
                                    break 'outer;
                                }
                            }
                            (
                                Change::Rewrite {
                                    source_location,
                                    entry_mode: tree_mode,
                                    ..
                                },
                                Change::Rewrite { location, .. },
                            ) if tree_mode.is_tree()
                                && location == source_location
                                && matches!(match_kind, Some(MatchKind::EraseTree)) =>
                            {
                                // The leaf rename occupies a path vacated by the directory rename.
                                // Descendant changes resolve the actual rename/delete conflict.
                                match tree_conflicts {
                                    None => {
                                        apply_change(&mut editor, theirs, None)?;
                                        theirs_disposition = ChangeDisposition::Applied;
                                    }
                                    Some(ResolveWith::Ours) => {
                                        apply_our_resolution(ours, theirs, outer_side, &mut editor)?;
                                        match outer_side {
                                            Original => ours_disposition = ChangeDisposition::Applied,
                                            Swapped => theirs_disposition = ChangeDisposition::Applied,
                                        }
                                    }
                                    Some(ResolveWith::Ancestor) => {}
                                }
                            }
                            (
                                Change::Rewrite {
                                    source_location: our_source_location,
                                    entry_mode: our_mode,
                                    id: our_id,
                                    location,
                                    ..
                                },
                                Change::Rewrite {
                                    source_location: their_source_location,
                                    entry_mode: their_mode,
                                    id: their_id,
                                    location: their_location,
                                    ..
                                },
                            ) if our_source_location != their_source_location
                                && location == their_location
                                && our_mode == their_mode
                                && our_id == their_id =>
                            {
                                editor.remove(toc(our_source_location))?;
                                editor.remove(toc(their_source_location))?;
                                our_tree.remove_change(our_source_location.as_bstr());
                                their_tree.remove_change(their_source_location.as_bstr());
                                editor.upsert(toc(location), our_mode.kind(), *our_id)?;
                                ours_disposition = ChangeDisposition::Applied;
                                theirs_disposition = ChangeDisposition::Applied;
                            }
                            (
                                Change::Rewrite {
                                    source_location: our_source_location,
                                    entry_mode: our_mode,
                                    id: our_id,
                                    location,
                                    ..
                                },
                                Change::Rewrite {
                                    source_location: their_source_location,
                                    entry_mode: their_mode,
                                    id: their_id,
                                    location: their_location,
                                    ..
                                },
                            ) if our_source_location != their_source_location
                                && location == their_location
                                && !involves_submodule(our_mode, their_mode) =>
                            {
                                match tree_conflicts {
                                    None => {
                                        editor.remove(toc(our_source_location))?;
                                        editor.remove(toc(their_source_location))?;
                                        our_tree.remove_change(our_source_location.as_bstr());
                                        their_tree.remove_change(their_source_location.as_bstr());
                                        let conflict = if let Some(merged_mode) = merge_modes(*our_mode, *their_mode) {
                                            let (merged_blob_id, resolution) = perform_blob_merge(
                                                labels,
                                                objects,
                                                blob_merge,
                                                &mut diff_state.buf1,
                                                &mut write_blob_to_odb,
                                                (location, *our_id, *our_mode),
                                                (location, *their_id, *their_mode),
                                                (location, our_id.kind().null(), merged_mode),
                                                (0, outer_side),
                                                &options,
                                            )?;
                                            editor.upsert(toc(location), merged_mode.kind(), merged_blob_id)?;
                                            Conflict::with_resolution(
                                                Resolution::OursModifiedTheirsModifiedThenBlobContentMerge {
                                                    merged_blob: ContentMerge {
                                                        resolution,
                                                        merged_blob_id,
                                                    },
                                                },
                                                (ours, theirs, Original, outer_side),
                                                [
                                                    None,
                                                    index_entry(our_mode, our_id),
                                                    index_entry(their_mode, their_id),
                                                ],
                                            )
                                        } else {
                                            // Like add/add type conflicts, retain the symlink at the contested path and
                                            // move the regular file to a side-qualified path.
                                            let (
                                                logical_side,
                                                label_of_side_to_be_moved,
                                                (our_mode, our_id, our_path_hint),
                                                (their_mode, their_id, their_path_hint),
                                                moved_tree,
                                            ) = if matches!(our_mode.kind(), EntryKind::Link | EntryKind::Tree) {
                                                (
                                                    Original,
                                                    labels.other.unwrap_or_default(),
                                                    (*our_mode, *our_id, ConflictIndexEntryPathHint::Current),
                                                    (
                                                        *their_mode,
                                                        *their_id,
                                                        ConflictIndexEntryPathHint::RenamedOrTheirs,
                                                    ),
                                                    &mut *their_tree,
                                                )
                                            } else {
                                                (
                                                    Swapped,
                                                    labels.current.unwrap_or_default(),
                                                    (*their_mode, *their_id, ConflictIndexEntryPathHint::Current),
                                                    (*our_mode, *our_id, ConflictIndexEntryPathHint::RenamedOrTheirs),
                                                    &mut *our_tree,
                                                )
                                            };
                                            let renamed_location = unique_path_in_tree(
                                                location.as_bstr(),
                                                &editor,
                                                moved_tree,
                                                label_of_side_to_be_moved,
                                            )?;
                                            editor.upsert(toc(location), our_mode.kind(), our_id)?;
                                            editor.upsert(toc(&renamed_location), their_mode.kind(), their_id)?;
                                            Conflict::without_resolution(
                                                ResolutionFailure::OursAddedTheirsAddedTypeMismatch {
                                                    their_unique_location: renamed_location,
                                                },
                                                (ours, theirs, logical_side, outer_side),
                                                [
                                                    None,
                                                    index_entry_at_path(&our_mode, &our_id, our_path_hint),
                                                    index_entry_at_path(&their_mode, &their_id, their_path_hint),
                                                ],
                                            )
                                        };
                                        if should_fail_on_conflict(conflict) {
                                            break 'outer;
                                        }
                                    }
                                    Some(resolve) => {
                                        if matches!(resolve, ResolveWith::Ours) {
                                            let (source, mode, id, tree) = match outer_side {
                                                Original => (our_source_location, our_mode, our_id, &mut *our_tree),
                                                Swapped => {
                                                    (their_source_location, their_mode, their_id, &mut *their_tree)
                                                }
                                            };
                                            editor.remove(toc(source))?;
                                            tree.remove_change(source.as_bstr());
                                            editor.upsert(toc(location), mode.kind(), *id)?;
                                        }
                                        if should_fail_on_conflict(Conflict::unknown((
                                            ours, theirs, Original, outer_side,
                                        ))) {
                                            break 'outer;
                                        }
                                    }
                                }
                            }
                            (
                                Change::Rewrite {
                                    source_location,
                                    entry_mode: our_mode,
                                    id: our_id,
                                    location,
                                    ..
                                },
                                Change::Addition {
                                    id: their_id,
                                    entry_mode: their_mode,
                                    location: add_location,
                                    ..
                                },
                            )
                            | (
                                Change::Addition {
                                    id: their_id,
                                    entry_mode: their_mode,
                                    location: add_location,
                                    ..
                                },
                                Change::Rewrite {
                                    source_location,
                                    entry_mode: our_mode,
                                    id: our_id,
                                    location,
                                    ..
                                },
                            ) if add_location
                                .strip_prefix(source_location.as_bytes())
                                .is_some_and(|suffix| suffix.starts_with(b"/")) =>
                            {
                                // The rewrite moves the file out of the way while the other side replaces it
                                // with a directory. The child is unrelated to the rewritten blob, so keep both
                                // instead of merging their contents at the rewrite destination. The preceding
                                // deletion/rewrite pairing already recorded the rename/delete conflict.
                                let side = if matches!(ours, Change::Rewrite { .. }) {
                                    Original
                                } else {
                                    Swapped
                                };
                                match tree_conflicts {
                                    None => {
                                        editor.remove(toc(source_location))?;
                                        pick_mut(side, our_tree, their_tree).remove_change(source_location.as_bstr());
                                        editor.upsert(toc(location), our_mode.kind(), *our_id)?;
                                        editor.upsert(toc(add_location), their_mode.kind(), *their_id)?;
                                        ours_disposition = ChangeDisposition::Applied;
                                        theirs_disposition = ChangeDisposition::Applied;
                                    }
                                    Some(ResolveWith::Ours) => match side.to_global(outer_side) {
                                        Original => {
                                            editor.remove(toc(source_location))?;
                                            editor.upsert(toc(location), our_mode.kind(), *our_id)?;
                                            match side {
                                                Original => ours_disposition = ChangeDisposition::Applied,
                                                Swapped => theirs_disposition = ChangeDisposition::Applied,
                                            }
                                        }
                                        Swapped => {
                                            editor.remove(toc(source_location))?;
                                            editor.upsert(toc(add_location), their_mode.kind(), *their_id)?;
                                            match side {
                                                Original => theirs_disposition = ChangeDisposition::Applied,
                                                Swapped => ours_disposition = ChangeDisposition::Applied,
                                            }
                                        }
                                    },
                                    Some(ResolveWith::Ancestor) => {}
                                }
                            }
                            (
                                Change::Rewrite {
                                    source_location: blocking_source,
                                    entry_mode: blocking_mode,
                                    id: blocking_id,
                                    location: blocking_location,
                                    ..
                                },
                                Change::Rewrite {
                                    source_location: nested_source,
                                    ..
                                },
                            ) if blocking_source != nested_source
                                && matches!(match_kind, Some(MatchKind::EraseLeaf)) =>
                            {
                                // These are unrelated renames whose destinations form a file/directory
                                // conflict. Keep the directory at its intended path and move the blocking file.
                                let renamed_location = unique_path_in_tree(
                                    blocking_location.as_bstr(),
                                    &editor,
                                    our_tree,
                                    labels.current.unwrap_or_default(),
                                )?;
                                let conflict = Conflict::without_resolution(
                                    ResolutionFailure::OursDirectoryTheirsNonDirectoryTheirsRenamed {
                                        renamed_unique_path_of_theirs: renamed_location.clone(),
                                    },
                                    (ours, theirs, Swapped, outer_side),
                                    [
                                        None,
                                        None,
                                        index_entry_at_path(
                                            blocking_mode,
                                            blocking_id,
                                            ConflictIndexEntryPathHint::RenamedOrTheirs,
                                        ),
                                    ],
                                );

                                match tree_conflicts {
                                    None => {
                                        editor.remove(toc(blocking_source))?;
                                        editor.remove(toc(blocking_location))?;
                                        our_tree.remove_change(blocking_location.as_bstr());
                                        editor.upsert(toc(&renamed_location), blocking_mode.kind(), *blocking_id)?;
                                        apply_change(&mut editor, theirs, None)?;
                                        ours_disposition = ChangeDisposition::Applied;
                                        theirs_disposition = ChangeDisposition::Applied;
                                    }
                                    Some(ResolveWith::Ours) => match outer_side {
                                        Original => {
                                            apply_change(&mut editor, ours, None)?;
                                            ours_disposition = ChangeDisposition::Applied;
                                        }
                                        Swapped => {
                                            apply_change(&mut editor, theirs, None)?;
                                            theirs_disposition = ChangeDisposition::Applied;
                                        }
                                    },
                                    Some(ResolveWith::Ancestor) => {}
                                }

                                if should_fail_on_conflict(conflict) {
                                    break 'outer;
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
                                // NOTE: renames are only tracked among these kinds of types anyway, but we make sure.
                            ) if our_mode.is_blob_or_symlink()
                                && their_mode.is_blob_or_symlink()
                                && merge_modes(*our_mode, *their_mode).is_some() =>
                            {
                                let (merged_blob_id, mut resolution) = if our_id == their_id {
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
                                        (u8::from(our_location != their_location), outer_side),
                                        &options,
                                    )?;
                                    (id, Some(resolution))
                                };

                                let merged_mode =
                                    merge_modes(*our_mode, *their_mode).expect("this case was assured earlier");

                                if matches!(tree_conflicts, None | Some(ResolveWith::Ours)) {
                                    editor.remove(toc(source_location))?;
                                    our_tree.remove_change(source_location.as_bstr());
                                    their_tree.remove_change(source_location.as_bstr());
                                }

                                let their_location =
                                    possibly_rewritten_location(our_tree, their_location.as_bstr(), our_changes)
                                        .map_or(Cow::Borrowed(their_location.as_bstr()), Cow::Owned);
                                let our_location =
                                    possibly_rewritten_location(their_tree, our_location.as_bstr(), their_changes)
                                        .map_or(Cow::Borrowed(our_location.as_bstr()), Cow::Owned);
                                let (our_addition, their_addition) = if our_location == their_location {
                                    (
                                        None,
                                        Some(Change::Addition {
                                            location: our_location.into_owned(),
                                            relation: None,
                                            entry_mode: merged_mode,
                                            id: merged_blob_id,
                                        }),
                                    )
                                } else {
                                    if should_fail_on_conflict(Conflict::without_resolution(
                                        ResolutionFailure::OursRenamedTheirsRenamedDifferently {
                                            merged_blob: resolution.take().map(|resolution| ContentMerge {
                                                resolution,
                                                merged_blob_id,
                                            }),
                                        },
                                        (ours, theirs, Original, outer_side),
                                        [
                                            index_entry_at_path(
                                                source_entry_mode,
                                                source_id,
                                                ConflictIndexEntryPathHint::Source,
                                            ),
                                            index_entry_at_path(
                                                our_mode,
                                                &merged_blob_id,
                                                ConflictIndexEntryPathHint::Current,
                                            ),
                                            index_entry_at_path(
                                                their_mode,
                                                &merged_blob_id,
                                                ConflictIndexEntryPathHint::RenamedOrTheirs,
                                            ),
                                        ],
                                    )) {
                                        break 'outer;
                                    }
                                    match tree_conflicts {
                                        None => {
                                            let our_addition = Change::Addition {
                                                location: our_location.into_owned(),
                                                relation: None,
                                                entry_mode: merged_mode,
                                                id: merged_blob_id,
                                            };
                                            let their_addition = Change::Addition {
                                                location: their_location.into_owned(),
                                                relation: None,
                                                entry_mode: merged_mode,
                                                id: merged_blob_id,
                                            };
                                            (Some(our_addition), Some(their_addition))
                                        }
                                        Some(ResolveWith::Ancestor) => (None, None),
                                        Some(ResolveWith::Ours) => {
                                            let our_addition = Change::Addition {
                                                location: match outer_side {
                                                    Original => our_location,
                                                    Swapped => their_location,
                                                }
                                                .into_owned(),
                                                relation: None,
                                                entry_mode: merged_mode,
                                                id: merged_blob_id,
                                            };
                                            (Some(our_addition), None)
                                        }
                                    }
                                };

                                if let Some(resolution) = resolution {
                                    if should_fail_on_conflict(Conflict::with_resolution(
                                        Resolution::OursModifiedTheirsModifiedThenBlobContentMerge {
                                            merged_blob: ContentMerge {
                                                resolution,
                                                merged_blob_id,
                                            },
                                        },
                                        (ours, theirs, Original, outer_side),
                                        [
                                            index_entry_at_path(
                                                source_entry_mode,
                                                source_id,
                                                ConflictIndexEntryPathHint::Source,
                                            ),
                                            index_entry_at_path(
                                                our_mode,
                                                &merged_blob_id,
                                                ConflictIndexEntryPathHint::Current,
                                            ),
                                            index_entry_at_path(
                                                their_mode,
                                                &merged_blob_id,
                                                ConflictIndexEntryPathHint::RenamedOrTheirs,
                                            ),
                                        ],
                                    )) {
                                        break 'outer;
                                    }
                                }
                                if let Some(addition) = our_addition {
                                    push_deferred((addition, Some(theirs_idx)), our_changes);
                                }
                                if let Some(addition) = their_addition {
                                    push_deferred((addition, Some(ours_idx)), their_changes);
                                }
                            }
                            (
                                Change::Deletion { .. },
                                Change::Rewrite {
                                    source_location,
                                    entry_mode: rewritten_mode,
                                    id: rewritten_id,
                                    location,
                                    ..
                                },
                            )
                            | (
                                Change::Rewrite {
                                    source_location,
                                    entry_mode: rewritten_mode,
                                    id: rewritten_id,
                                    location,
                                    ..
                                },
                                Change::Deletion { .. },
                            ) if !rewritten_mode.is_commit() => {
                                let side = if matches!(ours, Change::Deletion { .. }) {
                                    Original
                                } else {
                                    Swapped
                                };

                                match tree_conflicts {
                                    None | Some(ResolveWith::Ours) => {
                                        editor.remove(toc(source_location))?;
                                        pick_mut(side, our_tree, their_tree).remove_change(source_location.as_bstr());
                                        match side {
                                            Original => ours_disposition = ChangeDisposition::Applied,
                                            Swapped => theirs_disposition = ChangeDisposition::Applied,
                                        }
                                    }
                                    Some(ResolveWith::Ancestor) => {}
                                }

                                let their_rewritten_location = possibly_rewritten_location(
                                    pick_mut(side, our_tree, their_tree),
                                    location.as_ref(),
                                    pick(side, our_changes, their_changes),
                                )
                                .unwrap_or_else(|| location.to_owned());
                                let our_addition = Change::Addition {
                                    location: their_rewritten_location,
                                    relation: None,
                                    entry_mode: *rewritten_mode,
                                    id: *rewritten_id,
                                };

                                if should_fail_on_conflict(Conflict::without_resolution(
                                    ResolutionFailure::OursDeletedTheirsRenamed,
                                    (ours, theirs, side, outer_side),
                                    [
                                        None,
                                        None,
                                        index_entry_at_path(
                                            rewritten_mode,
                                            rewritten_id,
                                            ConflictIndexEntryPathHint::RenamedOrTheirs,
                                        ),
                                    ],
                                )) {
                                    break 'outer;
                                }

                                let ours_is_rewrite = side.is_swapped();
                                if tree_conflicts.is_none()
                                    || (matches!(tree_conflicts, Some(ResolveWith::Ours)) && ours_is_rewrite)
                                {
                                    push_deferred((our_addition, None), pick_mut(side, their_changes, our_changes));
                                }
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
                                    location: add_location,
                                    ..
                                },
                            )
                            | (
                                Change::Addition {
                                    id: their_id,
                                    entry_mode: their_mode,
                                    location: add_location,
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
                            ) if !involves_submodule(our_mode, their_mode) => {
                                let side = if matches!(ours, Change::Rewrite { .. }) {
                                    Original
                                } else {
                                    Swapped
                                };
                                if our_mode.is_tree() && add_location == source_location {
                                    // The leaf changes already represent the directory rename and its replacement.
                                    // Keep the replacement at the explicit source instead of relocating it with
                                    // the inferred directory rename and reporting a second conflict.
                                    match tree_conflicts {
                                        None => {
                                            editor.upsert(toc(add_location), their_mode.kind(), *their_id)?;
                                            ours_disposition = ChangeDisposition::Applied;
                                            theirs_disposition = ChangeDisposition::Applied;
                                        }
                                        Some(ResolveWith::Ours) => {
                                            apply_our_resolution(ours, theirs, outer_side, &mut editor)?;
                                            match outer_side {
                                                Original => ours_disposition = ChangeDisposition::Applied,
                                                Swapped => theirs_disposition = ChangeDisposition::Applied,
                                            }
                                        }
                                        Some(ResolveWith::Ancestor) => {}
                                    }
                                } else if let Some(merged_mode) = merge_modes(*our_mode, *their_mode) {
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
                                            (0, outer_side),
                                            &options,
                                        )?;
                                        (id, Some(resolution))
                                    };

                                    editor.remove(toc(source_location))?;
                                    pick_mut(side, our_tree, their_tree).remove_change(source_location.as_bstr());

                                    if let Some(resolution) = resolution {
                                        if should_fail_on_conflict(Conflict::with_resolution(
                                            Resolution::OursModifiedTheirsModifiedThenBlobContentMerge {
                                                merged_blob: ContentMerge {
                                                    resolution,
                                                    merged_blob_id,
                                                },
                                            },
                                            (ours, theirs, Original, outer_side),
                                            [None, index_entry(our_mode, our_id), index_entry(their_mode, their_id)],
                                        )) {
                                            break 'outer;
                                        }
                                    }

                                    // Because this constellation can only be found by the lookup tree, there is
                                    // no need to put it as addition, we know it's not going to intersect on the other side.
                                    editor.upsert(toc(location), merged_mode.kind(), merged_blob_id)?;
                                } else {
                                    // We always remove the source from the tree - it might be re-added later.
                                    let ours_is_rename =
                                        tree_conflicts == Some(ResolveWith::Ours) && side == outer_side;
                                    let remove_rename_source =
                                        tree_conflicts.is_none() || ours_is_rename || add_location != source_location;
                                    if remove_rename_source {
                                        editor.remove(toc(source_location))?;
                                        pick_mut(side, our_tree, their_tree).remove_change(source_location.as_bstr());
                                    }

                                    let (
                                        logical_side,
                                        label_of_side_to_be_moved,
                                        (our_mode, our_id, our_path_hint),
                                        (their_mode, their_id, their_path_hint),
                                    ) = if matches!(our_mode.kind(), EntryKind::Link | EntryKind::Tree) {
                                        (
                                            Original,
                                            labels.other.unwrap_or_default(),
                                            (*our_mode, *our_id, ConflictIndexEntryPathHint::Current),
                                            (*their_mode, *their_id, ConflictIndexEntryPathHint::RenamedOrTheirs),
                                        )
                                    } else {
                                        (
                                            Swapped,
                                            labels.current.unwrap_or_default(),
                                            (*their_mode, *their_id, ConflictIndexEntryPathHint::RenamedOrTheirs),
                                            (*our_mode, *our_id, ConflictIndexEntryPathHint::Current),
                                        )
                                    };
                                    let tree_with_rename = pick_mut(side, our_tree, their_tree);
                                    let renamed_location = unique_path_in_tree(
                                        location.as_bstr(),
                                        &editor,
                                        tree_with_rename,
                                        label_of_side_to_be_moved,
                                    )?;

                                    let upsert_rename_destination = tree_conflicts.is_none() || ours_is_rename;
                                    if upsert_rename_destination {
                                        editor.upsert(toc(location), our_mode.kind(), our_id)?;
                                        tree_with_rename.remove_existing_change(location.as_bstr());
                                    }

                                    let conflict = Conflict::without_resolution(
                                        ResolutionFailure::OursAddedTheirsAddedTypeMismatch {
                                            their_unique_location: renamed_location.clone(),
                                        },
                                        (ours, theirs, side, outer_side),
                                        [
                                            None,
                                            index_entry_at_path(&our_mode, &our_id, our_path_hint),
                                            index_entry_at_path(&their_mode, &their_id, their_path_hint),
                                        ],
                                    );

                                    if tree_conflicts.is_none() {
                                        let new_change_with_rename = Change::Addition {
                                            location: renamed_location,
                                            entry_mode: their_mode,
                                            id: their_id,
                                            relation: None,
                                        };
                                        push_deferred(
                                            (
                                                new_change_with_rename,
                                                Some(pick_idx(logical_side, theirs_idx, ours_idx)),
                                            ),
                                            pick_mut(logical_side, their_changes, our_changes),
                                        );
                                    }

                                    if should_fail_on_conflict(conflict) {
                                        break 'outer;
                                    }
                                }
                            }
                            _unknown => {
                                if let Some(ResolveWith::Ours) = tree_conflicts {
                                    apply_our_resolution(ours, theirs, outer_side, &mut editor)?;
                                }
                                if should_fail_on_conflict(Conflict::unknown((ours, theirs, Original, outer_side))) {
                                    break 'outer;
                                }
                            }
                        }
                        their_changes[theirs_idx].mark(theirs_disposition);
                        our_changes[ours_idx].mark(ours_disposition);
                    }
                }
            }
            segment_start = last_seen_len;
            last_seen_len = their_changes.len();
        }

        ((our_changes, our_tree), (their_changes, their_tree)) = ((their_changes, their_tree), (our_changes, our_tree));
        (labels.current, labels.other) = (labels.other, labels.current);
        outer_side = outer_side.swapped();
    }

    Ok(Outcome {
        tree: editor,
        conflicts,
        failed_on_first_unresolved_conflict: failed_on_first_conflict,
    })
}

fn apply_our_resolution(
    local_ours: &Change,
    local_theirs: &Change,
    outer_side: ConflictMapping,
    editor: &mut gix_object::tree::Editor<'_>,
) -> Result<(), Error> {
    let ours = match outer_side {
        Original => local_ours,
        Swapped => local_theirs,
    };
    Ok(apply_change(editor, ours, None)?)
}

fn involves_submodule(a: &EntryMode, b: &EntryMode) -> bool {
    a.is_commit() || b.is_commit()
}

/// Allows equal modes or prefers executables bits in case of blobs
///
/// Note that this is often not correct as the previous mode of each side should be taken into account so that:
///
/// on | on = on
/// off | off = off
/// on | off || off | on = conflict
fn merge_modes(a: EntryMode, b: EntryMode) -> Option<EntryMode> {
    match (a.kind(), b.kind()) {
        (_, _) if a == b => Some(a),
        (EntryKind::BlobExecutable, EntryKind::BlobExecutable | EntryKind::Blob)
        | (EntryKind::Blob, EntryKind::BlobExecutable) => Some(EntryKind::BlobExecutable.into()),
        _ => None,
    }
}

/// Use this version if there is a single common `prev` value for both `a` and `b` to detect
/// if the mode was turned on or off.
fn merge_modes_prev(a: EntryMode, b: EntryMode, prev: EntryMode) -> Option<EntryMode> {
    match (a.kind(), b.kind()) {
        (_, _) if a == b => Some(a),
        (a @ EntryKind::BlobExecutable, b @ (EntryKind::BlobExecutable | EntryKind::Blob))
        | (a @ EntryKind::Blob, b @ EntryKind::BlobExecutable) => {
            let prev = prev.kind();
            let changed = if a == prev { b } else { a };
            Some(
                match (prev, changed) {
                    (EntryKind::Blob, EntryKind::BlobExecutable) => EntryKind::BlobExecutable,
                    (EntryKind::BlobExecutable, EntryKind::Blob) => EntryKind::Blob,
                    _ => unreachable!("upper match already assured we only deal with blobs"),
                }
                .into(),
            )
        }
        _ => None,
    }
}

fn push_deferred(change_and_idx: (Change, Option<usize>), changes: &mut ChangeList) {
    push_deferred_with_rewrite(change_and_idx, None, changes);
}

fn push_deferred_with_rewrite(
    (change, ours_idx): (Change, Option<usize>),
    new_location: Option<(BString, usize)>,
    changes: &mut ChangeList,
) {
    changes.push(TrackedChange::new(change, Some(ours_idx), new_location));
}

fn pick<'a, T: ?Sized>(side: ConflictMapping, ours: &'a T, theirs: &'a T) -> &'a T {
    match side {
        Original => ours,
        Swapped => theirs,
    }
}

fn pick_idx(side: ConflictMapping, ours: usize, theirs: usize) -> usize {
    match side {
        Original => ours,
        Swapped => theirs,
    }
}

fn pick_mut<'a, T: ?Sized>(side: ConflictMapping, ours: &'a mut T, theirs: &'a mut T) -> &'a mut T {
    match side {
        Original => ours,
        Swapped => theirs,
    }
}

fn index_entry(mode: &gix_object::tree::EntryMode, id: &gix_hash::ObjectId) -> Option<ConflictIndexEntry> {
    Some(ConflictIndexEntry {
        mode: *mode,
        id: *id,
        path_hint: None,
    })
}

fn index_entry_at_path(
    mode: &gix_object::tree::EntryMode,
    id: &gix_hash::ObjectId,
    hint: ConflictIndexEntryPathHint,
) -> Option<ConflictIndexEntry> {
    Some(ConflictIndexEntry {
        mode: *mode,
        id: *id,
        path_hint: Some(hint),
    })
}
