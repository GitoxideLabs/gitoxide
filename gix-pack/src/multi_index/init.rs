use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use gix_error::{CorruptionError, ErrorExt, ResultExt, ValidationError, message};

use crate::multi_index::{File, Version, chunk};

/// The error returned by [File::at()].
pub type Error = gix_error::Exn;

fn corrupt(message: impl Into<Cow<'static, str>>) -> Error {
    CorruptionError::new(message).raise_erased()
}

/// Initialization
impl File<crate::MMap> {
    /// Open the multi-index file at the given `path`.
    ///
    /// `alloc_limit_bytes` bounds each allocation caused by user-controlled on-disk data, useful for untrusted input.
    /// Use `None` to disable the limit.
    pub fn at(path: impl AsRef<Path>, alloc_limit_bytes: Option<usize>) -> Result<Self, Error> {
        Self::at_inner(path.as_ref(), alloc_limit_bytes)
    }

    fn at_inner(path: &Path, alloc_limit_bytes: Option<usize>) -> Result<Self, Error> {
        let data = crate::mmap::read_only(path)
            .or_raise_erased(|| message!("Could not open multi-index file at '{}'", path.display()))?;
        Self::from_data(data, path.to_owned(), alloc_limit_bytes)
    }
}

impl<T> File<T>
where
    T: crate::FileData,
{
    /// Instantiate a multi-index file from `data` as assumed to be read or memory-mapped from `path`.
    ///
    /// `alloc_limit_bytes` bounds each allocation caused by untrusted on-disk multi-index data.
    /// Use `None` to disable the limit.
    ///
    ///  It is used to reject reserving the output `Vec<PathBuf>` if its capacity estimate exceeds the limit,
    ///  and to reject any single path entry whose byte length exceeds the limit before turning it into a `PathBuf`.
    pub fn from_data(data: T, path: PathBuf, alloc_limit_bytes: Option<usize>) -> Result<Self, Error> {
        const TRAILER_LEN: usize = gix_hash::Kind::shortest().len_in_bytes(); /* trailing hash */
        if data.len()
            < Self::HEADER_LEN
                + gix_chunk::file::Index::size_for_entries(4 /*index names, fan, offsets, oids*/)
                + chunk::fanout::SIZE
                + TRAILER_LEN
        {
            return Err(corrupt("multi-index file is truncated and too short"));
        }

        let (version, object_hash, num_chunks, num_indices) = {
            let (signature, data) = data.split_at(4);
            if signature != Self::SIGNATURE {
                return Err(corrupt("Invalid signature"));
            }
            let (version, data) = data.split_at(1);
            let version = match version[0] {
                1 => Version::V1,
                version => {
                    return Err(
                        ValidationError::new(format!("Unsupported multi-index version: {version}")).raise_erased(),
                    );
                }
            };

            let (object_hash, data) = data.split_at(1);
            let object_hash = gix_hash::Kind::try_from(object_hash[0])
                .map_err(|unknown| ValidationError::new(format!("Unsupported hash kind: {unknown}")).raise_erased())?;
            let (num_chunks, data) = data.split_at(1);
            let num_chunks = num_chunks[0];

            let (_num_base_files, data) = data.split_at(1); // TODO: handle base files once it's clear what this does

            let (num_indices, _) = data.split_at(4);
            let num_indices = crate::read_u32(num_indices);

            (version, object_hash, num_chunks, num_indices)
        };

        let chunks = gix_chunk::file::Index::from_bytes(&data, Self::HEADER_LEN, u32::from(num_chunks))
            .or_raise_erased(|| CorruptionError::new("Could not decode multi-index chunk table"))?;

        let index_names = chunks
            .data_by_id(&data, chunk::index_names::ID)
            .or_raise_erased(|| CorruptionError::new("Could not read multi-index pack names"))?;
        let index_names = chunk::index_names::from_bytes(index_names, num_indices, alloc_limit_bytes).or_erased()?;

        let fan = chunks
            .data_by_id(&data, chunk::fanout::ID)
            .or_raise_erased(|| CorruptionError::new("Could not read multi-index fan"))?;
        let fan = chunk::fanout::from_bytes(fan)
            .ok_or_else(|| corrupt("The multi-index fan doesn't have the correct size of 256 * 4 bytes"))?;
        let num_objects = fan[255];
        validate_fan(&fan)?;

        let lookup = chunks
            .validated_usize_offset_by_id(chunk::lookup::ID, |offset| {
                chunk::lookup::is_valid(&offset, object_hash, num_objects)
                    .then_some(offset)
                    .ok_or_else(|| {
                        corrupt("The chunk with alphabetically ordered object ids doesn't have the correct size")
                    })
            })
            .or_raise_erased(|| CorruptionError::new("Could not find the multi-index object-id lookup chunk"))??;
        let offsets = chunks
            .validated_usize_offset_by_id(chunk::offsets::ID, |offset| {
                chunk::offsets::is_valid(&offset, num_objects)
                    .then_some(offset)
                    .ok_or_else(|| corrupt("The chunk with offsets into the pack doesn't have the correct size"))
            })
            .or_raise_erased(|| CorruptionError::new("Could not find the multi-index pack-offset chunk"))??;
        let large_offsets = chunks
            .validated_usize_offset_by_id(chunk::large_offsets::ID, |offset| {
                chunk::large_offsets::is_valid(&offset)
                    .then_some(offset)
                    .ok_or_else(|| corrupt("The chunk with large offsets into the pack doesn't have the correct size"))
            })
            .ok()
            .transpose()?;

        let checksum_offset = chunks.highest_offset() as usize;
        let trailer = &data[checksum_offset..];
        if trailer.len() != object_hash.len_in_bytes() {
            return Err(corrupt(
                "Trailing checksum didn't have the expected size or there were unknown bytes after the checksum.",
            ));
        }

        Ok(File {
            data,
            path,
            version,
            hash_len: object_hash.len_in_bytes(),
            object_hash,
            fan,
            index_names,
            alloc_limit_bytes,
            lookup_ofs: lookup.start,
            offsets_ofs: offsets.start,
            large_offsets_ofs: large_offsets.map(|r| r.start),
            num_objects,
            num_indices,
        })
    }
}

fn validate_fan(fan: &[u32; 256]) -> Result<(), Error> {
    if !crate::fan_is_monotonically_increasing(fan) {
        return Err(corrupt("multi-index fan-out table must be monotonically increasing"));
    }
    Ok(())
}
