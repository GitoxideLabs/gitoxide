use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum SdkError {
    #[error("Repository not found: {0}")]
    RepoNotFound(PathBuf),

    #[error("Object not found: {0}")]
    ObjectNotFound(gix_hash::ObjectId),

    #[error("Reference not found: {0}")]
    RefNotFound(String),

    #[error("Tree entry not found: {0}")]
    TreeEntryNotFound(String),

    #[error("Invalid object type: expected {expected}, got {actual}")]
    InvalidObjectType { expected: String, actual: String },

    #[error("Invalid revision spec: {0}")]
    InvalidRevision(String),

    #[error("Operation failed: {0}")]
    Operation(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Git(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl From<gix_hash::decode::Error> for SdkError {
    fn from(err: gix_hash::decode::Error) -> Self {
        SdkError::Git(Box::new(err))
    }
}

impl From<gix_object::decode::Error> for SdkError {
    fn from(err: gix_object::decode::Error) -> Self {
        SdkError::Git(Box::new(err))
    }
}

impl From<gix_ref::file::find::Error> for SdkError {
    fn from(err: gix_ref::file::find::Error) -> Self {
        SdkError::Git(Box::new(err))
    }
}

impl From<gix_ref::file::find::existing::Error> for SdkError {
    fn from(err: gix_ref::file::find::existing::Error) -> Self {
        SdkError::Git(Box::new(err))
    }
}

impl From<gix_odb::store::find::Error> for SdkError {
    fn from(err: gix_odb::store::find::Error) -> Self {
        SdkError::Git(Box::new(err))
    }
}

impl From<gix_traverse::commit::simple::Error> for SdkError {
    fn from(err: gix_traverse::commit::simple::Error) -> Self {
        SdkError::Git(Box::new(err))
    }
}

impl From<gix_diff::tree::Error> for SdkError {
    fn from(err: gix_diff::tree::Error) -> Self {
        SdkError::Git(Box::new(err))
    }
}

pub type Result<T> = std::result::Result<T, SdkError>;
