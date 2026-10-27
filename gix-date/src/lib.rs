//! Date and time parsing similar to what git can do.
//!
//! Note that this is not a general purpose time library.
//!
//! Git is the compatibility reference, but correctness takes precedence over reproducing its bugs.
//! Calendar arithmetic and timezone conversion use Jiff's checked civil dates and timezone rules:
//! March 31 minus one month stays in February, rather than rolling back into March as Git does,
//! and `last Thursday` selects an actual Thursday after a month adjustment. Local clocks use the
//! target date's DST rules, so `today` remains midnight instead of becoming 23:00 on the previous
//! day at a spring transition. Explicit offsets remain authoritative: Jiff preserves the seconds
//! in `+01:02:03`, and `-0001` means minus one minute instead of Git's unset-zone sentinel.
//! Invalid calendar dates are rejected instead of silently normalized into a different month.
//! These intentional differences are documented and tested against Git alongside compatible cases;
//! [`parse()`] describes the supported syntax and its limitations.
//!
//! ## Examples
//!
//! ```
//! # fn main() -> gix_error::TestResult {
//! use gix_date::{
//!     parse,
//!     parse_header,
//!     time::{format, Format},
//! };
//!
//! let time = parse("Thu, 18 Aug 2022 12:45:06 +0800", None)?;
//! assert_eq!(time.offset, 8 * 60 * 60);
//! assert_eq!(time.format(Format::Raw)?, "1660797906 +0800");
//! assert_eq!(time.format(Format::Custom(format::ISO8601))?, "2022-08-18 12:45:06 +0800");
//!
//! let from_header = parse_header("1660797906 +0800").expect("a valid raw Git timestamp");
//! assert_eq!(from_header, time);
//! # Ok(())
//! # }
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
pub use jiff::Zoned;
pub use parse::function::{parse, parse_header};

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
