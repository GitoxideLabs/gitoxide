/// The error returned by [`commit()`](crate::commit()).
// TODO(review): hand-written impls preserve the `thiserror` semantics. `VirtualMergeBase` and
//                `MergeTree` are `#[error(transparent)]`: `Display` and `source()` forward to the
//                wrapped error. The other `#[from]` variants render their own message and expose the
//                wrapped error as `source()`; `NoMergeBase` has no source.
#[derive(Debug)]
#[allow(missing_docs)]
pub enum Error {
    MergeBase(gix_revision::merge_base::Error),
    VirtualMergeBase(virtual_merge_base::Error),
    MergeTree(crate::tree::Error),
    NoMergeBase {
        /// The commit on our side that was to be merged.
        our_commit_id: gix_hash::ObjectId,
        /// The commit on their side that was to be merged.
        their_commit_id: gix_hash::ObjectId,
    },
    FindCommit(gix_object::find::existing_object::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::MergeBase(_) => f.write_str("Failed to obtain the merge base between the two commits to be merged"),
            Error::VirtualMergeBase(err) => std::fmt::Display::fmt(err, f),
            Error::MergeTree(err) => std::fmt::Display::fmt(err, f),
            Error::NoMergeBase {
                our_commit_id,
                their_commit_id,
            } => write!(f, "No common ancestor between {our_commit_id} and {their_commit_id}"),
            Error::FindCommit(_) => f.write_str("Could not find ancestor, our or their commit to extract tree from"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::MergeBase(err) => Some(err),
            Error::VirtualMergeBase(err) => err.source(),
            Error::MergeTree(err) => err.source(),
            Error::FindCommit(err) => Some(err),
            Error::NoMergeBase { .. } => None,
        }
    }
}

impl From<gix_revision::merge_base::Error> for Error {
    fn from(err: gix_revision::merge_base::Error) -> Self {
        Error::MergeBase(err)
    }
}

impl From<virtual_merge_base::Error> for Error {
    fn from(err: virtual_merge_base::Error) -> Self {
        Error::VirtualMergeBase(err)
    }
}

impl From<crate::tree::Error> for Error {
    fn from(err: crate::tree::Error) -> Self {
        Error::MergeTree(err)
    }
}

impl From<gix_object::find::existing_object::Error> for Error {
    fn from(err: gix_object::find::existing_object::Error) -> Self {
        Error::FindCommit(err)
    }
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
    pub merge_bases: Option<nonempty::NonEmpty<gix_hash::ObjectId>>,
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
