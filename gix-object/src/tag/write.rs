use std::io;

use bstr::BStr;
use gix_date::parse::TimeBuf;

use crate::{encode, encode::NL, Kind, Tag, TagRef};

/// An Error used in [`Tag::write_to()`][crate::WriteTo::write_to()].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Tags must not start with a dash: '-'")]
    StartsWithDash,
    #[error("The tag name was no valid reference name")]
    InvalidRefName(#[from] gix_validate::tag::name::Error),
}

impl From<Error> for io::Error {
    fn from(err: Error) -> Self {
        io::Error::other(err)
    }
}

impl crate::WriteTo for Tag {
    fn write_to(&self, out: &mut dyn io::Write) -> io::Result<()> {
        encode::trusted_header_id(b"object", &self.target, out)?;
        encode::trusted_header_field(b"type", self.target_kind.as_bytes(), out)?;
        encode::header_field(b"tag", validated_name(self.name.as_ref())?, out)?;
        if let Some(tagger) = &self.tagger {
            let mut buf = TimeBuf::default();
            encode::trusted_header_signature(b"tagger", &tagger.to_ref(&mut buf), out)?;
        }

        if !self.message.iter().all(|b| *b == b'\n') {
            out.write_all(NL)?;
        }
        out.write_all(self.message.as_ref())?;
        if let Some(message) = &self.pgp_signature {
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
            + self.pgp_signature.as_ref().map_or(0, |m| 1 /* nl */ + m.len())) as u64
    }
}

impl crate::WriteTo for TagRef<'_> {
    fn write_to(&self, mut out: &mut dyn io::Write) -> io::Result<()> {
        encode::trusted_header_field(b"object", self.target, &mut out)?;
        encode::trusted_header_field(b"type", self.target_kind.as_bytes(), &mut out)?;
        encode::header_field(b"tag", validated_name(self.name)?, &mut out)?;
        if let Some(tagger) = self.tagger {
            encode::trusted_header_field(b"tagger", tagger.as_ref(), &mut out)?;
        }

        if !self.message.iter().all(|b| *b == b'\n') {
            out.write_all(NL)?;
        }
        out.write_all(self.message)?;
        if let Some(message) = self.pgp_signature {
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
            + self.pgp_signature.as_ref().map_or(0, |m| 1 /* nl */ + m.len())) as u64
    }
}

fn validated_name(name: &BStr) -> Result<&BStr, Error> {
    gix_validate::tag::name(name)?;
    if name[0] == b'-' {
        return Err(Error::StartsWithDash);
    }
    Ok(name)
}

#[cfg(test)]
mod tests;
