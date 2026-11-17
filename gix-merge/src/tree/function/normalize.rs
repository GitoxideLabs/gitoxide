//! Establish rename identities before resolving destination paths.
//!
//! Directory moves first determine effective destinations. File renames are then grouped by
//! their original source, so rename/modify content is merged once before a destination collision.
//! Normalized stages keep their original input paths. Diagnostic events describe decisions,
//! while resolved paths own the final index entries.

use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, HashMap},
};

use bstr::{BStr, BString, ByteSlice};
use gix_diff::tree_with_rewrites::Change;
use gix_error::Result;
use gix_hash::ObjectId;

use crate::{
    blob::{self, builtin_driver::text::Labels, platform::builtin_merge::Pick},
    tree::{ConflictMapping, DirectoryRenames, Options, Resolution, ResolutionFailure, ResolveWith},
};

use super::{
    entry::{self, ResolutionPolicy},
    path::{Entry, Stage, State},
};

/// Diagnostic history follows affected output paths without owning their index entries.
pub(super) struct Event {
    pub resolution: std::result::Result<Resolution, ResolutionFailure>,
    pub changes: [Change; 2],
    pub mapping: ConflictMapping,
    pub affected_paths: Vec<BString>,
    pub location: BString,
}

impl Event {
    pub fn affects(&self, path: &BStr) -> bool {
        self.affected_paths.iter().any(|candidate| candidate.as_bstr() == path)
    }

    pub fn relocate(&mut self, previous: &BStr, destinations: &[BString]) {
        if self.affects(previous) {
            self.affected_paths.retain(|path| path.as_bstr() != previous);
            self.affected_paths.extend_from_slice(destinations);
            self.affected_paths.sort();
            self.affected_paths.dedup();
        }
    }

    pub fn should_stop(&self, options: &Options) -> bool {
        options.fail_on_conflict.is_some_and(|how| {
            crate::tree::resolution_is_unresolved(
                &entry::final_resolution(self.resolution.clone(), ResolutionPolicy::new(options)),
                how,
            )
        })
    }
}

#[derive(Default)]
pub(super) struct Outcome {
    pub events: Vec<Event>,
    /// Original renames remain available even when a destination becomes an add/add conflict.
    pub renames: BTreeMap<BString, [Option<Change>; 2]>,
}

struct Rename {
    side: usize,
    original: Change,
    destination: BString,
    relocated: bool,
}

impl Rename {
    fn retained_location(&self, options: &Options) -> &BStr {
        if options.directory_renames == DirectoryRenames::Conflict {
            self.original.location()
        } else {
            self.destination.as_bstr()
        }
    }
}

struct Forced<'input> {
    paths: Vec<BString>,
    merged: Option<(BString, Stage<'input>)>,
}

struct Context<'a, 'labels, 'ctx, O, W> {
    labels: Labels<'labels>,
    objects: &'a O,
    blob_merge: &'a mut blob::Platform,
    buffer: &'a mut Vec<u8>,
    write_blob: &'a mut W,
    external: &'a mut crate::tree::Context<'ctx>,
    options: &'a Options,
    policy: ResolutionPolicy,
}

