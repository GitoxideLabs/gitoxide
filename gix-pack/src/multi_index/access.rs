use std::{
    ops::Range,
    path::{Path, PathBuf},
};

use gix_error::{Result, bail, corruption, validation};

use crate::{
    data,
    index::PrefixLookupResult,
    multi_index::{Entry, EntryIndex, File, PackIndex, Version},
};

/// Access methods
impl<T> File<T>
where
    T: crate::FileData,
{
    /// Returns the version of the multi-index file.
    pub fn version(&self) -> Version {
        self.version
    }
    /// Returns the path from which the multi-index file was loaded.
    ///
    /// Note that it might have changed in the mean time, or might have been removed as well.
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// Returns the amount of indices stored in this multi-index file. It's the same as [File::index_names().len()][File::index_names()],
    /// and returned as one past the highest known index.
    pub fn num_indices(&self) -> PackIndex {
        self.num_indices
    }
    /// Returns the total amount of objects available for lookup, and returned as one past the highest known entry index
    pub fn num_objects(&self) -> EntryIndex {
        self.num_objects
    }
    /// Returns the kind of hash function used for object ids available in this index.
    pub fn object_hash(&self) -> gix_hash::Kind {
        self.object_hash
    }
    /// Returns the checksum over the entire content of the file (excluding the checksum itself).
    ///
    /// It can be used to validate it didn't change after creation.
    pub fn checksum(&self) -> gix_hash::ObjectId {
        gix_hash::ObjectId::from_bytes_or_panic(&self.data[self.data.len() - self.hash_len..])
    }
    /// Return all names of index files (`*.idx`) whose objects we contain.
    ///
    /// The corresponding pack can be found by replacing the `.idx` extension with `.pack`.
    pub fn index_names(&self) -> &[PathBuf] {
        &self.index_names
    }
}

impl<T> File<T>
where
    T: crate::FileData,
{
    /// Return the object id at the given `index`, which ranges from 0 to [File::num_objects()].
    pub fn oid_at_index(&self, index: EntryIndex) -> &gix_hash::oid {
        debug_assert!(index < self.num_objects, "index out of bounds");
        let index: usize = index as usize;
        let start = self.lookup_ofs + index * self.hash_len;
        gix_hash::oid::from_bytes_unchecked(&self.data[start..][..self.hash_len])
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
        crate::index::access::lookup_prefix(
            prefix,
            candidates,
            &self.fan,
            &|idx| self.oid_at_index(idx),
            self.num_objects,
        )
    }

    /// Find the index ranging from 0 to [File::num_objects()] that belongs to data associated with `id`, or `None` if it wasn't found.
    ///
    /// Use this index for finding additional information via [`File::pack_id_and_pack_offset_at_index()`].
    pub fn lookup(&self, id: impl AsRef<gix_hash::oid>) -> Option<EntryIndex> {
        crate::index::access::lookup(id.as_ref(), &self.fan, &|idx| self.oid_at_index(idx))
    }

    /// Given the `index` ranging from 0 to [File::num_objects()], return the pack index and its absolute offset into the pack.
    ///
    /// The pack-index refers to an entry in the [`index_names`][File::index_names()] list, from which the pack can be derived.
    ///
    /// Returns an error if `index` is out of bounds or its pack or large-offset reference is corrupt.
    pub fn pack_id_and_pack_offset_at_index(&self, index: EntryIndex) -> Result<(PackIndex, data::Offset)> {
        if index >= self.num_objects {
            bail!(validation("Multi-index entry index is out of bounds"));
        }
        const OFFSET_ENTRY_SIZE: usize = 4 + 4;
        let index = index as usize;
        let start = self.offsets_ofs + index * OFFSET_ENTRY_SIZE;

        const HIGH_BIT: u32 = 1 << 31;

        let pack_index = crate::read_u32(&self.data[start..][..4]);
        if pack_index >= self.num_indices {
            bail!(corruption("Multi-index pack index is out of bounds"));
        }
        let offset = &self.data[start + 4..][..4];
        let ofs32 = crate::read_u32(offset);
        let pack_offset = if (ofs32 & HIGH_BIT) == HIGH_BIT {
            // We determine if large offsets are actually larger than 4GB and if not, we don't use the high-bit to signal anything
            // but allow the presence of the large-offset chunk to signal what's happening.
            if let Some(offsets_64) = &self.large_offsets {
                let ordinal = (ofs32 ^ HIGH_BIT) as usize;
                if ordinal >= offsets_64.len() / 8 {
                    bail!(corruption("Multi-index large-offset index is out of bounds"));
                }
                let from = offsets_64.start + ordinal * 8;
                crate::read_u64(&self.data[from..][..8])
            } else {
                u64::from(ofs32)
            }
        } else {
            u64::from(ofs32)
        };
        Ok((pack_index, pack_offset))
    }

    /// Return an iterator over all entries within this file, reporting corrupt references as errors.
    pub fn iter(&self) -> impl Iterator<Item = Result<Entry>> + '_ {
        (0..self.num_objects).map(move |idx| {
            let (pack_index, pack_offset) = self.pack_id_and_pack_offset_at_index(idx)?;
            Ok(Entry {
                oid: self.oid_at_index(idx).to_owned(),
                pack_offset,
                pack_index,
            })
        })
    }
}
