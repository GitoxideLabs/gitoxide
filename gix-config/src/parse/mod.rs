//! This module handles parsing a `git-config` file. Generally speaking, you
//! want to use a higher abstraction such as [`File`] unless you have some
//! explicit reason to work with events instead.
//!
//! The workflow for interacting with this is to use [`Events::from_bytes()`]
//! to obtain borrowed parse event views of the given input.
//!
//! ```compile_fail
//! use gix_config::parse::Bytes;
//! ```
//!
//! On a higher level, one can use [`Events`] to parse all events into a set
//! of easily interpretable data type, similar to what [`File`] does.
//!
//! [`File`]: crate::File

use bstr::{BStr, BString, ByteSlice};

mod from_bytes;

mod event;
#[path = "events.rs"]
mod events_type;
pub(crate) use events_type::FrontMatterEvents;
pub use events_type::{Events, Section};
mod comment;
mod error;
///
pub mod section;

#[cfg(test)]
pub(crate) mod tests;

/// A range into a shared parse buffer.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum Span {
    Range { start: u32, len: u32 },
    Owned(BString),
}

impl Span {
    pub(crate) fn append(backing: &mut Vec<u8>, bytes: &[u8]) -> Self {
        let start = backing.len();
        backing.extend_from_slice(bytes);
        Self::range(start, bytes.len())
    }

    pub(crate) fn range(start: usize, len: usize) -> Self {
        Span::Range {
            start: start
                .try_into()
                .expect("config backing buffers must be smaller than 4 GiB"),
            len: len.try_into().expect("config spans must be smaller than 4 GiB"),
        }
    }

    pub(crate) fn from_slice(backing: &[u8], bytes: &[u8]) -> Self {
        if bytes.is_empty() {
            return Self::range(0, 0);
        }
        debug_assert!(!backing.is_empty());
        let base = backing.as_ptr() as usize;
        let start = (bytes.as_ptr() as usize)
            .checked_sub(base)
            .expect("span must point into the parse buffer");
        let end = start + bytes.len();
        debug_assert!(end <= backing.len());
        Self::range(start, bytes.len())
    }

    /// Return ourselves as byte string slice using `backing` to resolve spans.
    pub fn as_bstr_in<'a>(&'a self, backing: &'a [u8]) -> &'a BStr {
        self.as_slice_in(backing).as_bstr()
    }

    /// Return ourselves as bytes using `backing` to resolve spans.
    pub fn as_slice_in<'a>(&'a self, backing: &'a [u8]) -> &'a [u8] {
        match self {
            Span::Range { start, len } => {
                let start = *start as usize;
                &backing[start..start + *len as usize]
            }
            Span::Owned(bytes) => bytes.as_slice(),
        }
    }

    /// Convert into owned bytes using `backing` to resolve spans.
    pub fn to_bstring_in(&self, backing: &[u8]) -> BString {
        match self {
            Span::Range { .. } => self.as_slice_in(backing).into(),
            Span::Owned(bytes) => bytes.clone(),
        }
    }

    pub(crate) fn copy_to_backing_in(&self, source: &[u8], target: &mut Vec<u8>) -> Self {
        Span::append(target, self.as_slice_in(source))
    }

    /// Move owned bytes into `backing`, turning ourselves into a span.
    pub(crate) fn intern(&mut self, backing: &mut Vec<u8>) {
        if let Span::Owned(bytes) = self {
            *self = Span::append(backing, bytes);
        }
    }

    pub(crate) fn rebase(&mut self, offset: usize) {
        if let Span::Range { start, .. } = self {
            *start = (*start as usize + offset)
                .try_into()
                .expect("config backing buffers must be smaller than 4 GiB");
        }
    }

    /// Return ourselves as byte string slice.
    ///
    /// This only works for owned values. Span-backed values must be accessed
    /// with [`Span::as_bstr_in()`].
    pub fn as_bstr(&self) -> &BStr {
        self.as_slice().as_bstr()
    }

    /// Return ourselves as bytes.
    ///
    /// This only works for owned values. Span-backed values must be accessed
    /// with [`Span::as_slice_in()`].
    pub fn as_slice(&self) -> &[u8] {
        match self {
            Span::Range { .. } => panic!("span-backed bytes need an owning backing buffer"),
            Span::Owned(bytes) => bytes.as_slice(),
        }
    }

    /// Convert owned bytes into a `BString`.
    ///
    /// This only works for owned values. Span-backed values must be accessed
    /// with [`Span::to_bstring_in()`].
    pub fn into_bstring(self) -> BString {
        match self {
            Span::Range { .. } => panic!("span-backed bytes need an owning backing buffer"),
            Span::Owned(bytes) => bytes,
        }
    }
}

impl Default for Span {
    fn default() -> Self {
        Span::Owned(BString::default())
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.as_bstr(), f)
    }
}

impl AsRef<[u8]> for Span {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl AsRef<BStr> for Span {
    fn as_ref(&self) -> &BStr {
        self.as_bstr()
    }
}

impl std::ops::Deref for Span {
    type Target = BStr;