#[expect(clippy::too_many_arguments)]
pub(super) fn run(
    state: &mut State<'_>,
    changes: [&[Change]; 2],
    directories: [&[gix_diff::tree_with_rewrites::DirectoryRename]; 2],
    labels: Labels<'_>,
    objects: &impl gix_object::FindObjectOrHeader,
    blob_merge: &mut blob::Platform,
    buffer: &mut Vec<u8>,
    write_blob: &mut impl FnMut(&[u8]) -> Result<ObjectId>,
    external: &mut crate::tree::Context<'_>,
    options: &Options,
    policy: ResolutionPolicy,
) -> Result<Outcome> {
    let mut outcome = Outcome::default();
    let mut forced = Vec::new();
    let destinations = relocate_directories(
        state,
        changes,
        directories,
        objects,
        options,
        policy,
        &mut outcome,
        &mut forced,
    )?;
    if options.fail_on_conflict.is_some() && outcome.events.iter().any(|event| event.should_stop(options)) {
        return Ok(outcome);
    }
    let mut sources: BTreeMap<BString, [Option<Rename>; 2]> = BTreeMap::new();
    for (side, side_changes) in changes.into_iter().enumerate() {
        for change in side_changes {
            if let Change::Rewrite {
                source_location,
                location,
                entry_mode,
                ..
            } = change
                && !entry_mode.is_tree()
            {
                let destination = destinations
                    .get(&(side + 1, location.clone()))
                    .cloned()
                    .unwrap_or_else(|| location.clone());
                outcome.renames.entry(destination.clone()).or_default()[side] = Some(change.clone());
                // An inferred move can return a rename to its source while its
                // explicit identity still participates in an opposite rename.
                sources.entry(source_location.clone()).or_default()[side] = Some(Rename {
                    side: side + 1,
                    original: change.clone(),
                    relocated: destination != *location,
                    destination,
                });
            }
        }
    }
    let original_changes = std::array::from_fn(|side| {
        if sources
            .values()
            .any(|renames| renames[side].is_none() && renames[1 - side].is_some())
        {
            index_changes(changes[side])
        } else {
            HashMap::new()
        }
    });
    let mut context = Context {
        labels,
        objects,
        blob_merge,
        buffer,
        write_blob,
        external,
        options,
        policy,
    };
    for (source, [ours, theirs]) in sources {
        for path in std::iter::once(&source).chain(ours.iter().chain(theirs.iter()).map(|rename| &rename.destination)) {
            if state.trivial.contains_key(path.as_bstr()) {
                state.materialize(path.as_bstr());
                state
                    .entries
                    .get_mut(path.as_bstr())
                    .expect("the rename input has been materialized")
                    .invalidate();
            }
        }
        match (ours, theirs) {
            (Some(ours), Some(theirs)) => {
                context.both(state, &source, ours, theirs, &mut outcome, &mut forced)?;
            }
            (Some(rename), None) | (None, Some(rename)) => {
                context.one(state, &source, rename, &original_changes, &mut outcome, &mut forced)?;
            }
            (None, None) => unreachable!("a source is inserted with a rename"),
        }
        if options.fail_on_conflict.is_some() && outcome.events.iter().any(|event| event.should_stop(options)) {
            return Ok(outcome);
        }
    }
    if policy.forced_resolution() == Some(ResolveWith::Ours) {
        for event in &mut outcome.events {
            if !matches!(event.resolution, Err(ResolutionFailure::DirectoryRename { .. }))
                || !outcome.renames.contains_key(&event.location)
            {
                continue;
            }
            let stages = &state.entries[event.location.as_bstr()].stages;
            if let [base, Some(ours), Some(theirs)] = stages
                && !ours.entry.mode.is_tree()
                && !theirs.entry.mode.is_tree()
                && let Some(merged) = context.merge(base.as_ref(), ours, theirs, 0)?
                && merged.force.is_none()
            {
                if let Err(ResolutionFailure::DirectoryRename { merged_blob, .. }) = &mut event.resolution {
                    *merged_blob = merged.content_merge();
                }
                forced.push(Forced {
                    paths: vec![
                        event.location.clone(),
                        ours.source.clone().into_owned(),
                        theirs.source.clone().into_owned(),
                    ],
                    merged: Some((
                        ours.source.clone().into_owned(),
                        merged_stage(
                            ours.clone(),
                            merged.entry,
                            &merged,
                            stages.each_ref().map(|stage| stage.as_ref().map(|stage| stage.entry)),
                        ),
                    )),
                });
                if event.should_stop(options) {
                    // Restore the selected source and its content stages before returning.
                    break;
                }
            }
        }
    }
    if let Some(choice) = policy.forced_resolution() {
        for (destination, renames) in &outcome.renames {
            if let [Some(ours), Some(theirs)] = renames
                && ours.source_location() != theirs.source_location()
                && state.entries[destination.as_bstr()].stages[1]
                    .as_ref()
                    .map(|stage| stage.entry)
                    != state.entries[destination.as_bstr()].stages[2]
                        .as_ref()
                        .map(|stage| stage.entry)
            {
                outcome.events.push(Event {
                    resolution: Err(ResolutionFailure::OursRenamedTheirsRenamedToSameLocation),
                    changes: [ours.clone(), theirs.clone()],
                    mapping: ConflictMapping::Original,
                    affected_paths: vec![destination.clone()],
                    location: destination.clone(),
                });
                forced.push(Forced {
                    paths: vec![
                        destination.clone(),
                        ours.source_location().to_owned(),
                        theirs.source_location().to_owned(),
                    ],
                    merged: None,
                });
            }
        }
        let mut retained = Vec::new();
        for plan in forced {
            let merged = force_originals(state, &outcome.renames, choice, plan, objects)?;
            if choice == ResolveWith::Ours {
                retained.extend(merged);
            }
        }
        // Overlapping rename groups can restore the same path. Apply their retained
        // content only after every topology choice has restored its original entries.
        for (location, _) in &retained {
            if let Some(original) = state.entries[location.as_bstr()].stages[1]
                .as_ref()
                .map(|stage| stage.source.clone().into_owned())
                && original != *location
                && let Some(node) = state.entries.get_mut(original.as_bstr())
            {
                node.set_result(None);
            }
            let mut prefix = location.clone();
            prefix.push(b'/');
            for (_, node) in state
                .entries
                .range_mut(Cow::Owned(prefix.clone())..)
                .take_while(|(path, _)| path.starts_with(prefix.as_bytes()))
            {
                node.set_result(None);
            }
        }
        for (location, stage) in retained {
            let node = state.entries.entry(Cow::Owned(location)).or_default();
            node.set_stage(1, Some(stage.clone()));
            node.set_result(Some(stage.entry));
            if let Some(content) = stage.merge
                && !content.forced
            {
                node.set_unmerged(content.stages, content.content_resolution());
            }
        }
    }
    Ok(outcome)
}

