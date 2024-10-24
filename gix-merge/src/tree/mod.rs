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
    /// Return `true` if there is any conflict that would still need to be resolved as they would yield undesirable trees.
    /// Note that this interpretation of conflicts and their resolution, see
    /// [`has_unresolved_conflicts_strict`](Self::has_unresolved_conflicts_strict).
    pub fn has_unresolved_conflicts(&self) -> bool {
        self.conflicts.iter().any(|c| {
            c.resolution.is_err()
                || c.content_merge().map_or(false, |info| {
                    matches!(info.resolution, crate::blob::Resolution::Conflict)
                })
        })
    }

    /// Return `true` only if there was any (even resolved) conflict in the tree structure, or if there are still conflict markers.
    pub fn has_unresolved_conflicts_strict(&self) -> bool {
        self.conflicts.iter().any(|c| match &c.resolution {
            Ok(success) => match success {
                Resolution::OursAddedTheirsAddedTypeMismatch { .. }
                | Resolution::OursModifiedTheirsRenamedAndChangedThenRename { .. } => true,
                Resolution::OursModifiedTheirsModifiedThenBlobContentMerge { merged_blob } => {
                    matches!(merged_blob.resolution, crate::blob::Resolution::Conflict)
                }
            },
            Err(_failure) => true,
        })
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

/// A utility to help define which side is what.
#[derive(Debug, Clone, Copy)]
enum ConflictMapping {
    Original,
    Swapped,
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

    /// Return information about the content merge if it was performed.
    pub fn content_merge(&self) -> Option<ContentMerge> {
        match &self.resolution {
            Ok(success) => match success {
                Resolution::OursAddedTheirsAddedTypeMismatch { .. } => None,
                Resolution::OursModifiedTheirsRenamedAndChangedThenRename { merged_blob, .. } => *merged_blob,
                Resolution::OursModifiedTheirsModifiedThenBlobContentMerge { merged_blob } => Some(*merged_blob),
            },
            Err(failure) => match failure {
                ResolutionFailure::OursModifiedTheirsDirectoryThenOursRenamed {
                    renamed_path_to_modified_blob: _,
                }
                | ResolutionFailure::OursDeletedTheirsRenamed => None,
            },
        }
    }
}

/// Describes of a conflict involving *our* change and *their* change was specifically resolved.
///
/// Note that all resolutions are side-agnostic, so *ours* could also have been *theirs* and vice versa.
#[derive(Debug, Clone)]
pub enum Resolution {
    /// *ours* was a modified blob and *theirs* renamed that blob.
    /// We moved the changed blob from *ours* to its new location, and merged it successfully.
    /// If this is a `copy`, the source of the copy was set to be the changed blob as well so both match.
    OursModifiedTheirsRenamedAndChangedThenRename {
        /// If not `None`, the content of the involved blob had to be merged.
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
    /// *ours* was added with a different mode than theirs, e.g. blob and symlink, and we kept
    OursAddedTheirsAddedTypeMismatch {
        /// The location at which *their* state was placed to resolve the name and type clash.
        their_final_location: BString,
    },
}

/// Information about a blob content merge for use in a [`Resolution`].
/// Note that content merges always count as success to avoid duplication of cases, which forces callers
/// to check for the [`resolution`](Self::resolution) field.
#[derive(Debug, Copy, Clone)]
pub struct ContentMerge {
    /// The fully merged blob.
    pub merged_blob_id: gix_hash::ObjectId,
    /// Identify the kind of resolution of the blob merge. Note that it may be conflicting.
    pub resolution: crate::blob::Resolution,
}

/// Describes of a conflict involving *our* change and *their* failed to be resolved.
#[derive(Debug, Clone)]
pub enum ResolutionFailure {
    /// *ours* was modified, but *theirs* was turned into a directory, so *ours* was renamed to a non-conflicting path.
    OursModifiedTheirsDirectoryThenOursRenamed {
        /// The path at which `ours` can be found in the tree - it's in the same directory that it was in before.
        renamed_path_to_modified_blob: BString,
    },
    /// *ours* was deleted, but *theirs* was renamed.
    OursDeletedTheirsRenamed,
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
    /// If greater than 0, each level indicates another merge-of-merge. This can be greater than
    /// 0 when merging merge-bases, which are merged like a pyramid.
    /// This value also affects the size of merge-conflict markers, to allow differentiating
    /// merge conflicts on each level.
    pub call_depth: u8,
    // TODO(Perf) add a flag to allow parallelizing the tree-diff itself.
}

pub(super) mod function;
mod utils;
