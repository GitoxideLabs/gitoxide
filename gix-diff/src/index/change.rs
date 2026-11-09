use std::borrow::Cow;

use bstr::BStr;
use gix_object::tree;

use crate::{
    index::{Change, ChangeRef},
    rewrites,
    rewrites::tracker::ChangeKind,
};

impl ChangeRef<'_, '_> {
    /// Copy everything into an owned version of this instance.
    pub fn into_owned(self) -> Change {
        match self {
            ChangeRef::Addition {
                location,
                index,
                entry_mode,
                id,
            } => ChangeRef::Addition {
                location: Cow::Owned(location.into_owned()),
                index,
                entry_mode,
                id: Cow::Owned(id.into_owned()),
            },
            ChangeRef::Deletion {
                location,
                index,
                entry_mode,
                id,
            } => ChangeRef::Deletion {
                location: Cow::Owned(location.into_owned()),
                index,
                entry_mode,
                id: Cow::Owned(id.into_owned()),
            },
            ChangeRef::Modification {
                location,
                previous_index,
                previous_entry_mode,
                previous_id,
                index,
                entry_mode,
                id,
            } => ChangeRef::Modification {
                location: Cow::Owned(location.into_owned()),
                previous_index,
                previous_entry_mode,
                previous_id: Cow::Owned(previous_id.into_owned()),
                index,
                entry_mode,
                id: Cow::Owned(id.into_owned()),
            },
            ChangeRef::Rewrite {
                source_location,
                source_index,
                source_entry_mode,
                source_id,
                location,
                index,
                entry_mode,
                id,
                copy,
            } => ChangeRef::Rewrite {
                source_location: Cow::Owned(source_location.into_owned()),
                source_index,
                source_entry_mode,
                source_id: Cow::Owned(source_id.into_owned()),
                location: Cow::Owned(location.into_owned()),
                index,
                entry_mode,
                id: Cow::Owned(id.into_owned()),
                copy,
            },
        }
    }
}

impl ChangeRef<'_, '_> {
    /// Return `(location, index, entry_mode, id)` for the entry in `lhs`, before the change.
    ///
    /// This is the removed entry for deletions, the previous entry for modifications, and the
    /// source entry for rewrites. Additions return `None` as they have no entry in `lhs`.
    /// See [`Self::rhs_fields()`] for the entry after the change.
    pub fn lhs_fields(&self) -> Option<(&BStr, usize, gix_index::entry::Mode, &gix_hash::oid)> {
        match self {
            ChangeRef::Addition { .. } => None,
            ChangeRef::Deletion {
                location,
                index,
                entry_mode,
                id,
            }
            | ChangeRef::Modification {
                location,
                previous_index: index,
                previous_entry_mode: entry_mode,
                previous_id: id,
                ..
            }
            | ChangeRef::Rewrite {
                source_location: location,
                source_index: index,
                source_entry_mode: entry_mode,
                source_id: id,
                ..
            } => Some((location.as_ref(), *index, *entry_mode, id)),
        }
    }

    /// Return `(location, index, entry_mode, id)` for the entry in `rhs`, after the change.
    ///
    /// This is the added entry for additions, the new entry for modifications, and the
    /// destination entry for rewrites. Deletions return `None` as they have no entry in `rhs`.
    /// See [`Self::lhs_fields()`] for the entry before the change.
    pub fn rhs_fields(&self) -> Option<(&BStr, usize, gix_index::entry::Mode, &gix_hash::oid)> {
        match self {
            ChangeRef::Deletion { .. } => None,
            ChangeRef::Addition {
                location,
                index,
                entry_mode,
                id,
            }
            | ChangeRef::Modification {
                location,
                index,
                entry_mode,
                id,
                ..
            }
            | ChangeRef::Rewrite {
                location,
                index,
                entry_mode,
                id,
                ..
            } => Some((location.as_ref(), *index, *entry_mode, id)),
        }
    }

    /// Return the `location`, in the case of rewrites referring to the current change.
    pub fn location(&self) -> &BStr {
        match self {
            ChangeRef::Addition { location, .. }
            | ChangeRef::Deletion { location, .. }
            | ChangeRef::Modification { location, .. }
            | ChangeRef::Rewrite { location, .. } => location.as_ref(),
        }
    }

    /// Return the `index`, in the case of rewrites referring to the current change.
    pub fn index(&self) -> usize {
        match self {
            ChangeRef::Addition { index, .. }
            | ChangeRef::Deletion { index, .. }
            | ChangeRef::Modification { index, .. }
            | ChangeRef::Rewrite { index, .. } => *index,
        }
    }

    /// Return the `entry_mode`, in the case of rewrites referring to the current change.
    pub fn entry_mode(&self) -> gix_index::entry::Mode {
        match self {
            ChangeRef::Addition { entry_mode, .. }
            | ChangeRef::Deletion { entry_mode, .. }
            | ChangeRef::Modification { entry_mode, .. }
            | ChangeRef::Rewrite { entry_mode, .. } => *entry_mode,
        }
    }

    /// Return the `id`, in the case of rewrites referring to the current change.
    pub fn id(&self) -> &gix_hash::oid {
        match self {
            ChangeRef::Addition { id, .. }
            | ChangeRef::Deletion { id, .. }
            | ChangeRef::Modification { id, .. }
            | ChangeRef::Rewrite { id, .. } => id,
        }
    }
}

impl rewrites::tracker::Change for ChangeRef<'_, '_> {
    fn id(&self) -> Option<&gix_hash::oid> {
        match self {
            ChangeRef::Addition { id, .. } | ChangeRef::Deletion { id, .. } => (!id.is_null()).then_some(id.as_ref()),
            ChangeRef::Modification { previous_id, .. } => (!previous_id.is_null()).then_some(previous_id.as_ref()),
            ChangeRef::Rewrite { .. } => {
                unreachable!("BUG")
            }
        }
    }

    fn kind(&self) -> ChangeKind {
        match self {
            ChangeRef::Addition { .. } => ChangeKind::Addition,
            ChangeRef::Deletion { .. } => ChangeKind::Deletion,
            ChangeRef::Modification { .. } => ChangeKind::Modification,
            ChangeRef::Rewrite { .. } => {
                unreachable!("BUG: rewrites can't be determined ahead of time")
            }
        }
    }

    fn entry_mode(&self) -> tree::EntryMode {
        let mode = match self {
            ChangeRef::Addition { entry_mode, .. } | ChangeRef::Deletion { entry_mode, .. } => entry_mode,
            ChangeRef::Modification {
                previous_entry_mode, ..
            } => previous_entry_mode,
            ChangeRef::Rewrite { .. } => unreachable!("BUG: rewrites can't be determined ahead of time"),
        };
        mode.to_tree_entry_mode()
            // Default is for the impossible case - just don't let it participate in rename tracking.
            .unwrap_or(tree::EntryKind::Tree.into())
    }
}
