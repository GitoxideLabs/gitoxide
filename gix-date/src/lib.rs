//! Date and time parsing similar to what git can do.
//!
//! Note that this is not a general purpose time library.
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg, doc_auto_cfg))]
#![deny(missing_docs, rust_2018_idioms)]
#![forbid(unsafe_code)]

///
pub mod time;

///
pub mod parse;
use bstr::{BStr, ByteSlice};
pub use parse::function::parse;
use parse::function::parse_raw;
use parse::Error;
use std::time::SystemTime;

/// A timestamp with timezone.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Time {
    /// The seconds that passed since UNIX epoch. This makes it UTC, or `<seconds>+0000`.
    pub seconds: SecondsSinceUnixEpoch,
    /// The time's offset in seconds, which may be negative to match the `sign` field.
    pub offset: OffsetInSeconds,
    /// the sign of `offset`, used to encode `-0000` which would otherwise lose sign information.
    pub sign: time::Sign,
}

impl Time {
    /// Parse date in config format into a Time
    pub fn from_config(i: &BStr) -> Result<Self, Error> {
        let s = i.as_bstr().to_str().expect("Input must be ascii");
        parse(s, Some(SystemTime::now()))
    }
    /// Parse raw bytes into a Time
    pub fn from_bytes(i: &BStr) -> Result<Self, Error> {
        let s = i.as_bstr().to_str().expect("Input must be ascii");
        parse_raw(s).ok_or(Error::InvalidDateString { input: s.into() })
    }
}

/// The amount of seconds since unix epoch.
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
