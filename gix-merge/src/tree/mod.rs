use bstr::BString;
use gix_diff::{Rewrites, tree_with_rewrites::Change};
use gix_error::ClassificationMarker;

pub(crate) mod index;
pub use index::IndexPlan;

/// A recoverable failure of [`tree()`](crate::tree()).
/// Other failures retain their original causes in the returned exception.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The selected binary merge resource is absent, such as the ancestor of an add/add conflict.
    /// Classified as [`gix_error::Class::NotFound`].
    MissingBinaryMergeResult,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingBinaryMergeResult => f.write_str(
                "The merge was performed, but the binary merge result couldn't be selected as it wasn't found",
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::MissingBinaryMergeResult => Some(const { &ClassificationMarker::NOT_FOUND }),
        }
    }
}

/// The outcome produced by [`tree()`](crate::tree()).
#[derive(Clone)]
pub struct Outcome<'a> {
    /// The unwritten merge result, including provisional files with conflict markers and paths
    /// chosen to preserve conflicting versions. Use [`Self::index`] for its unmerged stages.
    pub tree: gix_object::tree::Editor<'a>,
    /// The set of conflicts we encountered. Can be empty to indicate there was no conflict.
    /// Note that conflicts might have been auto-resolved, but they are listed here for completeness.
    /// These are diagnostics: editing this list does not change the tree, index, or conflict verdict.
    /// Use [`has_unresolved_conflicts()`](Outcome::has_unresolved_conflicts()) to see if any action is needed
    /// before using [`tree`](Outcome::tree).
    pub conflicts: Vec<Conflict>,
    /// The owned final index state, independent of diagnostic records and subsequent edits to `tree`.
    /// Construct a fresh index with [`IndexPlan::to_index()`].
    pub index: IndexPlan,
    /// `true` if `conflicts` contains only a single [*unresolved* conflict](ResolutionFailure) in the last slot, but
    /// possibly more [resolved ones](Resolution) before that.
    /// This also makes this outcome a very partial merge that cannot be completed.
    /// Only set if [`fail_on_conflict`](Options::fail_on_conflict) is `true`.
    pub failed_on_first_unresolved_conflict: bool,
}

/// Determine what should be considered an unresolved conflict.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TreatAsUnresolved {
    /// Determine which content merges should be considered unresolved.
    pub content_merge: treat_as_unresolved::ContentMerge,
    /// Determine which tree merges should be considered unresolved.
    pub tree_merge: treat_as_unresolved::TreeMerge,
}

///
pub mod treat_as_unresolved {
    use crate::tree::TreatAsUnresolved;

    /// Which kind of content merges should be considered unresolved?
    #[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
    pub enum ContentMerge {
        /// Content merges that still show conflict markers.
        #[default]
        Markers,
        /// Content merges who would have conflicted if it wasn't for a
        /// [resolution strategy](crate::blob::builtin_driver::text::Conflict::ResolveWithOurs).
        ForcedResolution,
    }

    /// Which kind of tree merges should be considered unresolved?
    #[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
    pub enum TreeMerge {
        /// All failed renames.
        Undecidable,
        /// All failed renames, and the ones where a tree item was renamed to avoid a clash.
        #[default]
        EvasiveRenames,
        /// All of `EvasiveRenames`, and tree merges that would have conflicted but which were resolved
        /// with a [resolution strategy](super::ResolveWith).
        ForcedResolution,
    }

    /// Instantiation/Presets
    impl TreatAsUnresolved {
        /// Return an instance with the highest sensitivity to what should be considered unresolved as it
        /// includes entries which have been resolved using a [merge strategy](super::ResolveWith).
        pub fn forced_resolution() -> Self {
            Self {
                content_merge: ContentMerge::ForcedResolution,
                tree_merge: TreeMerge::ForcedResolution,
            }
        }

        /// Return an instance that considers unresolved any conflict that Git would also consider unresolved.
        /// This is the same as the `default()` implementation.
        pub fn git() -> Self {
            Self::default()
        }

