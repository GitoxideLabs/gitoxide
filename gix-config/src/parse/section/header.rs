use std::fmt::Display;

use bstr::{BStr, BString, ByteSlice};

use crate::parse::{
    Event, Span,
    section::{Header, Name},
};

/// The error returned by [`Header::new(…)`][super::Header::new()].
#[derive(Debug, PartialOrd, PartialEq, Eq, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("section names can only be ascii, '-'")]
    InvalidName,
    #[error("sub-section names must not contain newlines or null bytes")]
    InvalidSubSection,
}

impl Header {
    /// Instantiate a new header either with a section `name`, e.g. "core" serializing to `["core"]`
    /// or `[remote "origin"]` for `subsection` being "origin" and `name` being "remote".
    pub fn new(name: impl AsRef<str>, subsection: impl Into<Option<BString>>) -> Result<Header, Error> {
        let name = Name(validated_name(name.as_ref().as_bytes().as_bstr())?.into());
        if let Some(subsection_name) = subsection.into() {
            Ok(Header {
                name,
                separator: Some(" ".into()),
                subsection_name: Some(validated_subsection(subsection_name.as_ref())?.into()),
            })
        } else {
            Ok(Header {
                name,
                separator: None,
                subsection_name: None,
            })
        }
    }
}

/// Return true if `name` is valid as subsection name, like `origin` in `[remote "origin"]`.
pub fn is_valid_subsection(name: &BStr) -> bool {
    name.find_byteset(b"\n\0").is_none()
}

fn validated_subsection(name: &BStr) -> Result<BString, Error> {
    is_valid_subsection(name)
        .then(|| name.into())
        .ok_or(Error::InvalidSubSection)
}

fn validated_name(name: &BStr) -> Result<BString, Error> {
    name.iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'-')
        .then(|| name.into())
        .ok_or(Error::InvalidName)
}

impl Header {
    pub(crate) fn intern(&mut self, backing: &mut Vec<u8>) {
        self.name.0.intern(backing);
        if let Some(separator) = &mut self.separator {
            separator.intern(backing);
        }
        if let Some(subsection_name) = &mut self.subsection_name {
            subsection_name.intern(backing);
        }
    }

    pub(crate) fn rebase(&mut self, offset: usize) {
        self.name.0.rebase(offset);
        if let Some(separator) = &mut self.separator {
            separator.rebase(offset);
        }
        if let Some(subsection_name) = &mut self.subsection_name {
            subsection_name.rebase(offset);
        }
    }

    pub(crate) fn to_owned_in(&self, backing: &[u8]) -> Header {
        Header {
            name: Name(self.name.0.to_bstring_in(backing).into()),
            separator: self.separator.as_ref().map(|bytes| bytes.to_bstring_in(backing).into()),
            subsection_name: self
                .subsection_name
                .as_ref()
                .map(|bytes| bytes.to_bstring_in(backing).into()),
        }
    }

    pub(crate) fn copy_to_backing_in(&self, source: &[u8], target: &mut Vec<u8>) -> Header {
        Header {
            name: Name(self.name.0.copy_to_backing_in(source, target)),
            separator: self
                .separator
                .as_ref()
                .map(|bytes| bytes.copy_to_backing_in(source, target)),
            subsection_name: self
                .subsection_name
                .as_ref()
                .map(|bytes| bytes.copy_to_backing_in(source, target)),
        }
    }

    ///Return true if this is a header like `[legacy.subsection]`, or false otherwise.
    pub fn is_legacy(&self) -> bool {
        self.separator.as_deref().is_some_and(|n| n == b".")
    }

    /// Return the subsection name, if present, i.e. "origin" in `[remote "origin"]`.
    ///
    /// It is parsed without quotes, and with escapes folded
    /// into their resulting characters.
    /// Thus during serialization, escapes and quotes must be re-added.
    /// This makes it possible to use parsed event data for lookups directly.
    pub fn subsection_name(&self) -> Option<&BStr> {
        self.subsection_name.as_ref().map(Span::as_bstr)
    }

    /// Return the name of the header, like "remote" in `[remote "origin"]`.
    pub fn name(&self) -> &BStr {
        &self.name
    }

    /// Serialize this type into a `BString` for convenience.
    ///
    /// Note that `to_string()` can also be used, but might not be lossless.
    #[must_use]
    pub fn to_bstring(&self) -> BString {
        let mut buf = Vec::new();
        self.write_to(&mut buf).expect("io error impossible");
        buf.into()
    }

    /// Stream ourselves to the given `out`, in order to reproduce this header mostly losslessly
    /// as it was parsed.
    pub fn write_to(&self, mut out: impl std::io::Write) -> std::io::Result<()> {
        out.write_all(b"[")?;
        out.write_all(&self.name)?;

        if let (Some(sep), Some(subsection)) = (&self.separator, &self.subsection_name) {
            let sep = sep.as_ref();
            out.write_all(sep)?;
            if sep == b"." {
                out.write_all(subsection.as_ref())?;
            } else {
                out.write_all(b"\"")?;
                write_escaped_subsection(subsection.as_ref(), &mut out)?;
                out.write_all(b"\"")?;
            }
        }

        out.write_all(b"]")
    }

    pub(crate) fn write_to_in(&self, backing: &[u8], mut out: impl std::io::Write) -> std::io::Result<()> {
        out.write_all(b"[")?;
        out.write_all(self.name.0.as_slice_in(backing))?;

        if let (Some(sep), Some(subsection)) = (&self.separator, &self.subsection_name) {
            let sep = sep.as_slice_in(backing);
            out.write_all(sep)?;
            if sep == b"." {
                out.write_all(subsection.as_slice_in(backing))?;
            } else {
                out.write_all(b"\"")?;
                write_escaped_subsection(subsection.as_bstr_in(backing), &mut out)?;
                out.write_all(b"\"")?;
            }
        }

        out.write_all(b"]")
    }

    /// Clone this instance.
    #[must_use]
    pub fn to_owned(&self) -> Header {
        self.clone()
    }
}

pub(crate) fn write_escaped_subsection(name: &BStr, mut out: impl std::io::Write) -> std::io::Result<()> {
    for b in name.iter().copied() {
        match b {
            b'\\' => out.write_all(br"\\")?,
            b'"' => out.write_all(br#"\""#)?,
            _ => out.write_all(&[b])?,
        }
    }
    Ok(())
}

impl Display for Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.to_bstring(), f)
    }
}

impl From<Header> for BString {
    fn from(header: Header) -> Self {
        header.to_bstring()
    }
}

impl From<&Header> for BString {
    fn from(header: &Header) -> Self {
        header.to_bstring()
    }
}

impl From<Header> for Event {
    fn from(header: Header) -> Event {
        Event::SectionHeader(header)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_header_names_are_legal() {
        assert!(Header::new("", None).is_ok(), "yes, git allows this, so do we");
    }

    #[test]
    fn empty_header_sub_names_are_legal() {
        assert!(
            Header::new("remote", Some("".into())).is_ok(),
            "yes, git allows this, so do we"
        );
    }
}
