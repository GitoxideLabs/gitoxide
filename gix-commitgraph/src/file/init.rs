use gix_error::Result;
use std::path::{Path, PathBuf};

use gix_error::{ResultExt, bail, corruption, message};

use crate::{
    File,
    file::{
        BASE_GRAPHS_LIST_CHUNK_ID, COMMIT_DATA_CHUNK_ID, COMMIT_DATA_ENTRY_SIZE_SANS_HASH,
        EXTENDED_EDGES_LIST_CHUNK_ID, FAN_LEN, HEADER_LEN, OID_FAN_CHUNK_ID, OID_LOOKUP_CHUNK_ID, SIGNATURE,
    },
};

const MIN_FILE_SIZE: usize = HEADER_LEN
    + gix_chunk::file::Index::size_for_entries(3 /*OIDF, OIDL, CDAT*/)
    + FAN_LEN * 4 /* FANOUT TABLE CHUNK OIDF */
    + gix_hash::Kind::shortest().len_in_bytes();

impl File {
    /// Try to parse the commit graph file at `path`.
    pub fn at(path: impl AsRef<Path>) -> Result<File> {
        Self::try_from(path.as_ref())
    }

    /// A lower-level constructor which constructs a new instance directly from the mapping in `data`,
    /// assuming that it originated from `path`.
    ///
    /// Note that `path` is only used for verification of the hash its basename contains, but otherwise
    /// is not of importance.
    pub fn new(data: memmap2::Mmap, path: PathBuf) -> Result<File> {
        let data_size = data.len();
        if data_size < MIN_FILE_SIZE {
            bail!(corruption("Commit-graph file too small even for an empty graph"));
        }

        let mut ofs = 0;
        if &data[ofs..ofs + SIGNATURE.len()] != SIGNATURE {
            bail!(corruption("Commit-graph file does not start with expected signature"));
        }
        ofs += SIGNATURE.len();

        match data[ofs] {
            1 => (),
            x => {
                bail!("Unsupported commit-graph file version: {x}".unsupported());
            }
        }
        ofs += 1;

        let object_hash = gix_hash::Kind::try_from(data[ofs])
            .map_err(|v| message!("Commit-graph file uses unsupported hash version: {v}").unsupported_error())?;
        ofs += 1;

        let chunk_count = data[ofs];
        // Can assert chunk_count >= MIN_CHUNKS here, but later OIDF+OIDL+CDAT presence checks make
        // it redundant.
        ofs += 1;

        let base_graph_count = data[ofs];
        ofs += 1;

        let chunks = gix_chunk::file::Index::from_bytes(&data, ofs, u32::from(chunk_count)).or_raise(|| {
            message!("Couldn't read commit-graph file with {chunk_count} chunks at offset {ofs}").corrupted()
        })?;

        let base_graphs_list_offset = chunks
            .validated_usize_offset_by_id(BASE_GRAPHS_LIST_CHUNK_ID, |chunk_range| -> Result<_> {
                let chunk_size = chunk_range.len();
                if chunk_size % object_hash.len_in_bytes() != 0 {
                    bail!("Commit-graph chunk {BASE_GRAPHS_LIST_CHUNK_ID:?} has invalid size: {msg}".corrupted(),
                        msg = format!(
                            "chunk size {} is not a multiple of {}",
                            chunk_size,
                            object_hash.len_in_bytes()
                        ),
                    );
                }
                let chunk_base_graph_count = chunk_size / object_hash.len_in_bytes();
                if chunk_base_graph_count != usize::from(base_graph_count) {
                    bail!("Commit-graph {BASE_GRAPHS_LIST_CHUNK_ID:?} chunk contains {chunk_base_graph_count} base graphs, but commit-graph file header claims {base_graph_count} base graphs".corrupted())
                }
                Ok(chunk_range.start)
            })
            .ok()
            .transpose()?;

        let (commit_data_offset, commit_data_count): (_, u32) = chunks
            .validated_usize_offset_by_id(COMMIT_DATA_CHUNK_ID, |chunk_range| -> Result<_> {
                let chunk_size = chunk_range.len();

                let entry_size = object_hash.len_in_bytes() + COMMIT_DATA_ENTRY_SIZE_SANS_HASH;
                if chunk_size % entry_size != 0 {
                    bail!("Commit-graph chunk {COMMIT_DATA_CHUNK_ID:?} has invalid size: chunk size {chunk_size} is not a multiple of {entry_size}".corrupted())
                }
                Ok((
                    chunk_range.start,
                    commit_count(chunk_size / entry_size)?,
                ))
            })
            .or_raise(|| corruption("Error getting offset for commit data chunk"))??;

        let fan_offset = chunks
            .validated_usize_offset_by_id(OID_FAN_CHUNK_ID, |chunk_range| -> Result<_> {
                let chunk_size = chunk_range.len();

                let expected_size = 4 * FAN_LEN;
                if chunk_size != expected_size {
                    bail!("Commit-graph chunk {OID_FAN_CHUNK_ID:?} has invalid size: expected chunk length {expected_size}, got {chunk_size}".corrupted())
                }
                Ok(chunk_range.start)
            })
            .or_raise(|| corruption("Error getting offset for OID fan chunk"))?
            .or_raise(|| message("Error getting offset for OID fan chunk"))?;

        let (oid_lookup_offset, oid_lookup_count): (_, u32) = chunks
            .validated_usize_offset_by_id(OID_LOOKUP_CHUNK_ID, |chunk_range| -> Result<_> {
                let chunk_size = chunk_range.len();

                if chunk_size % object_hash.len_in_bytes() != 0 {
                    bail!("Commit-graph chunk {OID_LOOKUP_CHUNK_ID:?} has invalid size: chunk size {chunk_size} is not a multiple of {hash_len}".corrupted(), hash_len = object_hash.len_in_bytes())
                }
                Ok((
                    chunk_range.start,
                    commit_count(chunk_size / object_hash.len_in_bytes())?,
                ))
            })
            .or_raise(|| corruption("Error getting offset for OID lookup chunk"))?
            .or_raise(|| message("Error getting offset for OID lookup chunk"))?;

        let extra_edges_list_range = chunks
            .validated_usize_offset_by_id(EXTENDED_EDGES_LIST_CHUNK_ID, |chunk_range| -> Result<_> {
                if chunk_range.len() % 4 != 0 {
                    bail!("Commit-graph chunk {EXTENDED_EDGES_LIST_CHUNK_ID:?} has invalid size: chunk size {} is not a multiple of 4".corrupted(), chunk_range.len());
                }
                Ok(chunk_range)
            })
            .ok()
            .transpose()?;

        let trailer = &data[chunks.highest_offset() as usize..];
        if trailer.len() != object_hash.len_in_bytes() {
            bail!(
                "Expected commit-graph trailer to contain {} bytes, got {}".corrupted(),
                object_hash.len_in_bytes(),
                trailer.len()
            );
        }

        if base_graph_count > 0 && base_graphs_list_offset.is_none() {
            bail!("Chunk named {BASE_GRAPHS_LIST_CHUNK_ID:?} was not found in chunk file index".corrupted());
        }

        let (fan, _) = read_fan(&data[fan_offset..]);
        if fan.windows(2).any(|counts| counts[0] > counts[1]) {
            bail!(corruption("Commit-graph fanout table is not monotonic"));
        }
        if oid_lookup_count != fan[255] {
            bail!(
                "Commit-graph {OID_FAN_CHUNK_ID:?} chunk contains {chunk1_commits} commits, but {OID_LOOKUP_CHUNK_ID:?} chunk contains {chunk2_commits} commits".corrupted(),
                chunk1_commits = fan[255],
                chunk2_commits = oid_lookup_count,
            );
        }
        if commit_data_count != fan[255] {
            bail!(
                "Commit-graph {OID_FAN_CHUNK_ID:?} chunk contains {chunk1_commits} commits, but {COMMIT_DATA_CHUNK_ID:?} chunk contains {chunk2_commits} commits".corrupted(),
                chunk1_commits = fan[255],
                chunk2_commits = commit_data_count,
            );
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
            parent_position_limit: if base_graph_count == 0 {
                fan[255]
            } else {
                crate::MAX_COMMITS
            },
        })
    }
}

