//!
#![allow(clippy::empty_docs)]
mod error {

    /// The error returned by [`tag(…)`][crate::Repository::tag()].
    pub type Error = gix_error::Error;
}
pub use error::Error;
