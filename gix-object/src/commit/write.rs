use std::io;

use bstr::ByteSlice;

use crate::{encode, encode::NL, Commit, CommitRef, Kind};

fn parse_signature(raw: &bstr::BStr) -> gix_actor::SignatureRef<'_> {
    gix_actor::SignatureRef::from_bytes::<()>(raw.as_ref()).expect("signatures were validated during parsing")
}

fn signature_requires_raw(raw: &bstr::BStr) -> bool {
    let signature = parse_signature(raw);
    signature.name.find_byteset(b"<>\n").is_some() || signature.email.find_byteset(b"<>\n").is_some()
}

fn signature_len(raw: &bstr::BStr) -> usize {
    if signature_requires_raw(raw) {
        raw.len()
    } else {
        parse_signature(raw).size()
    }
}

fn write_signature(mut out: &mut dyn io::Write, field: &[u8], raw: &bstr::BStr) -> io::Result<()> {
    if signature_requires_raw(raw) {
        encode::trusted_header_field(field, raw.as_ref(), &mut out)
    } else {
        let signature = parse_signature(raw);
        encode::trusted_header_signature(field, &signature, &mut out)
    }
}

impl crate::WriteTo for Commit {
    /// Serializes this instance to `out` in the git serialization format.
    fn write_to(&self, mut out: &mut dyn io::Write) -> io::Result<()> {
        encode::trusted_header_id(b"tree", &self.tree, &mut out)?;
        for parent in &self.parents {
            encode::trusted_header_id(b"parent", parent, &mut out)?;
        }
        let mut buf = gix_date::parse::TimeBuf::default();
        encode::trusted_header_signature(b"author", &self.author.to_ref(&mut buf), &mut out)?;
        encode::trusted_header_signature(b"committer", &self.committer.to_ref(&mut buf), &mut out)?;
        if let Some(encoding) = self.encoding.as_ref() {
            encode::header_field(b"encoding", encoding, &mut out)?;
        }
        for (name, value) in &self.extra_headers {
            encode::header_field_multi_line(name, value, &mut out)?;
        }
        out.write_all(NL)?;
        out.write_all(&self.message)
    }

    fn kind(&self) -> Kind {
        Kind::Commit
    }

    fn size(&self) -> u64 {
        let hash_in_hex = self.tree.kind().len_in_hex();
        (b"tree".len() + 1 /*space*/ + hash_in_hex + 1 /* nl */
        + self.parents.iter().count() * (b"parent".len() + 1 + hash_in_hex + 1)
            + b"author".len() + 1 /* space */ + self.author.size() + 1 /* nl */
            + b"committer".len() + 1 /* space */ + self.committer.size() + 1 /* nl */
            + self
                .encoding
                .as_ref()
                .map_or(0, |e| b"encoding".len() + 1 /* space */ + e.len() + 1 /* nl */)
            + self
                .extra_headers
                .iter()
                .map(|(name, value)| {
                    // each header *value* is preceded by a space, and it starts right after the name.
                    name.len() + value.lines_with_terminator().map(|s| s.len() + 1).sum::<usize>() + usize::from(!value.ends_with_str(b"\n"))
                })
                .sum::<usize>()
            + 1 /* nl */
            + self.message.len()) as u64
    }
}

impl crate::WriteTo for CommitRef<'_> {
    /// Serializes this instance to `out` in the git serialization format.
    fn write_to(&self, mut out: &mut dyn io::Write) -> io::Result<()> {
        encode::trusted_header_id(b"tree", &self.tree(), &mut out)?;
        for parent in self.parents() {
            encode::trusted_header_id(b"parent", &parent, &mut out)?;
        }
        write_signature(&mut out, b"author", self.author)?;
        write_signature(&mut out, b"committer", self.committer)?;
        if let Some(encoding) = self.encoding.as_ref() {
            encode::header_field(b"encoding", encoding, &mut out)?;
        }
        for (name, value) in &self.extra_headers {
            encode::header_field_multi_line(name, value, &mut out)?;
        }
        out.write_all(NL)?;
        out.write_all(self.message)
    }

    fn kind(&self) -> Kind {
        Kind::Commit
    }

    fn size(&self) -> u64 {
        let hash_in_hex = self.tree().kind().len_in_hex();
        (b"tree".len() + 1 /* space */ + hash_in_hex + 1 /* nl */
            + self.parents.iter().count() * (b"parent".len() + 1 /* space */ + hash_in_hex + 1 /* nl */)
            + b"author".len() + 1 /* space */ + signature_len(self.author) + 1 /* nl */
            + b"committer".len() + 1 /* space */ + signature_len(self.committer) + 1 /* nl */
            + self
                .encoding
                .as_ref()
                .map_or(0, |e| b"encoding".len() + 1 /* space */ + e.len() + 1 /* nl */)
            + self
                .extra_headers
                .iter()
                .map(|(name, value)| {
                    // each header *value* is preceded by a space, and it starts right after the name.
                    name.len() + value.lines_with_terminator().map(|s| s.len() + 1).sum::<usize>() + usize::from(!value.ends_with_str(b"\n"))
                })
                .sum::<usize>()
            + 1 /* nl */
            + self.message.len()) as u64
    }
}