impl<O: gix_object::FindObjectOrHeader, W: FnMut(&[u8]) -> Result<ObjectId>> Context<'_, '_, '_, O, W> {
    fn merge(
        &mut self,
        base: Option<&Stage<'_>>,
        ours: &Stage<'_>,
        theirs: &Stage<'_>,
        extra_markers: u8,
    ) -> Result<Option<entry::Outcome>> {
        entry::merge(
            self.labels,
            self.objects,
            self.blob_merge,
            self.buffer,
            self.write_blob,
            (&ours.source, ours.entry.id, ours.entry.mode),
            (&theirs.source, theirs.entry.id, theirs.entry.mode),
            (
                base.map_or(&ours.source, |base| &base.source),
                base.map(|base| (base.entry.id, base.entry.mode)),
            ),
            (extra_markers, ConflictMapping::Original, ConflictMapping::Original),
            self.external,
            self.options,
            self.policy,
        )
    }

    fn both<'input>(
        &mut self,
        state: &mut State<'input>,
        source: &BString,
        ours: Rename,
        theirs: Rename,
        outcome: &mut Outcome,
        forced: &mut Vec<Forced<'input>>,
    ) -> Result {
        let Some(base) = leaf_stage(state, source, 0) else {
            return Ok(());
        };
        let Some(our_stage) = leaf_stage(state, &ours.destination, 1) else {
            return Ok(());
        };
        let Some(their_stage) = leaf_stage(state, &theirs.destination, 2) else {
            return Ok(());
        };
        let relocated = ours.relocated || theirs.relocated;
        let force_location = relocated && self.options.directory_renames == DirectoryRenames::Conflict;
        let source_stages = [Some(base.entry), Some(our_stage.entry), Some(their_stage.entry)];
        if ours.destination == theirs.destination {
            let node = state.entries.entry(Cow::Owned(ours.destination.clone())).or_default();
            node.set_stage(0, Some(base.clone()));
            remove_source(state, source, &ours.destination, &[0, 1, 2]);
            if force_location && self.policy.forced_resolution().is_some() {
                let merged = self.merge(Some(&base), &our_stage, &their_stage, 0)?;
                forced.push(Forced {
                    paths: rename_paths(source, [&ours, &theirs]),
                    merged: merged.as_ref().map(|merged| {
                        (
                            ours.original.location().to_owned(),
                            merged_stage(our_stage.clone(), merged.entry, merged, source_stages),
                        )
                    }),
                });
                attach_relocated_content(outcome, [&ours.destination, &theirs.destination], merged.as_ref());
            }
            return Ok(());
        }

        let merged = self.merge(Some(&base), &our_stage, &their_stage, 1)?;
        let (our_entry, their_entry) = match &merged {
            Some(merged)
                if merged.is_conflict() && merged.pick == Pick::Ours && merged.entry.mode == our_stage.entry.mode =>
            {
                (our_stage.entry, their_stage.entry)
            }
            Some(merged) => (merged.entry, merged.entry),
            None => (our_stage.entry, their_stage.entry),
        };
        for (rename, stage, entry) in [(&ours, our_stage, our_entry), (&theirs, their_stage, their_entry)] {
            let node = state.entries.entry(Cow::Owned(rename.destination.clone())).or_default();
            let stage = match &merged {
                Some(merged) => merged_stage(stage, entry, merged, source_stages),
                None => stage,
            };
            node.set_stage(rename.side, Some(stage));
            node.path_conflict = true;
        }
        for rename in [&ours, &theirs] {
            remove_source(state, source, &rename.destination, &[rename.side]);
        }
        state
            .entries
            .get_mut(source.as_bstr())
            .expect("the rename source remains indexed")
            .path_conflict = true;
        let merged_blob = merged.as_ref().and_then(entry::Outcome::content_merge);
        outcome.events.push(Event {
            resolution: Err(ResolutionFailure::OursRenamedTheirsRenamedDifferently { merged_blob }),
            changes: [ours.original.clone(), theirs.original.clone()],
            mapping: ConflictMapping::Original,
            affected_paths: vec![source.clone(), ours.destination.clone(), theirs.destination.clone()],
            location: source.clone(),
        });
        if self.policy.forced_resolution().is_some() {
            forced.push(Forced {
                paths: rename_paths(source, [&ours, &theirs]),
                merged: Some((
                    ours.retained_location(self.options).to_owned(),
                    leaf_stage(state, &ours.destination, 1).expect("the normalized current rename stage exists"),
                )),
            });
            attach_relocated_content(outcome, [&ours.destination, &theirs.destination], merged.as_ref());
        }
        Ok(())
    }

    fn one<'input>(
        &mut self,
        state: &mut State<'input>,
        source: &BString,
        rename: Rename,
        changes: &[HashMap<&BStr, &Change>; 2],
        outcome: &mut Outcome,
        forced: &mut Vec<Forced<'input>>,
    ) -> Result {
        let Some(base) = leaf_stage(state, source, 0) else {
            return Ok(());
        };
        let Some(renamed) = leaf_stage(state, &rename.destination, rename.side) else {
            return Ok(());
        };
        let other = 3 - rename.side;
        let opposite_source = leaf_stage(state, source, other);
        let deleted = opposite_source.is_none();
        let type_changed = opposite_source
            .as_ref()
            .is_some_and(|stage| stage.entry.mode.is_blob() != renamed.entry.mode.is_blob());
        let destination_collision =
            source != &rename.destination && leaf_stage(state, &rename.destination, other).is_some();
        let collision = destination_collision && !type_changed;
        let mut global_changes = [rename.original.clone(), rename.original.clone()];
        global_changes[rename.side - 1] = rename.original.clone();
        global_changes[other - 1] =
            original_change(state, &changes[other - 1], source, other).unwrap_or_else(|| rename.original.clone());
        let mut merged = None;
        if let Some(opposite) = &opposite_source
            && !type_changed
            && (collision
                || (rename.relocated
                    && self.options.directory_renames == DirectoryRenames::Conflict
                    && self.policy.forced_resolution().is_some()))
        {
            merged = if rename.side == 1 {
                self.merge(Some(&base), &renamed, opposite, u8::from(collision))?
            } else {
                self.merge(Some(&base), opposite, &renamed, u8::from(collision))?
            };
            // A later forced tree choice may discard the source merge entirely.
            if self.options.fail_on_conflict.is_some()
                && self.options.tree_conflicts.is_none()
                && let Some(merged) = &merged
                && let Some(resolution) = merged.resolution()
            {
                let mut stages = [Some(base.entry), None, None];
                stages[rename.side] = Some(renamed.entry);
                stages[other] = Some(opposite.entry);
                let event = Event {
                    resolution,
                    changes: global_changes.clone(),
                    mapping: ConflictMapping::Original,
                    affected_paths: vec![source.clone(), rename.destination.clone()],
                    location: rename.destination.clone(),
                };
                if event.should_stop(self.options) {
                    // Only an aborted merge exposes this intermediate source merge before its collision.
                    let mut source_entries = stages;
                    source_entries[rename.side] = None;
                    state
                        .entries
                        .get_mut(source.as_bstr())
                        .expect("the source is indexed")
                        .set_unmerged(source_entries, merged.content_resolution());
                    let mut destination_entries = [None; 3];
                    destination_entries[rename.side] = Some(renamed.entry);
                    state
                        .entries
                        .get_mut(rename.destination.as_bstr())
                        .expect("the rename destination is indexed")
                        .set_unmerged(destination_entries, merged.content_resolution());
                    outcome.events.push(event);
                    return Ok(());
                }
            }
        }
        if collision {
            if let Some(merged) = &merged {
                let mut source_stages = [Some(base.entry), None, None];
                source_stages[rename.side] = Some(renamed.entry);
                source_stages[other] = opposite_source.as_ref().map(|stage| stage.entry);
                let node = state.entries.entry(Cow::Owned(rename.destination.clone())).or_default();
                node.set_stage(
                    rename.side,
                    Some(merged_stage(renamed.clone(), merged.entry, merged, source_stages)),
                );
            }
        } else if opposite_source.as_ref().map(|stage| stage.entry) != Some(base.entry) {
            let node = state.entries.entry(Cow::Owned(rename.destination.clone())).or_default();
            node.set_stage(0, Some(base.clone()));
            if !type_changed {
                node.set_stage(other, opposite_source.clone());
                // Distinct non-regular types still share the renamed destination. Their
                // identity conflict must survive even when one entry matches the base.
                node.path_conflict |= opposite_source.as_ref().is_some_and(|opposite| {
                    entry::merge_modes(opposite.entry.mode, renamed.entry.mode, None).is_none()
                });
            }
        }
        // An unchanged opposite source already resolves to removal without transferring
        // its base or side stage, including when a directory move changes the destination.
        remove_source(state, source, &rename.destination, &[0]);
        if !type_changed {
            remove_source(state, source, &rename.destination, &[other]);
        }

        if deleted || (type_changed && !destination_collision) {
            let node = state.entries.entry(Cow::Owned(rename.destination.clone())).or_default();
            node.path_conflict = true;
            node.invalidate();
            let failure = if deleted {
                ResolutionFailure::OursDeletedTheirsRenamed
            } else {
                ResolutionFailure::OursModifiedTheirsRenamedTypeMismatch
            };
            outcome.events.push(Event {
                resolution: Err(failure),
                changes: global_changes,
                mapping: if rename.side == 1 {
                    ConflictMapping::Swapped
                } else {
                    ConflictMapping::Original
                },
                affected_paths: vec![rename.destination.clone()],
                location: rename.destination.clone(),
            });
            if !collision && let Some(choice) = self.policy.forced_resolution() {
                let mut paths = rename_paths(source, [&rename]);
                let retained_location = if choice == ResolveWith::Ours && rename.side == 1 {
                    rename.retained_location(self.options)
                } else {
                    source.as_bstr()
                };
                // A replacement can obstruct the retained location or move another source.
                for end in source.find_iter(b"/") {
                    let parent = source[..end].as_bstr();
                    if state.entries.get(parent).is_some_and(|node| {
                        node.inputs[0].is_some_and(|entry| entry.mode.is_tree())
                            && node.inputs[other].is_some_and(|entry| entry.mode.is_no_tree())
                    }) && (below(retained_location, parent)
                        || outcome.renames.get(parent).is_some_and(|renames| {
                            matches!(renames[other - 1], Some(Change::Rewrite { copy: false, .. }))
                        }))
                    {
                        paths.push(parent.to_owned());
                    }
                }
                forced.push(Forced {
                    paths,
                    merged: (rename.side == 1)
                        .then(|| (rename.retained_location(self.options).to_owned(), renamed.clone())),
                });
            }
        }
        if rename.relocated
            && self.options.directory_renames == DirectoryRenames::Conflict
            && self.policy.forced_resolution().is_some()
        {
            let selected_location = if rename.side == 1 {
                rename.original.location()
            } else {
                source.as_bstr()
            };
            forced.push(Forced {
                paths: rename_paths(source, [&rename]),
                merged: merged.as_ref().map(|merged| {
                    let mut source_stages = [Some(base.entry), None, None];
                    source_stages[rename.side] = Some(renamed.entry);
                    source_stages[other] = opposite_source.as_ref().map(|stage| stage.entry);
                    let current = if rename.side == 1 {
                        &renamed
                    } else {
                        opposite_source.as_ref().expect("a merged source has both input stages")
                    };
                    (
                        selected_location.to_owned(),
                        merged_stage(current.clone(), merged.entry, merged, source_stages),
                    )
                }),
            });
            attach_relocated_content(outcome, [&rename.destination], merged.as_ref());
        }
        Ok(())
    }
}

