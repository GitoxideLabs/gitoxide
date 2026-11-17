use std::collections::{BTreeMap, HashSet};

use bstr::{BStr, BString, ByteSlice};
use gix_error::{Result, ResultExt, bail, message};
use gix_index::entry::Stage;
use gix_object::FindExt;

use super::{Resolution, ResolutionFailure, TreatAsUnresolved, treat_as_unresolved};

/// An owned plan for the merge's final index, independent of its diagnostic conflicts.
///
/// The plan retains an input tree and only the paths changed by the merge. Unchanged subtrees
/// are expanded when [`to_index()`](Self::to_index()) is called, so retaining a plan does not
/// require reading every unchanged file or keeping a repository open.
#[derive(Clone, Debug)]
pub struct IndexPlan {
    /// Input tree supplying unchanged leaf entries when the index is materialized.
    tree_id: gix_hash::ObjectId,
    /// Final leaf-path overrides keyed by full, `/`-separated Git path bytes relative to the tree root.
    /// Values hold a stage-zero entry (mode and object ID), or `None` if that leaf is absent,
    /// plus optional base/current/other entries that replace stage zero with stages 1/2/3
    /// when considered unmerged at the requested sensitivity.
    /// Each key suppresses the corresponding input leaf; leaves without a key come from `tree_id`.
    paths: BTreeMap<BString, Path>,
    /// Whether recorded resolutions need review under Git's default, forced-content,
    /// and forced-tree sensitivities, respectively.
    needs_review: [bool; 3],
}

