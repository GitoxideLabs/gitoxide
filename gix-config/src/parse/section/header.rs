use bstr::{BStr, BString, ByteSlice};

use crate::parse::{Span, section::HeaderData};

/// The error returned when creating a section header.
#[derive(Debug, PartialOrd, PartialEq, Eq)]
#[expect(missing_docs)]
pub enum Error {
    InvalidName,
    InvalidSubSection,
    Span(crate::parse::span::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidName => f.write_str("section names can only be ascii, '-'"),
            Error::InvalidSubSection => f.write_str("sub-section names must not contain newlines or null bytes"),
            Error::Span(err) => std::fmt::Display::fmt(err, f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::InvalidName | Error::InvalidSubSection => None,
            Error::Span(err) => err.source(),
        }
    }
}

impl From<crate::parse::span::Error> for Error {
    fn from(err: crate::parse::span::Error) -> Self {
        Error::Span(err)
    }
}

impl HeaderData {
    pub(crate) fn new_in(
        name: impl AsRef<str>,
        subsection: impl Into<Option<BString>>,
        backing: &mut Vec<u8>,
    ) -> Result<HeaderData, Error> {
        let name = validated_name(name.as_ref().as_bytes().as_bstr())?;
        let name = Span::append(backing, &name)?;
        let (separator, subsection_name) = match subsection.into() {
            Some(subsection_name) => {
                let subsection_name = validated_subsection(subsection_name.as_ref())?;
                let mut raw = Vec::with_capacity(subsection_name.len());
                write_escaped_subsection(subsection_name.as_bstr(), &mut raw).expect("writing to memory cannot fail");
                let raw_span = Span::append(backing, &raw)?;
                let subsection_name = if raw.as_slice() == subsection_name.as_slice() {
                    crate::parse::MaybeDecoded::raw(raw_span)
                } else {
                    crate::parse::MaybeDecoded::decoded(raw_span, subsection_name)
                };
                (Some(Span::append(backing, b" ")?), Some(subsection_name))
            }
            None => (None, None),
        };
        Ok(HeaderData {
            name,
            separator,
            subsection_name,
        })
    }
}

/// Return true if `name` is valid as subsection name, like `origin` in `[remote "origin"]`.
pub fn is_valid_subsection(name: impl crate::AsBStr) -> bool {
    name.as_bstr().find_byteset(b"\n\0").is_none()
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

impl HeaderData {
    pub(crate) fn rebase(&mut self, offset: usize) -> Result<(), crate::parse::span::Error> {
        self.name.rebase(offset)?;
        if let Some(separator) = &mut self.separator {
            separator.rebase(offset)?;
        }
        if let Some(subsection_name) = &mut self.subsection_name {
            subsection_name.rebase(offset)?;
        }
        Ok(())
    }

    pub(crate) fn copy_to_backing_in(
        &self,
        source: &[u8],
        target: &mut Vec<u8>,
    ) -> Result<HeaderData, crate::parse::span::Error> {
        Ok(HeaderData {
            name: self.name.copy_to_backing_in(source, target)?,
            separator: self
                .separator
                .as_ref()
                .map(|bytes| bytes.copy_to_backing_in(source, target))
                .transpose()?,
            subsection_name: self
                .subsection_name
                .as_ref()
                .map(|name| name.copy_to_backing_in(source, target))
                .transpose()?,
        })
    }

    pub(crate) fn write_to_in(&self, backing: &[u8], mut out: impl std::io::Write) -> std::io::Result<()> {
        out.write_all(b"[")?;
        out.write_all(self.name.as_slice_in(backing))?;

        if let (Some(sep), Some(subsection)) = (&self.separator, &self.subsection_name) {
            out.write_all(sep.as_slice_in(backing))?;
            if sep.as_slice_in(backing) == b"." {
                out.write_all(subsection.raw_span().as_slice_in(backing))?;
            } else {
                out.write_all(b"\"")?;
                out.write_all(subsection.raw_span().as_slice_in(backing))?;
                out.write_all(b"\"")?;
            }
        }

        out.write_all(b"]")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_header_names_are_legal() {
        assert!(
            HeaderData::new_in("", None, &mut Vec::new()).is_ok(),
            "yes, git allows this, so do we"
        );
    }

    #[test]
    fn empty_header_sub_names_are_legal() {
        assert!(
            HeaderData::new_in("remote", Some("".into()), &mut Vec::new()).is_ok(),
            "yes, git allows this, so do we"
        );
    }
}
