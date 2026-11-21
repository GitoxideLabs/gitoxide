use bstr::{BStr, BString, ByteSlice, ByteVec};
use gix_error::{OptionExt, Result, ResultExt, message};
use gix_hash::ObjectId;
use gix_object::tree::{EntryKind, EntryMode};

use crate::{
    blob::{
        self, ResourceKind,
        builtin_driver::text::Labels,
        platform::builtin_merge::{self, Pick},
    },
    tree::{ConflictMapping, ContentMerge, Options, Resolution, ResolutionFailure, ResolveWith},
};

/// Virtual ancestors preserve intermediate entries that a user's ancestor choice would discard.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ResolutionPolicy {
    Merge,
    Favor(ResolveWith),
    VirtualAncestor(Option<ResolveWith>),
}

impl ResolutionPolicy {
    pub fn new(options: &Options) -> Self {
        if options.blob_merge.is_virtual_ancestor {
            Self::VirtualAncestor(options.tree_conflicts)
        } else {
            options.tree_conflicts.map_or(Self::Merge, Self::Favor)
        }
    }

    pub fn tree_resolution(self) -> Option<ResolveWith> {
        match self {
            Self::Merge => None,
            Self::Favor(resolve) => Some(resolve),
            Self::VirtualAncestor(resolve) => resolve,
        }
    }

    pub fn forced_resolution(self) -> Option<ResolveWith> {
        match self {
            Self::Favor(resolve) => Some(resolve),
            Self::Merge | Self::VirtualAncestor(_) => None,
        }
    }

    pub fn is_virtual(self) -> bool {
        matches!(self, Self::VirtualAncestor(_))
    }
}

/// Apply the tree policy to conflict metadata only after selecting which content survives.
pub(super) fn final_resolution(
    resolution: std::result::Result<Resolution, ResolutionFailure>,
    policy: ResolutionPolicy,
) -> std::result::Result<Resolution, ResolutionFailure> {
    match resolution {
        Err(mut failure) if policy.tree_resolution().is_some() => {
            if policy == ResolutionPolicy::Favor(ResolveWith::Ancestor) {
                match &mut failure {
                    ResolutionFailure::ModeConflict { merged_blob }
                    | ResolutionFailure::DirectoryRename { merged_blob, .. }
                    | ResolutionFailure::OursRenamedTheirsRenamedDifferently { merged_blob } => *merged_blob = None,
                    _ => {}
                }
            }
            Ok(Resolution::Forced(failure))
        }
        resolution => resolution,
    }
}

pub(super) struct Outcome {
    pub entry: super::path::Entry,
    pub merged_blob: Option<ContentMerge>,
    pub mode_conflict: bool,
    pub force: Option<ResolveWith>,
    pub pick: Pick,
    trivial: bool,
    restored_ancestor: bool,
    submodule_conflict: Option<ResolutionFailure>,
}

impl Outcome {
    pub fn retained_merge(
        &self,
        entry: super::path::Entry,
        stages: [Option<super::path::Entry>; 3],
    ) -> Option<super::path::EntryMerge> {
        let resolution = if self.restored_ancestor {
            super::path::MergeResolution::Ancestor
        } else if self.submodule_conflict.is_some() {
            super::path::MergeResolution::Submodule
        } else {
            super::path::MergeResolution::Content(
                self.content_merge()
                    .filter(|content| content.resolution != blob::Resolution::Complete)?
                    .resolution,
            )
        };
        Some(super::path::EntryMerge {
            entry,
            resolution,
            forced: self.force.is_some(),
            stages,
        })
    }

    pub fn content_merge(&self) -> Option<ContentMerge> {
        self.merged_blob.filter(|_| !self.trivial && self.force.is_none())
    }

    pub fn content_resolution(&self) -> Option<blob::Resolution> {
        if self.restored_ancestor {
            Some(blob::Resolution::CompleteWithAutoResolvedConflict)
        } else {
            self.merged_blob.map(|content| content.resolution)
        }
    }

    pub fn has_tree_conflict(&self) -> bool {
        self.mode_conflict || self.submodule_conflict.is_some()
    }

    pub fn is_conflict(&self) -> bool {
        self.has_tree_conflict()
            || self
                .merged_blob
                .is_some_and(|content| content.resolution == blob::Resolution::Conflict)
    }

    pub fn resolution(&self) -> Option<std::result::Result<Resolution, ResolutionFailure>> {
        if let Some(conflict) = &self.submodule_conflict {
            Some(Err(conflict.clone()))
        } else if self.restored_ancestor {
            Some(Ok(Resolution::OursModifiedTheirsModifiedThenAncestor))
        } else if self.mode_conflict {
            Some(Err(ResolutionFailure::ModeConflict {
                merged_blob: self.merged_blob.filter(|_| self.force.is_none()),
            }))
        } else {
            self.content_merge()
                .map(|merged_blob| Ok(Resolution::OursModifiedTheirsModifiedThenBlobContentMerge { merged_blob }))
        }
    }
}

