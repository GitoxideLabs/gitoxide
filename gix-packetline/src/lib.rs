//! Read and write the git packet line wire format without copying it.
//!
//! ## Examples
//!
//! ```
//! # #[cfg(feature = "blocking-io")]
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use std::io::Write;
//!
//! use gix_packetline::{
//!     blocking_io::{encode, StreamingPeekableIter, Writer},
//!     PacketLineRef,
//! };
//!
//! let mut writer = Writer::new(Vec::new());
//! writer.enable_text_mode();
//! writer.write_all(b"command=ls-refs")?;
//! writer.write_all(b"agent=gitoxide")?;
//! encode::flush_to_write(writer.inner_mut())?;
//!
//! let bytes = writer.into_inner();
//! let mut reader = StreamingPeekableIter::new(&bytes[..], &[PacketLineRef::Flush], false);
//!
//! assert_eq!(
//!     reader.read_line().unwrap()??.as_text().unwrap().as_bstr(),
//!     "command=ls-refs"
//! );
//! assert_eq!(
//!     reader.read_line().unwrap()??.as_text().unwrap().as_bstr(),
//!     "agent=gitoxide"
//! );
//! assert!(reader.read_line().is_none());
//! assert_eq!(reader.stopped_at(), Some(PacketLineRef::Flush));
//! # Ok(()) }
//! # #[cfg(not(feature = "blocking-io"))]
//! # fn main() {}
//! ```
//!
//! ## Feature Flags
#![cfg_attr(
    all(doc, all(doc, feature = "document-features")),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs, unsafe_code)]

use bstr::BStr;

///
#[cfg(feature = "async-io")]
pub mod async_io {
    ///
    pub mod encode;
    mod read;
    pub use read::StreamingPeekableIter;
    mod sidebands;
    pub use sidebands::WithSidebands;
    mod write;
    pub use write::Writer;
}

///
#[cfg(feature = "blocking-io")]
pub mod blocking_io {
    ///
    pub mod encode;
    mod read;
    pub use read::StreamingPeekableIter;
    mod sidebands;
    pub use sidebands::WithSidebands;
    mod write;
    pub use write::Writer;
}

/// Various utilities for `io::Read` trait implementation.
///
/// Only useful in conjunction with the `async-io` and `blocking-io` cargo features.
pub mod read;

const U16_HEX_BYTES: usize = 4;
const MAX_DATA_LEN: usize = 65516;
const MAX_LINE_LEN: usize = MAX_DATA_LEN + U16_HEX_BYTES;
const FLUSH_LINE: &[u8] = b"0000";
const DELIMITER_LINE: &[u8] = b"0001";
const RESPONSE_END_LINE: &[u8] = b"0002";
const ERR_PREFIX: &[u8] = b"ERR ";

/// One of three sideband types allowing to multiplex information over a single connection.
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

impl<'a> PacketLineRef<'a> {
    /// Return this instance as slice if it's [`Data`](PacketLineRef::Data).
    pub fn as_slice(&self) -> Option<&'a [u8]> {
        match self {
            PacketLineRef::Data(d) => Some(d),
            PacketLineRef::Flush | PacketLineRef::Delimiter | PacketLineRef::ResponseEnd => None,
        }
    }
    /// Return this instance's [`as_slice()`](PacketLineRef::as_slice()) as [`BStr`].
    pub fn as_bstr(&self) -> Option<&'a BStr> {
        self.as_slice().map(Into::into)
    }
    /// Interpret this instance's [`as_slice()`](PacketLineRef::as_slice()) as [`ErrorRef`].
    ///
    /// This works for any data received in an error [channel](crate::Channel).
    ///
    /// Note that this creates an unchecked error using the slice verbatim, which is useful to serialize it.
    /// See [`check_error()`](PacketLineRef::check_error()) for a version that assures the error information is in the expected format.
    pub fn as_error(&self) -> Option<ErrorRef<'a>> {
        self.as_slice().map(ErrorRef)
    }
    /// Check this instance's [`as_slice()`](PacketLineRef::as_slice()) is a valid [`ErrorRef`] and return it.
    ///
    /// This works for any data received in an error [channel](crate::Channel).
    pub fn check_error(&self) -> Option<ErrorRef<'a>> {
        self.as_slice().and_then(|data| {
            if data.len() >= ERR_PREFIX.len() && &data[..ERR_PREFIX.len()] == ERR_PREFIX {
                Some(ErrorRef(&data[ERR_PREFIX.len()..]))
            } else {
                None
            }
        })
    }
    /// Return this instance as text, with the trailing newline truncated if present.
    pub fn as_text(&self) -> Option<TextRef<'a>> {
        self.as_slice().map(Into::into)
    }

    /// Interpret the data in this [`slice`](PacketLineRef::as_slice()) as [`BandRef`] according to the given `kind` of channel.
    ///
    /// Note that this is only relevant in a sideband channel.
    /// See [`decode_band()`](PacketLineRef::decode_band()) in case `kind` is unknown.
    pub fn as_band(&self, kind: Channel) -> Option<BandRef<'a>> {
        self.as_slice().map(|d| match kind {
            Channel::Data => BandRef::Data(d),
            Channel::Progress => BandRef::Progress(d),
            Channel::Error => BandRef::Error(d),
        })
    }

    /// Decode the band of this [`slice`](PacketLineRef::as_slice())
    pub fn decode_band(&self) -> Result<BandRef<'a>, decode::band::Error> {
        let d = self.as_slice().ok_or(decode::band::Error::NonDataLine)?;
        Ok(match d[0] {
            1 => BandRef::Data(&d[1..]),
            2 => BandRef::Progress(&d[1..]),
            3 => BandRef::Error(&d[1..]),
            band => return Err(decode::band::Error::InvalidSideBand { band_id: band }),
        })
    }
}

/// A packet line representing an Error in a sideband channel.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ErrorRef<'a>(pub &'a [u8]);

/// A packet line representing text, which may include a trailing newline.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextRef<'a>(pub &'a [u8]);

impl<'a> TextRef<'a> {
    /// Return this instance's data.
    pub fn as_slice(&self) -> &'a [u8] {
        self.0
    }
    /// Return this instance's data as [`BStr`].
    pub fn as_bstr(&self) -> &'a BStr {
        self.0.into()
    }
}

impl<'a> From<&'a [u8]> for TextRef<'a> {
    fn from(d: &'a [u8]) -> Self {
        let d = if d[d.len() - 1] == b'\n' { &d[..d.len() - 1] } else { d };
        TextRef(d)
    }
}

/// A band in a sideband channel.
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

/// Utilities to help decoding packet lines
pub mod decode;
#[doc(inline)]
pub use decode::all_at_once as decode;

/// Utilities to encode different kinds of packet lines
pub mod encode;
