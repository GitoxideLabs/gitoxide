use std::{mem::size_of, ops::Range};

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
    fn iter_v1(&self) -> impl Iterator<Item = Entry> + '_ {
        match self.version {
            index::Version::V1 => self.data[V1_HEADER_SIZE..]
                .chunks_exact(N32_SIZE + self.hash_len)
                .take(self.num_objects as usize)
                .map(|c| {
                    let (ofs, oid) = c.split_at(N32_SIZE);
                    Entry {
                        oid: gix_hash::ObjectId::from_bytes_or_panic(oid),
                        pack_offset: u64::from(crate::read_u32(ofs)),
                        crc32: None,
                    }
                }),
            _ => panic!("Cannot use iter_v1() on index of type {:?}", self.version),
        }
    }

    fn iter_v2(&self) -> impl Iterator<Item = Entry> + '_ {
        let large_offsets = self.large_offsets_v2();
        let oids = self.data[V2_HEADER_SIZE..]
            .chunks_exact(self.hash_len)
            .take(self.num_objects as usize);
        let crcs = self.data[self.offset_crc32_v2()..]
            .as_chunks::<N32_SIZE>()
            .0
            .iter()
            .take(self.num_objects as usize);
        let offsets = self.data[self.offset_pack_offset_v2()..]
            .as_chunks::<N32_SIZE>()
            .0
            .iter()
            .take(self.num_objects as usize);
        assert_eq!(oids.len(), crcs.len());
        assert_eq!(crcs.len(), offsets.len());
        match self.version {
            index::Version::V2 => izip!(oids, crcs, offsets).map(move |(oid, crc32, ofs32)| Entry {
                oid: gix_hash::ObjectId::from_bytes_or_panic(oid),
                pack_offset: pack_offset_v2(ofs32, large_offsets).expect("validated large-offset reference"),
                crc32: Some(crate::read_u32(crc32)),
            }),
            _ => panic!("Cannot use iter_v2() on index of type {:?}", self.version),
        }
    }

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
    /// # Panics
    ///
    /// If `index` is out of bounds.
    pub fn pack_offset_at_index(&self, index: EntryIndex) -> data::Offset {
        self.checked_pack_offset_at_index(index)
            .expect("in-bounds entry of an index with validated offsets")
    }

    fn checked_pack_offset_at_index(&self, index: EntryIndex) -> Option<data::Offset> {
        if index >= self.num_objects {
            return None;
        }
        let index = index as usize;
        match self.version {
            index::Version::V2 => {
                let start = self.offset_pack_offset_v2() + index * N32_SIZE;
                pack_offset_v2(&self.data[start..][..N32_SIZE], self.large_offsets_v2())
            }
            index::Version::V1 => {
                let start = V1_HEADER_SIZE + index * (N32_SIZE + self.hash_len);
                Some(u64::from(crate::read_u32(&self.data[start..][..N32_SIZE])))
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

    /// An iterator over all [`Entries`][Entry] of this index file.
    pub fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = Entry> + 'a> {
        match self.version {
            index::Version::V2 => Box::new(self.iter_v2()),
            index::Version::V1 => Box::new(self.iter_v1()),
        }
    }

    /// Return a vector of ascending offsets into our respective pack data file.
    ///
    /// Useful to control an iteration over all pack entries in a cache-friendly way.
    pub fn sorted_offsets(&self) -> Vec<data::Offset> {
        let mut ofs: Vec<_> = match self.version {
            index::Version::V1 => self.iter().map(|e| e.pack_offset).collect(),
            index::Version::V2 => {
                let offset32_start = &self.data[self.offset_pack_offset_v2()..];
                let offsets32 = offset32_start
                    .as_chunks::<N32_SIZE>()
                    .0
                    .iter()
                    .take(self.num_objects as usize);
                assert_eq!(self.num_objects as usize, offsets32.len());
                let large_offsets = self.large_offsets_v2();
                offsets32
                    .map(|offset| pack_offset_v2(offset, large_offsets).expect("validated large-offset reference"))
                    .collect()
            }
        };
        ofs.sort_unstable();
        ofs
    }

    #[inline]
    fn offset_crc32_v2(&self) -> usize {
        V2_HEADER_SIZE + self.num_objects as usize * self.hash_len
    }

    #[inline]
    pub(super) fn offset_pack_offset_v2(&self) -> usize {
        self.offset_crc32_v2() + self.num_objects as usize * N32_SIZE
    }

    #[inline]
    pub(super) fn offset_pack_offset64_v2(&self) -> usize {
        self.offset_pack_offset_v2() + self.num_objects as usize * N32_SIZE
    }

    #[inline]
    fn large_offsets_v2(&self) -> &[[u8; N64_SIZE]] {
        debug_assert_eq!(self.version, index::Version::V2);
        let table_end = self.data.len() - 2 * self.hash_len;
        self.data[self.offset_pack_offset64_v2()..table_end]
            .as_chunks::<N64_SIZE>()
            .0
    }
}

#[inline]
fn pack_offset_v2(offset: &[u8], large_offsets: &[[u8; N64_SIZE]]) -> Option<data::Offset> {
    let offset = crate::read_u32(offset);
    if offset & N32_HIGH_BIT == 0 {
        Some(u64::from(offset))
    } else {
        large_offsets
            .get((offset ^ N32_HIGH_BIT) as usize)
            .copied()
            .map(u64::from_be_bytes)
    }
}

impl<T: crate::FileData> index::LookupFile<T> {
    /// Return the number of indexed objects.
    pub fn num_objects(&self) -> EntryIndex {
        self.file.num_objects()
    }

    /// Return the object ID at `index`, which must be less than [`Self::num_objects()`].
    pub fn oid_at_index(&self, index: EntryIndex) -> &gix_hash::oid {
        self.file.oid_at_index(index)
    }

    /// Find the entry for `id` in the index.
    pub fn lookup(&self, id: impl AsRef<gix_hash::oid>) -> Option<EntryIndex> {
        self.file.lookup(id)
    }

    /// Look up a prefix; see [`index::File::lookup_prefix()`].
    pub fn lookup_prefix(
        &self,
        prefix: gix_hash::Prefix,
        candidates: Option<&mut Range<EntryIndex>>,
    ) -> Option<PrefixLookupResult> {
        self.file.lookup_prefix(prefix, candidates)
    }

    /// Read an offset, reporting an invalid entry or large-offset reference before accessing pack data.
    pub fn pack_offset_at_index(&self, index: EntryIndex) -> gix_error::Result<data::Offset> {
        self.file.checked_pack_offset_at_index(index).ok_or_else(|| {
            gix_error::corruption(format!("Invalid offset reference at pack index entry {index}")).into()
        })
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
