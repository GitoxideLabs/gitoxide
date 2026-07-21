use bstr::BStr;

use super::Ref;

///
pub mod parse {
    use bstr::BString;

    /// The error returned when parsing References/refs from the server response.
    // TODO(review): `Io`/`DecodePacketline`/`Id` hand-preserve `#[error(transparent)]` semantics:
    //                `Display` and `source()` forward to the wrapped error.
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        Io(std::io::Error),
        DecodePacketline(gix_transport::packetline::decode::Error),
        Id(gix_hash::decode::Error),
        MalformedSymref { symref: BString },
        MalformedV1RefLine(BString),
        MalformedV2RefLine(BString),
        UnknownAttribute { attribute: BString, line: BString },
        InvariantViolation { message: &'static str },
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Io(err) => std::fmt::Display::fmt(err, f),
                Error::DecodePacketline(err) => std::fmt::Display::fmt(err, f),
                Error::Id(err) => std::fmt::Display::fmt(err, f),
                Error::MalformedSymref { symref } => {
                    write!(
                        f,
                        "{symref:?} could not be parsed. A symref is expected to look like <NAME>:<target>."
                    )
                }
                Error::MalformedV1RefLine(line) => {
                    write!(
                        f,
                        "{line:?} could not be parsed. A V1 ref line should be '<hex-hash> <path>'."
                    )
                }
                Error::MalformedV2RefLine(line) => write!(
                    f,
                    "{line:?} could not be parsed. A V2 ref line should be '<hex-hash> <path>[ (peeled|symref-target):<value>'."
                ),
                Error::UnknownAttribute { attribute, line } => {
                    write!(f, "The ref attribute {attribute:?} is unknown. Found in line {line:?}")
                }
                Error::InvariantViolation { message } => f.write_str(message),
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::Io(err) => err.source(),
                Error::DecodePacketline(err) => err.source(),
                Error::Id(err) => err.source(),
                Error::MalformedSymref { .. }
                | Error::MalformedV1RefLine(_)
                | Error::MalformedV2RefLine(_)
                | Error::UnknownAttribute { .. }
                | Error::InvariantViolation { .. } => None,
            }
        }
    }

    impl From<std::io::Error> for Error {
        fn from(err: std::io::Error) -> Self {
            Error::Io(err)
        }
    }

    impl From<gix_transport::packetline::decode::Error> for Error {
        fn from(err: gix_transport::packetline::decode::Error) -> Self {
            Error::DecodePacketline(err)
        }
    }

    impl From<gix_hash::decode::Error> for Error {
        fn from(err: gix_hash::decode::Error) -> Self {
            Error::Id(err)
        }
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
