use gix_error::bstr::ByteSlice;
use gix_error::ParseError;
use std::ops::Range;

use crate::{file, file::index};

impl file::Index {
    /// Provided a mapped file at the beginning via `data`, starting at `toc_offset` decode all chunk information to return
    /// an index with `num_chunks` chunks.
    pub fn from_bytes(data: &[u8], toc_offset: usize, num_chunks: u32) -> Result<Self, ParseError> {
        if num_chunks == 0 {
            return Err(ParseError::new(
                "Empty chunk indices are not allowed as the point of chunked files is to have chunks.",
            ));
        }

        let data_len: u64 = data.len() as u64;
        let mut chunks = Vec::with_capacity(num_chunks as usize);
        let mut toc_entry = &data[toc_offset..];
        let expected_min_size = (num_chunks as usize + 1) * file::Index::ENTRY_SIZE;
        if toc_entry.len() < expected_min_size {
            return Err(format!(
                "The table of contents would be {expected_min_size} bytes, but got only {toc_entry_len}",
                toc_entry_len = toc_entry.len()
            ))?;
        }

        for chunk_idx in 0..num_chunks {
            let (kind, offset) = toc_entry.split_at(4);
            let kind = to_kind(kind);
            if kind == crate::SENTINEL {
                return Err(format!(
                    "Sentinel value encountered while processing chunks {chunk_idx} of {num_chunks}"
                ))?;
            }
            if chunks.iter().any(|c: &index::Entry| c.kind == kind) {
                return Err(format!(
                    "The chunk of kind '{}' was encountered more than once",
                    kind.as_bstr()
                ))?;
            }

            let offset = be_u64(offset);
            if offset > data_len {
                return Err(format!(
                    "The chunk offset {offset} went past the file of length {data_len} - was it truncated?",
                ))?;
            }
            toc_entry = &toc_entry[file::Index::ENTRY_SIZE..];
            let next_offset = be_u64(&toc_entry[4..]);
            if next_offset > data_len {
                return Err(format!(
                    "The chunk offset {next_offset} went past the file of length {data_len} - was it truncated?"
                ))?;
            }
            if next_offset <= offset {
                return Err("All chunk offsets must be incrementing.")?;
            }
            chunks.push(index::Entry {
                kind,
                offset: Range {
                    start: offset,
                    end: next_offset,
                },
            });
        }

        let sentinel = to_kind(&toc_entry[..4]);
        if sentinel != crate::SENTINEL {
            return Err(format!("Sentinel value wasn't found, saw '{}'", sentinel.as_bstr()))?;
        }

        Ok(file::Index {
            chunks,
            will_write: false,
        })
    }
}

fn to_kind(data: &[u8]) -> crate::Id {
    data[..4].try_into().unwrap()
}

fn be_u64(data: &[u8]) -> u64 {
    u64::from_be_bytes(data[..8].try_into().unwrap())
}
