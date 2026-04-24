use std::io;

use gix_features::decode::leb64_from_read;

use super::{BLOB, COMMIT, OFS_DELTA, REF_DELTA, TAG, TREE};
use crate::data;

/// The error returned by [data::Entry::from_bytes()].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Object type {type_id} is unsupported")]
    UnsupportedType { type_id: u8 },
    #[error("Pack entry is truncated: {message}")]
    Corrupt { message: &'static str },
    #[error("Pack entry header value overflowed while decoding")]
    Overflow,
}

/// Decoding
impl data::Entry {
    /// Decode an entry from the given entry data `d`, providing the `pack_offset` to allow tracking the start of the entry data section.
    ///
    /// # Panics
    ///
    /// If we cannot understand the header, garbage data is likely to trigger this.
    pub fn from_bytes(d: &[u8], pack_offset: data::Offset, hash_len: usize) -> Result<data::Entry, Error> {
        let (type_id, size, mut consumed) = parse_header_info(d)?;

        use crate::data::entry::Header::*;
        let object = match type_id {
            OFS_DELTA => {
                let (distance, leb_bytes) = parse_leb64(&d[consumed..])?;
                let delta = OfsDelta {
                    base_distance: distance,
                };
                consumed += leb_bytes;
                delta
            }
            REF_DELTA => {
                let delta = RefDelta {
                    base_id: gix_hash::ObjectId::from_bytes_or_panic(d.get(consumed..consumed + hash_len).ok_or(
                        Error::Corrupt {
                            message: "ref-delta base object id",
                        },
                    )?),
                };
                consumed += hash_len;
                delta
            }
            BLOB => Blob,
            TREE => Tree,
            COMMIT => Commit,
            TAG => Tag,
            other => return Err(Error::UnsupportedType { type_id: other }),
        };
        Ok(data::Entry {
            header: object,
            decompressed_size: size,
            data_offset: pack_offset + consumed as u64,
        })
    }

    /// Instantiate an `Entry` from the reader `r`, providing the `pack_offset` to allow tracking the start of the entry data section.
    pub fn from_read(r: &mut dyn io::Read, pack_offset: data::Offset, hash_len: usize) -> io::Result<data::Entry> {
        let (type_id, size, mut consumed) = streaming_parse_header_info(r)?;

        use crate::data::entry::Header::*;
        let object = match type_id {
            OFS_DELTA => {
                let (distance, leb_bytes) = leb64_from_read(&mut *r)?;
                let delta = OfsDelta {
                    base_distance: distance,
                };
                consumed += leb_bytes;
                delta
            }
            REF_DELTA => {
                let mut buf = gix_hash::Kind::buf();
                let hash = &mut buf[..hash_len];
                r.read_exact(hash)?;
                #[allow(clippy::redundant_slicing)]
                let delta = RefDelta {
                    base_id: gix_hash::ObjectId::from_bytes_or_panic(&hash[..]),
                };
                consumed += hash_len;
                delta
            }
            BLOB => Blob,
            TREE => Tree,
            COMMIT => Commit,
            TAG => Tag,
            other => return Err(io::Error::other(format!("Object type {other} is unsupported"))),
        };
        Ok(data::Entry {
            header: object,
            decompressed_size: size,
            data_offset: pack_offset + consumed as u64,
        })
    }
}

#[inline]
fn streaming_parse_header_info(read: &mut dyn io::Read) -> Result<(u8, u64, usize), io::Error> {
    let mut byte = [0u8; 1];
    read.read_exact(&mut byte)?;
    let mut c = byte[0];
    let mut i = 1;
    let type_id = (c >> 4) & 0b0000_0111;
    let mut size = u64::from(c) & 0b0000_1111;
    let mut shift = 4u32;
    while c & 0b1000_0000 != 0 {
        read.read_exact(&mut byte)?;
        c = byte[0];
        i += 1;
        let component = u64::from(c & 0b0111_1111)
            .checked_shl(shift)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "pack entry header overflowed"))?;
        size = size
            .checked_add(component)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "pack entry header overflowed"))?;
        shift += 7;
    }
    if i != encoded_pack_entry_header_size(size) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "pack entry header uses a non-canonical size encoding",
        ));
    }
    Ok((type_id, size, i))
}

/// Parses the header of a pack-entry, yielding object type id, decompressed object size, and consumed bytes
#[inline]
fn parse_header_info(data: &[u8]) -> Result<(u8, u64, usize), Error> {
    let mut c = *data.first().ok_or(Error::Corrupt {
        message: "need a pack entry header, got empty input",
    })?;
    let mut i = 1;
    let type_id = (c >> 4) & 0b0000_0111;
    let mut size = u64::from(c) & 0b0000_1111;
    let mut shift = 4u32;
    while c & 0b1000_0000 != 0 {
        c = *data.get(i).ok_or(Error::Corrupt {
            message: "pack entry header continuation byte",
        })?;
        i += 1;
        let component = u64::from(c & 0b0111_1111).checked_shl(shift).ok_or(Error::Overflow)?;
        size = size.checked_add(component).ok_or(Error::Overflow)?;
        shift += 7;
    }
    if i != encoded_pack_entry_header_size(size) {
        return Err(Error::Corrupt {
            message: "pack entry header uses a non-canonical size encoding",
        });
    }
    Ok((type_id, size, i))
}

fn parse_leb64(data: &[u8]) -> Result<(u64, usize), Error> {
    let mut i = 0;
    let mut c = *data.first().ok_or(Error::Corrupt {
        message: "an ofs-delta base distance",
    })?;
    i += 1;
    let mut value = u64::from(c) & 0x7f;
    while c & 0x80 != 0 {
        c = *data.get(i).ok_or(Error::Corrupt {
            message: "an ofs-delta base distance continuation byte",
        })?;
        i += 1;
        value = value
            .checked_add(1)
            .and_then(|value| value.checked_shl(7))
            .and_then(|value| value.checked_add(u64::from(c) & 0x7f))
            .ok_or(Error::Overflow)?;
    }
    Ok((value, i))
}

/// Return the canonical byte length of a pack-entry size header for `size`.
///
/// We use this to reject overlong size encodings during parsing.
/// That matters for our delta resolution implementation, which later reconstructs an entry's
/// pack offset from `data_offset - header_size()`. If we accepted non-canonical encodings here,
/// `header_size()` would compute the canonical length while `data_offset` would reflect the
/// actually consumed bytes, breaking that invariant and allowing malformed delta entries to point
/// back to themselves or otherwise walk the wrong base objects.
fn encoded_pack_entry_header_size(mut size: u64) -> usize {
    let mut bytes = 1;
    size >>= 4;
    while size != 0 {
        bytes += 1;
        size >>= 7;
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_canonical_pack_entry_header_encoding() {
        assert!(matches!(
            data::Entry::from_bytes(&[0xed, 0x00], 0, gix_hash::Kind::Sha1.len_in_bytes()),
            Err(Error::Corrupt {
                message: "pack entry header uses a non-canonical size encoding"
            })
        ));
    }
}
