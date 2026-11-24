use std::{
    mem::size_of,
    ops::Range,
    sync::atomic::{AtomicBool, Ordering},
};

use gix_error::{OptionExt, Result, bail, cancelled, message, validation};

use crate::{
    data,
    index::{self, EntryIndex, FAN_LEN, PrefixLookupResult},
};

const N32_SIZE: usize = size_of::<u32>();
const N64_SIZE: usize = size_of::<u64>();
const V1_HEADER_SIZE: usize = FAN_LEN * N32_SIZE;
const V2_HEADER_SIZE: usize = N32_SIZE * 2 + FAN_LEN * N32_SIZE;
const N32_HIGH_BIT: u32 = 1 << 31;

/// Represents an entry within a pack index file, effectively mapping object [`IDs`][gix_hash::ObjectId] to pack data file locations.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Entry {
    /// The ID of the object
    pub oid: gix_hash::ObjectId,
    /// The offset to the object's header in the pack data file
    pub pack_offset: data::Offset,
    /// The CRC32 hash over all bytes of the pack data entry.
    ///
    /// This can be useful for direct copies of pack data entries from one pack to another with insurance there was no bit rot.
    /// _Note_: Only available in index version 2 or newer
    pub crc32: Option<u32>,
}

/// Iteration and access
impl<T> index::File<T>
where
    T: crate::FileData,
{
    /// Returns the object hash at the given index in our list of (sorted) sha1 hashes.
    /// The index ranges from 0 to `self.num_objects()`
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    pub fn oid_at_index(&self, index: EntryIndex) -> &gix_hash::oid {
        let index = index as usize;
        let start = match self.version {
            index::Version::V2 => V2_HEADER_SIZE + index * self.hash_len,
            index::Version::V1 => V1_HEADER_SIZE + index * (N32_SIZE + self.hash_len) + N32_SIZE,
        };
        gix_hash::oid::from_bytes_unchecked(&self.data[start..][..self.hash_len])
    }

    /// Returns the offset into our pack data file at which to start reading the object at `index`.
    ///
    /// Returns an error if `index` is out of bounds or its large-offset reference is corrupt.
    pub fn pack_offset_at_index(&self, index: EntryIndex) -> Result<data::Offset> {
        if index >= self.num_objects {
            bail!(validation("Pack index entry index is out of bounds"));
        }
        let index = index as usize;
        match self.version {
            index::Version::V2 => {
                let start = self.offset_pack_offset_v2() + index * N32_SIZE;
                let offset = crate::read_u32(&self.data[start..][..N32_SIZE]);
                if offset & N32_HIGH_BIT == 0 {
                    return Ok(u64::from(offset));
                }
                let slot = (offset ^ N32_HIGH_BIT) as usize;
                self.large_offsets_v2()
                    .get(slot)
                    .map(|offset| u64::from_be_bytes(*offset))
                    .ok_or_raise(|| {
                        message!("Pack index entry {index} references missing large offset {slot}").corrupted()
                    })
            }
            index::Version::V1 => {
                let start = V1_HEADER_SIZE + index * (N32_SIZE + self.hash_len);
                Ok(u64::from(crate::read_u32(&self.data[start..][..N32_SIZE])))
            }
        }
    }

    /// Returns the CRC32 of the object at the given `index`.
    ///
    /// _Note_: These are always present for index version 2 or higher.
    /// # Panics
    ///
    /// If `index` is out of bounds.
    pub fn crc32_at_index(&self, index: EntryIndex) -> Option<u32> {
        let index = index as usize;
        match self.version {
            index::Version::V2 => {
                let start = self.offset_crc32_v2() + index * N32_SIZE;
                Some(crate::read_u32(&self.data[start..start + N32_SIZE]))
            }
            index::Version::V1 => None,
        }
    }

    /// Returns the `index` of the given hash for use with the [`oid_at_index()`][index::File::oid_at_index()],
    /// [`pack_offset_at_index()`][index::File::pack_offset_at_index()] or [`crc32_at_index()`][index::File::crc32_at_index()].
    // NOTE: pretty much the same things as in `multi_index::File::lookup`, change things there
    //       as well.
    pub fn lookup(&self, id: impl AsRef<gix_hash::oid>) -> Option<EntryIndex> {
        lookup(id.as_ref(), &self.fan, &|idx| self.oid_at_index(idx))
    }

    /// Given a `prefix`, find an object that matches it uniquely within this index and return `Some(Ok(entry_index))`.
    /// If there is more than one object matching the object `Some(Err(())` is returned.
    ///
    /// Finally, if no object matches the index, the return value is `None`.
    ///
    /// Pass `candidates` to obtain the set of entry-indices matching `prefix`, with the same return value as
    /// one would have received if it remained `None`. It will be empty if no object matched the `prefix`.
    ///
    // NOTE: pretty much the same things as in `index::File::lookup`, change things there
    //       as well.
    pub fn lookup_prefix(
        &self,
        prefix: gix_hash::Prefix,
        candidates: Option<&mut Range<EntryIndex>>,
    ) -> Option<PrefixLookupResult> {
        lookup_prefix(
            prefix,
            candidates,
            &self.fan,
            &|idx| self.oid_at_index(idx),
            self.num_objects,
        )
    }

    /// Iterate over all entries in this index, reporting corrupt offset references as errors.
    pub fn iter(&self) -> impl Iterator<Item = Result<Entry>> + '_ {
        (0..self.num_objects).map(|index| {
            Ok(Entry {
                pack_offset: self.pack_offset_at_index(index)?,
                oid: self.oid_at_index(index).to_owned(),
                crc32: self.crc32_at_index(index),
            })
        })
    }

    /// Return a vector of ascending offsets into our respective pack data file.
    ///
    /// Useful to control an iteration over all pack entries in a cache-friendly way.
    ///
    /// Returns an error if any offset reference is corrupt or the offsets cannot be allocated.
    pub fn sorted_offsets(&self) -> Result<Vec<data::Offset>> {
        let mut offsets = crate::try_exact_vec(self.num_objects as usize, None)?;
        for index in 0..self.num_objects {
            offsets.push(self.pack_offset_at_index(index)?);
        }
        offsets.sort_unstable();
        Ok(offsets)
    }

    #[inline]
    fn offset_crc32_v2(&self) -> usize {
        V2_HEADER_SIZE + self.num_objects as usize * self.hash_len
    }

    #[inline]
    fn offset_pack_offset_v2(&self) -> usize {
        self.offset_crc32_v2() + self.num_objects as usize * N32_SIZE
    }

    #[inline]
    fn offset_pack_offset64_v2(&self) -> usize {
        self.offset_pack_offset_v2() + self.num_objects as usize * N32_SIZE
    }

    #[inline]
    fn large_offsets_v2(&self) -> &[[u8; N64_SIZE]] {
        let table_end = self.data.len() - 2 * self.hash_len;
        self.data[self.offset_pack_offset64_v2()..table_end]
            .as_chunks::<N64_SIZE>()
            .0
    }

    /// Check large-offset references and the correspondence between the two offset tables.
    pub(crate) fn verify_offsets(&self, should_interrupt: &AtomicBool) -> Result {
        if should_interrupt.load(Ordering::Relaxed) {
            bail!(cancelled("Interrupted"));
        }
        if self.version == index::Version::V2 {
            let offsets = &self.data[self.offset_pack_offset_v2()..self.offset_pack_offset64_v2()];
            let mut references = 0;
            for (index, offset) in offsets.as_chunks::<N32_SIZE>().0.iter().enumerate() {
                if index.is_multiple_of(10_000) && should_interrupt.load(Ordering::Relaxed) {
                    bail!(cancelled("Interrupted"));
                }
                if crate::read_u32(offset) & N32_HIGH_BIT != 0 {
                    self.pack_offset_at_index(index as EntryIndex)?;
                    references += 1;
                }
            }
            let large_offsets = self.large_offsets_v2().len();
            if references != large_offsets {
                bail!("Pack index has {large_offsets} large offsets for {references} references".corrupted());
            }
        }
        Ok(())
    }
}

