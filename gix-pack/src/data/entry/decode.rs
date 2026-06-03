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
            encoded_header_size: consumed.try_into().expect("pack entry headers fit into u16"),
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
            encoded_header_size: consumed.try_into().expect("pack entry headers fit into u16"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_non_canonical_pack_entry_header_encoding() {
        let pack_offset = 42;
        let entry = data::Entry::from_bytes(&[0xb3, 0x00], pack_offset, gix_hash::Kind::Sha1.len_in_bytes())
            .expect("non-canonical size encodings are accepted by git");

        assert_eq!(entry.header, data::entry::Header::Blob);
        assert_eq!(entry.decompressed_size, 3);
        assert_eq!(entry.header_size(), 2);
        assert_eq!(entry.pack_offset(), pack_offset);
        assert_eq!(entry.data_offset, pack_offset + 2);
    }

    #[test]
    fn non_canonical_pack_entry_header_keeps_ofs_delta_base_offsets_correct() {
        let pack_offset = 100;
        let base_distance = 5;
        let entry = data::Entry::from_bytes(
            &[0xe4, 0x00, base_distance],
            pack_offset,
            gix_hash::Kind::Sha1.len_in_bytes(),
        )
        .expect("non-canonical ofs-delta size encodings are accepted by git");

        assert_eq!(
            entry.header,
            data::entry::Header::OfsDelta {
                base_distance: base_distance.into()
            }
        );
        assert_eq!(entry.header_size(), 3);
        assert_eq!(entry.pack_offset(), pack_offset);
        assert_eq!(
            entry.checked_base_pack_offset(base_distance.into()),
            Some(pack_offset - u64::from(base_distance))
        );
    }
}