/// Merge a compatible entry as a whole, preserving which input a blob merge selected.
#[expect(clippy::too_many_arguments)]
pub(super) fn merge(
    mut labels: Labels<'_>,
    objects: &impl gix_object::FindObjectOrHeader,
    blob_merge: &mut blob::Platform,
    buf: &mut Vec<u8>,
    write_blob_to_odb: &mut impl FnMut(&[u8]) -> Result<ObjectId>,
    ours @ (_, our_id, our_mode): (&BStr, ObjectId, EntryMode),
    theirs @ (_, their_id, their_mode): (&BStr, ObjectId, EntryMode),
    (base_location, base): (&BStr, Option<(ObjectId, EntryMode)>),
    (extra_markers, side, outer_side): (u8, ConflictMapping, ConflictMapping),
    context: &mut crate::tree::Context<'_>,
    options: &Options,
    policy: ResolutionPolicy,
) -> Result<Option<Outcome>> {
    // Directory ancestors are resolved through their children and cannot be selected as leaves.
    let base = base.filter(|(_, mode)| mode.is_no_tree());
    let global_side = side.to_global(outer_side);
    let (current_mode, other_mode) = if global_side.is_swapped() {
        (their_mode, our_mode)
    } else {
        (our_mode, their_mode)
    };
    let Some((mut mode, mode_conflict)) = merge_modes(current_mode, other_mode, base.map(|(_, mode)| mode)) else {
        return Ok(None);
    };
    if side.is_swapped() {
        (labels.current, labels.other) = (labels.other, labels.current);
    }
    let (base_id, base_mode) = base.unwrap_or((our_id.kind().null(), mode));
    let compatible_base =
        merge_modes(our_mode, base_mode, None).is_some() && merge_modes(their_mode, base_mode, None).is_some();
    let trivial = our_id == their_id || our_id == base_id || their_id == base_id;
    if mode.is_commit() && !trivial {
        let (current_commit_id, other_commit_id) = if global_side.is_swapped() {
            (their_id, our_id)
        } else {
            (our_id, their_id)
        };
        if base.is_some_and(|(_, mode)| mode.is_commit())
            && let Some(commit_id) = crate::submodule::merge(
                base_location.as_bstr(),
                &base_id,
                &current_commit_id,
                &other_commit_id,
                context,
            )?
        {
            return Ok(Some(Outcome {
                entry: super::path::Entry { mode, id: commit_id },
                merged_blob: None,
                mode_conflict: false,
                force: None,
                pick: if commit_id == current_commit_id {
                    Pick::Ours
                } else {
                    Pick::Theirs
                },
                trivial: true,
                restored_ancestor: false,
                submodule_conflict: None,
            }));
        }
        return Ok(Some(Outcome {
            entry: super::path::Entry {
                mode,
                id: if global_side.is_swapped() { their_id } else { our_id },
            },
            merged_blob: None,
            mode_conflict: false,
            force: policy.tree_resolution(),
            pick: Pick::Ours,
            trivial: false,
            restored_ancestor: false,
            submodule_conflict: Some(if base.is_some_and(|(_, mode)| mode.is_commit()) {
                ResolutionFailure::SubmoduleMerge
            } else {
                ResolutionFailure::SubmoduleAddAdd
            }),
        }));
    }
    let (merged_blob_id, resolution, pick) = if trivial {
        let take_ours = our_id == their_id || their_id == base_id;
        (
            if take_ours { our_id } else { their_id },
            blob::Resolution::Complete,
            if take_ours != global_side.is_swapped() {
                Pick::Ours
            } else {
                Pick::Theirs
            },
        )
    } else {
        let (merge_base_id, merge_base_mode) = if compatible_base || our_mode.is_link() {
            (base_id, base_mode)
        } else {
            (base_id.kind().null(), mode)
        };
        perform_blob_merge(
            labels,
            objects,
            blob_merge,
            buf,
            write_blob_to_odb,
            ours,
            theirs,
            (base_location, merge_base_id, merge_base_mode),
            (extra_markers, global_side),
            options,
        )?
    };
    let restored_ancestor = our_mode.is_link() && pick == Pick::Ancestor;
    if restored_ancestor {
        mode = base_mode;
    }
    let force = if mode_conflict {
        policy.forced_resolution()
    } else {
        None
    };
    Ok(Some(Outcome {
        entry: super::path::Entry {
            mode,
            id: merged_blob_id,
        },
        merged_blob: if mode.is_commit() {
            None
        } else {
            Some(ContentMerge {
                merged_blob_id,
                resolution,
            })
        },
        mode_conflict,
        force,
        pick,
        trivial,
        restored_ancestor,
        submodule_conflict: None,
    }))
}

