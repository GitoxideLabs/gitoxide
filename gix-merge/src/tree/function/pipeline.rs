//! Three-input tree merging: normalize identities, resolve paths, then construct the result.

use std::collections::{BTreeMap, BTreeSet};

use bstr::{BStr, BString, ByteSlice};
use gix_diff::tree_with_rewrites::Change;
use gix_error::{Result, bail, message};
use gix_object::{FindExt, tree};

use super::{
    change,
    entry::{self, ResolutionPolicy, merge_modes},
    normalize::{self, Event},
    path::{Entry, EntryMerge, Resolution as PathResolution, Stage, State},
};
use crate::tree::{Conflict, ConflictMapping, IndexPlan, Options, Outcome, Resolution, ResolutionFailure, ResolveWith};

/// Merge three trees while retaining their original entries and paths throughout resolution.
///
/// Rename normalization establishes identities and destinations before content or structural
/// conflicts are finalized. Paths are then resolved from children to parents so a directory
/// blocks a file only when its resolved descendants actually survive. A shared namespace
/// reserves conflict names against every input and every previously chosen destination.
///
/// `base_tree` may be the empty tree when the sides have no common ancestor. `labels` name
/// the three resources in merge drivers and conflict markers. `objects` supplies trees and
/// blobs and backs the returned editor; an object cache accelerates repeated tree retrieval.
/// `write_blob_to_odb` stores newly merged contents; existing blobs are reused directly.
/// `diff_state` and `diff_resource_cache` provide reusable scratch space and rename-similarity
/// resources. `blob_merge` must not fall back to worktree data: all inputs come from the
/// supplied trees and object database. The returned editor remains unwritten for the caller.
/// `context` supplies optional submodule history queries without requiring repository discovery here.
///
/// Like Git's `merge-ort`, this resolves normalized three-input paths from children to parents.
/// Rename identities come from `gix-diff`, with destination ordering derived from these
/// three inputs. Ambiguous identities can still differ from Git. Copies are treated as additions.
/// Explicit renames returned to their source by directory relocation still participate in opposite
/// rename conflicts. Both destinations are retained; Git can miss this conflict depending on side order.
///
/// Selecting an absent binary resource returns [`crate::tree::Error::MissingBinaryMergeResult`],
/// classified as [`gix_error::Class::NotFound`]. Virtual-base construction instead represents
/// an absent binary ancestor with an empty blob, like Git.
#[expect(clippy::too_many_arguments)]
pub fn tree<'objects>(
    base_tree: &gix_hash::oid,
    our_tree: &gix_hash::oid,
    their_tree: &gix_hash::oid,
    labels: crate::blob::builtin_driver::text::Labels<'_>,
    objects: &'objects impl gix_object::FindObjectOrHeader,
    mut write_blob_to_odb: impl FnMut(&[u8]) -> Result<gix_hash::ObjectId>,
    diff_state: &mut gix_diff::tree::State,
    diff_resource_cache: &mut gix_diff::blob::Platform,
    blob_merge: &mut crate::blob::Platform,
    mut context: crate::tree::Context<'_>,
    options: Options,
) -> Result<Outcome<'objects>> {
    let _span = gix_trace::coarse!("gix_merge::tree", ?base_tree, ?our_tree, ?their_tree, ?labels);
    let selected = if our_tree == their_tree || base_tree == their_tree {
        Some(our_tree)
    } else if base_tree == our_tree {
        Some(their_tree)
    } else {
        None
    };
    if let Some(tree_id) = selected {
        let mut buf = Vec::new();
        let tree = if tree_id == tree_id.kind().empty_tree() {
            gix_object::Tree::default()
        } else {
            objects.find_tree(tree_id, &mut buf)?.to_owned()
        };
        return Ok(Outcome {
            tree: tree::Editor::new(tree, objects, tree_id.kind()),
            index: IndexPlan::new(tree_id),
            conflicts: Vec::new(),
            failed_on_first_unresolved_conflict: false,
        });
    }
    let mut paths = State::collect([base_tree, our_tree, their_tree], objects)?;
    let mut base_buf = Vec::new();
    let ancestor_tree = if base_tree == base_tree.kind().empty_tree() {
        gix_object::Tree::default()
    } else {
        objects.find_tree(base_tree, &mut base_buf)?.to_owned()
    };
    let mut side_buf = Vec::new();
    let destination_orders = if options.rewrites.is_some() {
        change::destination_orders(&paths)
    } else {
        Default::default()
    };
    let mut ours = change::collect(
        base_tree,
        our_tree,
        &base_buf,
        &mut side_buf,
        objects,
        diff_resource_cache,
        diff_state,
        options.rewrites,
        options.directory_renames,
        &destination_orders[0],
    )?;
    let mut theirs = change::collect(
        base_tree,
        their_tree,
        &base_buf,
        &mut side_buf,
        objects,
        diff_resource_cache,
        diff_state,
        options.rewrites,
        options.directory_renames,
        &destination_orders[1],
    )?;
    change::retain_relevant_directory_renames(&mut ours, &theirs.entries);
    change::retain_relevant_directory_renames(&mut theirs, &ours.entries);
    let directories = [ours.directories, theirs.directories];
    let changes = [ours.entries, theirs.entries];
    let policy = ResolutionPolicy::new(&options);
    let mut normalized = normalize::run(
        &mut paths,
        [&changes[0], &changes[1]],
        [&directories[0], &directories[1]],
        labels,
        objects,
        blob_merge,
        &mut diff_state.buf1,
        &mut write_blob_to_odb,
        &mut context,
        &options,
        policy,
    )?;
    if !normalized.events.iter().any(|event| event.should_stop(&options)) {
        resolve_paths(
            &mut paths,
            &mut normalized.events,
            &normalized.renames,
            labels,
            &options,
            objects,
            blob_merge,
            |location, stages, blob_merge| {
                let ours = stages[1].as_ref().expect("content merging requires both sides");
                let theirs = stages[2].as_ref().expect("content merging requires both sides");
                let base_location = stages[0].as_ref().map_or(location, |stage| &stage.source);
                entry::merge(
                    labels,
                    objects,
                    blob_merge,
                    &mut diff_state.buf1,
                    &mut write_blob_to_odb,
                    (&ours.source, ours.entry.id, ours.entry.mode),
                    (&theirs.source, theirs.entry.id, theirs.entry.mode),
                    (
                        base_location,
                        stages[0].as_ref().map(|stage| (stage.entry.id, stage.entry.mode)),
                    ),
                    (0, ConflictMapping::Original, ConflictMapping::Original),
                    &mut context,
                    &options,
                    policy,
                )
            },
        )?;
    }

    let stopped = normalized.events.iter().any(|event| event.should_stop(&options));
    if !stopped {
        normalized.events.retain_mut(|event| {
            let Some(merged_blob) = crate::tree::resolution_content_merge(&event.resolution) else {
                return true;
            };
            // Binary merges can reuse an input ID, so retained conflicts also need their index owner.
            if event.affected_paths.iter().any(|path| {
                paths.entries.get(path).is_some_and(|node| {
                    matches!(node.result, PathResolution::Present(entry) if entry.id == merged_blob.merged_blob_id)
                        && (merged_blob.resolution == crate::blob::Resolution::Complete || node.unmerged.is_some())
                })
            }) {
                return true;
            }
            match &mut event.resolution {
                Ok(Resolution::OursModifiedTheirsModifiedThenBlobContentMerge { .. }) => false,
                Err(
                    ResolutionFailure::ModeConflict { merged_blob }
                    | ResolutionFailure::DirectoryRename { merged_blob, .. }
                    | ResolutionFailure::OursRenamedTheirsRenamedDifferently { merged_blob },
                )
                | Ok(
                    Resolution::Forced(
                        ResolutionFailure::ModeConflict { merged_blob }
                        | ResolutionFailure::DirectoryRename { merged_blob, .. }
                        | ResolutionFailure::OursRenamedTheirsRenamedDifferently { merged_blob },
                    )
                    | Resolution::OursModifiedTheirsRenamedAndChangedThenRename { merged_blob, .. },
                ) => {
                    *merged_blob = None;
                    true
                }
                _ => true,
            }
        });
    }
    if stopped {
        for path in paths.paths_bottom_up() {
            if paths.entries[&path].result == PathResolution::Pending {
                let node = paths.entries.get_mut(&path).expect("the pending path is indexed");
                if node.unmerged.is_none() && node.path_conflict && policy.tree_resolution().is_none() {
                    node.set_unmerged(
                        node.stages
                            .each_ref()
                            .map(|stage| stage.as_ref().map(|stage| stage.entry)),
                        None,
                    );
                }
                let unmerged = node.unmerged.take();
                let entry = paths.entries[&path].inputs[0]
                    .filter(|entry| entry.mode.is_no_tree() && !paths.has_live_descendants(path.as_bstr()));
                set_result(&mut paths, &path, entry);
                paths
                    .entries
                    .get_mut(&path)
                    .expect("the pending path remains indexed")
                    .unmerged = unmerged;
            }
        }
    }
    // The path state owns all decisions; unchanged input entries can remain in the sparse editor.
    // Remove discarded inputs before inserting results, so descendant removals cannot disturb a new parent leaf.
    let mut editor = tree::Editor::new(ancestor_tree, objects, base_tree.kind());
    for (path, node) in &paths.entries {
        if node.result == PathResolution::Absent
            && let Some(base) = node.inputs[0]
        {
            if base.mode.is_no_tree() {
                editor.remove_if_leaf(path.split(|&byte| byte == b'/'))?;
            } else if !paths.has_live_descendants(path.as_bstr()) {
                editor.remove(path.split(|&byte| byte == b'/'))?;
            }
        }
    }
    for (path, node) in &paths.entries {
        if let PathResolution::Present(entry) = node.result {
            if entry.mode.is_no_tree() && paths.has_live_descendants(path.as_bstr()) {
                bail!(message!("Tree merge retained a file and descendants at {path:?}"));
            }
            let canonical = Entry {
                mode: entry.mode.kind().into(),
                ..entry
            };
            if Some(canonical) != node.inputs[0] {
                editor.upsert(path.split(|&byte| byte == b'/'), entry.mode.kind(), entry.id)?;
            }
        }
    }
    let mut index = IndexPlan::new(base_tree);
    for (path, node) in &paths.entries {
        let mut unmerged = node.unmerged.clone();
        if let Some(owner) = &mut unmerged
            && owner.entries[0].is_some()
            && paths.has_live_descendants(path.as_bstr())
        {
            // A source that survives only as a directory cannot retain a file ancestor stage.
            owner.entries[0] = None;
            if owner.entries.iter().all(Option::is_none) {
                unmerged = None;
            }
        }
        let entry = match node.result {
            PathResolution::Present(entry) if entry.mode.is_no_tree() => Some(Entry {
                mode: entry.mode.kind().into(),
                ..entry
            }),
            _ => None,
        };
        if entry != node.inputs[0].filter(|entry| entry.mode.is_no_tree()) || unmerged.is_some() {
            index.record(path.clone(), entry, unmerged);
        }
    }
    let mut conflicts = Vec::with_capacity(normalized.events.len());
    let mut failed_on_first_unresolved_conflict = false;
    for event in normalized.events {
        let mut conflict = into_conflict(event);
        conflict.resolution = entry::final_resolution(conflict.resolution, policy);
        index.record_resolution(&conflict.resolution);
        let stop = options.fail_on_conflict.is_some_and(|how| conflict.is_unresolved(how));
        conflicts.push(conflict);
        if stop {
            failed_on_first_unresolved_conflict = true;
            break;
        }
    }
    Ok(Outcome {
        tree: editor,
        index,
        conflicts,
        failed_on_first_unresolved_conflict,
    })
}