fn leaf_stage<'input>(state: &State<'input>, location: &BString, side: usize) -> Option<Stage<'input>> {
    state.entries.get(location.as_bstr())?.stages[side]
        .as_ref()
        .filter(|stage| !stage.entry.mode.is_tree())
        .cloned()
}

fn merged_stage<'input>(
    stage: Stage<'input>,
    entry: Entry,
    merged: &entry::Outcome,
    source_stages: [Option<Entry>; 3],
) -> Stage<'input> {
    let merge = merged
        .retained_merge(entry, source_stages)
        .map(Box::new)
        .or(stage.merge.filter(|merge| merge.matches_entry(entry)));
    Stage { entry, merge, ..stage }
}

fn remove_source(state: &mut State<'_>, source: &BString, destination: &BString, sides: &[usize]) {
    if source == destination {
        return;
    }
    if let Some(node) = state.entries.get_mut(source.as_bstr()) {
        for &side in sides {
            if node.stages[side]
                .as_ref()
                .is_some_and(|stage| !stage.entry.mode.is_tree())
            {
                node.set_stage(side, None);
            }
        }
    }
}

fn rename_paths<const N: usize>(source: &BString, renames: [&Rename; N]) -> Vec<BString> {
    std::iter::once(source.clone())
        .chain(
            renames
                .into_iter()
                .flat_map(|rename| [rename.original.location().to_owned(), rename.destination.clone()]),
        )
        .collect()
}

