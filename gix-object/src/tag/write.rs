use std::io;

use bstr::BStr;
use gix_date::parse::TimeBuf;
use gix_error::{ErrorExt, ResultExt, ValidationError};

use crate::{Kind, Tag, TagRef, encode, encode::NL};

/// An Error used in [`Tag::write_to()`][crate::WriteTo::write_to()].
pub type Error = gix_error::Exn<ValidationError>;

impl crate::WriteTo for Tag {
    fn write_to(&self, out: &mut dyn io::Write) -> io::Result<()> {
        encode::trusted_header_id(b"object", &self.target, out)?;
        encode::trusted_header_field(b"type", self.target_kind.as_bytes(), out)?;
        encode::header_field(
            b"tag",
            validated_name(self.name.as_ref()).map_err(|err| io::Error::other(err.into_error()))?,
            out,
        )?;
        if let Some(tagger) = &self.tagger {
            let mut buf = TimeBuf::default();
            encode::trusted_header_signature(b"tagger", &tagger.to_ref(&mut buf), out)?;
        }

        if !self.message.iter().all(|b| *b == b'\n') {
            out.write_all(NL)?;
        }
        out.write_all(self.message.as_ref())?;
        if let Some(message) = &self.signature {
            out.write_all(NL)?;
            out.write_all(message.as_ref())?;
        }
        Ok(())
    }

    fn kind(&self) -> Kind {
        Kind::Tag
    }

    fn size(&self) -> u64 {
        (b"object".len() + 1 /* space */ + self.target.kind().len_in_hex() + 1 /* nl */
            + b"type".len() + 1 /* space */ + self.target_kind.as_bytes().len() + 1 /* nl */
            + b"tag".len() + 1 /* space */ + self.name.len() + 1 /* nl */
            + self
            .tagger
            .as_ref()
            .map_or(0, |t| b"tagger".len() + 1 /* space */ + t.size() + 1 /* nl */)
            + if self.message.iter().all(|b| *b == b'\n') { 0 } else { 1 /* nl */ } + self.message.len()
            + self.signature.as_ref().map_or(0, |m| 1 /* nl */ + m.len())) as u64
    }
}

impl crate::WriteTo for TagRef<'_> {
    fn write_to(&self, mut out: &mut dyn io::Write) -> io::Result<()> {
        encode::trusted_header_field(b"object", self.target, &mut out)?;
        encode::trusted_header_field(b"type", self.target_kind.as_bytes(), &mut out)?;
        encode::header_field(
            b"tag",
            validated_name(self.name).map_err(|err| io::Error::other(err.into_error()))?,
            &mut out,
        )?;
        if let Some(tagger) = self.tagger {
            encode::trusted_header_field(b"tagger", tagger.as_ref(), &mut out)?;
        }

        if !self.message.iter().all(|b| *b == b'\n') {
            out.write_all(NL)?;
        }
        out.write_all(self.message)?;
        if let Some(message) = self.signature {
            out.write_all(NL)?;
            out.write_all(message)?;
        }
        Ok(())
    }

    fn kind(&self) -> Kind {
        Kind::Tag
    }

    fn size(&self) -> u64 {
        (b"object".len() + 1 /* space */ + self.target().kind().len_in_hex() + 1 /* nl */
            + b"type".len() + 1 /* space */ + self.target_kind.as_bytes().len() + 1 /* nl */
            + b"tag".len() + 1 /* space */ + self.name.len() + 1 /* nl */
            + self
                .tagger
                .map_or(0, |raw| b"tagger".len() + 1 /* space */ + raw.len() + 1 /* nl */)
            + if self.message.iter().all(|b| *b == b'\n') { 0 } else { 1 /* nl */ } + self.message.len()
            + self.signature.as_ref().map_or(0, |m| 1 /* nl */ + m.len())) as u64
    }
}

fn validated_name(name: &BStr) -> Result<&BStr, Error> {
    gix_validate::tag::name(name).or_raise(|| ValidationError::new("The tag name was no valid reference name"))?;
    if name[0] == b'-' {
        return Err(ValidationError::new("Tags must not start with a dash: '-'").raise());
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    mod validated_name {
        mod invalid {
            use bstr::ByteSlice;

            use super::super::super::*;

            #[test]
            fn only_dash() {
                assert!(validated_name(b"-".as_bstr()).is_err());
            }
            #[test]
            fn leading_dash() {
                assert!(validated_name(b"-hello".as_bstr()).is_err());
            }
        }

        mod valid {
            use bstr::ByteSlice;

            use super::super::super::*;

            #[test]
            fn version() {
                for version in &["v1.0.0", "0.2.1", "0-alpha1"] {
                    assert!(validated_name(version.as_bytes().as_bstr()).is_ok());
                }
            }
        }
    }
}