#[expect(clippy::too_many_arguments)]
fn resolve_paths(
    paths: &mut State,
    events: &mut Vec<Event>,
    renames: &BTreeMap<BString, [Option<Change>; 2]>,
    labels: crate::blob::builtin_driver::text::Labels<'_>,
    options: &Options,
    objects: &impl gix_object::FindObjectOrHeader,
    blob_merge: &mut crate::blob::Platform,
    mut merge: impl FnMut(&BString, &[Option<Stage>; 3], &mut crate::blob::Platform) -> Result<Option<entry::Outcome>>,
) -> Result {
    let policy = ResolutionPolicy::new(options);
    let mut checked_events = 0;
    for location in paths.paths_bottom_up() {
        if events[checked_events..].iter().any(|event| event.should_stop(options)) {
            break;
        }
        checked_events = events.len();
        let node = &paths.entries[&location];
        if node.result != PathResolution::Pending {
            let stages = node.stages.clone();
            let retained: Vec<_> = stages
                .iter()
                .flatten()
                .filter_map(|stage| stage.merge.as_deref().copied())
                .filter(|merge| matches!(node.result, PathResolution::Present(entry) if merge.matches_entry(entry)))
                .collect();
            for merge in retained {
                retain_merge(paths, events, &location, &stages, merge);
            }
            continue;
        }
        let path_conflict = node.path_conflict;
        let stages = node.stages.clone();
        let leaves = stages.each_ref().map(|stage| {
            stage
                .as_ref()
                .map(|stage| stage.entry)
                .filter(|entry| entry.mode.is_no_tree())
        });
        let [_, ours, theirs] = leaves;
        set_result(paths, &location, None);

        if let Some(side) = paths.entries[&location].trivial_choice() {
            let result = leaves[side];
            if result.is_none() || !paths.has_live_descendants(location.as_bstr()) {
                set_result(paths, &location, result);
                if let Some(content) = stages[side].as_ref().and_then(|stage| stage.merge.as_deref().copied()) {
                    retain_merge(paths, events, &location, &stages, content);
                }
                continue;
            }
        }

        let mut output_location = location.clone();
        let mut directory_conflict = None;
        if paths.has_live_descendants(location.as_bstr()) && (ours.is_some() || theirs.is_some()) {
            let side = match (ours, theirs) {
                (Some(_), None) => 1,
                (None, Some(_)) => 2,
                _ if paths.entries[&location].directory_mask & 2 != 0 => 2,
                _ => 1,
            };
            output_location = paths.reserve_unique(
                location.as_bstr(),
                if side == 1 {
                    labels.current.unwrap_or_default()
                } else {
                    labels.other.unwrap_or_default()
                },
                objects,
            )?;
            let mut conflict = event(
                &location,
                &stages,
                Err(ResolutionFailure::OursDirectoryTheirsNonDirectoryTheirsRenamed {
                    renamed_unique_path_of_theirs: output_location.clone(),
                }),
                if side == 1 {
                    ConflictMapping::Swapped
                } else {
                    ConflictMapping::Original
                },
            );
            if let Some(choice) = policy.forced_resolution() {
                let mut merged_file = None;
                if choice == ResolveWith::Ours && side == 1 && ours.is_some() && theirs.is_some() {
                    resolve_leaf(
                        paths,
                        &location,
                        &location,
                        &stages,
                        path_conflict,
                        events,
                        renames,
                        labels,
                        policy,
                        objects,
                        blob_merge,
                        &mut merge,
                    )?;
                    merged_file = Some((
                        paths.entries[&location].result,
                        paths.entries[&location].unmerged.clone(),
                    ));
                }
                let content = force_path(paths, &location, choice, renames);
                if let Some((result, unmerged)) = merged_file {
                    let node = paths.entries.get_mut(&location).expect("the selected file is indexed");
                    node.result = result;
                    node.unmerged = unmerged;
                }
                events.push(conflict);
                for (location, content) in content {
                    retain_merge(paths, events, &location, &stages, content);
                }
                continue;
            }
            relocate_events(events, &location, &output_location);
            conflict.relocate(&location, std::slice::from_ref(&output_location));
            directory_conflict = Some((side, conflict));
        }
        let content_clean = resolve_leaf(
            paths,
            &output_location,
            &location,
            &stages,
            path_conflict,
            events,
            renames,
            labels,
            policy,
            objects,
            blob_merge,
            &mut merge,
        )?;
        if let Some((side, conflict)) = directory_conflict {
            // Renormalization can make the file disappear, removing the D/F conflict too.
            if content_clean && paths.entries[&output_location].result == PathResolution::Absent {
                continue;
            }
            let mut entries = leaves;
            if content_clean && let PathResolution::Present(entry) = paths.entries[&output_location].result {
                // A clean content merge leaves only the file-owning side unresolved at a D/F destination.
                entries = [None; 3];
                entries[side] = Some(entry);
            }
            if policy.tree_resolution().is_none()
                && let Some(node) = paths.entries.get_mut(&output_location)
                && matches!(node.result, PathResolution::Present(entry) if entry.mode.is_no_tree())
            {
                node.set_unmerged(entries, None);
            }
            events.push(conflict);
        }
    }
    Ok(())
}