fn attach_relocated_content<const N: usize>(
    outcome: &mut Outcome,
    locations: [&BString; N],
    merged: Option<&entry::Outcome>,
) {
    let merged_blob = merged.and_then(entry::Outcome::content_merge);
    for event in &mut outcome.events {
        if locations.contains(&&event.location)
            && let Err(ResolutionFailure::DirectoryRename {
                merged_blob: content, ..
            }) = &mut event.resolution
        {
            *content = merged_blob;
        }
    }
}

fn index_changes(changes: &[Change]) -> HashMap<&BStr, &Change> {
    let mut indexed = HashMap::with_capacity(changes.len());
    for change in changes {
        if !change.entry_mode().is_tree() {
            indexed.entry(change.source_location()).or_insert(change);
        }
    }
    indexed
}

fn original_change(
    state: &State<'_>,
    changes: &HashMap<&BStr, &Change>,
    path: &BString,
    side: usize,
) -> Option<Change> {
    if let Some(change) = changes.get(path.as_bstr()) {
        return Some((**change).clone());
    }
    let node = state.entries.get(path.as_bstr())?;
    match (node.inputs[0], node.inputs[side]) {
        (Some(base), Some(current)) => Some(Change::Modification {
            location: path.to_owned(),
            previous_entry_mode: base.mode,
            previous_id: base.id,
            entry_mode: current.mode,
            id: current.id,
        }),
        (Some(base), None) => Some(Change::Deletion {
            location: path.to_owned(),
            relation: None,
            entry_mode: base.mode,
            id: base.id,
        }),
        (None, Some(current)) => Some(Change::Addition {
            location: path.to_owned(),
            relation: None,
            entry_mode: current.mode,
            id: current.id,
        }),
        (None, None) => None,
    }
}

