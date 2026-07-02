use std::fmt::Display;

use bstr::{BStr, BString};

use crate::parse::{Event, EventRef};

impl Event {
    pub(crate) fn intern(&mut self, backing: &mut Vec<u8>) {
        match self {
            Event::Comment(comment) => comment.text.intern(backing),
            Event::SectionHeader(header) => header.intern(backing),
            Event::SectionValueName(name) => name.0.intern(backing),
            Event::Value(bytes)
            | Event::Newline(bytes)
            | Event::ValueNotDone(bytes)
            | Event::ValueDone(bytes)
            | Event::Whitespace(bytes) => bytes.intern(backing),
            Event::KeyValueSeparator => {}
        }
    }

    pub(crate) fn rebase(&mut self, offset: usize) {
        match self {
            Event::Comment(comment) => comment.text.rebase(offset),
            Event::SectionHeader(header) => header.rebase(offset),
            Event::SectionValueName(name) => name.0.rebase(offset),
            Event::Value(bytes)
            | Event::Newline(bytes)
            | Event::ValueNotDone(bytes)
            | Event::ValueDone(bytes)
            | Event::Whitespace(bytes) => bytes.rebase(offset),
            Event::KeyValueSeparator => {}
        }
    }

    pub(crate) fn to_owned_in(&self, backing: &[u8]) -> Event {
        match self {
            Event::Comment(comment) => Event::Comment(comment.to_owned_in(backing)),
            Event::SectionHeader(header) => Event::SectionHeader(header.to_owned_in(backing)),
            Event::SectionValueName(name) => {
                Event::SectionValueName(crate::parse::section::ValueName(name.0.to_bstring_in(backing).into()))
            }
            Event::Value(bytes) => Event::Value(bytes.to_bstring_in(backing).into()),
            Event::Newline(bytes) => Event::Newline(bytes.to_bstring_in(backing).into()),
            Event::ValueNotDone(bytes) => Event::ValueNotDone(bytes.to_bstring_in(backing).into()),
            Event::ValueDone(bytes) => Event::ValueDone(bytes.to_bstring_in(backing).into()),
            Event::Whitespace(bytes) => Event::Whitespace(bytes.to_bstring_in(backing).into()),
            Event::KeyValueSeparator => Event::KeyValueSeparator,
        }
    }

    pub(crate) fn copy_to_backing_in(&self, source: &[u8], target: &mut Vec<u8>) -> Event {
        match self {
            Event::Comment(comment) => Event::Comment(comment.copy_to_backing_in(source, target)),
            Event::SectionHeader(header) => Event::SectionHeader(header.copy_to_backing_in(source, target)),
            Event::SectionValueName(name) => Event::SectionValueName(crate::parse::section::ValueName(
                name.0.copy_to_backing_in(source, target),
            )),
            Event::Value(bytes) => Event::Value(bytes.copy_to_backing_in(source, target)),
            Event::Newline(bytes) => Event::Newline(bytes.copy_to_backing_in(source, target)),
            Event::ValueNotDone(bytes) => Event::ValueNotDone(bytes.copy_to_backing_in(source, target)),
            Event::ValueDone(bytes) => Event::ValueDone(bytes.copy_to_backing_in(source, target)),
            Event::Whitespace(bytes) => Event::Whitespace(bytes.copy_to_backing_in(source, target)),
            Event::KeyValueSeparator => Event::KeyValueSeparator,
        }
    }

    /// Resolve this event against `backing` without allocating.
    pub(crate) fn as_ref_in<'a>(&'a self, backing: &'a [u8]) -> EventRef<'a> {
        match self {
            Event::Comment(comment) => EventRef::Comment {
                tag: comment.tag,
                text: comment.text.as_bstr_in(backing),
            },
            Event::SectionHeader(header) => EventRef::SectionHeader {
                name: header.name.0.as_bstr_in(backing),
                separator: header.separator.as_ref().map(|separator| separator.as_bstr_in(backing)),
                subsection_name: header
                    .subsection_name
                    .as_ref()
                    .map(|subsection_name| subsection_name.as_bstr_in(backing)),
            },
            Event::SectionValueName(name) => EventRef::SectionValueName(name.0.as_bstr_in(backing)),
            Event::Value(bytes) => EventRef::Value(bytes.as_bstr_in(backing)),
            Event::Newline(bytes) => EventRef::Newline(bytes.as_bstr_in(backing)),
            Event::ValueNotDone(bytes) => EventRef::ValueNotDone(bytes.as_bstr_in(backing)),
            Event::ValueDone(bytes) => EventRef::ValueDone(bytes.as_bstr_in(backing)),
            Event::Whitespace(bytes) => EventRef::Whitespace(bytes.as_bstr_in(backing)),
            Event::KeyValueSeparator => EventRef::KeyValueSeparator,
        }
    }

