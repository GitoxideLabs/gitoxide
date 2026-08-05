//! This module handles parsing a `git-config` file. Generally speaking, you
//! want to use a higher abstraction such as [`File`] unless you have some
//! explicit reason to work with events instead.
//!
//! Use [`Events::from_bytes()`] to obtain a self-contained parsed representation,
//! then iterate over its event views.
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
pub use events_type::{Events, SectionRef};
mod comment;
mod error;
///
pub mod section;

#[cfg(test)]
pub(crate) mod tests;

/// A range into a shared backing buffer.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct Span {
    start: u32,
    len: u32,
}

/// Errors produced when a span cannot be represented.
pub mod span {
    /// A span offset or length exceeded the supported 32-bit representation.
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    pub struct Error;

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "configuration data exceeds the supported span size of {} bytes",
                u32::MAX
            )
        }
    }

    impl std::error::Error for Error {}
}

/// A raw span whose semantic value may have required decoding while parsing.
///
/// The raw bytes are retained for lossless serialization. `decoded` is present only when those
/// bytes had to be transformed for semantic access, such as an escaped quoted subsection name.
#[derive(Clone, Debug)]
pub(crate) struct MaybeDecoded {
    raw: Span,
    decoded: Option<BString>,
}

impl MaybeDecoded {
    pub(crate) fn raw(raw: Span) -> Self {
        Self { raw, decoded: None }
    }

    pub(crate) fn decoded(raw: Span, decoded: BString) -> Self {
        Self {
            raw,
            decoded: Some(decoded),
        }
    }

    pub(crate) fn raw_span(&self) -> Span {
        self.raw
    }

    pub(crate) fn value_in<'a>(&'a self, backing: &'a [u8]) -> &'a BStr {
        self.decoded
            .as_ref()
            .map_or_else(|| self.raw.as_bstr_in(backing), |value| value.as_bstr())
    }

    pub(crate) fn rebase(&mut self, offset: usize) -> Result<(), span::Error> {
        self.raw.rebase(offset)
    }

    pub(crate) fn copy_to_backing_in(&self, source: &[u8], target: &mut Vec<u8>) -> Result<Self, span::Error> {
        Ok(Self {
            raw: self.raw.copy_to_backing_in(source, target)?,
            decoded: self.decoded.clone(),
        })
    }
}

impl Span {
    pub(crate) fn append(backing: &mut Vec<u8>, bytes: &[u8]) -> Result<Self, span::Error> {
        let start = backing.len();
        let span = Self::range(start, bytes.len())?;
        backing.len().checked_add(bytes.len()).ok_or(span::Error)?;
        backing.extend_from_slice(bytes);
        Ok(span)
    }

    pub(crate) fn range(start: usize, len: usize) -> Result<Self, span::Error> {
        Ok(Span {
            start: start.try_into().map_err(|_| span::Error)?,
            len: len.try_into().map_err(|_| span::Error)?,
        })
    }

    pub(crate) fn new(backing: &[u8], bytes: &[u8]) -> Self {
        if bytes.is_empty() {
            return Self::default();
        }
        debug_assert!(!backing.is_empty());
        let base = backing.as_ptr() as usize;
        let start = (bytes.as_ptr() as usize)
            .checked_sub(base)
            .expect("span must point into the backing buffer");
        let end = start + bytes.len();
        debug_assert!(end <= backing.len());
        Self::range(start, bytes.len()).expect("the parser rejects backing buffers that exceed the span limit")
    }

    /// Return ourselves as byte string slice using `backing` to resolve spans.
    pub fn as_bstr_in<'a>(&'a self, backing: &'a [u8]) -> &'a BStr {
        self.as_slice_in(backing).as_bstr()
    }

    /// Return ourselves as bytes using `backing` to resolve spans.
    pub fn as_slice_in<'a>(&'a self, backing: &'a [u8]) -> &'a [u8] {
        let start = self.start as usize;
        &backing[start..start + self.len as usize]
    }

    /// Convert into owned bytes using `backing` to resolve spans.
    pub fn to_bstring_in(self, backing: &[u8]) -> BString {
        self.as_slice_in(backing).into()
    }

    pub(crate) fn copy_to_backing_in(&self, source: &[u8], target: &mut Vec<u8>) -> Result<Self, span::Error> {
        Span::append(target, self.as_slice_in(source))
    }

    pub(crate) fn rebase(&mut self, offset: usize) -> Result<(), span::Error> {
        self.start = (self.start as usize)
            .checked_add(offset)
            .and_then(|start| start.try_into().ok())
            .ok_or(span::Error)?;
        Ok(())
    }
}

/// Syntactic events that occurs in the config.
#[derive(Clone, Debug)]
pub(crate) enum Event {
    /// A comment with a comment tag and the comment itself. Note that the
    /// comment itself may contain additional whitespace and comment markers
    /// at the beginning, like `# comment` or `; comment`.
    Comment(Comment),
    /// A section header containing the section name and a subsection, if it
    /// exists. For instance, `remote "origin"` is parsed to `remote` as section
    /// name and `origin` as subsection name.
    SectionHeader(section::HeaderData),
    /// A name to a value in a section, like `url` in `remote.origin.url`.
    SectionValueName(Span),
    /// A completed value. This may be any single-line string, including the empty string
    /// if an implicit boolean value is used.
    /// Note that these values may contain spaces and any special character. This value is
    /// also unprocessed, so it may contain double quotes that should be
    /// [normalized][crate::value::normalize()] before interpretation.
    Value(Span),
    /// One or more consecutive line endings.
    ///
    /// Both `\n` and `\r\n` are accepted, including mixed runs. Multiple line endings, such as
    /// `\n\n`, are merged into a single event whose span contains the entire run.
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

/// A view of a syntactic event in a parsed representation.
///
/// Values in parsed events can be stored as spans into an owning backing buffer.
/// This type exposes their resolved byte-string references.
#[derive(Copy, Clone, Hash, Debug, Eq, PartialEq, PartialOrd, Ord)]
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
    /// One or more consecutive line endings.
    ///
    /// Both `\n` and `\r\n` are accepted, including mixed runs. Multiple line endings, such as
    /// `\n\n`, are merged into a single event whose byte string contains the entire run.
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
#[derive(Clone, Debug)]
pub(crate) struct SectionData {
    /// The section name and subsection name, if any.
    pub(crate) header: section::HeaderData,
    /// The syntactic events found in this section.
    pub(crate) events: Vec<Event>,
}

/// A parsed comment containing the comment marker and comment.
#[derive(Clone, Debug, Default)]
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
    kind: error::Kind,
}