fn force_originals<'input>(
    state: &mut State<'_>,
    renames: &BTreeMap<BString, [Option<Change>; 2]>,
    choice: ResolveWith,
    mut plan: Forced<'input>,
    objects: &impl gix_object::Find,
) -> Result<Option<(BString, Stage<'input>)>> {
    if choice == ResolveWith::Ours
        && let Some((location, _)) = &plan.merged
    {
        let mut prefix = location.clone();
        prefix.push(b'/');
        for (_, incoming) in renames
            .range(prefix.clone()..)
            .take_while(|(path, _)| path.starts_with(prefix.as_bytes()))
        {
            for rename in incoming.iter().flatten() {
                plan.paths.push(rename.source_location().to_owned());
                plan.paths.push(rename.location().to_owned());
            }
        }
    }
    for path in plan.paths.clone() {
        if let Some(renames) = renames.get(&path) {
            for rename in renames.iter().flatten() {
                plan.paths.push(rename.source_location().to_owned());
                plan.paths.push(rename.location().to_owned());
            }
        }
    }
    // Include rename sources discovered above before restoring their entire namespace.
    for path in &plan.paths {
        state.ensure_subtree(path.as_bstr(), objects)?;
    }
    let side = usize::from(choice == ResolveWith::Ours);
    // Retained directories must not resurrect descendants independently renamed away.
    let selected_leaves: Vec<_> = plan
        .paths
        .iter()
        .filter(|path| {
            state
                .entries
                .get(path.as_bstr())
                .and_then(|node| node.inputs[side])
                .is_some_and(|entry| !entry.mode.is_tree())
        })
        .cloned()
        .collect();
    for path in &plan.paths {
        if let Some(node) = state.entries.get_mut(path.as_bstr()) {
            node.restore_input(side);
        }
    }
    for mut parent in selected_leaves {
        parent.push(b'/');
        for (_, node) in state
            .entries
            .range_mut(Cow::Owned(parent.clone())..)
            .take_while(|(path, _)| path.starts_with(parent.as_bytes()))
        {
            node.set_result(None);
        }
    }
    Ok(plan.merged)
}