#[expect(clippy::too_many_arguments)]
fn resolve_leaf(
    paths: &mut State,
    location: &BString,
    original_location: &BString,
    stages: &[Option<Stage>; 3],
    path_conflict: bool,
    events: &mut Vec<Event>,
    renames: &BTreeMap<BString, [Option<Change>; 2]>,
    labels: crate::blob::builtin_driver::text::Labels<'_>,
    policy: ResolutionPolicy,
    objects: &impl gix_object::FindObjectOrHeader,
    blob_merge: &mut crate::blob::Platform,
    merge: &mut impl FnMut(&BString, &[Option<Stage>; 3], &mut crate::blob::Platform) -> Result<Option<entry::Outcome>>,
) -> Result<bool> {
    let [base, ours, theirs] = stages.each_ref().map(|stage| {
        stage
            .as_ref()
            .map(|stage| stage.entry)
            .filter(|entry| entry.mode.is_no_tree())
    });
    let mut content_clean = false;
    let mut retained_merges = Vec::new();
    match (ours, theirs) {
        (None, None) => {
            if path_conflict
                && base.is_some()
                && policy.tree_resolution().is_none()
                && !paths.has_live_descendants(location.as_bstr())
            {
                paths
                    .entries
                    .entry(location.clone())
                    .or_default()
                    .set_unmerged([base, None, None], None);
            }
        }
        (Some(_), None) | (None, Some(_)) => {
            let side = if ours.is_some() { 1 } else { 2 };
            if let Some(base) = base.filter(|_| !path_conflict) {
                let side_entry = ours.or(theirs).expect("one modified side remains after deletion");
                if blob_merge.unchanged_after_renormalization(
                    location.as_bstr(),
                    (base.id, base.mode),
                    (side_entry.id, side_entry.mode),
                    if side == 1 {
                        crate::blob::ResourceKind::CurrentOrOurs
                    } else {
                        crate::blob::ResourceKind::OtherOrTheirs
                    },
                    objects,
                )? {
                    set_result(paths, location, None);
                    return Ok(true);
                }
                let conflict = event(
                    location,
                    stages,
                    Err(ResolutionFailure::OursModifiedTheirsDeleted),
                    if side == 1 {
                        ConflictMapping::Original
                    } else {
                        ConflictMapping::Swapped
                    },
                );
                if let Some(choice) = policy.tree_resolution() {
                    retained_merges.extend(force_path(paths, original_location, choice, renames));
                } else {
                    select_stage(paths, location, stages[side].as_ref(), &mut retained_merges);
                    paths
                        .entries
                        .get_mut(location)
                        .expect("the modified path is indexed")
                        .set_unmerged([Some(base), ours, theirs], None);
                }
                events.push(conflict);
            } else {
                select_stage(
                    paths,
                    location,
                    if policy.is_virtual() && path_conflict && base.is_some() {
                        stages[0].as_ref()
                    } else {
                        stages[side].as_ref()
                    },
                    &mut retained_merges,
                );
                if path_conflict && policy.tree_resolution().is_none() {
                    paths
                        .entries
                        .get_mut(location)
                        .expect("the conflicted path is indexed")
                        .set_unmerged([base, ours, theirs], None);
                }
            }
        }
        (Some(ours), Some(theirs)) => {
            if merge_modes(ours.mode, theirs.mode, base.map(|base| base.mode)).is_none() {
                let moved = [
                    ours.mode.is_blob() || !theirs.mode.is_blob(),
                    theirs.mode.is_blob() || !ours.mode.is_blob(),
                ];
                let mut destinations = [location.clone(), location.clone()];
                if policy.is_virtual() && policy.tree_resolution() == Some(ResolveWith::Ancestor) {
                    set_result(paths, location, base);
                } else if let Some(choice) = policy.tree_resolution() {
                    retained_merges.extend(force_path(paths, original_location, choice, renames));
                } else {
                    for (side, label) in [labels.current.unwrap_or_default(), labels.other.unwrap_or_default()]
                        .into_iter()
                        .enumerate()
                    {
                        if moved[side] {
                            destinations[side] = paths.reserve_unique(location.as_bstr(), label, objects)?;
                        }
                        select_stage(
                            paths,
                            &destinations[side],
                            stages[side + 1].as_ref(),
                            &mut retained_merges,
                        );
                        let mut entries = [None; 3];
                        entries[side + 1] = Some([ours, theirs][side]);
                        entries[0] =
                            base.filter(|base| merge_modes(base.mode, [ours, theirs][side].mode, None).is_some());
                        paths
                            .entries
                            .get_mut(&destinations[side])
                            .expect("the split path is indexed")
                            .set_unmerged(entries, None);
                    }
                }
                let conflict = event(
                    location,
                    stages,
                    Err(ResolutionFailure::DistinctTypes {
                        our_location: destinations[0].clone(),
                        their_location: destinations[1].clone(),
                    }),
                    ConflictMapping::Original,
                );
                events.push(conflict);
                for event in events.iter_mut() {
                    event.relocate(location, &destinations);
                }
            } else {
                let merged = merge(location, stages, blob_merge)?.expect("compatible leaf entries can be merged");
                content_clean = !merged.is_conflict();
                if let Some(choice) = merged.force {
                    retained_merges.extend(force_path(paths, original_location, choice, renames));
                } else {
                    set_result(paths, location, Some(merged.entry));
                    if (path_conflict || merged.has_tree_conflict()) && policy.tree_resolution().is_none() {
                        paths
                            .entries
                            .get_mut(location)
                            .expect("the tree conflict path is indexed")
                            .set_unmerged([base, Some(ours), Some(theirs)], None);
                    } else if let Some(resolution) = merged.content_resolution()
                        && resolution != crate::blob::Resolution::Complete
                    {
                        paths
                            .entries
                            .get_mut(location)
                            .expect("the content merge path is indexed")
                            .set_unmerged([base, Some(ours), Some(theirs)], Some(resolution));
                    }
                    retained_merges.extend(
                        stages
                            .iter()
                            .flatten()
                            .filter_map(|stage| stage.merge.as_deref().copied())
                            .filter(|content| content.matches_entry(merged.entry))
                            .map(|content| (location.clone(), content)),
                    );
                }
                if let Some(resolution) = merged.resolution() {
                    events.push(event(location, stages, resolution, ConflictMapping::Original));
                }
            }
        }
    }
    for (location, content) in retained_merges {
        retain_merge(paths, events, &location, stages, content);
    }
    Ok(content_clean)
}

