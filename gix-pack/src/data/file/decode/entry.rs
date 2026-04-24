use std::ops::Range;

use gix_features::zlib;
use smallvec::SmallVec;

use crate::{
    cache, data,
    data::{delta, file::decode::Error, File},
};

/// A return value of a resolve function, which given an [`ObjectId`][gix_hash::ObjectId] determines where an object can be found.
#[derive(Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ResolvedBase {
    /// Indicate an object is within this pack, at the given entry, and thus can be looked up locally.
    InPack(data::Entry),
    /// Indicates the object of `kind` was found outside of the pack, and its data was written into an output
    /// vector which now has a length of `end`.
    #[allow(missing_docs)]
    OutOfPack { kind: gix_object::Kind, end: usize },
}

#[derive(Debug)]
struct Delta {
    data: Range<usize>,
    base_size: usize,
    result_size: usize,

    decompressed_size: usize,
    data_offset: data::Offset,
}

/// Additional information and statistics about a successfully decoded object produced by [`File::decode_entry()`].
///
/// Useful to understand the effectiveness of the pack compression or the cost of decompression.
#[derive(Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Outcome {
    /// The kind of resolved object.
    pub kind: gix_object::Kind,
    /// The amount of deltas in the chain of objects that had to be resolved beforehand.
    ///
    /// This number is affected by the [`Cache`][cache::DecodeEntry] implementation, with cache hits shortening the
    /// delta chain accordingly
    pub num_deltas: u32,
    /// The total decompressed size of all pack entries in the delta chain
    pub decompressed_size: u64,
    /// The total compressed size of all pack entries in the delta chain
    pub compressed_size: usize,
    /// The total size of the decoded object.
    pub object_size: u64,
}

impl Outcome {
    pub(crate) fn default_from_kind(kind: gix_object::Kind) -> Self {
        Self {
            kind,
            num_deltas: 0,
            decompressed_size: 0,
            compressed_size: 0,
            object_size: 0,
        }
    }
    fn from_object_entry(kind: gix_object::Kind, entry: &data::Entry, compressed_size: usize) -> Self {
        Self {
            kind,
            num_deltas: 0,
            decompressed_size: entry.decompressed_size,
            compressed_size,
            object_size: entry.decompressed_size,
        }
    }
}