fn below(path: &BStr, parent: &BStr) -> bool {
    path.strip_prefix(parent.as_bytes())
        .is_some_and(|suffix| suffix.starts_with(b"/"))
}

#[expect(clippy::too_many_arguments)]
fn relocate_directories(
    state: &mut State<'_>,
    changes: [&[Change]; 2],
    inferred: [&[gix_diff::tree_with_rewrites::DirectoryRename]; 2],
    objects: &impl gix_object::Find,
    options: &Options,
    policy: ResolutionPolicy,
    outcome: &mut Outcome,
    forced: &mut Vec<Forced<'_>>,
) -> Result<BTreeMap<(usize, BString), BString>> {
    let mut destinations = BTreeMap::new();
    if options.directory_renames == DirectoryRenames::Disabled {
        return Ok(destinations);
    }
    for (side, inferred) in inferred.iter().enumerate() {
        for directory in *inferred {
            if directory.destinations.len() <= 1 {
                continue;
            }
            let original = directory_change(changes[side], &directory.source);
            let opposite = changes[1 - side]
                .iter()
                .find(|change| below(change.location(), directory.source.as_bstr()))
                .unwrap_or(original);
            let mut global_changes = [original.clone(), original.clone()];
            global_changes[1 - side] = opposite.clone();
            outcome.events.push(Event {
                resolution: Err(ResolutionFailure::DirectoryRenameSplit {
                    source_location: directory.source.clone(),
                    destinations: directory.destinations.iter().cloned().collect(),
                }),
                changes: global_changes,
                mapping: ConflictMapping::Original,
                affected_paths: vec![directory.source.clone()],
                location: directory.source.clone(),
            });
        }
    }
    let directories: [BTreeMap<_, _>; 2] = inferred.map(|directories| {
        directories
            .iter()
            .filter_map(|directory| {
                let (destination, []) = directory.destinations.split_first() else {
                    return None;
                };
                Some((&directory.source, destination))
            })
            .collect()
    });
    let mut planned = Vec::new();
    let mut counts = BTreeMap::new();
    for (side, side_changes) in changes.into_iter().enumerate() {
        for change in side_changes {
            if change.entry_mode().is_tree() || !matches!(change, Change::Addition { .. } | Change::Rewrite { .. }) {
                continue;
            }
            let Some((directory, destination)) = directories[1 - side]
                .iter()
                .rev()
                .find(|(directory, _)| below(change.location(), directory.as_bstr()))
            else {
                continue;
            };
            if directories[side].contains_key(directory) || directories[side].contains_key(destination) {
                continue;
            }
            let mut location = (*destination).clone();
            if !location.is_empty() {
                location.push(b'/');
            }
            location.extend_from_slice(&change.location()[directory.len() + 1..]);
            counts
                .entry((side + 1, location.clone()))
                .or_insert_with(Vec::new)
                .push(change.location().to_owned());
            planned.push((
                side + 1,
                change,
                directory_change(changes[1 - side], directory),
                location,
            ));
        }
    }
    let mut reported = BTreeSet::new();
    for (side, change, rewrite, location) in planned {
        state.ensure_path(location.as_bstr(), objects)?;
        let blocked = state.entries.get(location.as_bstr()).is_some_and(|node| {
            node.stages[side].is_some()
                || (node.inputs[0].is_some_and(|entry| !entry.mode.is_tree()) && change.source_location() != location)
        });
        let original_paths = &counts[&(side, location.clone())];
        if blocked || original_paths.len() > 1 {
            if reported.insert((side, location.clone())) {
                let mut global_changes = [rewrite.clone(), rewrite.clone()];
                global_changes[side - 1] = change.clone();
                outcome.events.push(Event {
                    resolution: Err(if blocked {
                        ResolutionFailure::DirectoryRenameBlocked {
                            original_paths: original_paths.clone(),
                            final_location: location.clone(),
                        }
                    } else {
                        ResolutionFailure::DirectoryRenameCollision {
                            original_paths: original_paths.clone(),
                            final_location: location.clone(),
                        }
                    }),
                    changes: global_changes,
                    mapping: ConflictMapping::Original,
                    affected_paths: original_paths.clone(),
                    location,
                });
            }
            continue;
        }
        state.ensure_subtree(location.as_bstr(), objects)?;
        state.ensure_path(change.location(), objects)?;
        let Some(stage) = state
            .entries
            .get_mut(change.location())
            .and_then(|node| node.take_stage(side))
        else {
            continue;
        };
        let mut global_changes = [rewrite.clone(), rewrite.clone()];
        global_changes[side - 1] = change.clone();
        let resolution = if options.directory_renames == DirectoryRenames::Conflict {
            Err(ResolutionFailure::DirectoryRename {
                final_location: location.clone(),
                merged_blob: None,
            })
        } else {
            Ok(Resolution::SourceLocationAffectedByRename {
                final_location: location.clone(),
            })
        };
        outcome.events.push(Event {
            resolution,
            changes: global_changes,
            mapping: if side == 1 {
                ConflictMapping::Swapped
            } else {
                ConflictMapping::Original
            },
            affected_paths: vec![location.clone()],
            location: location.clone(),
        });
        let node = state.entries.entry(Cow::Owned(location.clone())).or_default();
        node.set_stage(side, Some(stage));
        node.path_conflict |= options.directory_renames == DirectoryRenames::Conflict;
        let mut parent = location.as_bstr();
        while let Some(slash) = parent.rfind_byte(b'/') {
            parent = parent[..slash].as_bstr();
            let node = state.entries.entry(Cow::Owned(parent.to_owned())).or_default();
            // The inferred directory replaces inherited files, but modifications on
            // either side must survive through ordinary file/directory resolution.
            for input in [1, 2] {
                if node.stages[input]
                    .as_ref()
                    .is_some_and(|stage| stage.entry.mode.is_no_tree() && Some(stage.entry) == node.inputs[0])
                {
                    node.set_stage(input, None);
                }
            }
            node.directory_mask |= 1 << side;
            node.invalidate();
        }
        destinations.insert((side, change.location().to_owned()), location.clone());
        if options.directory_renames == DirectoryRenames::Conflict && policy.forced_resolution().is_some() {
            forced.push(Forced {
                paths: vec![change.location().to_owned(), location],
                merged: None,
            });
        }
    }
    Ok(destinations)
}

