/// Information for the chunk about index names
pub mod index_names {
    use std::path::{Path, PathBuf};

    use gix_object::bstr::{BString, ByteSlice};

    /// The ID used for the index-names chunk.
    pub const ID: gix_chunk::Id = *b"PNAM";

    ///
    pub mod decode {
        use std::collections::TryReserveError;

        use gix_object::bstr::BString;

        /// The error returned by [`from_bytes()`][super::from_bytes()].
        #[derive(Debug, thiserror::Error)]
        #[allow(missing_docs)]
        pub enum Error {
            #[error("The pack names were not ordered alphabetically.")]
            NotOrderedAlphabetically,
            #[error("Each pack path name must be terminated with a null byte")]
            MissingNullByte,
            #[error("Entry too large to fit in memory")]
            OutOfMemory,
            #[error("Couldn't turn path '{path}' into OS path due to encoding issues")]
            PathEncoding { path: BString },
            #[error("non-padding bytes found after all paths were read.")]
            UnknownTrailerBytes,
        }

        impl From<TryReserveError> for Error {
            #[cold]
            fn from(_: TryReserveError) -> Self {
                Self::OutOfMemory
            }
        }
    }

    /// Parse null-separated index names from the given `chunk` of bytes.
    ///
    /// `chunk`
    ///: Contains `num_packs` null-terminated index names, optionally followed by padding bytes which are typically `\0`.
    ///
    /// `num_packs`
    ///: The number of index names expected to be present in `chunk`.
    ///
    /// `alloc_limit_bytes`
    ///: Limits allocations caused by attacker-controlled on-disk multi-index data.
    ///  It is used to reject reserving the output `Vec<PathBuf>` if its capacity estimate exceeds the limit,
    ///  and to reject any single path entry whose byte length exceeds the limit before turning it into a `PathBuf`.
    ///  Use `None` to disable this limit.
    pub fn from_bytes(
        mut chunk: &[u8],
        num_packs: u32,
        alloc_limit_bytes: Option<usize>,
    ) -> Result<Vec<PathBuf>, decode::Error> {
        let mut out = Vec::new();
        let num_packs = usize::try_from(num_packs).map_err(|_| decode::Error::OutOfMemory)?;
        let vec_allocation = num_packs
            .checked_mul(std::mem::size_of::<PathBuf>())
            .ok_or(decode::Error::OutOfMemory)?;
        if alloc_limit_bytes.is_some_and(|limit| vec_allocation > limit) {
            return Err(decode::Error::OutOfMemory);
        }
        out.try_reserve(num_packs)?;

        for _ in 0..num_packs {
            let null_byte_pos = chunk.find_byte(b'\0').ok_or(decode::Error::MissingNullByte)?;

            let path = &chunk[..null_byte_pos];
            if alloc_limit_bytes.is_some_and(|limit| path.len() > limit) {
                return Err(decode::Error::OutOfMemory);
            }
            let path = gix_path::try_from_byte_slice(path)
                .map_err(|_| decode::Error::PathEncoding {
                    path: BString::from(path),
                })?
                .to_owned();

            if let Some(previous) = out.last() {
                if previous >= &path {
                    return Err(decode::Error::NotOrderedAlphabetically);
                }
            }
            out.push(path);

            chunk = &chunk[null_byte_pos + 1..];
        }

        if !chunk.is_empty() && !chunk.iter().all(|b| *b == 0) {
            return Err(decode::Error::UnknownTrailerBytes);
        }
        // NOTE: git writes garbage into this chunk, usually extra \0 bytes, which we simply ignore. If we were strict
        // about it we couldn't read this chunk data at all.
        Ok(out)
    }

    /// Calculate the size on disk for our chunk with the given index paths. Note that these are expected to have been processed already
    /// to actually be file names.
    pub fn storage_size(paths: impl IntoIterator<Item = impl AsRef<Path>>) -> u64 {
        let mut count = 0u64;
        for path in paths {
            let path = path.as_ref();
            let ascii_path = path.to_str().expect("UTF-8 compatible paths");
            assert!(
                ascii_path.is_ascii(),
                "must use ascii bytes for correct size computation"
            );
            count += (ascii_path.len() + 1/* null byte */) as u64;
        }

        let needed_alignment = CHUNK_ALIGNMENT - (count % CHUNK_ALIGNMENT);
        if needed_alignment < CHUNK_ALIGNMENT {
            count += needed_alignment;
        }
        count
    }

    /// Write all `paths` in order to `out`, including padding.
    pub fn write(
        paths: impl IntoIterator<Item = impl AsRef<Path>>,
        out: &mut dyn std::io::Write,
    ) -> std::io::Result<()> {
        let mut written_bytes = 0;
        for path in paths {
            let path = path.as_ref().to_str().expect("UTF-8 path");
            out.write_all(path.as_bytes())?;
            out.write_all(&[0])?;
            written_bytes += path.len() as u64 + 1;
        }

        let needed_alignment = CHUNK_ALIGNMENT - (written_bytes % CHUNK_ALIGNMENT);
        if needed_alignment < CHUNK_ALIGNMENT {
            let padding = [0u8; CHUNK_ALIGNMENT as usize];
            out.write_all(&padding[..needed_alignment as usize])?;
        }
        Ok(())
    }

    const CHUNK_ALIGNMENT: u64 = 4;
}

/// Information for the chunk with the fanout table
pub mod fanout {
    use crate::multi_index;

    /// The size of the fanout table
    pub const SIZE: usize = 4 * 256;

    /// The id uniquely identifying the fanout table.
    pub const ID: gix_chunk::Id = *b"OIDF";