        /// Only undecidable tree merges and conflict markers are considered unresolved.
        /// This also means that renamed entries to make space for a conflicting one is considered acceptable,
        /// making this preset the most lenient.
        pub fn undecidable() -> Self {
            Self {
                content_merge: ContentMerge::Markers,
                tree_merge: TreeMerge::Undecidable,
            }
        }
    }
}

impl Outcome<'_> {
    /// Return `true` if there is any conflict that would still need to be resolved as they would yield undesirable trees.
    /// This is based on `how` to determine what should be considered unresolved.
    pub fn has_unresolved_conflicts(&self, how: TreatAsUnresolved) -> bool {
        self.index.has_unresolved_conflicts(how)
    }
}

/// A description of a conflict (i.e. merge issue without an auto-resolution) as seen during a [tree-merge](crate::tree()).
/// They may have a resolution that was applied automatically, or be left for the caller to resolve.
/// Final unmerged entries belong to [`IndexPlan`], independently of these diagnostic records.
#[derive(Debug, Clone)]
pub struct Conflict {
    /// A record on how the conflict resolution succeeded with `Ok(_)` or failed with `Err(_)`.
    /// Note that in case of `Err(_)`, edits may still have been made to the tree to aid resolution.
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
    /// Determine how to interpret the `ours` and `theirs` fields. This is used to implement [`Self::changes_in_resolution()`]
    /// and [`Self::into_parts_by_resolution()`].
    map: ConflictMapping,
}

/// A utility to help define which side is what in the [`Conflict`] type.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ConflictMapping {
    /// The sides are as described in the field documentation, i.e. `ours` is `ours`.
    Original,
    /// The sides are the opposite of the field documentation. i.e. `ours` is `theirs` and `theirs` is `ours`.
    Swapped,
}

impl ConflictMapping {
    fn is_swapped(&self) -> bool {
        matches!(self, ConflictMapping::Swapped)
    }
    fn swapped(self) -> ConflictMapping {
        match self {
            ConflictMapping::Original => ConflictMapping::Swapped,
            ConflictMapping::Swapped => ConflictMapping::Original,
        }
    }
    fn to_global(self, global: ConflictMapping) -> ConflictMapping {
        match global {
            ConflictMapping::Original => self,
            ConflictMapping::Swapped => self.swapped(),
        }
    }
}

impl Conflict {
    /// Return `true` if this instance is considered unresolved based on the criterion specified by `how`.
    pub fn is_unresolved(&self, how: TreatAsUnresolved) -> bool {
        resolution_is_unresolved(&self.resolution, how)
    }

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

    /// Return information about a content merge retained in the result.
    pub fn content_merge(&self) -> Option<ContentMerge> {
        resolution_content_merge(&self.resolution)
    }
}

fn resolution_is_unresolved(resolution: &Result<Resolution, ResolutionFailure>, how: TreatAsUnresolved) -> bool {
    use crate::blob;
    let content_merge_unresolved = |info: &ContentMerge| match how.content_merge {
        treat_as_unresolved::ContentMerge::Markers => matches!(info.resolution, blob::Resolution::Conflict),
        treat_as_unresolved::ContentMerge::ForcedResolution => {
            matches!(
                info.resolution,
                blob::Resolution::Conflict | blob::Resolution::CompleteWithAutoResolvedConflict
            )
        }
    };
    match resolution {
        Err(_) => true,
        Ok(Resolution::OursModifiedTheirsModifiedThenAncestor) => {
            how.content_merge == treat_as_unresolved::ContentMerge::ForcedResolution
        }
        Ok(Resolution::Forced(_)) if how.tree_merge == treat_as_unresolved::TreeMerge::ForcedResolution => true,
        Ok(_) => resolution_content_merge(resolution).is_some_and(|info| content_merge_unresolved(&info)),
    }
}

