use std::sync::LazyLock;

use gix_diff::{
    rewrites::tracker::ChangeKind,
    tree::visit::{ChangeId, Relation},
};
use gix_hash::{ObjectId, oid};
use gix_object::tree::{EntryKind, EntryMode};

mod git_baseline;
mod tracker;

#[test]
fn similarity_preserves_git_scores_and_validates_the_range() -> gix_error::TestResult {
    use gix_diff::rewrites::Similarity;

    let half = Similarity::from_score(30_000).expect("half of Git's maximum score is valid");
    assert_eq!(half.fraction(), 0.5, "fractional thresholds remain comparable");
    let partial = Similarity::from_score(32_307).expect("the partial score is in range");
    assert_eq!(partial.score(), 32_307, "integer scores survive without float rounding");
    assert_eq!(partial.percentage(), 53, "Git truncates displayed percentages");
    assert_eq!(
        Similarity::from_score(0),
        Some(Similarity::default()),
        "zero similarity is valid"
    );
    assert_eq!(Similarity::from_score(60_000).expect("100% is valid").percentage(), 100);
    assert!(
        Similarity::from_score(60_001).is_none(),
        "similarity cannot exceed 100%"
    );
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Change {
    id: ObjectId,
    kind: ChangeKind,
    mode: EntryMode,
    relation: Option<Relation>,
}

impl gix_diff::rewrites::tracker::Change for Change {
    fn id(&self) -> Option<&oid> {
        (!self.id.is_null()).then_some(self.id.as_ref())
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
