/// The error returned by [`commit()`](crate::commit()).
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Failed to obtain the merge base between the two commits to be merged")]
    MergeBase(#[from] gix_revision::merge_base::Error),
    #[error(transparent)]
    VirtualMergeBase(#[from] virtual_merge_base::Error),
    #[error(transparent)]
    MergeTree(#[from] crate::tree::Error),
    #[error("No common ancestor between {our_commit_id} and {their_commit_id}")]
    NoMergeBase {
        /// The commit on our side that was to be merged.
        our_commit_id: gix_hash::ObjectId,
        /// The commit on their side that was to be merged.
        their_commit_id: gix_hash::ObjectId,
    },
    #[error("Could not find ancestor, our or their commit to extract tree from")]
    FindCommit(#[from] gix_object::find::existing_object::Error),
}

/// A way to configure [`commit()`](crate::commit()).
#[derive(Default, Debug, Clone)]
pub struct Options {
    /// If `true`, merging unrelated commits is allowed, with the merge-base being assumed as empty tree.
    pub allow_missing_merge_base: bool,
    /// Options to define how trees should be merged.
    pub tree_merge: crate::tree::Options,
    /// If `true`, do not merge multiple merge-bases into one. Instead, just use the first one.
    // TODO: test
    #[doc(alias = "no_recursive", alias = "git2")]
    pub use_first_merge_base: bool,
}

/// The result of [`commit()`](crate::commit()).
#[derive(Clone)]
pub struct Outcome<'a> {
    /// The outcome of the actual tree-merge.
    pub tree_merge: crate::tree::Outcome<'a>,
    /// The tree id of the base commit we used. This is either…
    /// * the single merge-base we found
    /// * the first of multiple merge-bases if [`use_first_merge_base`](Options::use_first_merge_base) was `true`.
    /// * the merged tree of all merge-bases, which then isn't linked to an actual commit.
    /// * an empty tree, if [`allow_missing_merge_base`](Options::allow_missing_merge_base) is enabled.
    pub merge_base_tree_id: gix_hash::ObjectId,
    /// The object ids of all the commits which were found to be merge-bases, or `None` if there was no merge-base.
    pub merge_bases: Option<gix_revision::merge_base::Bases>,
    /// A list of virtual commits that were created to merge multiple merge-bases into one, the last one being
    /// the one we used as merge-base for the merge.
    /// As they are not reachable by anything they will be garbage collected, but knowing them provides options.
    /// Would be empty if no virtual commit was needed at all as there was only a single merge-base.
    /// Otherwise, the last commit id is the one with the `merge_base_tree_id`.
    pub virtual_merge_bases: Vec<gix_hash::ObjectId>,
}

pub(super) mod function;

///
pub mod virtual_merge_base;
pub use virtual_merge_base::function::virtual_merge_base;
