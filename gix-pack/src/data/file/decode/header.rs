use gix_features::zlib;

use crate::{
    data,
    data::{File, delta, file::decode::Error},
};

/// A return value of a resolve function, which given an [`ObjectId`][gix_hash::ObjectId] determines where an object can be found.
#[derive(Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ResolvedBase {
    /// Indicate an object is within this pack, at the given entry, and thus can be looked up locally.
    InPack(data::Entry),
    /// Indicates the object of `kind` was found outside of the pack.
    OutOfPack {
        /// The kind of object we found when reading the header of the out-of-pack base.
        kind: gix_object::Kind,
        /// The amount of deltas encountered if the object was packed as well.
        num_deltas: Option<u32>,
    },
}

/// Additional information and statistics about a successfully decoded object produced by [`File::decode_header()`].
///
/// Useful to understand the effectiveness of the pack compression or the cost of decompression.
#[derive(Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Outcome {
    /// The kind of resolved object.
    pub kind: gix_object::Kind,
    /// The decompressed size of the object.
    pub object_size: u64,
    /// The amount of deltas in the chain of objects that had to be resolved beforehand.
    pub num_deltas: u32,
}

/// Obtain object information quickly.
impl<T> File<T>
where
    T: crate::FileData,
{
    /// Resolve the object header information starting at `entry`, following the chain of entries as needed.
    ///
    /// The `entry` determines which object to decode, and is commonly obtained with the help of a pack index file or through pack iteration.
    /// `inflate` will be used for (partially) decompressing entries, and will be reset before first use, but not after the last use.
    ///
    /// `resolve` is a function to lookup objects with the given [`ObjectId`][gix_hash::ObjectId], in case the full object id
    /// is used to refer to a base object, instead of an in-pack offset.
    ///
    /// For delta entries, this only probes the initial delta header bytes to determine the result
    /// object size. It can reject streams that end or overflow within that probe, but it does not
    /// fully validate that the compressed stream produces exactly the decompressed size declared in
    /// the pack entry header. Use [`File::decode_entry()`][crate::data::File::decode_entry()] when
    /// callers need that full validation.
    pub fn decode_header(
        &self,
        mut entry: data::Entry,
        inflate: &mut zlib::Inflate,
        resolve: &dyn Fn(&gix_hash::oid) -> Option<ResolvedBase>,
    ) -> Result<Outcome, Error> {
        use crate::data::entry::Header::*;
        let mut num_deltas = 0;
        let mut first_delta_decompressed_size = None::<u64>;
        loop {
            match entry.header {
                Tree | Blob | Commit | Tag => {
                    return Ok(Outcome {
                        kind: entry.header.as_kind().expect("always valid for non-refs"),
                        object_size: first_delta_decompressed_size.unwrap_or(entry.decompressed_size),
                        num_deltas,
                    });
                }
                OfsDelta { base_distance } => {
                    num_deltas += 1;
                    if first_delta_decompressed_size.is_none() {
                        first_delta_decompressed_size = Some(self.decode_delta_object_size(inflate, &entry)?);
                    }
                    entry = self.entry(entry.checked_base_pack_offset(base_distance).ok_or(
                        crate::data::entry::decode::Error::Corrupt {
                            message: "an ofs-delta base distance pointing before pack start",
                        },
                    )?)?;
                }
                RefDelta { base_id } => {
                    num_deltas += 1;
                    if first_delta_decompressed_size.is_none() {
                        first_delta_decompressed_size = Some(self.decode_delta_object_size(inflate, &entry)?);
                    }
                    match resolve(base_id.as_ref()) {
                        Some(ResolvedBase::InPack(base_entry)) => entry = base_entry,
                        Some(ResolvedBase::OutOfPack {
                            kind,
                            num_deltas: origin_num_deltas,
                        }) => {
                            return Ok(Outcome {
                                kind,
                                object_size: first_delta_decompressed_size.unwrap_or(entry.decompressed_size),
                                num_deltas: origin_num_deltas.unwrap_or_default() + num_deltas,
                            });
                        }
                        None => return Err(Error::DeltaBaseUnresolved(base_id)),
                    }
                }
            }
        }
    }

    /// Decode the result object size from the initial delta header bytes in `inflate`, using `entry`
    /// for offsets.
    ///
    /// This intentionally mirrors Git's cheap header probe: only the first 20 decompressed bytes
    /// are inspected, which is enough for the two `u64` varints that make up a valid delta header.
    /// If the zlib stream ends within that probe, we can reject declared-size mismatches here.
    /// Otherwise this result only proves that the delta header prefix is parseable; full
    /// decompression through `decode_entry()` must still validate that the stream length matches
    /// the pack entry header.
    #[inline]
    fn decode_delta_object_size(&self, inflate: &mut zlib::Inflate, entry: &data::Entry) -> Result<u64, Error> {
        let mut buf = [0_u8; 20];
        let max_size = entry.decompressed_size.min(buf.len() as u64) as usize;
        let (status, _consumed_in, consumed_out) =
            self.decompress_entry_from_data_offset_unchecked(entry.data_offset, inflate, &mut buf[..max_size])?;
        if status == zlib::Status::StreamEnd {
            if consumed_out as u64 != entry.decompressed_size {
                return Err(data::entry::decode::Error::Corrupt {
                    message: "pack entry decompressed to fewer bytes than declared in the entry header",
                }
                .into());
            }
        } else if entry.decompressed_size == max_size as u64 {
            return Err(data::entry::decode::Error::Corrupt {
                message: "pack entry decompressed to more bytes than declared in the entry header",
            }
            .into());
        }
        let buf = &buf[..consumed_out];
        let (_base_size, offset) = delta::decode_header_size(buf)?;
        let (result_size, _offset) = delta::decode_header_size(&buf[offset..])?;
        Ok(result_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_of_decode_entry_outcome() {
        assert_eq!(
            std::mem::size_of::<Outcome>(),
            16,
            "this shouldn't change without use noticing as it's returned a lot"
        );
    }
}