    fn deref(&self) -> &Self::Target {
        self.as_bstr()
    }
}

impl From<BString> for Span {
    fn from(value: BString) -> Self {
        Span::Owned(value)
    }
}

impl From<Vec<u8>> for Span {
    fn from(value: Vec<u8>) -> Self {
        Span::Owned(value.into())
    }
}

impl From<&BStr> for Span {
    fn from(value: &BStr) -> Self {
        Span::Owned(value.into())
    }
}

impl From<&[u8]> for Span {
    fn from(value: &[u8]) -> Self {
        Span::Owned(value.into())
    }
}

impl<const N: usize> From<&[u8; N]> for Span {
    fn from(value: &[u8; N]) -> Self {
        Span::Owned(value.as_slice().into())
    }
}

impl From<&str> for Span {
    fn from(value: &str) -> Self {
        Span::Owned(value.into())
    }
}

impl From<String> for Span {
    fn from(value: String) -> Self {
        Span::Owned(value.into())
    }
}

/// Syntactic events that occurs in the config.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum Event {
    /// A comment with a comment tag and the comment itself. Note that the
    /// comment itself may contain additional whitespace and comment markers
    /// at the beginning, like `# comment` or `; comment`.
    Comment(Comment),
    /// A section header containing the section name and a subsection, if it
    /// exists. For instance, `remote "origin"` is parsed to `remote` as section
    /// name and `origin` as subsection name.
    SectionHeader(section::Header),
    /// A name to a value in a section, like `url` in `remote.origin.url`.
    SectionValueName(section::ValueName),
    /// A completed value. This may be any single-line string, including the empty string
    /// if an implicit boolean value is used.
    /// Note that these values may contain spaces and any special character. This value is
    /// also unprocessed, so it may contain double quotes that should be
    /// [normalized][crate::value::normalize()] before interpretation.
    Value(Span),
    /// Represents any token used to signify a newline character. On Unix
    /// platforms, this is typically just `\n`, but can be any valid newline
    /// *sequence*. Multiple newlines (such as `\n\n`) will be merged as a single
    /// newline event containing a string of multiple newline characters.
    Newline(Span),
    /// Any value that isn't completed. This occurs when the value is continued
    /// onto the next line by ending it with a backslash.
    /// A [`Newline`][Self::Newline] event usually follows, followed by either
    /// `ValueDone`, `Whitespace`, or another `ValueNotDone`. The exception is a
    /// trailing backslash at EOF, which Git accepts as a continuation and which
    /// is represented by `ValueNotDone` followed directly by `ValueDone`.
    ValueNotDone(Span),
    /// The last line of a value which was continued onto another line.
    /// With this it's possible to obtain the complete value by concatenating
    /// the prior [`ValueNotDone`][Self::ValueNotDone] events.
    ValueDone(Span),
    /// A continuous section of insignificant whitespace.
    ///
    /// Note that values with internal whitespace will not be separated by this event,
    /// hence interior whitespace there is always part of the value.
    Whitespace(Span),
    /// This event is emitted when the parser counters a valid `=` character
    /// separating the key and value.
    /// This event is necessary as it eliminates the ambiguity for whitespace
    /// events between a key and value event.
    KeyValueSeparator,
}

/// A backing-aware view of a syntactic event.
///
/// Values in parsed events can be stored as spans into an owning backing buffer.
/// This type resolves these spans into byte string references without allocating.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum EventRef<'a> {
    /// A comment with a comment tag and the comment itself.
    Comment {
        /// The comment marker used.
        tag: u8,
        /// The parsed comment text.
        text: &'a BStr,
    },
    /// A section header with its name and optional subsection details.
    SectionHeader {
        /// The section name.
        name: &'a BStr,
        /// The separator between section and subsection, if any.
        separator: Option<&'a BStr>,
        /// The subsection name, if any.
        subsection_name: Option<&'a BStr>,
    },
    /// A name to a value in a section.
    SectionValueName(&'a BStr),
    /// A completed value.
    Value(&'a BStr),
    /// A newline token.
    Newline(&'a BStr),
    /// An incomplete continued value.
    ValueNotDone(&'a BStr),
    /// The final part of a continued value.
    ValueDone(&'a BStr),
    /// Insignificant whitespace.
    Whitespace(&'a BStr),
    /// A `=` separator between key and value.
    KeyValueSeparator,
}

/// A parsed section containing the header and the section events, typically
/// comprising the keys and their values.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct SectionData {
    /// The section name and subsection name, if any.
    pub(crate) header: section::Header,
    /// The syntactic events found in this section.
    pub(crate) events: Vec<Event>,
}

/// A parsed comment containing the comment marker and comment.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub(crate) struct Comment {
    /// The comment marker used. This is either a semicolon or octothorpe/hash.
    pub(crate) tag: u8,
    /// The parsed comment.
    pub(crate) text: Span,
}

/// A parser error reports the one-indexed line number where the parsing error
/// occurred, as well as the last parser node and the remaining data to be
/// parsed.
#[derive(PartialEq, Debug)]
pub struct Error {
    line_number: usize,
    last_attempted_parser: error::ParseNode,
    parsed_until: bstr::BString,
}
