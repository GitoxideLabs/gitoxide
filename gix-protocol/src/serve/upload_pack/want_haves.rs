use std::io::Read;

use crate::transport::packetline::blocking_io::StreamingPeekableIter;
use crate::transport::packetline::PacketLineRef;
use bstr::{BString, ByteSlice};
use gix_hash::ObjectId;

/// A parsed `want` from the client.
#[derive(Debug)]
pub struct Want {
    /// The object ID the client wants.
    pub id: ObjectId,
}

/// The result of parsing client `want` lines.
#[derive(Debug)]
pub struct Wants {
    /// The wanted object IDs.
    pub wants: Vec<Want>,
    /// Capabilities sent on the first `want` line (V1 only).
    pub capabilities: Vec<BString>,
}

/// The result of parsing client `have` lines.
#[derive(Debug)]
pub struct Haves {
    /// The object IDs the client already has.
    pub haves: Vec<ObjectId>,
    /// Whether the client sent `done`.
    pub done: bool,
}

/// Errors from parsing want/have lines.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Invalid object ID: {hex}")]
    InvalidObjectId { hex: BString },
    #[error("Packetline decode error")]
    PacketlineDecode(#[from] crate::transport::packetline::decode::Error),
    #[error("Unexpected line: {line}")]
    UnexpectedLine { line: BString },
}

/// Parse `want` lines from the packetline reader until a flush packet.
///
/// In V1, the first `want` line may include capabilities after the OID.
pub fn parse_wants<R: Read>(reader: &mut StreamingPeekableIter<R>) -> Result<Wants, Error> {
    let mut wants = Vec::new();
    let mut capabilities = Vec::new();
    let mut is_first = true;

    while let Some(line) = reader.read_line() {
        let line = line??;
        match line {
            PacketLineRef::Data(data) => {
                let data = data.trim();
                let rest = data
                    .strip_prefix(b"want ")
                    .ok_or_else(|| Error::UnexpectedLine { line: data.into() })?;

                let (hex, caps_str) = if rest.len() > 40 {
                    (&rest[..40], Some(&rest[41..]))
                } else {
                    (rest, None)
                };

                let id = ObjectId::from_hex(hex).map_err(|_| Error::InvalidObjectId { hex: hex.into() })?;
                wants.push(Want { id });

                if is_first {
                    if let Some(caps) = caps_str {
                        capabilities = caps.split(|b| *b == b' ').map(Into::into).collect();
                    }
                    is_first = false;
                }
            }
            _ => {
                return Err(Error::UnexpectedLine {
                    line: "non-data packet".into(),
                })
            }
        }
    }

    Ok(Wants { wants, capabilities })
}

/// Parse `have` lines from the packetline reader.
///
/// Returns the OIDs and whether `done` was seen.
pub fn parse_haves<R: Read>(reader: &mut StreamingPeekableIter<R>) -> Result<Haves, Error> {
    let mut haves = Vec::new();
    let mut done = false;

    while let Some(line) = reader.read_line() {
        let line = line??;
        match line {
            PacketLineRef::Data(data) => {
                let data = data.trim();
                if data == b"done" {
                    done = true;
                    break;
                }
                let hex = data
                    .strip_prefix(b"have ")
                    .ok_or_else(|| Error::UnexpectedLine { line: data.into() })?;
                let id = ObjectId::from_hex(hex).map_err(|_| Error::InvalidObjectId { hex: hex.into() })?;
                haves.push(id);
            }
            _ => break,
        }
    }

    Ok(Haves { haves, done })
}
