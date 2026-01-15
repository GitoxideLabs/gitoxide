//! Date and time parsing similar to what git can do.
//!
//! Note that this is not a general purpose time library.
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs, rust_2018_idioms, unsafe_code)]
///
pub mod time;

///
pub mod parse;
pub use parse::function::{parse, parse_header};

pub use gix_error::ParseError as Error;

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
