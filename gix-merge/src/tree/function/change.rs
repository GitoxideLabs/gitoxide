//! Change discovery and structural matching for tree merges.
//!
//! This module turns each base-to-side tree diff into a [`SideState`]: a flat
//! [`ChangeList`] containing scheduling state, paired with a [`TreeNodes`] path
//! index whose entries point back into that list. It also finds path and rename
//! interactions between sides and classifies the pairs consumed by the resolver.
//!
//! Semantic conflict resolution and edits to the result tree belong to the
//! sibling `resolve` module; this module only prepares, indexes, and matches work.

use std::convert::Infallible;

use bstr::{BString, ByteSlice};
use gix_diff::{tree::recorder::Location, tree_with_rewrites::Change};
use gix_object::FindExt;

use crate::tree::{
    Error,
    utils::{ChangeList, ChangeListRef, PossibleConflict, TreeNodes, track},
};

pub(super) struct SideState {
    changes: ChangeList,
    tree: TreeNodes,
}

/// Lifecycle
impl SideState {
    fn from_changes(changes: ChangeList) -> Self {
        let mut tree = TreeNodes::new();
        for (idx, change) in changes.iter().enumerate() {
            tree.track_change(&change.inner, idx);
        }
        SideState { changes, tree }
    }
}

impl SideState {
    /// Borrow the change schedule and its path index separately for resolution.
    ///
    /// The resolver swaps these pairs between "ours" and "theirs", appends deferred
    /// changes to the list, and updates the corresponding index while processing them.
    /// Every index stored in `TreeNodes` continues to refer into the returned `ChangeList`.
    pub(super) fn parts_mut(&mut self) -> (&mut ChangeList, &mut TreeNodes) {
        (&mut self.changes, &mut self.tree)
    }
    /// Return whether unrelated rewrite destinations can claim the same source identity.
    ///
    /// Rewrites derived from a parent directory rename have a relation and don't represent
    /// ambiguous identity pairing themselves.
    pub(super) fn has_ambiguous_rewrite_sources(&self) -> bool {
        let mut sources = std::collections::HashSet::new();
        self.changes.iter().any(|change| match &change.inner {
            Change::Rewrite {
                source_id,
                relation: None,
                ..
            } => !sources.insert(*source_id),
            _ => false,
        })
    }

    /// Compare both sides by their ordered `(source, destination)` paths.
    ///
    /// When unrelated rewrites share an object ID, whichever side is scheduled first can
    /// otherwise decide how those identities are paired. The resolver uses this ordering
    /// to choose the same first side after the caller reverses ours and theirs. Paths are
    /// used instead of object IDs so the choice remains stable across hash kinds.
    pub(super) fn cmp_for_scheduling(&self, other: &Self) -> std::cmp::Ordering {
        self.changes
            .iter()
            .map(|change| (change.inner.source_location(), change.inner.location()))
            .cmp(
                other
                    .changes
                    .iter()
                    .map(|change| (change.inner.source_location(), change.inner.location())),
            )
    }
}

#[derive(Debug)]
pub(super) enum MatchKind {
    /// A tree is supposed to be superseded by something else.
    EraseTree,
    /// A leaf node is superseded by a tree.
    EraseLeaf,
}

/// Collect one side's changes relative to the base and build their path index.
///
/// `base_buf` contains `base_tree`, while `side_buf` is reused to load `side_tree`.
/// Equal tree IDs produce an empty state without diffing. Otherwise, the path-aware
/// diff, including the requested rewrite tracking, is normalized through [`track`]
/// into a [`ChangeList`] and indexed by [`TreeNodes`] in the returned [`SideState`].
#[expect(clippy::too_many_arguments)]
pub(super) fn collect(
    base_tree: &gix_hash::oid,
    side_tree: &gix_hash::oid,
    base_buf: &[u8],
    side_buf: &mut Vec<u8>,
    objects: &impl gix_object::FindObjectOrHeader,
    diff_resource_cache: &mut gix_diff::blob::Platform,
    diff_state: &mut gix_diff::tree::State,
    rewrites: Option<gix_diff::Rewrites>,
) -> Result<SideState, Error> {
    let mut changes = Vec::new();
    if base_tree != side_tree {
        let side_tree = objects
            .find_tree_iter(side_tree, side_buf)
            .map_err(|err| Error::FindTreeIter(err.into_error()))?;
        gix_diff::tree_with_rewrites(
            gix_object::TreeRefIter::from_bytes(base_buf, base_tree.kind()),
            side_tree,
            diff_resource_cache,
            diff_state,
            objects,
            |change| -> Result<_, Infallible> {
                track(change, &mut changes);
                Ok(std::ops::ControlFlow::Continue(()))
            },
            gix_diff::tree_with_rewrites::Options {
                location: Some(Location::Path),
                rewrites,
            },
        )?;
    }
    Ok(SideState::from_changes(changes))
}