fn resolution_content_merge(resolution: &Result<Resolution, ResolutionFailure>) -> Option<ContentMerge> {
    fn failure_merged_blob(failure: &ResolutionFailure) -> Option<ContentMerge> {
        match failure {
            ResolutionFailure::ModeConflict { merged_blob }
            | ResolutionFailure::OursRenamedTheirsRenamedDifferently { merged_blob }
            | ResolutionFailure::DirectoryRename { merged_blob, .. } => *merged_blob,
            ResolutionFailure::Unknown
            | ResolutionFailure::SubmoduleMerge
            | ResolutionFailure::SubmoduleAddAdd
            | ResolutionFailure::OursRenamedTheirsRenamedToSameLocation
            | ResolutionFailure::OursDirectoryTheirsNonDirectoryTheirsRenamed { .. }
            | ResolutionFailure::OursModifiedTheirsDeleted
            | ResolutionFailure::OursModifiedTheirsRenamedTypeMismatch
            | ResolutionFailure::OursModifiedTheirsDirectoryThenOursRenamed {
                renamed_unique_path_to_modified_blob: _,
            }
            | ResolutionFailure::OursAddedTheirsAddedTypeMismatch { .. }
            | ResolutionFailure::DistinctTypes { .. }
            | ResolutionFailure::OursDeletedTheirsRenamed => None,
        }
    }
    match resolution {
        Ok(success) => match success {
            Resolution::Forced(failure) => failure_merged_blob(failure),
            Resolution::SourceLocationAffectedByRename { .. } | Resolution::OursModifiedTheirsModifiedThenAncestor => {
                None
            }
            Resolution::OursModifiedTheirsRenamedAndChangedThenRename { merged_blob, .. } => *merged_blob,
            Resolution::OursModifiedTheirsModifiedThenBlobContentMerge { merged_blob } => Some(*merged_blob),
        },
        Err(failure) => failure_merged_blob(failure),
    }
}

/// Describes of a conflict involving *our* change and *their* change was specifically resolved.
///
/// Note that all resolutions are side-agnostic, so *ours* could also have been *theirs* and vice versa.
/// Also note that symlink merges are always done via binary merge, using the same logic.
#[derive(Debug, Clone)]
pub enum Resolution {
    /// *ours* had a renamed directory and *theirs* made a change in the now renamed directory.
    /// We moved that change into its location.
    SourceLocationAffectedByRename {
        /// The repository-relative path to the location that the change ended up in after
        /// being affected by a renamed directory.
        final_location: BString,
    },
    /// *ours* was a modified blob and *theirs* renamed that blob.
    /// We moved the changed blob from *ours* to its new location, and merged it successfully.
    /// If this is a `copy`, the source of the copy was set to be the changed blob as well so both match.
    OursModifiedTheirsRenamedAndChangedThenRename {
        /// The merged mode, retaining an executable-bit change from either side relative to the ancestor.
        merged_mode: Option<gix_object::tree::EntryMode>,
        /// If `Some(…)`, the content of the involved blob had to be merged.
        merged_blob: Option<ContentMerge>,
        /// The repository relative path to the location the blob finally ended up in.
        /// It's `Some()` only if *they* rewrote the blob into a directory which *we* renamed on *our* side.
        final_location: Option<BString>,
    },
    /// *ours* and *theirs* carried changes and where content-merged.
    ///
    /// Note that *ours* and *theirs* may also be rewrites with the same destination and mode,
    /// or additions.
    OursModifiedTheirsModifiedThenBlobContentMerge {
        /// The outcome of the content merge.
        merged_blob: ContentMerge,
    },
    /// Both sides replaced the ancestor with conflicting symlinks, and the
    /// [symlink conflict policy](Options::symlink_conflicts) restored the whole ancestor entry.
    /// The selected ancestor need not be a blob; its mode and object ID are preserved in the result.
    /// If the ancestor is absent, the entry is removed instead.
    /// This counts as a forced content resolution for [`Conflict::is_unresolved()`].
    OursModifiedTheirsModifiedThenAncestor,
    /// This is a resolution failure was forcefully turned into a usable resolution, i.e. [making a choice](ResolveWith)
    /// is turned into a valid resolution.
    Forced(ResolutionFailure),
}

