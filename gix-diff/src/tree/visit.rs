use gix_hash::ObjectId;
use gix_object::{tree, tree::EntryMode};

/// A way to recognize and associate different [`Change`] instances.
///
/// These are unique only within one diff operation.
pub type ChangeId = u32;

/// Identifies a relationship between this instance and another one.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub enum Relation {
    /// This is a parent with the given ID, which will always have at least one child
    /// assuming that empty directories are not allowed in valid trees.
    /// It's also always a tree which is the start of a recursive deletion or addition.
    ///
    /// The change with this relation is always emitted first.
    Parent(ChangeId),
    /// This is a direct or indirect child, tree or not tree, of the parent with the given ID.
    ChildOfParent(ChangeId),
}

/// Represents any possible change in order to turn one tree into another.
#[derive(Debug, Clone, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub enum Change {
    /// An entry was added, like the addition of a file or directory.
    Addition {
        /// The mode of the added entry.
        entry_mode: tree::EntryMode,
        /// The object id of the added entry.
        oid: ObjectId,
        /// Possibly associate this change with another for hierarchical rename tracking.
        relation: Option<Relation>,
    },
    /// An entry was deleted, like the deletion of a file or directory.
    Deletion {
        /// The mode of the deleted entry.
        entry_mode: tree::EntryMode,
        /// The object id of the deleted entry.
        oid: ObjectId,
        /// Possibly associate this change with another for hierarchical rename tracking.
        relation: Option<Relation>,
    },
    /// An entry was modified, e.g. changing the contents of a file adjusts its object id and turning
    /// a file into a symbolic link adjusts its mode.
    Modification {
        /// The mode of the entry before the modification.
        previous_entry_mode: tree::EntryMode,
        /// The object id of the entry before the modification.
        previous_oid: ObjectId,

        /// The mode of the entry after the modification.
        entry_mode: tree::EntryMode,
        /// The object id after the modification.
        oid: ObjectId,
    },
}

impl Change {
    /// Return the current object id.
    pub fn oid(&self) -> &gix_hash::oid {
        match self {
            Change::Addition { oid, .. } | Change::Deletion { oid, .. } | Change::Modification { oid, .. } => oid,
        }
    }
    /// Return the current tree entry mode.
    pub fn entry_mode(&self) -> EntryMode {
        match self {
            Change::Addition { entry_mode, .. }
            | Change::Deletion { entry_mode, .. }
            | Change::Modification { entry_mode, .. } => *entry_mode,
        }
    }
    /// Return the current object id and tree entry mode of a change.
    pub fn oid_and_entry_mode(&self) -> (&gix_hash::oid, EntryMode) {
        match self {
            Change::Addition {
                oid,
                entry_mode,
                relation: _,
            }
            | Change::Deletion {
                oid,
                entry_mode,
                relation: _,
            }
            | Change::Modification { oid, entry_mode, .. } => (oid, *entry_mode),
        }
    }
}

/// What to do after a [Change] was [recorded](super::Visit::visit()).
///
/// Use [`std::ops::ControlFlow::Continue`] to continue the traversal of changes.
/// Use [`std::ops::ControlFlow::Break`] to stop the traversal of changes, making this the last call to [visit(…)](super::Visit::visit()).
pub type Action = std::ops::ControlFlow<()>;

#[cfg(feature = "blob")]
mod change_impls {
    use gix_hash::oid;
    use gix_object::tree::EntryMode;

    use crate::{
        rewrites::tracker::ChangeKind,
        tree::visit::{Change, Relation},
    };

    impl crate::rewrites::tracker::Change for crate::tree::visit::Change {
        fn id(&self) -> &oid {
            match self {
                Change::Addition { oid, .. } | Change::Deletion { oid, .. } | Change::Modification { oid, .. } => oid,
            }
        }

        fn relation(&self) -> Option<Relation> {
            match self {
                Change::Addition { relation, .. } | Change::Deletion { relation, .. } => *relation,
                Change::Modification { .. } => None,
            }
        }

        fn kind(&self) -> ChangeKind {
            match self {
                Change::Addition { .. } => ChangeKind::Addition,
                Change::Deletion { .. } => ChangeKind::Deletion,
                Change::Modification { .. } => ChangeKind::Modification,
            }
        }

        fn entry_mode(&self) -> EntryMode {
            match self {
                Change::Addition { entry_mode, .. }
                | Change::Deletion { entry_mode, .. }
                | Change::Modification { entry_mode, .. } => *entry_mode,
            }
        }

        fn id_and_entry_mode(&self) -> (&oid, EntryMode) {
            match self {
                Change::Addition { entry_mode, oid, .. }
                | Change::Deletion { entry_mode, oid, .. }
                | Change::Modification { entry_mode, oid, .. } => (oid, *entry_mode),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_of_change() {
        let actual = std::mem::size_of::<Change>();
        assert!(
            actual <= 48,
            "{actual} <= 48: this type shouldn't grow without us knowing"
        );
    }
}
