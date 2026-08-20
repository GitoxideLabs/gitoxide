use bstr::BStr;

use super::Ref;

///
pub mod parse {
    use bstr::BString;

    /// The error returned when parsing References/refs from the server response.
    #[derive(Debug, thiserror::Error)]
    #[expect(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        Io(#[from] std::io::Error),
        #[error(transparent)]
        DecodePacketline(gix_error::Error),
        #[error(transparent)]
        Id(#[from] gix_hash::decode::Error),
        #[error("{symref:?} could not be parsed. A symref is expected to look like <NAME>:<target>.")]
        MalformedSymref { symref: BString },
        #[error("{0:?} could not be parsed. A V1 ref line should be '<hex-hash> <path>'.")]
        MalformedV1RefLine(BString),
        #[error(
            "{0:?} could not be parsed. A V2 ref line should be '<hex-hash> <path>[ (peeled|symref-target):<value>'."
        )]
        MalformedV2RefLine(BString),
        #[error("The ref attribute {attribute:?} is unknown. Found in line {line:?}")]
        UnknownAttribute { attribute: BString, line: BString },
        #[error("{message}")]
        InvariantViolation { message: &'static str },
    }
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