pub(crate) fn lookup_prefix<'a>(
    prefix: gix_hash::Prefix,
    candidates: Option<&mut Range<EntryIndex>>,
    fan: &[u32; FAN_LEN],
    oid_at_index: &dyn Fn(EntryIndex) -> &'a gix_hash::oid,
    num_objects: u32,
) -> Option<PrefixLookupResult> {
    let first_byte = prefix.as_oid().first_byte() as usize;
    let mut upper_bound = fan[first_byte];
    let mut lower_bound = if first_byte != 0 { fan[first_byte - 1] } else { 0 };

    // Bisect using indices
    while lower_bound < upper_bound {
        let mid = u32::midpoint(lower_bound, upper_bound);
        let mid_sha = oid_at_index(mid);

        use std::cmp::Ordering::*;
        match prefix.cmp_oid(mid_sha) {
            Less => upper_bound = mid,
            Equal => match candidates {
                Some(candidates) => {
                    let first_past_entry = ((0..mid).rev())
                        .take_while(|prev| prefix.cmp_oid(oid_at_index(*prev)) == Equal)
                        .last();

                    let last_future_entry = ((mid + 1)..num_objects)
                        .take_while(|next| prefix.cmp_oid(oid_at_index(*next)) == Equal)
                        .last();

                    *candidates = match (first_past_entry, last_future_entry) {
                        (Some(first), Some(last)) => first..last + 1,
                        (Some(first), None) => first..mid + 1,
                        (None, Some(last)) => mid..last + 1,
                        (None, None) => mid..mid + 1,
                    };

                    return if candidates.len() > 1 {
                        Some(Err(()))
                    } else {
                        Some(Ok(mid))
                    };
                }
                None => {
                    let next = mid + 1;
                    if next < num_objects && prefix.cmp_oid(oid_at_index(next)) == Equal {
                        return Some(Err(()));
                    }
                    if mid != 0 && prefix.cmp_oid(oid_at_index(mid - 1)) == Equal {
                        return Some(Err(()));
                    }
                    return Some(Ok(mid));
                }
            },
            Greater => lower_bound = mid + 1,
        }
    }

    if let Some(candidates) = candidates {
        *candidates = 0..0;
    }
    None
}

pub(crate) fn lookup<'a>(
    id: &gix_hash::oid,
    fan: &[u32; FAN_LEN],
    oid_at_index: &dyn Fn(EntryIndex) -> &'a gix_hash::oid,
) -> Option<EntryIndex> {
    let first_byte = id.first_byte() as usize;
    let mut upper_bound = fan[first_byte];
    let mut lower_bound = if first_byte != 0 { fan[first_byte - 1] } else { 0 };

    while lower_bound < upper_bound {
        let mid = u32::midpoint(lower_bound, upper_bound);
        let mid_sha = oid_at_index(mid);

        use std::cmp::Ordering::*;
        match id.cmp(mid_sha) {
            Less => upper_bound = mid,
            Equal => return Some(mid),
            Greater => lower_bound = mid + 1,
        }
    }
    None
}