    /// Decode the fanout table contained in `chunk`, or return `None` if it didn't have the expected size.
    pub fn from_bytes(chunk: &[u8]) -> Option<[u32; 256]> {
        if chunk.len() != SIZE {
            return None;
        }
        let mut out = [0; 256];
        for (c, f) in chunk.chunks_exact(4).zip(out.iter_mut()) {
            *f = u32::from_be_bytes(c.try_into().unwrap());
        }
        out.into()
    }

    /// Write the fanout for the given entries, which must be sorted by oid
    pub(crate) fn write(
        sorted_entries: &[multi_index::write::Entry],
        out: &mut dyn std::io::Write,
    ) -> std::io::Result<()> {
        let fanout = crate::index::encode::fanout(&mut sorted_entries.iter().map(|e| e.id.first_byte()));

        for value in fanout.iter() {
            out.write_all(&value.to_be_bytes())?;
        }
        Ok(())
    }
}

/// Information about the oid lookup table.
pub mod lookup {
    use std::ops::Range;

    use crate::multi_index;

    /// The id uniquely identifying the oid lookup table.
    pub const ID: gix_chunk::Id = *b"OIDL";

    /// Return the number of bytes needed to store the data on disk for the given amount of `entries`
    pub fn storage_size(entries: usize, object_hash: gix_hash::Kind) -> u64 {
        (entries * object_hash.len_in_bytes()) as u64
    }

    pub(crate) fn write(
        sorted_entries: &[multi_index::write::Entry],
        out: &mut dyn std::io::Write,
    ) -> std::io::Result<()> {
        for entry in sorted_entries {
            out.write_all(entry.id.as_slice())?;
        }
        Ok(())
    }

    /// Return true if the size of the `offset` range seems to match for a `hash` of the given kind and the amount of objects.
    pub fn is_valid(offset: &Range<usize>, hash: gix_hash::Kind, num_objects: u32) -> bool {
        (offset.end - offset.start) == (num_objects as usize).saturating_mul(hash.len_in_bytes())
    }
}

/// Information about the offsets table.
pub mod offsets {
    use std::ops::Range;

    use crate::multi_index;

    /// The id uniquely identifying the offsets table.
    pub const ID: gix_chunk::Id = *b"OOFF";

    /// Return the amount of bytes needed to offset data for `entries`.
    pub fn storage_size(entries: usize) -> u64 {
        (entries * (4 /*pack-id*/ + 4/* pack offset */)) as u64
    }

    pub(crate) fn write(
        sorted_entries: &[multi_index::write::Entry],
        large_offsets_needed: bool,
        out: &mut dyn std::io::Write,
    ) -> std::io::Result<()> {
        use crate::index::encode::{HIGH_BIT, LARGE_OFFSET_THRESHOLD};
        let mut num_large_offsets = 0u32;

        for entry in sorted_entries {
            out.write_all(&entry.pack_index.to_be_bytes())?;

            let offset: u32 = if large_offsets_needed {
                if entry.pack_offset > LARGE_OFFSET_THRESHOLD {
                    let res = num_large_offsets | HIGH_BIT;
                    num_large_offsets += 1;
                    res
                } else {
                    entry.pack_offset as u32
                }
            } else {
                entry
                    .pack_offset
                    .try_into()
                    .expect("without large offsets, pack-offset fits u32")
            };
            out.write_all(&offset.to_be_bytes())?;
        }
        Ok(())
    }

    /// Returns true if the `offset` range seems to match the size required for the untrusted `num_objects`.
    pub fn is_valid(offset: &Range<usize>, num_objects: u32) -> bool {
        let entry_size = 4 /* pack-id */ + 4 /* pack-offset */;
        (offset.end - offset.start) == (num_objects as usize).saturating_mul(entry_size)
    }
}

/// Information about the large offsets table.
pub mod large_offsets {
    use std::ops::Range;

    use crate::{index::encode::LARGE_OFFSET_THRESHOLD, multi_index};

    /// The id uniquely identifying the large offsets table (with 64 bit offsets)
    pub const ID: gix_chunk::Id = *b"LOFF";

    /// Returns Some(num-large-offset) if there are offsets larger than u32.
    pub(crate) fn num_large_offsets(entries: &[multi_index::write::Entry]) -> Option<usize> {
        let mut num_large_offsets = 0;
        let mut needs_large_offsets = false;
        for entry in entries {
            if entry.pack_offset > LARGE_OFFSET_THRESHOLD {
                num_large_offsets += 1;
            }
            if entry.pack_offset > crate::data::Offset::from(u32::MAX) {
                needs_large_offsets = true;
            }
        }

        needs_large_offsets.then_some(num_large_offsets)
    }
    /// Returns true if the `offsets` range seems to be properly aligned for the data we expect.
    pub fn is_valid(offset: &Range<usize>) -> bool {
        (offset.end - offset.start) % 8 == 0
    }

    pub(crate) fn write(
        sorted_entries: &[multi_index::write::Entry],
        mut num_large_offsets: usize,
        out: &mut dyn std::io::Write,
    ) -> std::io::Result<()> {
        for offset in sorted_entries
            .iter()
            .filter_map(|e| (e.pack_offset > LARGE_OFFSET_THRESHOLD).then_some(e.pack_offset))
        {
            out.write_all(&offset.to_be_bytes())?;
            num_large_offsets = num_large_offsets
                .checked_sub(1)
                .expect("BUG: wrote more offsets the previously found");
        }
        assert_eq!(num_large_offsets, 0, "BUG: wrote less offsets than initially counted");
        Ok(())
    }

    /// Return the number of bytes needed to store the given amount of `large_offsets`
    pub(crate) fn storage_size(large_offsets: usize) -> u64 {
        8 * large_offsets as u64
    }
}