fn select_stage(
    paths: &mut State,
    location: &BString,
    stage: Option<&Stage>,
    content: &mut Vec<(BString, EntryMerge)>,
) {
    set_result(paths, location, stage.map(|stage| stage.entry));
    content.extend(
        stage
            .and_then(|stage| stage.merge.as_deref().copied())
            .map(|content| (location.clone(), content)),
    );
}

/// Preserve source-merge conflicts only when their normalized entry is selected for the result.
fn retain_merge(
    paths: &mut State,
    events: &mut Vec<Event>,
    location: &BString,
    stages: &[Option<Stage>; 3],
    retained: EntryMerge,
) {
    let node = paths.entries.entry(location.clone()).or_default();
    if !retained.forced {
        match &mut node.unmerged {
            Some(unmerged) => {
                if let Some(resolution) = &mut unmerged.content
                    && let Some(retained_resolution) = retained.content_resolution()
                {
                    *resolution = (*resolution).max(retained_resolution);
                }
            }
            None => node.set_unmerged(retained.stages, retained.content_resolution()),
        }
    }
    let Some(content) = retained.content_merge() else {
        let resolution = retained.resolution();
        if !events.iter().any(|event| {
            event.affects(location)
                && matches!(
                    (&event.resolution, &resolution),
                    (
                        Err(ResolutionFailure::SubmoduleMerge),
                        Err(ResolutionFailure::SubmoduleMerge)
                    ) | (
                        Err(ResolutionFailure::SubmoduleAddAdd),
                        Err(ResolutionFailure::SubmoduleAddAdd)
                    ) | (
                        Ok(Resolution::OursModifiedTheirsModifiedThenAncestor),
                        Ok(Resolution::OursModifiedTheirsModifiedThenAncestor)
                    )
                )
        }) {
            let source_stages = std::array::from_fn(|side| {
                retained.stages[side].map(|entry| Stage {
                    entry,
                    source: stages[side]
                        .as_ref()
                        .map_or_else(|| location.clone(), |stage| stage.source.clone()),
                    merge: None,
                })
            });
            events.push(event(location, &source_stages, resolution, ConflictMapping::Original));
        }
        return;
    };
    for event in events.iter_mut().rev() {
        if event.affects(location) {
            if crate::tree::resolution_content_merge(&event.resolution) == Some(content) {
                return;
            }
            let slot = match &mut event.resolution {
                Err(
                    ResolutionFailure::ModeConflict { merged_blob }
                    | ResolutionFailure::DirectoryRename { merged_blob, .. }
                    | ResolutionFailure::OursRenamedTheirsRenamedDifferently { merged_blob },
                ) => merged_blob,
                Ok(Resolution::OursModifiedTheirsModifiedThenBlobContentMerge { merged_blob }) => {
                    if merged_blob.merged_blob_id == content.merged_blob_id {
                        merged_blob.resolution = merged_blob.resolution.max(content.resolution);
                        return;
                    }
                    continue;
                }
                _ => continue,
            };
            *slot = Some(content);
            return;
        }
    }
    events.push(event(
        location,
        stages,
        Ok(Resolution::OursModifiedTheirsModifiedThenBlobContentMerge { merged_blob: content }),
        ConflictMapping::Original,
    ));
}