/// Describes of a conflict involving *our* change and *their* failed to be resolved.
#[derive(Debug, Clone)]
pub enum ResolutionFailure {
    /// Regular files have differing executable bits and no shared regular-file ancestor to resolve them.
    /// The provisional tree uses the current side's mode, independently of the content merge.
    ModeConflict {
        /// The content merge, which may be clean even though the modes conflict.
        /// Absent when a forced tree resolution selected a whole entry instead.
        merged_blob: Option<ContentMerge>,
    },
    /// A new path was relocated through a directory renamed on the other side, but its location needs confirmation.
    DirectoryRename {
        /// The suggested repository-relative path, used in the provisional tree and unmerged index.
        final_location: BString,
        /// The content merge, if the relocated path also met a change at its destination.
        merged_blob: Option<ContentMerge>,
    },
    /// Both sides produced different non-tree types at one path. Both versions are retained,
    /// moving regular files aside, or moving both entries if neither is a regular file.
    DistinctTypes {
        /// The resulting location of our entry.
        our_location: BString,
        /// The resulting location of their entry.
        their_location: BString,
    },
    /// *ours* and *theirs* produced conflicting commits for a submodule, but submodule history isn't available to this
    /// tree merge to determine whether either commit contains the other.
    SubmoduleMerge,
    /// *ours* and *theirs* added different submodule commits at the same path, possibly by renaming one into place.
    /// There is no common gitlink at this path, so this is not a candidate for three-way reachability resolution.
    SubmoduleAddAdd,
    /// *ours* and *theirs* renamed different source entries to the same destination.
    OursRenamedTheirsRenamedToSameLocation,
    /// *ours* was renamed, but *theirs* was renamed differently. Both versions will be present in the tree,
    OursRenamedTheirsRenamedDifferently {
        /// If `Some(…)`, the content of the involved blob had to be merged.
        merged_blob: Option<ContentMerge>,
    },
    /// *ours* was modified, but *theirs* was turned into a directory, so *ours* was renamed to a non-conflicting path.
    OursModifiedTheirsDirectoryThenOursRenamed {
        /// The path at which `ours` can be found in the tree - it's in the same directory that it was in before.
        renamed_unique_path_to_modified_blob: BString,
    },
    /// *ours* is a directory, but *theirs* is a non-directory (i.e. file), which wants to be in its place, even though
    /// *ours* has a modification in that subtree.
    /// Rename *theirs* to retain that modification.
    ///
    /// Important: there is no actual modification on *ours* side, so *ours* is filled in with *theirs* as the data structure
    /// cannot represent this case.
    // TODO: Can we have a better data-structure? This would be for a rewrite though.
    OursDirectoryTheirsNonDirectoryTheirsRenamed {
        /// The non-conflicting path of *their* non-tree entry.
        renamed_unique_path_of_theirs: BString,
    },
    /// *ours* was added (or renamed into place) with a different mode than theirs, e.g. blob and symlink, and we kept
    /// the symlink in its original location, renaming the other side to `their_unique_location`.
    OursAddedTheirsAddedTypeMismatch {
        /// The location at which *their* state was placed to resolve the name and type clash, named to indicate
        /// where the entry is coming from.
        their_unique_location: BString,
    },
    /// *ours* was modified, and they renamed the same file, but there is also a non-mergable type-change.
    /// Here we keep both versions of the file.
    OursModifiedTheirsRenamedTypeMismatch,
    /// *ours* was deleted, but *theirs* was renamed.
    OursDeletedTheirsRenamed,
    /// *ours* was modified and *theirs* was deleted. We keep the modified one and ignore the deletion.
    OursModifiedTheirsDeleted,
    /// *ours* and *theirs* are in an untested state so it can't be handled yet, and is considered a conflict
    /// without adding our *or* their side to the resulting tree.
    Unknown,
}

/// Information about a blob content merge for use in a [`Resolution`] or [`ResolutionFailure`].
/// Check [`resolution`](Self::resolution) for content conflicts; a clean content merge can still have a tree conflict.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ContentMerge {
    /// The fully merged blob.
    pub merged_blob_id: gix_hash::ObjectId,
    /// Identify the kind of resolution of the blob merge. Note that it may be conflicting.
    pub resolution: crate::blob::Resolution,
}

