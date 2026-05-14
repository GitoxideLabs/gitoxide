//! [Parse][parse()] .mailmap files as used in git repositories and remap names and emails
//! using an [accelerated data-structure][Snapshot].
//!
//! ## Examples
//!
//! ```
//! use gix_actor::SignatureRef;
//!
//! let input = b"
//! Joe R. Developer <joe@example.com> <bugs@example.com>
//! Jane Doe <jane@example.com> Jane <bugs@example.com>
//! ";
//!
//! let parsed = gix_mailmap::parse(input)
//!     .collect::<Result<Vec<_>, _>>()
//!     .unwrap();
//! assert_eq!(parsed.len(), 2);
//!
//! let snapshot = gix_mailmap::Snapshot::new(parsed);
//! let resolved = snapshot.resolve(
//!     SignatureRef::from_bytes(b"Jane <bugs@example.com> 1711398853 +0800").unwrap(),
//! );
//!
//! assert_eq!(resolved.name, "Jane Doe");
//! assert_eq!(resolved.email, "jane@example.com");
//! ```
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

use bstr::BStr;

///
pub mod parse;

/// Parse the given `buf` of bytes line by line into mapping [Entries][Entry].
///
/// Errors may occur per line, but it's up to the caller to stop iteration when
/// one is encountered.
pub fn parse(buf: &[u8]) -> parse::Lines<'_> {
    parse::Lines::new(buf)
}

/// Similar to [parse()], but will skip all lines that didn't parse correctly, silently squelching all errors.
pub fn parse_ignore_errors(buf: &[u8]) -> impl Iterator<Item = Entry<'_>> {
    parse(buf).filter_map(Result::ok)
}

mod entry;

///
pub mod snapshot;

/// A data-structure to efficiently store a list of entries for optimal, case-insensitive lookup by email and
/// optionally name to find mappings to new names and/or emails.
///
/// The memory layout is efficient, even though lots of small allocations are performed to store strings of emails and names.
///
/// ### Handling of invalid `SignatureRef::time`
///
/// As the `time` field in [`SignatureRef`](gix_actor::SignatureRef) as passed by the caller maybe invalid,
/// something that should be very rare but is possible, we decided to not expose this fallibility in the API.
/// Hence, the user may separately check for the correctness of `time`, which we replace with [`gix_date::Time::default()`]
/// in case of parse errors.
#[derive(Default, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Snapshot {
    /// Sorted by `old_email`
    entries_by_old_email: Vec<snapshot::EmailEntry>,
}

/// An typical entry of a mailmap, which always contains an `old_email` by which
/// the mapping is performed to replace the given `new_name` and `new_email`.
///
/// Optionally, `old_name` is also used for lookup.
///
/// Typically created by [parse()].
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Entry<'a> {
    #[cfg_attr(feature = "serde", serde(borrow))]
    /// The name to map to.
    pub(crate) new_name: Option<&'a BStr>,
    /// The email map to.
    pub(crate) new_email: Option<&'a BStr>,
    /// The name to look for and replace.
    pub(crate) old_name: Option<&'a BStr>,
    /// The email to look for and replace.
    pub(crate) old_email: &'a BStr,
}
