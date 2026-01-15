use std::path::{Path, PathBuf};

use bstr::ByteSlice;
use gix_error::message;

use crate::{
    file::{
        ChunkId, BASE_GRAPHS_LIST_CHUNK_ID, COMMIT_DATA_CHUNK_ID, COMMIT_DATA_ENTRY_SIZE_SANS_HASH,
        EXTENDED_EDGES_LIST_CHUNK_ID, FAN_LEN, HEADER_LEN, OID_FAN_CHUNK_ID, OID_LOOKUP_CHUNK_ID, SIGNATURE,
    },
    File,
};

/// The error used in [`File::at()`].
#[derive(thiserror::Error, Debug)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Commit-graph {:?} chunk contains {from_chunk} base graphs, but commit-graph file header claims {from_header} base graphs", BASE_GRAPHS_LIST_CHUNK_ID.as_bstr())]
    BaseGraphMismatch { from_header: u8, from_chunk: u32 },
    #[error("Commit-graph {:?} chunk contains {chunk1_commits} commits, but {:?} chunk contains {chunk2_commits} commits", .chunk1_id.as_bstr(), .chunk2_id.as_bstr())]
    CommitCountMismatch {
        chunk1_id: ChunkId,
        chunk1_commits: u32,
        chunk2_id: ChunkId,
        chunk2_commits: u32,
    },
    #[error("{0}")]
    Corrupt(String),
    // This error case is disabled, as git allows extra garbage in the extra edges list?
    // #[error("The last entry in commit-graph's extended edges list does is not marked as being terminal")]
    // ExtraEdgesOverflow,
    #[error("Could not open commit-graph file at '{}'", .path.display())]
    Io {
        #[source]
        err: std::io::Error,
        path: std::path::PathBuf,
    },
    #[error("{0}")]
    Trailer(String),
    #[error("Commit-graph file uses unsupported hash version: {0}")]
    UnsupportedHashVersion(u8),
    #[error("Unsupported commit-graph file version: {0}")]
    UnsupportedVersion(u8),
    #[error(transparent)]
    ChunkFileQuery(#[from] gix_error::Message),
    #[error(transparent)]
    ChunkFileDecode(#[from] gix_error::ParseError),
    #[error("Commit-graph chunk {:?} has invalid size: {msg}", .id.as_bstr())]
    InvalidChunkSize { id: ChunkId, msg: String },
}

const MIN_FILE_SIZE: usize = HEADER_LEN
    + gix_chunk::file::Index::size_for_entries(3 /*OIDF, OIDL, CDAT*/)
    + FAN_LEN * 4 /* FANOUT TABLE CHUNK OIDF */
    + gix_hash::Kind::shortest().len_in_bytes();

impl File {
    /// Try to parse the commit graph file at `path`.
    pub fn at(path: impl AsRef<Path>) -> Result<File, Error> {
        Self::try_from(path.as_ref())
    }

    /// A lower-level constructor which constructs a new instance directly from the mapping in `data`,
    /// assuming that it originated from `path`.
    ///
    /// Note that `path` is only used for verification of the hash its basename contains, but otherwise
    /// is not of importance.
    pub fn new(data: memmap2::Mmap, path: PathBuf) -> Result<File, Error> {
        let data_size = data.len();
        if data_size < MIN_FILE_SIZE {
            return Err(Error::Corrupt(
                "Commit-graph file too small even for an empty graph".to_owned(),
            ));
        }

        let mut ofs = 0;
        if &data[ofs..ofs + SIGNATURE.len()] != SIGNATURE {
            return Err(Error::Corrupt(
                "Commit-graph file does not start with expected signature".to_owned(),
            ));
        }
        ofs += SIGNATURE.len();

        match data[ofs] {
            1 => (),
            x => {
                return Err(Error::UnsupportedVersion(x));
            }
        }
        ofs += 1;

        let object_hash = gix_hash::Kind::try_from(data[ofs]).map_err(Error::UnsupportedHashVersion)?;
        ofs += 1;

        let chunk_count = data[ofs];
        // Can assert chunk_count >= MIN_CHUNKS here, but later OIDF+OIDL+CDAT presence checks make
        // it redundant.
        ofs += 1;

        let base_graph_count = data[ofs];
        ofs += 1;

        let chunks = gix_chunk::file::Index::from_bytes(&data, ofs, u32::from(chunk_count))?;

        let base_graphs_list_offset = chunks
            .validated_usize_offset_by_id(BASE_GRAPHS_LIST_CHUNK_ID, |chunk_range| {
                let chunk_size = chunk_range.len();
                if chunk_size % object_hash.len_in_bytes() != 0 {
                    return Err(Error::InvalidChunkSize {
                        id: BASE_GRAPHS_LIST_CHUNK_ID,
                        msg: format!(
                            "chunk size {} is not a multiple of {}",
                            chunk_size,
                            object_hash.len_in_bytes()
                        ),
                    });
                }
                let chunk_base_graph_count: u32 = (chunk_size / object_hash.len_in_bytes())
                    .try_into()
                    .expect("base graph count to fit in 32-bits");
                if chunk_base_graph_count != u32::from(base_graph_count) {
                    return Err(Error::BaseGraphMismatch {
                        from_chunk: chunk_base_graph_count,
                        from_header: base_graph_count,
                    });
                }
                Ok(chunk_range.start)
            })
            .ok()
            .transpose()?;

        let (commit_data_offset, commit_data_count) =
            chunks.validated_usize_offset_by_id(COMMIT_DATA_CHUNK_ID, |chunk_range| {
                let chunk_size = chunk_range.len();

                let entry_size = object_hash.len_in_bytes() + COMMIT_DATA_ENTRY_SIZE_SANS_HASH;
                if chunk_size % entry_size != 0 {
                    return Err(Error::InvalidChunkSize {
                        id: COMMIT_DATA_CHUNK_ID,
                        msg: format!("chunk size {chunk_size} is not a multiple of {entry_size}"),
                    });
                }
                Ok((
                    chunk_range.start,
                    (chunk_size / entry_size)
                        .try_into()
                        .expect("number of commits in CDAT chunk to fit in 32 bits"),
                ))
            })??;

        let fan_offset = chunks.validated_usize_offset_by_id(OID_FAN_CHUNK_ID, |chunk_range| {
            let chunk_size = chunk_range.len();

            let expected_size = 4 * FAN_LEN;
            if chunk_size != expected_size {
                return Err(Error::InvalidChunkSize {
                    id: OID_FAN_CHUNK_ID,
                    msg: format!("expected chunk length {expected_size}, got {chunk_size}"),
                });
            }
            Ok(chunk_range.start)
        })??;

        let (oid_lookup_offset, oid_lookup_count) =
            chunks.validated_usize_offset_by_id(OID_LOOKUP_CHUNK_ID, |chunk_range| {
                let chunk_size = chunk_range.len();

                if chunk_size % object_hash.len_in_bytes() != 0 {
                    return Err(Error::InvalidChunkSize {
                        id: OID_LOOKUP_CHUNK_ID,
                        msg: format!(
                            "chunk size {} is not a multiple of {}",
                            chunk_size,
                            object_hash.len_in_bytes()
                        ),
                    });
                }
                Ok((
                    chunk_range.start,
                    (chunk_size / object_hash.len_in_bytes())
                        .try_into()
                        .expect("number of commits in OIDL chunk to fit in 32 bits"),
                ))
            })??;

        let extra_edges_list_range = chunks.usize_offset_by_id(EXTENDED_EDGES_LIST_CHUNK_ID).ok();

        let trailer = &data[chunks.highest_offset() as usize..];
        if trailer.len() != object_hash.len_in_bytes() {
            return Err(Error::Trailer(format!(
                "Expected commit-graph trailer to contain {} bytes, got {}",
                object_hash.len_in_bytes(),
                trailer.len()
            )));
        }

        if base_graph_count > 0 && base_graphs_list_offset.is_none() {
            return Err(message!(
                "Chunk named {:?} was not found in chunk file index",
                BASE_GRAPHS_LIST_CHUNK_ID.as_bstr()
            )
            .into());
        }

        let (fan, _) = read_fan(&data[fan_offset..]);
        if oid_lookup_count != fan[255] {
            return Err(Error::CommitCountMismatch {
                chunk1_id: OID_FAN_CHUNK_ID,
                chunk1_commits: fan[255],
                chunk2_id: OID_LOOKUP_CHUNK_ID,
                chunk2_commits: oid_lookup_count,
            });
        }
        if commit_data_count != fan[255] {
            return Err(Error::CommitCountMismatch {
                chunk1_id: OID_FAN_CHUNK_ID,
                chunk1_commits: fan[255],
                chunk2_id: COMMIT_DATA_CHUNK_ID,
                chunk2_commits: commit_data_count,
            });
        }
        Ok(File {
            base_graph_count,
            base_graphs_list_offset,
            commit_data_offset,
            data,
            extra_edges_list_range,
            fan,
            oid_lookup_offset,
            path,
            hash_len: object_hash.len_in_bytes(),
            object_hash,
        })
    }
}

impl TryFrom<&Path> for File {
    type Error = Error;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let data = std::fs::File::open(path)
            .and_then(|file| {
                // SAFETY: we have to take the risk of somebody changing the file underneath. Git never writes into the same file.
                #[allow(unsafe_code)]
                unsafe {
                    memmap2::MmapOptions::new().map_copy_read_only(&file)
                }
            })
            .map_err(|e| Error::Io {
                err: e,
                path: path.to_owned(),
            })?;
        Self::new(data, path.to_owned())
    }
}

// Copied from gix-odb/pack/index/init.rs
fn read_fan(d: &[u8]) -> ([u32; FAN_LEN], usize) {
    assert!(d.len() >= FAN_LEN * 4);

    let mut fan = [0; FAN_LEN];
    for (c, f) in d.chunks_exact(4).zip(fan.iter_mut()) {
        *f = u32::from_be_bytes(c.try_into().unwrap());
    }
    (fan, FAN_LEN * 4)
}