#[derive(Clone, Debug)]
struct Path {
    /// Final stage-zero leaf entry, used when `unmerged` is absent or considered resolved.
    /// `None` means no stage-zero entry is emitted at this path.
    entry: Option<Entry>,
    /// Retained original base/current/other leaf entries and their conflict status.
    /// When unresolved at the requested sensitivity, they replace `entry` with stages 1/2/3.
    unmerged: Option<Box<Unmerged>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Entry {
    pub mode: gix_object::tree::EntryMode,
    pub id: gix_hash::ObjectId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Unmerged {
    /// Original base/current/other entries at this final path, never diagnostic-side order.
    pub entries: [Option<Entry>; 3],
    /// Structural conflicts are always unmerged; content conflicts follow the requested sensitivity.
    pub content: Option<crate::blob::Resolution>,
}

impl Unmerged {
    pub(crate) fn is_unmerged(&self, how: TreatAsUnresolved) -> bool {
        self.entries.iter().any(Option::is_some)
            && match self.content {
                None | Some(crate::blob::Resolution::Conflict) => true,
                Some(crate::blob::Resolution::CompleteWithAutoResolvedConflict) => {
                    how.content_merge == treat_as_unresolved::ContentMerge::ForcedResolution
                }
                Some(crate::blob::Resolution::Complete) => false,
            }
    }
}

impl IndexPlan {
    /// Create a fresh, fully expanded index for this merge using `objects` to read unchanged subtrees.
    ///
    /// Each path has either stage zero or its final unmerged stages, selected according to `how`.
    /// If `existing` describes the same worktree, reuse unchanged stage-zero entries' stat data and
    /// persistent flags, and adapt its `TREE` and `UNTR` caches by invalidating affected paths.
    /// Its timestamp is retained for racy-stat detection. Without it, stat data is zero and extensions are absent
    /// which affects Git status performance.
    /// Cached sizes of racy retained entries are zeroed to force content checks after writing the index.
    /// New, changed, and unmerged entries have zero stat data; checkout must populate new filesystem stats.
    /// Removal flags, intent-to-add, and entry-position-dependent extensions are not inherited.
    /// All emitted paths are checked with `validate`, including mode-specific symlink protection.
    /// Null object IDs, duplicate entries, and overlapping stage-zero files are rejected. Unmerged source stages
    /// may appear below a stage-zero file, as in a rename/rename conflict whose source directory was replaced.
    /// Repeated calls produce the same entries, regardless of edits to diagnostic conflicts.
    ///
    /// A merge stopped by `fail_on_conflict` yields the same partial result as its tree; constructing
    /// its index does not complete that merge. Objects written by the merge must remain available.
    pub fn to_index(
        &self,
        how: TreatAsUnresolved,
        existing: Option<&gix_index::State>,
        objects: &impl gix_object::Find,
        validate: gix_validate::path::component::Options,
    ) -> Result<gix_index::State> {
        let mut index = gix_index::State::new(self.tree_id.kind());
        let mut pending = vec![(BString::default(), self.tree_id, None::<BString>)];
        let mut buffer = Vec::new();
        // Filter discarded inputs before validation: only the final index paths will be checked out.
        while let Some((mut path, tree_id, invalid_parent)) = pending.pop() {
            if tree_id == tree_id.kind().empty_tree() {
                continue;
            }
            let prefix_len = path.len();
            for entry in objects.find_tree_iter(&tree_id, &mut buffer)? {
                let entry = entry?;
                path.truncate(prefix_len);
                if prefix_len != 0 {
                    path.push(b'/');
                }
                path.extend_from_slice(entry.filename);
                if entry.mode.is_tree() {
                    let invalid_component = invalid_parent.clone().or_else(|| {
                        gix_validate::path::component(entry.filename, None, validate)
                            .is_err()
                            .then(|| entry.filename.to_owned())
                    });
                    pending.push((path.clone(), entry.oid.to_owned(), invalid_component));
                } else if !self.paths.contains_key(path.as_bstr()) {
                    // Splitting the full path cannot detect separators or empty names in raw tree components.
                    for component in invalid_parent
                        .iter()
                        .map(|name| name.as_bstr())
                        .chain(Some(entry.filename))
                    {
                        gix_validate::path::component(component, None, validate)
                            .or_raise(|| message!("The merge index path {path:?} has an invalid tree component"))?;
                    }
                    push_entry(
                        &mut index,
                        path.as_bstr(),
                        Entry {
                            mode: entry.mode,
                            id: entry.oid.to_owned(),
                        },
                        Stage::Unconflicted,
                        validate,
                    )?;
                }
            }
        }
        for (path, state) in &self.paths {
            let entries = match state.unmerged.as_deref().filter(|stages| stages.is_unmerged(how)) {
                Some(stages) => [None, stages.entries[0], stages.entries[1], stages.entries[2]],
                None => [state.entry, None, None, None],
            };
            for (entry, stage) in
                entries
                    .into_iter()
                    .zip([Stage::Unconflicted, Stage::Base, Stage::Ours, Stage::Theirs])
            {
                let Some(entry) = entry else { continue };
                push_entry(&mut index, path.as_bstr(), entry, stage, validate)?;
            }
        }
        index.sort_entries();
        index.verify_entries()?;
        let mut resolved = HashSet::new();
        for entry in index
            .entries()
            .iter()
            .filter(|entry| entry.stage() == Stage::Unconflicted)
        {
            let path = entry.path(&index);
            for (end, byte) in path.iter().enumerate() {
                if *byte == b'/' && resolved.contains(path[..end].as_bstr()) {
                    bail!(message!("The merge index has a stage-zero file above {path:?}").validation());
                }
            }
            resolved.insert(path);
        }
        if let Some(existing) = existing {
            index.copy_metadata_from(existing)?;
        }
        Ok(index)
    }

    /// Return whether the index produced with `how` contains stages 1, 2, or 3.
    /// Forced tree resolutions stay at stage zero even when `how` requests their review.
    pub fn has_unmerged_entries(&self, how: TreatAsUnresolved) -> bool {
        self.paths
            .values()
            .any(|path| path.unmerged.as_deref().is_some_and(|stages| stages.is_unmerged(how)))
    }

    /// Return whether this merge needs attention at the requested sensitivity.
    /// Unlike [`has_unmerged_entries()`](Self::has_unmerged_entries()), this also includes forced tree
    /// resolutions when `how` requests their review. Diagnostic edits cannot change this verdict.
    pub fn has_unresolved_conflicts(&self, how: TreatAsUnresolved) -> bool {
        self.needs_review[0]
            || (how.content_merge == treat_as_unresolved::ContentMerge::ForcedResolution && self.needs_review[1])
            || (how.tree_merge == treat_as_unresolved::TreeMerge::ForcedResolution && self.needs_review[2])
            || self.has_unmerged_entries(how)
    }

    pub(crate) fn new(tree_id: &gix_hash::oid) -> Self {
        Self {
            tree_id: tree_id.to_owned(),
            paths: BTreeMap::new(),
            needs_review: [false; 3],
        }
    }

    pub(crate) fn record(&mut self, path: BString, entry: Option<Entry>, unmerged: Option<Box<Unmerged>>) {
        debug_assert!(
            entry.is_none_or(|entry| entry.mode.is_no_tree()),
            "index plans contain leaves only"
        );
        debug_assert!(
            unmerged
                .as_ref()
                .is_none_or(|stages| stages.entries.iter().flatten().all(|entry| entry.mode.is_no_tree())),
            "index stages contain leaves only"
        );
        self.paths.insert(path, Path { entry, unmerged });
    }

    pub(crate) fn record_resolution(&mut self, resolution: &std::result::Result<Resolution, ResolutionFailure>) {
        for (needs_review, how) in self.needs_review.iter_mut().zip([
            TreatAsUnresolved::git(),
            TreatAsUnresolved {
                content_merge: treat_as_unresolved::ContentMerge::ForcedResolution,
                ..TreatAsUnresolved::git()
            },
            TreatAsUnresolved {
                tree_merge: treat_as_unresolved::TreeMerge::ForcedResolution,
                ..TreatAsUnresolved::git()
            },
        ]) {
            *needs_review |= super::resolution_is_unresolved(resolution, how);
        }
    }
}

fn push_entry(
    index: &mut gix_index::State,
    path: &BStr,
    entry: Entry,
    stage: Stage,
    validate: gix_validate::path::component::Options,
) -> Result {
    if entry.id.is_null() {
        bail!(message!("The merge index path {path:?} has a null object ID").validation());
    }
    let mut components = path.split(|&byte| byte == b'/').peekable();
    while let Some(component) = components.next() {
        let mode = (components.peek().is_none() && entry.mode.is_link())
            .then_some(gix_validate::path::component::Mode::Symlink);
        gix_validate::path::component(component.as_bstr(), mode, validate)
            .or_raise(|| message!("The merge index path {path:?} is invalid"))?;
    }
    index.dangerously_push_entry(Default::default(), entry.id, stage.into(), entry.mode.into(), path);
    Ok(())
}