fn set_result(paths: &mut State, path: &BString, entry: Option<Entry>) {
    paths.entries.entry(path.clone()).or_default().set_result(entry);
}

/// Selecting an input restores every affected rename source from that same immutable input.
fn force_path(
    paths: &mut State,
    path: &BString,
    choice: ResolveWith,
    renames: &BTreeMap<BString, [Option<Change>; 2]>,
) -> Vec<(BString, EntryMerge)> {
    let input = if choice == ResolveWith::Ancestor { 0 } else { 1 };
    let mut pending = vec![path.clone()];
    let mut restored = BTreeSet::new();
    let mut retained_entries = Vec::new();
    while let Some(path) = pending.pop() {
        if !restored.insert(path.clone()) {
            continue;
        }
        let path = &path;
        let retained = (choice == ResolveWith::Ours)
            .then(|| paths.entries.get(path)?.stages[1].clone())
            .flatten()
            .filter(|stage| stage.entry.mode.is_no_tree())
            .map(|stage| {
                let location = if renames.get(path).is_some_and(|renames| {
                    renames[0].is_none()
                        && renames[1]
                            .as_ref()
                            .is_some_and(|rename| rename.source_location() == stage.source)
                }) {
                    stage.source.clone()
                } else {
                    path.clone()
                };
                (location, stage)
            });
        let selected_directory = paths
            .entries
            .get(path)
            .is_some_and(|node| node.inputs[input].is_some_and(|entry| entry.mode.is_tree()));
        let originals: Vec<_> = paths
            .entries
            .get(path)
            .into_iter()
            .flat_map(|node| node.stages.iter().flatten())
            .map(|stage| stage.source.clone())
            .filter(|source| source != path)
            .collect();
        for source in originals {
            restore_subtree(paths, source.as_bstr(), input, &mut pending);
        }
        for (destination, incoming) in renames {
            if destination == path
                || (!selected_directory
                    && destination
                        .strip_prefix(path.as_bytes())
                        .is_some_and(|suffix| suffix.starts_with(b"/")))
            {
                for rename in incoming.iter().flatten() {
                    if let Change::Rewrite {
                        source_location,
                        location,
                        ..
                    } = rename
                    {
                        restore_subtree(paths, source_location.as_bstr(), input, &mut pending);
                        restore_subtree(paths, location.as_bstr(), input, &mut pending);
                    }
                }
            }
        }
        restore_subtree(paths, path.as_bstr(), input, &mut pending);
        retained_entries.extend(retained);
    }
    // Complete all topology restoration before retaining source merges, as another
    // rejected rename can restore an input at the same location.
    let mut content = Vec::new();
    for (location, stage) in retained_entries {
        if location != stage.source
            && let Some(original) = paths.entries.get_mut(&stage.source)
        {
            original.set_result(None);
        }
        set_result(paths, &location, Some(stage.entry));
        content.extend(stage.merge.map(|content| (location, *content)));
    }
    content
}

