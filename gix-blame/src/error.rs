use gix_object::bstr::BString;

/// The error returned by [file()](crate::file()).
// TODO(review): the `BlobDiff*`/`DiffTree*` variants hand-preserve `#[error(transparent)]`
//                semantics (Display and `source()` pass through to the inner error), and
//                `source()` for the `Box<dyn Error>`-carrying `FindObject`/`Traverse` variants
//                yields the boxed error via `&**err` — matching the previous thiserror output.
#[derive(Debug)]
#[expect(missing_docs)]
pub enum Error {
    EmptyTraversal,
    BlobDiffSetResource(gix_diff::blob::platform::set_resource::Error),
    BlobDiffPrepare(gix_diff::blob::platform::prepare_diff::Error),
    FileMissing {
        /// The file-path to the object to blame.
        file_path: BString,
        /// The commit whose tree didn't contain `file_path`.
        commit_id: gix_hash::ObjectId,
    },
    FindObject(gix_object::find::Error),
    FindExistingObject(gix_object::find::existing_object::Error),
    FindExistingIter(gix_object::find::existing_iter::Error),
    Traverse(Box<dyn std::error::Error + Send + Sync>),
    DiffTree(gix_diff::tree::Error),
    DiffTreeWithRewrites(gix_diff::tree_with_rewrites::Error),
    InvalidOneBasedLineRange,
    DecodeCommit(gix_object::decode::Error),
    GetParentFromCommitGraph(gix_error::Message),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::EmptyTraversal => f.write_str("No commit was given"),
            Error::BlobDiffSetResource(err) => std::fmt::Display::fmt(err, f),
            Error::BlobDiffPrepare(err) => std::fmt::Display::fmt(err, f),
            Error::FileMissing { file_path, commit_id } => write!(
                f,
                "The file to blame at '{file_path}' wasn't found in the first commit at {commit_id}"
            ),
            Error::FindObject(_) => f.write_str("Couldn't find commit or tree in the object database"),
            Error::FindExistingObject(_) => f.write_str("Could not find existing blob or commit"),
            Error::FindExistingIter(_) => f.write_str("Could not find existing iterator over a tree"),
            Error::Traverse(_) => f.write_str("Failed to obtain the next commit in the commit-graph traversal"),
            Error::DiffTree(err) => std::fmt::Display::fmt(err, f),
            Error::DiffTreeWithRewrites(err) => std::fmt::Display::fmt(err, f),
            Error::InvalidOneBasedLineRange => f.write_str(
                "Invalid line range was given, line range is expected to be a 1-based inclusive range in the format '<start>,<end>'"
            ),
            Error::DecodeCommit(_) => f.write_str("Failure to decode commit during traversal"),
            Error::GetParentFromCommitGraph(_) => {
                f.write_str("Failed to get parent from commitgraph during traversal")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::EmptyTraversal | Error::FileMissing { .. } | Error::InvalidOneBasedLineRange => None,
            Error::BlobDiffSetResource(err) => err.source(),
            Error::BlobDiffPrepare(err) => err.source(),
            Error::FindObject(err) => Some(&**err),
            Error::FindExistingObject(err) => Some(err),
            Error::FindExistingIter(err) => Some(err),
            Error::Traverse(err) => Some(&**err),
            Error::DiffTree(err) => err.source(),
            Error::DiffTreeWithRewrites(err) => err.source(),
            Error::DecodeCommit(err) => Some(err),
            Error::GetParentFromCommitGraph(err) => Some(err),
        }
    }
}

impl From<gix_diff::blob::platform::set_resource::Error> for Error {
    fn from(err: gix_diff::blob::platform::set_resource::Error) -> Self {
        Error::BlobDiffSetResource(err)
    }
}

impl From<gix_diff::blob::platform::prepare_diff::Error> for Error {
    fn from(err: gix_diff::blob::platform::prepare_diff::Error) -> Self {
        Error::BlobDiffPrepare(err)
    }
}

impl From<gix_object::find::Error> for Error {
    fn from(err: gix_object::find::Error) -> Self {
        Error::FindObject(err)
    }
}

impl From<gix_object::find::existing_object::Error> for Error {
    fn from(err: gix_object::find::existing_object::Error) -> Self {
        Error::FindExistingObject(err)
    }
}

impl From<gix_object::find::existing_iter::Error> for Error {
    fn from(err: gix_object::find::existing_iter::Error) -> Self {
        Error::FindExistingIter(err)
    }
}

impl From<gix_diff::tree::Error> for Error {
    fn from(err: gix_diff::tree::Error) -> Self {
        Error::DiffTree(err)
    }
}

impl From<gix_diff::tree_with_rewrites::Error> for Error {
    fn from(err: gix_diff::tree_with_rewrites::Error) -> Self {
        Error::DiffTreeWithRewrites(err)
    }
}

impl From<gix_object::decode::Error> for Error {
    fn from(err: gix_object::decode::Error) -> Self {
        Error::DecodeCommit(err)
    }
}

impl From<gix_error::Message> for Error {
    fn from(err: gix_error::Message) -> Self {
        Error::GetParentFromCommitGraph(err)
    }
}
