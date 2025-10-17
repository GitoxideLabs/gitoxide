//! Read and write the git packet line wire format without copying it.
//!
//! For reading the packet line format use the [`StreamingPeekableIter`], and for writing the [`Writer`].
//! ## Feature Flags
#![cfg_attr(
    all(doc, all(doc, feature = "document-features")),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg, doc_auto_cfg))]
#![deny(missing_docs, rust_2018_idioms, unsafe_code)]

const U16_HEX_BYTES: usize = 4;
const MAX_DATA_LEN: usize = 65516;
const MAX_LINE_LEN: usize = MAX_DATA_LEN + U16_HEX_BYTES;
const FLUSH_LINE: &[u8] = b"0000";
const DELIMITER_LINE: &[u8] = b"0001";
const RESPONSE_END_LINE: &[u8] = b"0002";
const ERR_PREFIX: &[u8] = b"ERR ";

/// One of three side-band types allowing to multiplex information over a single connection.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Channel {
    /// The usable data itself in any format.
    Data = 1,
    /// Progress information in a user-readable format.
    Progress = 2,
    /// Error information in a user readable format. Receiving it usually terminates the connection.
    Error = 3,
}

mod line;
///
pub mod read;

///
#[cfg(any(feature = "async-io", feature = "blocking-io"))]
mod write;
#[cfg(all(not(feature = "blocking-io"), feature = "async-io"))]
pub use write::async_io::Writer;
#[cfg(feature = "blocking-io")]
pub use write::blocking_io::Writer;

/// A borrowed packet line as it refers to a slice of data by reference.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PacketLineRef<'a> {
    /// A chunk of raw data.
    Data(&'a [u8]),
    /// A flush packet.
    Flush,
    /// A delimiter packet.
    Delimiter,
    /// The end of the response.
    ResponseEnd,
}

/// A packet line representing an Error in a side-band channel.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ErrorRef<'a>(pub &'a [u8]);

/// A packet line representing text, which may include a trailing newline.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextRef<'a>(pub &'a [u8]);

/// A band in a side-band channel.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BandRef<'a> {
    /// A band carrying data.
    Data(&'a [u8]),
    /// A band carrying user readable progress information.
    Progress(&'a [u8]),
    /// A band carrying user readable errors.
    Error(&'a [u8]),
}

/// State for `StreamingPeekableIter` implementations.
pub struct StreamingPeekableIterState<T> {
    read: T,
    peek_buf: Vec<u8>,
    #[cfg(any(feature = "blocking-io", feature = "async-io"))]
    buf: Vec<u8>,
    fail_on_err_lines: bool,
    delimiters: &'static [PacketLineRef<'static>],
    is_done: bool,
    stopped_at: Option<PacketLineRef<'static>>,
    #[cfg_attr(all(not(feature = "async-io"), not(feature = "blocking-io")), allow(dead_code))]
    trace: bool,
}

/// Utilities to help decoding packet lines
pub mod decode;
#[doc(inline)]
pub use decode::all_at_once as decode;
/// Utilities to encode different kinds of packet lines
pub mod encode;
