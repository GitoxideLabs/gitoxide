use std::path::Path;

use crate::multi_index::{chunk, File, Version};

mod error {
    use crate::multi_index::chunk;

    /// The error returned by [File::at()][super::File::at()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Could not open multi-index file at '{path}'")]
        Io {
            source: std::io::Error,
            path: std::path::PathBuf,
        },
        #[error("{message}")]
        Corrupt { message: &'static str },
        #[error("Unsupported multi-index version: {version})")]
        UnsupportedVersion { version: u8 },
        #[error("Unsupported hash kind: {kind})")]
        UnsupportedObjectHash { kind: u8 },
        #[error(transparent)]
        ChunkFileQuery(#[from] gix_error::Message),
        #[error(transparent)]
        ChunkFileDecode(#[from] gix_error::ParseError),
        #[error("The multi-pack fan doesn't have the correct size of 256 * 4 bytes")]
        MultiPackFanSize,
        #[error(transparent)]
        PackNames(#[from] chunk::index_names::decode::Error),
        #[error("multi-index chunk {:?} has invalid size: {message}", String::from_utf8_lossy(.id))]
        InvalidChunkSize { id: gix_chunk::Id, message: &'static str },
    }
}

pub use error::Error;

/// Initialization
impl File {
    /// Open the multi-index file at the given `path`.
    pub fn at(path: impl AsRef<Path>) -> Result<Self, Error> {
        Self::try_from(path.as_ref())
    }
}

impl TryFrom<&Path> for File {
    type Error = Error;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let data = crate::mmap::read_only(path).map_err(|source| Error::Io {
            source,
            path: path.to_owned(),
        })?;

        const TRAILER_LEN: usize = gix_hash::Kind::shortest().len_in_bytes(); /* trailing hash */
        if data.len()
            < Self::HEADER_LEN
                + gix_chunk::file::Index::size_for_entries(4 /*index names, fan, offsets, oids*/)
                + chunk::fanout::SIZE
                + TRAILER_LEN
        {
            return Err(Error::Corrupt {
                message: "multi-index file is truncated and too short",
            });
        }

        let (version, object_hash, num_chunks, num_indices) = {
            let (signature, data) = data.split_at(4);
            if signature != Self::SIGNATURE {
                return Err(Error::Corrupt {
                    message: "Invalid signature",
                });
            }
            let (version, data) = data.split_at(1);
            let version = match version[0] {
                1 => Version::V1,
                version => return Err(Error::UnsupportedVersion { version }),
            };

            let (object_hash, data) = data.split_at(1);
            let object_hash = gix_hash::Kind::try_from(object_hash[0])
                .map_err(|unknown| Error::UnsupportedObjectHash { kind: unknown })?;
            let (num_chunks, data) = data.split_at(1);
            let num_chunks = num_chunks[0];

            let (_num_base_files, data) = data.split_at(1); // TODO: handle base files once it's clear what this does

            let (num_indices, _) = data.split_at(4);
            let num_indices = crate::read_u32(num_indices);

            (version, object_hash, num_chunks, num_indices)
        };

        let chunks = gix_chunk::file::Index::from_bytes(&data, Self::HEADER_LEN, u32::from(num_chunks))?;

        let index_names = chunks.data_by_id(&data, chunk::index_names::ID)?;
        let index_names = chunk::index_names::from_bytes(index_names, num_indices)?;

        let fan = chunks.data_by_id(&data, chunk::fanout::ID)?;
        let fan = chunk::fanout::from_bytes(fan).ok_or(Error::MultiPackFanSize)?;
        let num_objects = fan[255];

        let lookup = chunks.validated_usize_offset_by_id(chunk::lookup::ID, |offset| {
            chunk::lookup::is_valid(&offset, object_hash, num_objects)
                .then_some(offset)
                .ok_or(Error::InvalidChunkSize {
                    id: chunk::lookup::ID,
                    message: "The chunk with alphabetically ordered object ids doesn't have the correct size",
                })
        })??;
        let offsets = chunks.validated_usize_offset_by_id(chunk::offsets::ID, |offset| {
            chunk::offsets::is_valid(&offset, num_objects)
                .then_some(offset)
                .ok_or(Error::InvalidChunkSize {
                    id: chunk::offsets::ID,
                    message: "The chunk with offsets into the pack doesn't have the correct size",
                })
        })??;
        let large_offsets = chunks
            .validated_usize_offset_by_id(chunk::large_offsets::ID, |offset| {
                chunk::large_offsets::is_valid(&offset)
                    .then_some(offset)
                    .ok_or(Error::InvalidChunkSize {
                        id: chunk::large_offsets::ID,
                        message: "The chunk with large offsets into the pack doesn't have the correct size",
                    })
            })
            .ok()
            .transpose()?;

        let checksum_offset = chunks.highest_offset() as usize;
        let trailer = &data[checksum_offset..];
        if trailer.len() != object_hash.len_in_bytes() {
            return Err(Error::Corrupt {
                message:
                    "Trailing checksum didn't have the expected size or there were unknown bytes after the checksum.",
            });
        }

        Ok(File {
            data,
            path: path.to_owned(),
            version,
            hash_len: object_hash.len_in_bytes(),
            object_hash,
            fan,
            index_names,
            lookup_ofs: lookup.start,
            offsets_ofs: offsets.start,
            large_offsets_ofs: large_offsets.map(|r| r.start),
            num_objects,
            num_indices,
        })
    }
}
