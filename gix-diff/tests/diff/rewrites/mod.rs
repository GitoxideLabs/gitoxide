use std::sync::LazyLock;

use gix_diff::{
    rewrites::tracker::ChangeKind,
    tree::visit::{ChangeId, Relation},
};
use gix_hash::{ObjectId, oid};
use gix_object::tree::{EntryKind, EntryMode};

mod tracker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Change {
    id: ObjectId,
    kind: ChangeKind,
    mode: EntryMode,
    relation: Option<Relation>,
}

impl gix_diff::rewrites::tracker::Change for Change {
    fn id(&self) -> &oid {
        &self.id
    }

    fn relation(&self) -> Option<Relation> {
        self.relation
    }

    fn kind(&self) -> ChangeKind {
        self.kind
    }

    fn entry_mode(&self) -> EntryMode {
        self.mode
    }

    fn id_and_entry_mode(&self) -> (&oid, EntryMode) {
        (&self.id, self.mode)
    }
}

static ID: LazyLock<gix_hash::ObjectId> = LazyLock::new(|| {
    gix_object::compute_hash(crate::fixture_hash_kind(), gix_object::Kind::Blob, b"identity").expect("valid blob")
});

impl Change {
    fn modification() -> Self {
        Change {
            id: *ID,
            kind: ChangeKind::Modification,
            mode: EntryKind::Blob.into(),
            relation: None,
        }
    }
    fn deletion() -> Self {
        Change {
            id: *ID,
            kind: ChangeKind::Deletion,
            mode: EntryKind::Blob.into(),
            relation: None,
        }
    }
    fn addition() -> Self {
        Change {
            id: *ID,
            kind: ChangeKind::Addition,
            mode: EntryKind::Blob.into(),
            relation: None,
        }
    }

    fn addition_in_tree(id: ChangeId) -> Self {
        Change {
            id: *ID,
            kind: ChangeKind::Addition,
            mode: EntryKind::Blob.into(),
            relation: Some(Relation::ChildOfParent(id)),
        }
    }

    fn deletion_in_tree(id: ChangeId) -> Self {
        Change {
            id: *ID,
            kind: ChangeKind::Deletion,
            mode: EntryKind::Blob.into(),
            relation: Some(Relation::ChildOfParent(id)),
        }
    }

    fn tree_addition(id: ChangeId) -> Self {
        Change {
            id: *ID,
            kind: ChangeKind::Addition,
            mode: EntryKind::Tree.into(),
            relation: Some(Relation::Parent(id)),
        }
    }

    fn tree_deletion(id: ChangeId) -> Self {
        Change {
            id: *ID,
            kind: ChangeKind::Deletion,
            mode: EntryKind::Tree.into(),
            relation: Some(Relation::Parent(id)),
        }
    }
}
