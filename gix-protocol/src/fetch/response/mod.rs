use bstr::BString;
use gix_transport::{client, Protocol};

use crate::command::Feature;
use crate::fetch::Response;

/// The error returned in the [response module][crate::fetch::response].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Failed to read from line reader")]
    Io(#[source] std::io::Error),
    #[error(transparent)]
    UploadPack(#[from] gix_transport::packetline::read::Error),
    #[error(transparent)]
    Transport(#[from] client::Error),
    #[error("Currently we require feature {feature:?}, which is not supported by the server")]
    MissingServerCapability { feature: &'static str },
    #[error("Encountered an unknown line prefix in {line:?}")]
    UnknownLineType { line: String },
    #[error("Unknown or unsupported header: {header:?}")]
    UnknownSectionHeader { header: String },
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        if err.kind() == std::io::ErrorKind::Other {
            match err.into_inner() {
                Some(err) => match err.downcast::<gix_transport::packetline::read::Error>() {
                    Ok(err) => Error::UploadPack(*err),
                    Err(err) => Error::Io(std::io::Error::new(std::io::ErrorKind::Other, err)),
                },
                None => Error::Io(std::io::ErrorKind::Other.into()),
            }
        } else {
            Error::Io(err)
        }
    }
}

impl gix_transport::IsSpuriousError for Error {
    fn is_spurious(&self) -> bool {
        match self {
            Error::Io(err) => err.is_spurious(),
            Error::Transport(err) => err.is_spurious(),
            _ => false,
        }
    }
}

/// An 'ACK' line received from the server.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Acknowledgement {
    /// The contained `id` is in common.
    Common(gix_hash::ObjectId),
    /// The server is ready to receive more lines.
    Ready,
    /// The server isn't ready yet.
    Nak,
}

pub use gix_shallow::Update as ShallowUpdate;

/// A wanted-ref line received from the server.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WantedRef {
    /// The object id of the wanted ref, as seen by the server.
    pub id: gix_hash::ObjectId,
    /// The name of the ref, as requested by the client as a `want-ref` argument.
    pub path: BString,
}

/// Parse a `ShallowUpdate` from a `line` as received to the server.
pub fn shallow_update_from_line(line: &str) -> Result<ShallowUpdate, Error> {
    match line.trim_end().split_once(' ') {
        Some((prefix, id)) => {
            let id = gix_hash::ObjectId::from_hex(id.as_bytes())
                .map_err(|_| Error::UnknownLineType { line: line.to_owned() })?;
            Ok(match prefix {
                "shallow" => ShallowUpdate::Shallow(id),
                "unshallow" => ShallowUpdate::Unshallow(id),
                _ => return Err(Error::UnknownLineType { line: line.to_owned() }),
            })
        }
        None => Err(Error::UnknownLineType { line: line.to_owned() }),
    }
}

impl Acknowledgement {
    /// Parse an `Acknowledgement` from a `line` as received to the server.
    pub fn from_line(line: &str) -> Result<Acknowledgement, Error> {
        let mut tokens = line.trim_end().splitn(3, ' ');
        match (tokens.next(), tokens.next(), tokens.next()) {
            (Some(first), id, description) => Ok(match first {
                "ready" => Acknowledgement::Ready, // V2
                "NAK" => Acknowledgement::Nak,     // V1
                "ACK" => {
                    let id = match id {
                        Some(id) => gix_hash::ObjectId::from_hex(id.as_bytes())
                            .map_err(|_| Error::UnknownLineType { line: line.to_owned() })?,
                        None => return Err(Error::UnknownLineType { line: line.to_owned() }),
                    };
                    if let Some(description) = description {
                        match description {
                            "common" => {}
                            "ready" => return Ok(Acknowledgement::Ready),
                            _ => return Err(Error::UnknownLineType { line: line.to_owned() }),
                        }
                    }
                    Acknowledgement::Common(id)
                }
                _ => return Err(Error::UnknownLineType { line: line.to_owned() }),
            }),
            (None, _, _) => Err(Error::UnknownLineType { line: line.to_owned() }),
        }
    }
    /// Returns the hash of the acknowledged object if this instance acknowledges a common one.
    pub fn id(&self) -> Option<&gix_hash::ObjectId> {
        match self {
            Acknowledgement::Common(id) => Some(id),
            _ => None,
        }
    }
}