/// A way to configure [`tree()`](crate::tree()).
#[derive(Default, Debug, Clone)]
pub struct Options {
    /// How to handle new paths inside a directory renamed on the other side, defaulting to [`DirectoryRenames::Conflict`].
    ///
    /// When [`rewrites`](Self::rewrites) is `None`, file-rename detection is disabled, so directory renames cannot
    /// be inferred. Every value of this field then behaves like [`DirectoryRenames::Disabled`]: new paths stay
    /// at their explicit locations, without relocation or file-location conflicts from inferred directory renames.
    /// With rewrite tracking enabled, this field only controls inferred directory relocation; selecting
    /// [`DirectoryRenames::Disabled`] does not disable explicit file-rename detection.
    pub directory_renames: DirectoryRenames,
    /// If *not* `None`, rename tracking will be performed when determining the changes of each side of the merge.
    ///
    /// Note that [empty blobs](Rewrites::track_empty) should not be tracked for best results.
    pub rewrites: Option<Rewrites>,
    /// Decide how blob-merges should be done. This relates to if conflicts can be resolved or not.
    pub blob_merge: crate::blob::platform::merge::Options,
    /// The context to use when invoking merge-drivers.
    pub blob_merge_command_ctx: gix_command::Context,
    /// If `Some(what-is-unresolved)`, the first unresolved conflict will cause the entire merge to stop.
    /// This is useful to see if there is any conflict, without performing the whole operation, something
    /// that can be very relevant during merges that would cause a lot of blob-diffs.
    pub fail_on_conflict: Option<TreatAsUnresolved>,
    /// This value also affects the size of merge-conflict markers, to allow differentiating
    /// merge conflicts on each level, for any value greater than 0, with values `N` causing `N*2`
    /// markers to be added to the configured value.
    ///
    /// This is used automatically when merging merge-bases recursively.
    pub marker_size_multiplier: u8,
    /// If `None`, when symlinks clash *ours* will be chosen and a conflict will occur.
    /// Otherwise, the same logic applies as for the merge of binary resources.
    pub symlink_conflicts: Option<crate::blob::builtin_driver::binary::ResolveWith>,
    /// If `None`, tree irreconcilable tree conflicts will result in [resolution failures](ResolutionFailure).
    /// Otherwise, one can choose a side. Note that it's still possible to determine that auto-resolution happened
    /// despite this choice, which allows carry forward the conflicting information, possibly for later resolution.
    /// If `Some(…)`, irreconcilable conflicts are reconciled by making a choice.
    /// Forced tree decisions remain resolved in [`Outcome::index`], even when
    /// [`TreatAsUnresolved::forced_resolution()`] requests that they be reviewed.
    pub tree_conflicts: Option<ResolveWith>,
}

/// Control inferred relocation of additions and rename destinations through directory renames.
///
/// This corresponds to Git's `merge.directoryRenames` and defaults to [`Conflict`](Self::Conflict), like Git.
/// Tied destination counts leave new paths in place without reporting Git's directory-split conflict.
#[derive(Default, Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum DirectoryRenames {
    /// Leave new paths at their explicit locations, without disabling file-rename detection.
    Disabled,
    /// Relocate new paths, but report a file-location conflict even if their contents merge cleanly.
    #[default]
    Conflict,
    /// Relocate new paths automatically, only reporting other conflicts encountered at the destination.
    Enabled,
}

/// Decide how to resolve tree-related conflicts, but only those that have [no way of being correct](ResolutionFailure).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ResolveWith {
    /// On irreconcilable conflict, choose neither *our* nor *their* state, but keep the common *ancestor* state instead.
    Ancestor,
    /// On irreconcilable conflict, choose *our* side.
    ///
    /// Note that in order to get something equivalent to *theirs*, put *theirs* into the side of *ours*,
    /// swapping the sides essentially.
    Ours,
}

pub(super) mod function;