/// Merge compatible modes; an executable-bit conflict without a regular-file base retains global ours.
pub(super) fn merge_modes(a: EntryMode, b: EntryMode, previous: Option<EntryMode>) -> Option<(EntryMode, bool)> {
    match (a.kind(), b.kind()) {
        (_, _) if a == b => Some((a, false)),
        (a, b) if a == b => Some((a.into(), false)),
        (EntryKind::BlobExecutable, EntryKind::Blob) | (EntryKind::Blob, EntryKind::BlobExecutable) => {
            if let Some(previous) = previous.filter(EntryMode::is_blob) {
                let changed = if a.kind() == previous.kind() { b } else { a };
                Some((changed.kind().into(), false))
            } else {
                Some((a, true))
            }
        }
        _ => None,
    }
}

/// Merge nontrivial blob contents, retaining the selected input alongside its object ID and resolution.
#[expect(clippy::too_many_arguments)]
fn perform_blob_merge(
    mut labels: crate::blob::builtin_driver::text::Labels<'_>,
    objects: &impl gix_object::FindObjectOrHeader,
    blob_merge: &mut crate::blob::Platform,
    buf: &mut Vec<u8>,
    write_blob_to_odb: &mut impl FnMut(&[u8]) -> Result<ObjectId>,
    (our_location, our_id, our_mode): (&BStr, ObjectId, EntryMode),
    (their_location, their_id, their_mode): (&BStr, ObjectId, EntryMode),
    (previous_location, previous_id, previous_mode): (&BStr, ObjectId, EntryMode),
    (extra_markers, outer_side): (u8, ConflictMapping),
    options: &Options,
) -> Result<(ObjectId, crate::blob::Resolution, builtin_merge::Pick)> {
    use crate::blob::builtin_driver::binary::Pick;
    if our_mode.is_link() && their_mode.is_link() {
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
        return Ok((
            id,
            resolution,
            match pick {
                Pick::Ancestor => builtin_merge::Pick::Ancestor,
                Pick::Ours => builtin_merge::Pick::Ours,
                Pick::Theirs => builtin_merge::Pick::Theirs,
            },
        ));
    }
    let (our_kind, their_kind) = match outer_side {
        ConflictMapping::Original => (ResourceKind::CurrentOrOurs, ResourceKind::OtherOrTheirs),
        ConflictMapping::Swapped => (ResourceKind::OtherOrTheirs, ResourceKind::CurrentOrOurs),
    };
    fn combined(side: &BStr, location: &BStr) -> BString {
        let mut buf = side.to_owned();
        buf.push_byte(b':');
        buf.push_str(location);
        buf
    }

    let (current_location, other_location) = if outer_side.is_swapped() {
        (labels.current, labels.other) = (labels.other, labels.current);
        (their_location, our_location)
    } else {
        (our_location, their_location)
    };

    let (ancestor, current, other);
    let labels = if our_location == their_location {
        labels
    } else {
        ancestor = labels.ancestor.map(|side| combined(side, previous_location));
        current = labels.current.map(|side| combined(side, current_location));
        other = labels.other.map(|side| combined(side, other_location));
        crate::blob::builtin_driver::text::Labels {
            ancestor: ancestor.as_ref().map(|n| n.as_bstr()),
            current: current.as_ref().map(|n| n.as_bstr()),
            other: other.as_ref().map(|n| n.as_bstr()),
        }
    };
    let mut prep = blob_merge.prepare_tree_merge(current_location, objects, options.blob_merge, |cache| {
        cache.set_resource(Some(our_id), our_mode.kind(), our_location, our_kind, objects)?;
        cache.set_resource(Some(their_id), their_mode.kind(), their_location, their_kind, objects)?;
        cache.set_resource(
            Some(previous_id),
            previous_mode.kind(),
            previous_location,
            ResourceKind::CommonAncestorOrBase,
            objects,
        )
    })?;
    if let crate::blob::builtin_driver::text::Conflict::Keep { marker_size, .. } = &mut prep.options.text.conflict {
        *marker_size =
            marker_size.saturating_add(extra_markers.saturating_add(options.marker_size_multiplier.saturating_mul(2)));
    }
    let (mut pick, resolution) = prep.merge(buf, labels, &options.blob_merge_command_ctx)?;
    if options.blob_merge.is_virtual_ancestor && previous_id.is_null() && matches!(pick, builtin_merge::Pick::Ancestor)
    {
        // Like merge-ort, represent an absent binary ancestor as an empty file in a virtual base.
        buf.clear();
        pick = builtin_merge::Pick::Buffer;
    }

    let merged_blob_id = prep
        .id_by_pick(pick, buf, write_blob_to_odb)
        .or_raise(|| message("Failed to write merged blob content as blob to the object database"))?
        .ok_or_raise(|| crate::tree::Error::MissingBinaryMergeResult)?;
    Ok((merged_blob_id, resolution, pick))
}