fn restore_subtree(paths: &mut State, path: &BStr, input: usize, pending: &mut Vec<BString>) {
    let selected_directory = paths
        .entries
        .get(path)
        .is_some_and(|node| node.inputs[input].is_some_and(|entry| entry.mode.is_tree()));
    if let Some(node) = paths.entries.get_mut(path) {
        node.restore_input(input);
    }
    // Retaining a directory does not reverse independent clean changes to its children.
    if !selected_directory {
        let mut prefix = path.to_owned();
        prefix.push(b'/');
        for (candidate, node) in paths.entries.range_mut(prefix.clone()..) {
            if !candidate.starts_with(prefix.as_bytes()) {
                break;
            }
            node.restore_input(input);
        }
    }
    if !paths
        .entries
        .get(path)
        .is_some_and(|node| matches!(node.result, PathResolution::Present(_)))
        && !paths.has_live_descendants(path)
    {
        return;
    }
    // Rejecting a rename can restore its source after a parent was resolved as a file.
    // Restore the selected input's directory topology along with the source.
    for (end, byte) in path.iter().enumerate() {
        if *byte == b'/'
            && let Some(parent) = paths.entries.get_mut(path[..end].as_bstr())
            && parent.inputs[input].is_some_and(|entry| entry.mode.is_tree())
            && matches!(parent.result, PathResolution::Present(entry) if entry.mode.is_no_tree())
        {
            parent.restore_input(input);
            pending.push(path[..end].into());
        }
    }
}

