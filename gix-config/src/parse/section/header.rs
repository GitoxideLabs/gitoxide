use bstr::{BStr, BString, ByteSlice};
use gix_error::{OptionExt, Result, validation};

use crate::parse::{Span, section::HeaderData};

impl HeaderData {
    /// Invalid section or subsection name bytes are stored as `input` [metadata](gix_error::Error::metadata())
    /// on validation errors.
    pub(crate) fn new_in(
        name: impl AsRef<str>,
        subsection: impl Into<Option<BString>>,
        backing: &mut Vec<u8>,
    ) -> Result<HeaderData> {
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

impl HeaderData {
    pub(crate) fn subsection_name_original<'a>(&'a self, backing: &'a [u8]) -> Option<&'a BStr> {
        let name = self.subsection_name.as_ref()?;
        Some(
            if self
                .separator
                .as_ref()
                .is_some_and(|sep| sep.as_slice_in(backing) == b".")
            {
                name.raw_value_in(backing)
            } else {
                name.value_in(backing)
            },
        )
    }

    /// Return the subsection name to look this section up by. Like git, use the subsection name of
    /// legacy headers like `[section.SubSection]` in lower case.
    pub(crate) fn subsection_name<'a>(&'a self, backing: &'a [u8]) -> Option<&'a BStr> {
        self.subsection_name.as_ref().map(|name| name.value_in(backing))
    }
}

/// Return true if `name` is valid as subsection name, like `origin` in `[remote "origin"]`.
pub fn is_valid_subsection(name: impl crate::AsBStr) -> bool {
    name.as_bstr().find_byteset(b"\n\0").is_none()
}

fn validated_subsection(name: &BStr) -> Result<BString> {
    is_valid_subsection(name)
        .then(|| name.into())
        .ok_or_raise(|| validation("sub-section names must not contain newlines or null bytes").with_input(name))
}

fn validated_name(name: &BStr) -> Result<BString> {
    name.iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'-')
        .then(|| name.into())
        .ok_or_raise(|| validation("section names can only be ascii, '-'").with_input(name))
}

impl HeaderData {
    pub(crate) fn rebase(&mut self, offset: usize) -> Result {
        self.name.rebase(offset)?;
        if let Some(separator) = &mut self.separator {
            separator.rebase(offset)?;
        }
        if let Some(subsection_name) = &mut self.subsection_name {
            subsection_name.rebase(offset)?;
        }
        Ok(())
    }

    pub(crate) fn copy_to_backing_in(&self, source: &[u8], target: &mut Vec<u8>) -> Result<HeaderData> {
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
    fn modern_subsection_names_are_borrowed() -> gix_error::TestResult {
        for name in ["Origin", "escaped\\\"Origin"] {
            let mut backing = Vec::new();
            let header = HeaderData::new_in("remote", Some(name.into()), &mut backing)?;
            let lookup_name = header.subsection_name(&backing);
            assert_eq!(
                lookup_name,
                Some(name.as_bytes().as_bstr()),
                "modern names retain their case, including decoded names"
            );
        }
        let mut backing = Vec::new();
        let header = HeaderData::new_in("remote", None, &mut backing)?;
        assert_eq!(header.subsection_name(&backing), None);
        Ok(())
    }

    #[test]
    fn legacy_subsection_names_are_lower_case() -> gix_error::TestResult {
        let mut backing = Vec::new();
        let mut header = HeaderData::new_in("remote", Some("Origin".into()), &mut backing)?;
        header.separator = Some(Span::append(&mut backing, b".")?);
        let raw = header
            .subsection_name
            .as_ref()
            .expect("subsection is present")
            .raw_span();
        header.subsection_name = Some(crate::parse::MaybeDecoded::decoded(raw, "origin".into()));
        let lookup_name = header.subsection_name(&backing);
        assert_eq!(lookup_name, Some("origin".as_bytes().as_bstr()));
        assert_eq!(
            header.subsection_name_original(&backing),
            Some("Origin".as_bytes().as_bstr()),
            "lowercasing the lookup name preserves the original subsection"
        );
        Ok(())
    }

    #[test]
    fn invalid_names_retain_validation_and_input() {
        for (name, subsection, input) in [
            ("invalid name", None, b"invalid name".as_slice()),
            ("remote", Some(b"new\nline".as_slice()), b"new\nline".as_slice()),
            ("remote", Some(b"nul\0\xff".as_slice()), b"nul\0\xff".as_slice()),
        ] {
            let err = HeaderData::new_in(name, subsection.map(BString::from), &mut Vec::new())
                .expect_err("invalid section or subsection names are rejected");
            assert!(err.is_validation(), "invalid names remain validation errors");
            assert_eq!(
                err.metadata().find_map(|metadata| metadata.get("input")),
                Some(&gix_error::MetadataValue::Bytes(input.into())),
                "header validation retains the original invalid bytes"
            );
        }
    }

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
