use bstr::BStr;

use super::Ref;

///
pub mod parse {
    /// The error returned when parsing References/refs from the server response.
    pub type Error = gix_error::Exn;
}

impl Ref {
    /// Provide shared fields referring to the ref itself, namely `(name, target, [peeled])`.
    /// In case of peeled refs, the tag object itself is returned as it is what the ref directly refers to, and target of the tag is returned
    /// as `peeled`.
    /// If `unborn`, the first object id will be the null oid.
    pub fn unpack(&self) -> (&BStr, Option<&gix_hash::oid>, Option<&gix_hash::oid>) {
        match self {
            Ref::Direct { full_ref_name, object } => (full_ref_name.as_ref(), Some(object), None),
            Ref::Symbolic {
                full_ref_name,
                tag,
                object,
                ..
            } => (
                full_ref_name.as_ref(),
                Some(tag.as_deref().unwrap_or(object)),
                tag.as_deref().map(|_| object.as_ref()),
            ),
            Ref::Peeled {
                full_ref_name,
                tag: object,
                object: peeled,
            } => (full_ref_name.as_ref(), Some(object), Some(peeled)),
            Ref::Unborn {
                full_ref_name,
                target: _,
            } => (full_ref_name.as_ref(), None, None),
        }
    }
}

#[cfg(any(feature = "blocking-client", feature = "async-client"))]
pub(crate) mod shared;

#[cfg(feature = "async-client")]
pub(crate) mod async_io;
#[cfg(all(feature = "async-client", not(feature = "blocking-client")))]
pub use async_io::{from_v1_refs_received_as_part_of_handshake_and_capabilities, from_v2_refs};

#[cfg(feature = "blocking-client")]
pub(crate) mod blocking_io;
#[cfg(feature = "blocking-client")]
pub use blocking_io::{from_v1_refs_received_as_part_of_handshake_and_capabilities, from_v2_refs};
