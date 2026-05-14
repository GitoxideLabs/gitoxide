//! Date and time parsing similar to what git can do.
//!
//! Note that this is not a general purpose time library.
//!
//! ## Examples
//!
//! ```
//! use gix_date::{
//!     parse,
//!     parse_header,
//!     time::{format, Format},
//! };
//!
//! let time = parse("Thu, 18 Aug 2022 12:45:06 +0800", None).unwrap();
//! assert_eq!(time.offset, 8 * 60 * 60);
//! assert_eq!(time.format(Format::Raw).unwrap(), "1660797906 +0800");
//! assert_eq!(time.format(Format::Custom(format::ISO8601)).unwrap(), "2022-08-18 12:45:06 +0800");
//!
//! let from_header = parse_header("1660797906 +0800").unwrap();
//! assert_eq!(from_header, time);
//! ```
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs, unsafe_code)]
///
pub mod time;

///
pub mod parse;
pub use parse::function::{parse, parse_header};

pub use gix_error::ValidationError as Error;

/// A timestamp with timezone.
#[derive(Default, PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Time {
    /// The seconds that have passed since UNIX epoch. This makes it UTC, or `<seconds>+0000`.
    pub seconds: SecondsSinceUnixEpoch,
    /// The time's offset in seconds, which may be negative to match the `sign` field.
    pub offset: OffsetInSeconds,
}

/// The number of seconds since unix epoch.
///
/// Note that negative dates represent times before the unix epoch.
///
/// ### Deviation
///
/// `git` only supports dates *from* the UNIX epoch, whereas we chose to be more flexible at the expense of stopping time
/// a few million years before the heat-death of the universe.
pub type SecondsSinceUnixEpoch = i64;
/// time offset in seconds.
pub type OffsetInSeconds = i32;