fn relocate_events(events: &mut [Event], previous: &BString, destination: &BString) {
    for event in events {
        if &event.location == previous {
            event.location = destination.clone();
        }
        event.relocate(previous, std::slice::from_ref(destination));
        match &mut event.resolution {
            Err(ResolutionFailure::DirectoryRename { final_location, .. })
            | Ok(Resolution::SourceLocationAffectedByRename { final_location })
                if final_location == previous =>
            {
                *final_location = destination.clone();
            }
            _ => {}
        }
    }
}

fn event(
    location: &BString,
    stages: &[Option<Stage>; 3],
    resolution: std::result::Result<Resolution, ResolutionFailure>,
    mapping: ConflictMapping,
) -> Event {
    Event {
        location: location.clone(),
        resolution,
        changes: [
            change_for_stage(location, stages, 1),
            change_for_stage(location, stages, 2),
        ],
        mapping,
        affected_paths: vec![location.clone()],
    }
}

fn change_for_stage(location: &BString, stages: &[Option<Stage>; 3], side: usize) -> Change {
    match (&stages[0], &stages[side]) {
        (Some(base), Some(stage)) if base.source != stage.source => Change::Rewrite {
            source_location: base.source.clone(),
            source_entry_mode: base.entry.mode,
            source_id: base.entry.id,
            source_relation: None,
            entry_mode: stage.entry.mode,
            id: stage.entry.id,
            location: stage.source.clone(),
            relation: None,
            similarity: None,
            copy: false,
        },
        (Some(base), Some(stage)) => Change::Modification {
            previous_entry_mode: base.entry.mode,
            previous_id: base.entry.id,
            entry_mode: stage.entry.mode,
            id: stage.entry.id,
            location: stage.source.clone(),
        },
        (Some(base), None) => Change::Deletion {
            entry_mode: base.entry.mode,
            id: base.entry.id,
            location: base.source.clone(),
            relation: None,
        },
        (None, Some(stage)) => Change::Addition {
            entry_mode: stage.entry.mode,
            id: stage.entry.id,
            location: stage.source.clone(),
            relation: None,
        },
        (None, None) => {
            let entry = stages
                .iter()
                .flatten()
                .next()
                .expect("a recorded conflict has an input stage")
                .entry;
            Change::Deletion {
                entry_mode: entry.mode,
                id: entry.id,
                location: location.clone(),
                relation: None,
            }
        }
    }
}