fn directory_change<'a>(changes: &'a [Change], source: &BString) -> &'a Change {
    changes
        .iter()
        .find(|change| change.entry_mode().is_tree() && change.source_location() == source)
        .expect("directory inference starts from a removed tree")
}

#[cfg(test)]
mod tests {
    use bstr::{BStr, BString};
    use gix_error::TestResult;
    use gix_object::tree::EntryKind;

    use super::{Change, Entry, State, index_changes, original_change};

    #[test]
    fn original_changes_preserve_the_first_leaf_and_immutable_inputs() -> TestResult {
        let blob_id = gix_testtools::object_hash().null();
        let path: BString = "source".into();
        let tree_change = Change::Deletion {
            location: path.clone(),
            relation: None,
            entry_mode: EntryKind::Tree.into(),
            id: blob_id,
        };
        let first = Change::Deletion {
            location: path.clone(),
            relation: None,
            entry_mode: EntryKind::Blob.into(),
            id: blob_id,
        };
        let later = Change::Addition {
            location: path.clone(),
            relation: None,
            entry_mode: EntryKind::BlobExecutable.into(),
            id: blob_id,
        };
        let changes = [tree_change, first.clone(), later];
        let changes = index_changes(&changes);
        let mut state = State::default();
        assert_eq!(
            original_change(&state, &changes, &path, 1),
            Some(first),
            "tree boundaries are skipped and the first leaf change retains its identity"
        );
        let node = state.entries.entry(BStr::new("untouched").into()).or_default();
        let base = Entry {
            mode: EntryKind::Blob.into(),
            id: blob_id,
        };
        let current = Entry {
            mode: EntryKind::BlobExecutable.into(),
            id: blob_id,
        };
        node.inputs = [Some(base), Some(current), None];
        assert_eq!(
            original_change(&state, &changes, &"untouched".into(), 1),
            Some(Change::Modification {
                location: "untouched".into(),
                previous_entry_mode: base.mode,
                previous_id: base.id,
                entry_mode: current.mode,
                id: current.id,
            }),
            "original input versions survive even when normalization has removed their stages"
        );
        Ok(())
    }
}
