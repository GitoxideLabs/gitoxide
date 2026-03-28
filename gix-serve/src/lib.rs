//! Server-side git protocol support for serving repositories.
#![deny(missing_docs, rust_2018_idioms, unsafe_code)]

///
pub mod pack;
///
pub mod refs;
///
#[cfg(feature = "blocking-server")]
pub mod serve;

use bstr::BString;
use gix_hash::ObjectId;

/// A reference ready for advertisement to clients.
pub struct AdvertisableRef {
    /// The ref name, e.g. `refs/heads/main`.
    pub name: BString,
    /// The object ID the ref points to.
    pub object_id: ObjectId,
    /// The peeled object ID, if this is an annotated tag.
    pub peeled: Option<ObjectId>,
    /// The symref target, e.g. `refs/heads/main` for `HEAD`.
    pub symref_target: Option<BString>,
}

/// Errors from collecting refs.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    RefIter(#[from] gix_ref::packed::buffer::open::Error),
    #[error(transparent)]
    RefIterEntry(#[from] gix_ref::file::iter::loose_then_packed::Error),
    #[error(transparent)]
    RefFind(#[from] gix_ref::file::find::Error),
}