fn into_conflict(event: Event) -> Conflict {
    let [ours, theirs] = event.changes;
    Conflict {
        resolution: event.resolution,
        ours,
        theirs,
        map: event.mapping,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::function::path::MergeResolution;

    #[test]
    fn selected_intermediate_entries_preserve_their_original_conflict_stages() -> gix_error::TestResult {
        let ids = [
            "1111111111111111111111111111111111111111",
            "2222222222222222222222222222222222222222",
            "3333333333333333333333333333333333333333",
        ]
        .map(|id| gix_hash::ObjectId::from_hex(id.as_bytes()));
        let [base_id, our_id, their_id] = ids;
        let ids = [base_id?, our_id?, their_id?];
        for (kind, resolution) in [
            (
                tree::EntryKind::Blob,
                MergeResolution::Content(crate::blob::Resolution::Conflict),
            ),
            (tree::EntryKind::Commit, MergeResolution::Submodule),
        ] {
            for forced in [false, true] {
                if forced && kind == tree::EntryKind::Blob {
                    continue;
                }
                let original = ids.map(|id| Some(Entry { mode: kind.into(), id }));
                let mut selected = original[1].expect("the current input exists");
                let retained = EntryMerge {
                    entry: selected,
                    resolution,
                    forced,
                    stages: original,
                };
                if kind == tree::EntryKind::Blob {
                    selected.mode = tree::EntryKind::BlobExecutable.into();
                }
                assert!(
                    retained.matches_entry(selected),
                    "changing the executable bit does not discard retained payload conflicts"
                );
                let location: BString = "renamed".into();
                let stages = [
                    None,
                    Some(Stage {
                        entry: selected,
                        source: "source".into(),
                        merge: Some(Box::new(retained)),
                    }),
                    None,
                ];
                let mut paths = State::default();
                let mut pending = Vec::new();
                select_stage(&mut paths, &location, stages[1].as_ref(), &mut pending);
                let mut events = Vec::new();
                for (path, merge) in pending {
                    retain_merge(&mut paths, &mut events, &path, &stages, merge);
                }
                let result = &paths.entries[&location];
                assert_eq!(
                    result.result,
                    PathResolution::Present(selected),
                    "retention does not replace the selected entry"
                );
                if forced {
                    assert!(
                        result.unmerged.is_none(),
                        "an explicitly chosen gitlink has no unmerged index entries"
                    );
                } else {
                    assert_eq!(
                        result.unmerged.as_ref().expect("the source conflict survives").entries,
                        original,
                        "a one-sided normalized destination retains all three original conflict inputs"
                    );
                }
                assert_eq!(events.len(), 1, "the retained merge has one diagnostic");
                let conflict = into_conflict(events.pop().expect("one diagnostic"));
                if kind == tree::EntryKind::Commit {
                    assert!(
                        matches!(conflict.resolution, Err(ResolutionFailure::SubmoduleMerge)),
                        "gitlink source merges retain their typed diagnosis"
                    );
                    assert!(
                        conflict.content_merge().is_none(),
                        "a commit ID never appears in blob metadata"
                    );
                } else {
                    assert_eq!(
                        conflict
                            .content_merge()
                            .expect("the selected blob contains conflicts")
                            .merged_blob_id,
                        selected.id
                    );
                }
            }
        }
        Ok(())
    }
}