/// Decompression of objects
impl<T> File<T>
where
    T: crate::FileData,
{
    fn decoded_object_size(&self, size: u64) -> Result<usize, Error> {
        decoded_object_size(size, self.alloc_limit_bytes)
    }

    /// Decompress the given `entry` into `out` and return the amount of bytes read from the pack data.
    /// Note that `inflate` is not reset after usage, but will be reset before using it.
    ///
    /// _Note_ that this method does not resolve deltified objects, but merely decompresses their content
    /// `out` is expected to be large enough to hold `entry.size` bytes.
    ///
    /// # Panics
    ///
    /// If `out` isn't large enough to hold the decompressed `entry`
    pub fn decompress_entry(
        &self,
        entry: &data::Entry,
        inflate: &mut zlib::Inflate,
        out: &mut [u8],
    ) -> Result<usize, Error> {
        assert!(
            out.len() as u64 >= entry.decompressed_size,
            "output buffer isn't large enough to hold decompressed result, want {}, have {}",
            entry.decompressed_size,
            out.len()
        );

        self.decompress_entry_from_data_offset(entry.data_offset, inflate, out)
    }

    /// Obtain the [`Entry`][crate::data::Entry] at the given `offset` into the pack.
    ///
    /// The `offset` is typically obtained from the pack index file.
    pub fn entry(&self, offset: data::Offset) -> Result<data::Entry, data::entry::decode::Error> {
        let pack_offset: usize = offset.try_into().expect("offset representable by machine");
        if pack_offset > self.data.len() {
            return Err(data::entry::decode::Error::Corrupt {
                message: "an entry offset pointing beyond pack data",
            });
        }

        let object_data = &self.data[pack_offset..];
        data::Entry::from_bytes(object_data, offset, self.hash_len)
    }

    /// Decompress the object expected at the given data offset, sans pack header. This information is only
    /// known after the pack header was parsed.
    /// Note that this method does not resolve deltified objects, but merely decompresses their content
    /// `out` is expected to be large enough to hold `entry.size` bytes.
    /// Returns the amount of packed bytes there read from the pack data file.
    pub(crate) fn decompress_entry_from_data_offset(
        &self,
        data_offset: data::Offset,
        inflate: &mut zlib::Inflate,
        out: &mut [u8],
    ) -> Result<usize, Error> {
        let offset: usize = data_offset.try_into().expect("offset representable by machine");
        if offset >= self.data.len() {
            return Err(data::entry::decode::Error::Corrupt {
                message: "an entry data offset pointing beyond pack data",
            }
            .into());
        }

        inflate.reset();
        inflate
            .once(&self.data[offset..], out)
            .map(|(_status, consumed_in, _consumed_out)| consumed_in)
            .map_err(Into::into)
    }

    /// Like `decompress_entry_from_data_offset`, but returns consumed input and output.
    pub(crate) fn decompress_entry_from_data_offset_2(
        &self,
        data_offset: data::Offset,
        inflate: &mut zlib::Inflate,
        out: &mut [u8],
    ) -> Result<(usize, usize), Error> {
        let offset: usize = data_offset.try_into().expect("offset representable by machine");
        if offset >= self.data.len() {
            return Err(data::entry::decode::Error::Corrupt {
                message: "an entry data offset pointing beyond pack data",
            }
            .into());
        }

        inflate.reset();
        inflate
            .once(&self.data[offset..], out)
            .map(|(_status, consumed_in, consumed_out)| (consumed_in, consumed_out))
            .map_err(Into::into)
    }

    /// Decode an entry, resolving delta's as needed, while growing the `out` vector if there is not enough
    /// space to hold the result object.
    ///
    /// The `entry` determines which object to decode, and is commonly obtained with the help of a pack index file or through pack iteration.
    /// `inflate` will be used for decompressing entries, and will not be reset after usage, but before first using it.
    ///
    /// `resolve` is a function to lookup objects with the given [`ObjectId`][gix_hash::ObjectId], in case the full object id is used to refer to
    /// a base object, instead of an in-pack offset.
    ///
    /// `delta_cache` is a mechanism to avoid looking up base objects multiple times when decompressing multiple objects in a row.
    /// Use a [Noop-Cache][cache::Never] to disable caching all together at the cost of repeating work.
    pub fn decode_entry(
        &self,
        entry: data::Entry,
        out: &mut Vec<u8>,
        inflate: &mut zlib::Inflate,
        resolve: &dyn Fn(&gix_hash::oid, &mut Vec<u8>) -> Option<ResolvedBase>,
        delta_cache: &mut dyn cache::DecodeEntry,
    ) -> Result<Outcome, Error> {
        use crate::data::entry::Header::*;
        match entry.header {
            Tree | Blob | Commit | Tag => {
                let size = self.decoded_object_size(entry.decompressed_size)?;
                if let Some(additional) = size.checked_sub(out.len()) {
                    out.try_reserve(additional)?;
                }
                out.resize(size, 0);
                self.decompress_entry(&entry, inflate, out.as_mut_slice())
                    .map(|consumed_input| {
                        Outcome::from_object_entry(
                            entry.header.as_kind().expect("a non-delta entry"),
                            &entry,
                            consumed_input,
                        )
                    })
            }
            OfsDelta { .. } | RefDelta { .. } => self.resolve_deltas(entry, resolve, inflate, out, delta_cache),
        }
    }

    /// resolve: technically, this shouldn't ever be required as stored local packs don't refer to objects by id
    /// that are outside of the pack. Unless, of course, the ref refers to an object within this pack, which means
    /// it's very, very large as 20bytes are smaller than the corresponding MSB encoded number
    fn resolve_deltas(
        &self,
        last: data::Entry,
        resolve: &dyn Fn(&gix_hash::oid, &mut Vec<u8>) -> Option<ResolvedBase>,
        inflate: &mut zlib::Inflate,
        out: &mut Vec<u8>,
        cache: &mut dyn cache::DecodeEntry,
    ) -> Result<Outcome, Error> {
        // all deltas, from the one that produces the desired object (first) to the oldest at the end of the chain
        let mut chain = SmallVec::<[Delta; 10]>::default();
        let first_entry = last.clone();
        let mut cursor = last;
        let mut base_buffer_size: Option<usize> = None;
        let mut object_kind: Option<gix_object::Kind> = None;
        let mut consumed_input: Option<usize> = None;

        // Find the first full base, either an undeltified object in the pack or a reference to another object.
        let mut total_delta_data_size: u64 = 0;
        while cursor.header.is_delta() {
            if let Some((kind, packed_size)) = cache.get(self.id, cursor.data_offset, out) {
                base_buffer_size = Some(out.len());
                object_kind = Some(kind);
                // If the input entry is a cache hit, keep the packed size as it must be returned.
                // Otherwise, the packed size will be determined later when decompressing the input delta
                if total_delta_data_size == 0 {
                    consumed_input = Some(packed_size);
                }
                break;
            }
            // This is a pessimistic guess, as worst possible compression should not be bigger than the data itself.
            // TODO: is this assumption actually true?
            total_delta_data_size = total_delta_data_size
                .checked_add(cursor.decompressed_size)
                .ok_or(Error::OutOfMemory)?;
            let decompressed_size = self.decoded_object_size(cursor.decompressed_size)?;
            chain.push(Delta {
                data: Range {
                    start: 0,
                    end: decompressed_size,
                },
                base_size: 0,
                result_size: 0,
                decompressed_size,
                data_offset: cursor.data_offset,
            });
            use crate::data::entry::Header;
            cursor = match cursor.header {
                Header::OfsDelta { base_distance } => {
                    self.entry(cursor.checked_base_pack_offset(base_distance).ok_or(
                        crate::data::entry::decode::Error::Corrupt {
                            message: "an ofs-delta base distance pointing before pack start",
                        },
                    )?)?
                }
                Header::RefDelta { base_id } => match resolve(base_id.as_ref(), out) {
                    Some(ResolvedBase::InPack(entry)) => entry,
                    Some(ResolvedBase::OutOfPack { end, kind }) => {
                        base_buffer_size = Some(end);
                        object_kind = Some(kind);
                        break;
                    }
                    None => return Err(Error::DeltaBaseUnresolved(base_id)),
                },
                _ => unreachable!("cursor.is_delta() only allows deltas here"),
            };
        }

        // This can happen if the cache held the first entry itself
        // We will just treat it as an object then, even though it's technically incorrect.
        if chain.is_empty() {
            return Ok(Outcome::from_object_entry(
                object_kind.expect("object kind as set by cache"),
                &first_entry,
                consumed_input.expect("consumed bytes as set by cache"),
            ));
        }

        // First pass will decompress all delta data and keep it in our output buffer
        // [<possibly resolved base object>]<delta-1..delta-n>...
        // so that we can find the biggest result size.
        let total_delta_data_size: usize = total_delta_data_size.try_into().map_err(|_| Error::OutOfMemory)?;

        let chain_len = chain.len();
        let (first_buffer_end, second_buffer_end) = {
            let delta_start = base_buffer_size.unwrap_or(0);

            let delta_range = Range {
                start: delta_start,
                end: delta_start
                    .checked_add(total_delta_data_size)
                    .ok_or(Error::OutOfMemory)?,
            };
            out.try_reserve(delta_range.end.saturating_sub(out.len()))?;
            out.resize(delta_range.end, 0);

            let mut instructions = &mut out[delta_range.clone()];
            let mut relative_delta_start = 0;
            let mut biggest_result_size = 0;
            for (delta_idx, delta) in chain.iter_mut().rev().enumerate() {
                let consumed_from_data_offset = self.decompress_entry_from_data_offset(
                    delta.data_offset,
                    inflate,
                    &mut instructions[..delta.decompressed_size],
                )?;
                let is_last_delta_to_be_applied = delta_idx + 1 == chain_len;
                if is_last_delta_to_be_applied {
                    consumed_input = Some(consumed_from_data_offset);
                }

                let (base_size, offset) = delta::decode_header_size(instructions)?;
                let mut bytes_consumed_by_header = offset;
                biggest_result_size = biggest_result_size.max(base_size);
                delta.base_size = self.decoded_object_size(base_size)?;

                let (result_size, offset) = delta::decode_header_size(&instructions[offset..])?;
                bytes_consumed_by_header += offset;
                biggest_result_size = biggest_result_size.max(result_size);
                delta.result_size = self.decoded_object_size(result_size)?;

                // the absolute location into the instructions buffer, so we keep track of the end point of the last
                delta.data.start = relative_delta_start + bytes_consumed_by_header;
                relative_delta_start += delta.decompressed_size;
                delta.data.end = relative_delta_start;

                instructions = &mut instructions[delta.decompressed_size..];
            }

            // Now we can produce a buffer like this
            // [<biggest-result-buffer, possibly filled with resolved base object data>]<biggest-result-buffer><delta-1..delta-n>
            // from [<possibly resolved base object>]<delta-1..delta-n>...
            let biggest_result_size = self.decoded_object_size(biggest_result_size)?;
            let first_buffer_size = biggest_result_size;
            let second_buffer_size = first_buffer_size;
            let out_size = first_buffer_size
                .checked_add(second_buffer_size)
                .and_then(|size| size.checked_add(total_delta_data_size))
                .ok_or(Error::OutOfMemory)?;
            out.try_reserve(out_size.saturating_sub(out.len()))?;
            out.resize(out_size, 0);

            // Now 'rescue' the deltas, because in the next step we possibly overwrite that portion
            // of memory with the base object (in the majority of cases)
            let second_buffer_end = {
                let end = first_buffer_size
                    .checked_add(second_buffer_size)
                    .ok_or(Error::OutOfMemory)?;
                // Move the decompressed delta instructions behind the two work buffers so they remain intact
                // while we repurpose the front of `out` for base-object materialization and delta application.
                out.copy_within(delta_range, end);
                end
            };

            // If we don't have a out-of-pack object already, fill the base-buffer by decompressing the full object
            // at which the cursor is left after the iteration
            if base_buffer_size.is_none() {
                let base_entry = cursor;
                debug_assert!(!base_entry.header.is_delta());
                object_kind = base_entry.header.as_kind();
                let out_base = &mut out[..out_size - total_delta_data_size];
                self.decompress_entry_from_data_offset(base_entry.data_offset, inflate, out_base)?;
            }

            (first_buffer_size, second_buffer_end)
        };

        // From oldest to most recent, apply all deltas, swapping the buffer back and forth
        // TODO: once we have more tests, we could optimize this memory-intensive work to
        //       analyse the delta-chains to only copy data once - after all, with 'copy-from-base' deltas,
        //       all data originates from one base at some point.
        // `out` is: [source-buffer][target-buffer][max-delta-instructions-buffer]
        let (buffers, instructions) = out.split_at_mut(second_buffer_end);
        let (mut source_buf, mut target_buf) = buffers.split_at_mut(first_buffer_end);

        let mut last_result_size = None;
        for (
            delta_idx,
            Delta {
                data,
                base_size,
                result_size,
                ..
            },
        ) in chain.into_iter().rev().enumerate()
        {
            let data = &mut instructions[data];
            if delta_idx + 1 == chain_len {
                last_result_size = Some(result_size);
            }
            delta::apply(&source_buf[..base_size], &mut target_buf[..result_size], data)?;
            // use the target as source for the next delta
            std::mem::swap(&mut source_buf, &mut target_buf);
        }

        let last_result_size = last_result_size.expect("at least one delta chain item");
        // uneven chains leave the target buffer after the source buffer
        // FIXME(Performance) If delta-chains are uneven, we know we will have to copy bytes over here
        //      Instead we could use a different start buffer, to naturally end up with the result in the
        //      right one.
        //      However, this is a bit more complicated than just that - you have to deal with the base
        //      object, which should also be placed in the second buffer right away. You don't have that
        //      control/knowledge for out-of-pack bases, so this is a special case to deal with, too.
        //      Maybe these invariants can be represented in the type system though.
        if chain_len % 2 == 1 {
            // this seems inverted, but remember: we swapped the buffers on the last iteration
            target_buf[..last_result_size].copy_from_slice(&source_buf[..last_result_size]);
        }
        debug_assert!(out.len() >= last_result_size);
        out.truncate(last_result_size);

        let object_kind = object_kind.expect("a base object as root of any delta chain that we are here to resolve");
        let consumed_input = consumed_input.expect("at least one decompressed delta object");
        cache.put(
            self.id,
            first_entry.data_offset,
            out.as_slice(),
            object_kind,
            consumed_input,
        );
        Ok(Outcome {
            kind: object_kind,
            // technically depending on the cache, the chain size is not correct as it might
            // have been cut short by a cache hit. The caller must deactivate the cache to get
            // actual results
            num_deltas: chain_len as u32,
            decompressed_size: first_entry.decompressed_size,
            compressed_size: consumed_input,
            object_size: last_result_size as u64,
        })
    }
}

/// Convert user-controlled sizes from pack data into allocation sizes while enforcing the configured allocation cap.
fn decoded_object_size(size: u64, alloc_limit_bytes: Option<usize>) -> Result<usize, Error> {
    let size: usize = size.try_into().map_err(|_| Error::OutOfMemory)?;
    if alloc_limit_bytes.is_some_and(|limit| size > limit) {
        return Err(Error::OutOfMemory);
    }
    Ok(size)
}

#[cfg(test)]
mod tests {
    use gix_testtools::size_ok;

    use super::*;

    #[test]
    fn size_of_decode_entry_outcome() {
        let actual = std::mem::size_of::<Outcome>();
        let expected = 32;
        assert!(
            size_ok(actual, expected),
            "this shouldn't change without use noticing as it's returned a lot: {actual} <~ {expected}"
        );
    }
}
