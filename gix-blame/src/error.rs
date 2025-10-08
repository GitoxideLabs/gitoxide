use gix_object::bstr::BString;

/// The error returned by [file()](crate::file()).
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("No commit was given")]
    EmptyTraversal,
    #[error(transparent)]
    BlobDiffSetResource(#[from] gix_diff::blob::platform::set_resource::Error),
    #[error(transparent)]
    BlobDiffPrepare(#[from] gix_diff::blob::platform::prepare_diff::Error),
    #[error("The file to blame at '{file_path}' wasn't found in the first commit at {commit_id}")]
    FileMissing {
        /// The file-path to the object to blame.
        file_path: BString,
        /// The commit whose tree didn't contain `file_path`.
        commit_id: gix_hash::ObjectId,
    },
    #[error("Couldn't find commit or tree in the object database")]
    FindObject(#[from] gix_object::find::Error),
    #[error("Could not find existing blob or commit")]
    FindExistingObject(#[from] gix_object::find::existing_object::Error),
    #[error("Could not find existing iterator over a tree")]
    FindExistingIter(#[from] gix_object::find::existing_iter::Error),
    #[error("Failed to obtain the next commit in the commit-graph traversal")]
    Traverse(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error(transparent)]
    DiffTree(#[from] gix_diff::tree::Error),
    #[error(transparent)]
    DiffTreeWithRewrites(#[from] gix_diff::tree_with_rewrites::Error),
    #[error("Invalid line range was given, line range is expected to be a 1-based inclusive range in the format '<start>,<end>'")]
    InvalidOneBasedLineRange,
    #[error("Failure to decode commit during traversal")]
    DecodeCommit(#[from] gix_object::decode::Error),
    #[error("Failed to get parent from commitgraph during traversal")]
    GetParentFromCommitGraph(#[from] gix_commitgraph::file::commit::Error),
}