/// Find an eligible change on our indexed side that structurally interacts with `theirs`.
///
/// `rewritten_location` belongs to a deferred change whose path passed through a directory
/// rewrite on our side. For example, if our side renamed `a` to `b`, their change at `a/x`
/// must be retried at `b/x`. The tuple contains that effective path and the index of the
/// directory rewrite which produced it, so matching starts where the change will actually
/// be applied. A rewrite is also checked at its destination, but only for a rewrite on our
/// side, to expose rewrite/rewrite interactions which are not visible at the source path.
///
/// `needs_tree_insertion` identifies a clone appended for a later retry. The resolver inserts
/// such a deferred change into its own side's [`TreeNodes`] immediately before processing it,
/// keeping future work from seeing it prematurely. Its inner index, when present, identifies
/// the opposite-side change which caused the deferral; that candidate is ignored on retry so
/// the same pair cannot defer or resolve each other again.
///
/// Identical changes need no conflict resolution. For a deletion, applying `theirs` removes
/// the same base entry our deletion would remove. An identical rewrite additionally creates
/// the same destination. Marking our deletion or rewrite applied records that one editor
/// update represents both sides and prevents a later scheduling pass from removing the shared
/// source after descendant entries have been added there.
///
/// Applied deletions are otherwise ignored because their indexed path remains available for
/// structural lookup even though the entry itself has already been removed from the editor.
/// A deletion processed without application remains eligible: conflict resolution may have
/// retained the ancestor entry, so it can still block a tree or descendant at that path.
pub(super) fn matching(
    theirs: &Change,
    needs_tree_insertion: Option<Option<usize>>,
    rewritten_location: Option<&(BString, usize)>,
    our_tree: &TreeNodes,
    our_changes: &mut ChangeList,
) -> Option<PossibleConflict> {
    let candidate = our_tree
        .check_conflict(rewritten_location.map_or_else(|| theirs.source_location(), |(location, _)| location.as_bstr()))
        .or_else(|| match theirs {
            Change::Rewrite { location, .. } => our_tree.check_conflict(location.as_bstr()).filter(|candidate| {
                candidate
                    .change_idx()
                    .is_some_and(|idx| matches!(our_changes[idx].inner, Change::Rewrite { .. }))
            }),
            _ => None,
        });

    candidate.filter(|ours| {
        ours.change_idx()
            .zip(needs_tree_insertion.flatten())
            .is_none_or(|(ours_idx, ignore_idx)| ours_idx != ignore_idx)
            && ours.change_idx().is_none_or(|ours_idx| {
                let ours = &mut our_changes[ours_idx];
                if ours.inner == *theirs {
                    // Applying `theirs` also consumes an identical source removal, which must not
                    // run again after descendants have been added.
                    if matches!(theirs, Change::Deletion { .. } | Change::Rewrite { .. }) {
                        ours.mark_applied();
                    }
                    false
                } else {
                    !(ours.was_applied() && matches!(ours.inner, Change::Deletion { .. }))
                }
            })
    })
}

/// Turn a path-index match, `candidate`, from our side into a concrete change pair for the resolution matrix.
///
/// The first tuple element indexes `our_changes`; `None` means the overlap has no eligible change to pair with. The
/// second describes a tree/non-tree boundary: [`MatchKind::EraseTree`] replaces our tree with their leaf, while
/// [`MatchKind::EraseLeaf`] replaces our leaf with their tree. An exact-path match needs no `MatchKind`.
///
/// A tree-to-non-tree candidate is pairable only when its indexed change can remove or replace that tree. A passed
/// rewritten directory is deliberately not paired here because the scheduler relocates it before calling this function.
pub(super) fn pair(candidate: &PossibleConflict, our_changes: &ChangeListRef) -> (Option<usize>, Option<MatchKind>) {
    match *candidate {
        PossibleConflict::TreeToNonTree { change_idx: Some(idx) }
            if matches!(
                our_changes[idx].inner,
                Change::Deletion { .. } | Change::Addition { .. } | Change::Rewrite { .. }
            ) =>
        {
            (Some(idx), Some(MatchKind::EraseTree))
        }
        PossibleConflict::NonTreeToTree { change_idx } => (change_idx, Some(MatchKind::EraseLeaf)),
        PossibleConflict::Match { change_idx } => (Some(change_idx), None),
        _ => (None, None),
    }
}
