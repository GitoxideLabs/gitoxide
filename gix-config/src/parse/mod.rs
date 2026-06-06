//! This module handles parsing a `git-config` file. Generally speaking, you
//! want to use a higher abstraction such as [`File`] unless you have some
//! explicit reason to work with events instead.
//!
//! The workflow for interacting with this is to use
//! [`from_bytes()`] to obtain all parse events or tokens of the given input.
//!
//! On a higher level, one can use [`Events`] to parse all events into a set
//! of easily interpretable data type, similar to what [`File`] does.
//!
//! [`File`]: crate::File

use std::{borrow::Cow, hash::Hash};

use bstr::BStr;

mod from_bytes;
pub use self::from_bytes::from_bytes;

mod event;
#[path = "events.rs"]
mod events_type;
pub use events_type::{Events, FrontMatterEvents};
mod comment;
mod error;
///
pub mod format;
///
pub mod section;

#[cfg(test)]
pub(crate) mod tests;

/// Syntactic events that occurs in the config. Despite all these variants
/// holding a [`Cow`] instead over a simple reference, the parser will only emit
/// borrowed `Cow` variants.
///
/// The `Cow` is used here for ease of inserting new, typically owned events as used
/// in the [`File`] struct when adding values, allowing a mix of owned and borrowed
/// values.
///
/// [`Cow`]: std::borrow::Cow
/// [`File`]: crate::File
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Event<'a> {
    /// A comment with a comment tag and the comment itself. Note that the
    /// comment itself may contain additional whitespace and comment markers
    /// at the beginning, like `# comment` or `; comment`.
    Comment(Comment<'a>),
    /// A section header containing the section name and a subsection, if it
    /// exists. For instance, `remote "origin"` is parsed to `remote` as section
    /// name and `origin` as subsection name.
    SectionHeader(section::Header<'a>),
    /// A name to a value in a section, like `url` in `remote.origin.url`.
    SectionValueName(section::ValueName<'a>),
    /// A completed value. This may be any single-line string, including the empty string
    /// if an implicit boolean value is used.
    /// Note that these values may contain spaces and any special character. This value is
    /// also unprocessed, so it may contain double quotes that should be
    /// [normalized][crate::value::normalize()] before interpretation.
    Value(Cow<'a, BStr>),
    /// Represents any token used to signify a newline character. On Unix
    /// platforms, this is typically just `\n`, but can be any valid newline
    /// *sequence*. Multiple newlines (such as `\n\n`) will be merged as a single
    /// newline event containing a string of multiple newline characters.
    Newline(Cow<'a, BStr>),
    /// Any value that isn't completed. This occurs when the value is continued
    /// onto the next line by ending it with a backslash.
    /// A [`Newline`][Self::Newline] event usually follows, followed by either
    /// `ValueDone`, `Whitespace`, or another `ValueNotDone`. The exception is a
    /// trailing backslash at EOF, which Git accepts as a continuation and which
    /// is represented by `ValueNotDone` followed directly by `ValueDone`.
    ValueNotDone(Cow<'a, BStr>),
    /// The last line of a value which was continued onto another line.
    /// With this it's possible to obtain the complete value by concatenating
    /// the prior [`ValueNotDone`][Self::ValueNotDone] events.
    ValueDone(Cow<'a, BStr>),
    /// A continuous section of insignificant whitespace.
    ///
    /// Note that values with internal whitespace will not be separated by this event,
    /// hence interior whitespace there is always part of the value.
    Whitespace(Cow<'a, BStr>),
    /// This event is emitted when the parser counters a valid `=` character
    /// separating the key and value.
    /// This event is necessary as it eliminates the ambiguity for whitespace
    /// events between a key and value event.
    KeyValueSeparator,
}

/// A parsed section containing the header and the section events, typically
/// comprising the keys and their values.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Section<'a> {
    /// The section name and subsection name, if any.
    pub header: section::Header<'a>,
    /// The syntactic events found in this section.
    pub events: Vec<Event<'a>>,
}

/// A parsed comment containing the comment marker and comment.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Comment<'a> {
    /// The comment marker used. This is either a semicolon or octothorpe/hash.
    pub tag: u8,
    /// The parsed comment.
    pub text: Cow<'a, BStr>,
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
