pub use gix_mailmap::*;

///
pub mod load {
    /// The error returned by [`crate::Repository::open_mailmap_into()`].
    pub type Error = gix_error::Error;
}