fn commit_count(count: usize) -> Result<u32> {
    if count > crate::MAX_COMMITS as usize {
        bail!(
            "Commit-graph chunk contains {count} commits, but only {} commits are allowed".corrupted(),
            crate::MAX_COMMITS
        );
    }
    Ok(count as u32)
}

impl TryFrom<&Path> for File {
    type Error = gix_error::Error;

    fn try_from(path: &Path) -> Result<Self> {
        let data = std::fs::File::open(path)
            .and_then(|file| {
                // SAFETY: we have to take the risk of somebody changing the file underneath. Git never writes into the same file.
                #[expect(unsafe_code)]
                unsafe {
                    memmap2::MmapOptions::new().map_copy_read_only(&file)
                }
            })
            .or_raise(|| message!("Could not open commit-graph file at \"{path}\"", path = path.display()))?;
        Self::new(data, path.to_owned())
    }
}

// Copied from gix-odb/pack/index/init.rs
fn read_fan(d: &[u8]) -> ([u32; FAN_LEN], usize) {
    assert!(d.len() >= FAN_LEN * 4);

    let mut fan = [0; FAN_LEN];
    for (c, f) in d.as_chunks::<4>().0.iter().zip(fan.iter_mut()) {
        *f = u32::from_be_bytes(*c);
    }
    (fan, FAN_LEN * 4)
}

#[cfg(test)]
mod tests {
    #[test]
    fn commit_counts_cannot_exceed_the_format_limit() -> gix_error::TestResult {
        assert_eq!(
            super::commit_count(crate::MAX_COMMITS as usize)?,
            crate::MAX_COMMITS,
            "the format's maximum commit count is representable"
        );
        for count in [crate::MAX_COMMITS as usize + 1, usize::MAX] {
            let err = super::commit_count(count).expect_err("oversized counts are rejected before narrowing");
            assert!(
                err.is_corrupted(),
                "unrepresentable stored commit counts are corruption"
            );
        }
        Ok(())
    }
}