impl WantedRef {
    /// Parse a `WantedRef` from a `line` as received from the server.
    pub fn from_line(line: &str) -> Result<WantedRef, Error> {
        match line.trim_end().split_once(' ') {
            Some((id, path)) => {
                let id = gix_hash::ObjectId::from_hex(id.as_bytes())
                    .map_err(|_| Error::UnknownLineType { line: line.to_owned() })?;
                Ok(WantedRef { id, path: path.into() })
            }
            None => Err(Error::UnknownLineType { line: line.to_owned() }),
        }
    }
}

impl Response {
    /// Return true if the response has a pack which can be read next.
    pub fn has_pack(&self) -> bool {
        self.has_pack
    }

    /// Return an error if the given `features` don't contain the required ones (the ones this implementation needs)
    /// for the given `version` of the protocol.
    ///
    /// Even though technically any set of features supported by the server could work, we only implement the ones that
    /// make it easy to maintain all versions with a single code base that aims to be and remain maintainable.
    pub fn check_required_features(version: Protocol, features: &[Feature]) -> Result<(), Error> {
        match version {
            Protocol::V0 | Protocol::V1 => {
                let has = |name: &str| features.iter().any(|f| f.0 == name);
                // Let's focus on V2 standards, and simply not support old servers to keep our code simpler
                if !has("multi_ack_detailed") {
                    return Err(Error::MissingServerCapability {
                        feature: "multi_ack_detailed",
                    });
                }
                // It's easy to NOT do sideband for us, but then again, everyone supports it.
                // CORRECTION: If side-band is off, it would send the packfile without packet line encoding,
                // which is nothing we ever want to deal with (despite it being more efficient). In V2, this
                // is not even an option anymore, sidebands are always present.
                if !has("side-band") && !has("side-band-64k") {
                    return Err(Error::MissingServerCapability {
                        feature: "side-band OR side-band-64k",
                    });
                }
            }
            Protocol::V2 => {}
        }
        Ok(())
    }

    /// Return all acknowledgements [parsed previously][Response::from_line_reader()].
    pub fn acknowledgements(&self) -> &[Acknowledgement] {
        &self.acks
    }

    /// Return all shallow update lines [parsed previously][Response::from_line_reader()].
    pub fn shallow_updates(&self) -> &[ShallowUpdate] {
        &self.shallows
    }

    /// Append the given `updates` which may have been obtained from a
    /// (handshake::Outcome)[crate::handshake::Outcome::v1_shallow_updates].
    ///
    /// In V2, these are received as part of the pack, but V1 sends them early, so we
    /// offer to re-integrate them here.
    pub fn append_v1_shallow_updates(&mut self, updates: Option<Vec<ShallowUpdate>>) {
        self.shallows.extend(updates.into_iter().flatten());
    }

    /// Return all wanted-refs [parsed previously][Response::from_line_reader()].
    pub fn wanted_refs(&self) -> &[WantedRef] {
        &self.wanted_refs
    }
}

#[cfg(any(feature = "async-client", feature = "blocking-client"))]
impl Response {
    /// with a friendly server, we just assume that a non-ack line is a pack line
    /// which is our hint to stop here.
    fn parse_v1_ack_or_shallow_or_assume_pack(
        acks: &mut Vec<Acknowledgement>,
        shallows: &mut Vec<ShallowUpdate>,
        peeked_line: &str,
    ) -> bool {
        match Acknowledgement::from_line(peeked_line) {
            Ok(ack) => match ack.id() {
                Some(id) => {
                    if !acks.iter().any(|a| a.id() == Some(id)) {
                        acks.push(ack);
                    }
                }
                None => acks.push(ack),
            },
            Err(_) => match shallow_update_from_line(peeked_line) {
                Ok(shallow) => {
                    shallows.push(shallow);
                }
                Err(_) => return true,
            },
        }
        false
    }
}

#[cfg(feature = "async-client")]
mod async_io;
#[cfg(feature = "blocking-client")]
mod blocking_io;