    /// Serialize this type into a `BString` for convenience.
    ///
    /// Note that `to_string()` can also be used, but might not be lossless.
    #[must_use]
    pub(crate) fn to_bstring(&self) -> BString {
        let mut buf = Vec::new();
        self.write_to(&mut buf).expect("io error impossible");
        buf.into()
    }

    pub(crate) fn to_bstr_lossy_in<'a>(&'a self, backing: &'a [u8]) -> &'a BStr {
        match self {
            Self::ValueNotDone(e) | Self::Whitespace(e) | Self::Newline(e) | Self::Value(e) | Self::ValueDone(e) => {
                e.as_bstr_in(backing)
            }
            Self::KeyValueSeparator => "=".into(),
            Self::SectionValueName(k) => k.0.as_bstr_in(backing),
            Self::SectionHeader(h) => h.name.0.as_bstr_in(backing),
            Self::Comment(c) => c.text.as_bstr_in(backing),
        }
    }

    /// Stream ourselves to the given `out`, in order to reproduce this event mostly losslessly
    /// as it was parsed.
    pub(crate) fn write_to(&self, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        match self {
            Self::ValueNotDone(e) => {
                out.write_all(e.as_slice())?;
                out.write_all(br"\")
            }
            Self::Whitespace(e) | Self::Newline(e) | Self::Value(e) | Self::ValueDone(e) => out.write_all(e.as_slice()),
            Self::KeyValueSeparator => out.write_all(b"="),
            Self::SectionValueName(k) => out.write_all(k.0.as_slice()),
            Self::SectionHeader(h) => h.write_to(out),
            Self::Comment(c) => c.write_to(out),
        }
    }

    pub(crate) fn write_to_in(&self, backing: &[u8], out: &mut dyn std::io::Write) -> std::io::Result<()> {
        match self {
            Self::ValueNotDone(e) => {
                out.write_all(e.as_slice_in(backing))?;
                out.write_all(br"\")
            }
            Self::Whitespace(e) | Self::Newline(e) | Self::Value(e) | Self::ValueDone(e) => {
                out.write_all(e.as_slice_in(backing))
            }
            Self::KeyValueSeparator => out.write_all(b"="),
            Self::SectionValueName(k) => out.write_all(k.0.as_slice_in(backing)),
            Self::SectionHeader(h) => h.write_to_in(backing, out),
            Self::Comment(c) => c.write_to_in(backing, out),
        }
    }
}

impl EventRef<'_> {
    /// Turn ourselves into the text we represent, lossy.
    ///
    /// Note that this mirrors `Event::to_bstr_lossy_in()`.
    pub fn to_bstr_lossy(&self) -> &BStr {
        match self {
            EventRef::ValueNotDone(bytes)
            | EventRef::Whitespace(bytes)
            | EventRef::Newline(bytes)
            | EventRef::Value(bytes)
            | EventRef::ValueDone(bytes) => bytes,
            EventRef::KeyValueSeparator => "=".into(),
            EventRef::SectionValueName(name) => name,
            EventRef::SectionHeader { name, .. } => name,
            EventRef::Comment { text, .. } => text,
        }
    }

    /// Stream ourselves to the given `out`, reproducing this event mostly losslessly.
    pub fn write_to(&self, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        match self {
            EventRef::ValueNotDone(bytes) => {
                out.write_all(bytes)?;
                out.write_all(br"\")
            }
            EventRef::Whitespace(bytes)
            | EventRef::Newline(bytes)
            | EventRef::Value(bytes)
            | EventRef::ValueDone(bytes) => out.write_all(bytes),
            EventRef::KeyValueSeparator => out.write_all(b"="),
            EventRef::SectionValueName(name) => out.write_all(name),
            EventRef::SectionHeader {
                name,
                separator,
                subsection_name,
            } => {
                out.write_all(b"[")?;
                out.write_all(name)?;
                if let (Some(separator), Some(subsection_name)) = (separator, subsection_name) {
                    out.write_all(separator)?;
                    if *separator == b"." {
                        out.write_all(subsection_name)?;
                    } else {
                        out.write_all(b"\"")?;
                        crate::parse::section::header::write_escaped_subsection(subsection_name, &mut *out)?;
                        out.write_all(b"\"")?;
                    }
                }
                out.write_all(b"]")
            }
            EventRef::Comment { tag, text } => {
                out.write_all(&[*tag])?;
                out.write_all(text)
            }
        }
    }
}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.to_bstring(), f)
    }
}

impl Display for EventRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&BString::from(self), f)
    }
}

impl From<&EventRef<'_>> for BString {
    fn from(event: &EventRef<'_>) -> Self {
        let mut buf = Vec::new();
        event.write_to(&mut buf).expect("io error impossible");
        buf.into()
    }
}

impl From<Event> for BString {
    fn from(event: Event) -> Self {
        event.to_bstring()
    }
}

impl From<&Event> for BString {
    fn from(event: &Event) -> Self {
        event.to_bstring()
    }
}
