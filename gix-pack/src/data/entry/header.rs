use std::io;

use super::{BLOB, COMMIT, OFS_DELTA, REF_DELTA, TAG, TREE};
use crate::data;

/// The header portion of a pack data entry, identifying the kind of stored object.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[expect(missing_docs)]
pub enum Header {
    /// The object is a commit
    Commit,
    /// The object is a tree
    Tree,
    /// The object is a blob
    Blob,
    /// The object is a tag
    Tag,
    /// Describes a delta-object which needs to be applied to a base identified by `base_id`.
    /// The base may occur anywhere in the same pack or in the parent repository, as it does in a **thin-pack**.
    ///
    /// **The naming** is exactly the same as the canonical implementation uses, namely **REF_DELTA**.
    RefDelta { base_id: gix_hash::ObjectId },
    /// Describes a delta-object present in this pack which acts as base for this object.
    /// The base object is measured as a distance from this objects
    /// pack offset, so that `base_pack_offset = this_objects_pack_offset - base_distance`
    ///
    /// # Note
    ///
    /// **The naming** is exactly the same as the canonical implementation uses, namely **OFS_DELTA**.
    OfsDelta { base_distance: u64 },
}

impl Header {
    /// Subtract `distance` from `pack_offset` safely, returning only offsets beyond the pack header.
    pub fn verified_base_pack_offset(pack_offset: data::Offset, distance: u64) -> Option<data::Offset> {
        if distance == 0 {
            return None;
        }
        pack_offset
            .checked_sub(distance)
            .filter(|offset| *offset >= data::header::SIZE as data::Offset)
    }
    /// Convert the header's object kind into [`gix_object::Kind`] if possible
    pub fn as_kind(&self) -> Option<gix_object::Kind> {
        use gix_object::Kind::*;
        Some(match self {
            Header::Tree => Tree,
            Header::Blob => Blob,
            Header::Commit => Commit,
            Header::Tag => Tag,
            Header::RefDelta { .. } | Header::OfsDelta { .. } => return None,
        })
    }
    /// Convert this header's object kind into the packs internal representation
    pub fn as_type_id(&self) -> u8 {
        use Header::*;
        match self {
            Blob => BLOB,
            Tree => TREE,
            Commit => COMMIT,
            Tag => TAG,
            OfsDelta { .. } => OFS_DELTA,
            RefDelta { .. } => REF_DELTA,
        }
    }
    /// Return's true if this is a delta object, i.e. not a full object.
    pub fn is_delta(&self) -> bool {
        matches!(self, Header::OfsDelta { .. } | Header::RefDelta { .. })
    }
    /// Return's true if this is a base object, i.e. not a delta object.
    pub fn is_base(&self) -> bool {
        !self.is_delta()
    }
}

impl Header {
    /// Encode this header along the given `decompressed_size_in_bytes` into the `out` write stream for use within a data pack.
    ///
    /// Returns the amount of bytes written to `out`.
    /// `decompressed_size_in_bytes` is the full size in bytes of the object that this header represents
    pub fn write_to(&self, decompressed_size_in_bytes: u64, out: &mut dyn io::Write) -> io::Result<usize> {
        let mut size = decompressed_size_in_bytes;
        let mut written = 1;
        let mut c: u8 = (self.as_type_id() << 4) | (size as u8 & 0b0000_1111);
        size >>= 4;
        while size != 0 {
            out.write_all(&[c | 0b1000_0000])?;
            written += 1;
            c = size as u8 & 0b0111_1111;
            size >>= 7;
        }
        out.write_all(&[c])?;

        use Header::*;
        match self {
            RefDelta { base_id: oid } => {
                out.write_all(oid.as_slice())?;
                written += oid.as_slice().len();
            }
            OfsDelta { base_distance } => {
                let mut buf = [0u8; 10];
                let buf = leb64_encode(*base_distance, &mut buf);
                out.write_all(buf)?;
                written += buf.len();
            }
            Blob | Tree | Commit | Tag => {}
        }
        Ok(written)
    }

    /// The size of the header in bytes when written in canonical form.
    ///
    /// This is the number of bytes [`Self::write_to()`] would emit for `decompressed_size`.
    /// It does not inspect existing pack bytes and therefore does not preserve non-canonical
    /// overlong size encodings. Use [`data::Entry::header_size()`] for decoded entries when the
    /// result has to match the header length present in the pack.
    pub fn size(&self, decompressed_size: u64) -> usize {
        self.write_to(decompressed_size, &mut io::sink())
            .expect("io::sink() to never fail")
    }
}

#[inline]
fn leb64_encode(mut n: u64, buf: &mut [u8; 10]) -> &[u8] {
    let mut bytes_written = 1;
    buf[buf.len() - 1] = n as u8 & 0b0111_1111;
    for out in buf.iter_mut().rev().skip(1) {
        n >>= 7;
        if n == 0 {
            break;
        }
        n -= 1;
        *out = 0b1000_0000 | (n as u8 & 0b0111_1111);
        bytes_written += 1;
    }
    debug_assert_eq!(n, 0, "BUG: buffer must be large enough to hold a 64 bit integer");
    &buf[buf.len() - bytes_written..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leb64_encode_max_int() {
        let mut buf = [0u8; 10];
        let buf = leb64_encode(u64::MAX, &mut buf);
        assert_eq!(buf.len(), 10, "10 bytes should be used when 64bits are encoded");
    }

    #[test]
    fn verified_base_pack_offset_rejects_the_pack_header() {
        let first_entry_offset = data::header::SIZE as data::Offset;
        assert_eq!(
            Header::verified_base_pack_offset(first_entry_offset + 1, 1),
            Some(first_entry_offset)
        );
        for (pack_offset, distance) in [
            (first_entry_offset + 1, 0),
            (first_entry_offset, 1),
            (first_entry_offset, first_entry_offset),
            (first_entry_offset, first_entry_offset + 1),
        ] {
            assert_eq!(
                Header::verified_base_pack_offset(pack_offset, distance),
                None,
                "offset {pack_offset} and distance {distance} cannot point to a pack entry"
            );
        }
    }
}
