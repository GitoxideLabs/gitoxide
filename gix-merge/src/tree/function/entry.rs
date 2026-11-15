use bstr::BString;
use gix_error::Result;
use gix_hash::ObjectId;
use gix_object::tree::{EntryKind, EntryMode};

use crate::{
    blob::{self, builtin_driver::text::Labels, platform::builtin_merge::Pick},
    tree::{
        ConflictMapping, ContentMerge, Options, Resolution, ResolutionFailure, ResolveWith, utils::perform_blob_merge,
    },
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

pub(super) struct Outcome {
    pub mode: EntryMode,
    pub merged_blob: ContentMerge,
    pub mode_conflict: bool,
    pub force: Option<ResolveWith>,
    pub pick: Pick,
    trivial: bool,
    restored_ancestor: bool,
}

impl Outcome {
    pub fn content_merge(&self) -> Option<ContentMerge> {
        (!self.trivial && self.force.is_none()).then_some(self.merged_blob)
    }

    pub fn resolution(&self) -> std::result::Result<Resolution, ResolutionFailure> {
        if self.restored_ancestor {
            Ok(Resolution::OursModifiedTheirsModifiedThenAncestor)
        } else if self.mode_conflict {
            Err(ResolutionFailure::ModeConflict {
                merged_blob: self.force.is_none().then_some(self.merged_blob),
            })
        } else {
            Ok(Resolution::OursModifiedTheirsModifiedThenBlobContentMerge {
                merged_blob: self.merged_blob,
            })
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
    ours @ (_, our_id, our_mode): (&BString, ObjectId, EntryMode),
    theirs @ (_, their_id, their_mode): (&BString, ObjectId, EntryMode),
    (base_location, base): (&BString, Option<(ObjectId, EntryMode)>),
    (extra_markers, side, outer_side): (u8, ConflictMapping, ConflictMapping),
    options: &Options,
    policy: ResolutionPolicy,
) -> Result<Option<Outcome>> {
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
    let restored_ancestor = our_mode.is_link() && base.is_some() && pick == Pick::Ancestor;
    if restored_ancestor {
        mode = base_mode;
    }
    let force = if mode_conflict {
        policy.forced_resolution()
    } else {
        None
    };
    Ok(Some(Outcome {
        mode,
        merged_blob: ContentMerge {
            merged_blob_id,
            resolution,
        },
        mode_conflict,
        force,
        pick,
        trivial,
        restored_ancestor,
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
