//!
#![allow(clippy::empty_docs)]
mod error {

    /// The error returned by [`tag(…)`][crate::Repository::tag()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        ReferenceNameValidation(#[from] gix_ref::name::Error),
        #[error(transparent)]
        WriteObject(#[from] crate::object::write::Error),
        #[error(transparent)]
        ReferenceEdit(#[from] crate::reference::edit::Error),
        #[error(transparent)]
        DateParseError(#[from] gix_date::Error),
    }
}
pub use error::Error;
