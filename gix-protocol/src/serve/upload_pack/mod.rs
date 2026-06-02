///
pub mod want_haves;
pub use want_haves::{parse_haves, parse_wants};

///
pub mod ack;
pub use ack::{write_ack, write_nak, AckStatus};

///
pub mod function;
pub use function::{serve_upload_pack_v1, serve_upload_pack_v2};

/// Errors from serving upload-pack.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Failed to parse client wants/haves")]
    WantHaves(#[from] want_haves::Error),
    #[error("Packetline decode error")]
    PacketlineDecode(#[from] crate::transport::packetline::decode::Error),
    #[error("Unexpected line: {line}")]
    UnexpectedLine { line: bstr::BString },
}
