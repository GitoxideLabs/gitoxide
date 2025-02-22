use std::sync::atomic::{AtomicBool, Ordering};

use gix_features::{parallel, progress::DynNestedProgress};

use super::Error;
use crate::{
    cache::delta::traverse,
    index::{self, traverse::Outcome, util::index_entries_sorted_by_offset_ascending},
};

/// Traversal options for [`traverse_with_index()`][index::File::traverse_with_index()]
#[derive(Default)]
pub struct Options {
    /// If `Some`, only use the given amount of threads. Otherwise, the amount of threads to use will be selected based on
    /// the amount of available logical cores.
    pub thread_limit: Option<usize>,
    /// The kinds of safety checks to perform.
    pub check: crate::index::traverse::SafetyCheck,
}

/// The progress ids used in [`index::File::traverse_with_index()`].
///
/// Use this information to selectively extract the progress of interest in case the parent application has custom visualization.
#[derive(Debug, Copy, Clone)]
pub enum ProgressId {
    /// The amount of bytes currently processed to generate a checksum of the *pack data file*.
    HashPackDataBytes,
    /// The amount of bytes currently processed to generate a checksum of the *pack index file*.
    HashPackIndexBytes,
    /// Collect all object hashes into a vector and sort it by their pack offset.
    CollectSortedIndexEntries,
    /// Count the objects processed when building a cache tree from all objects in a pack index.
    TreeFromOffsetsObjects,
    /// The amount of objects which were decoded.
    DecodedObjects,
    /// The amount of bytes that were decoded in total, as the sum of all bytes to represent all decoded objects.
    DecodedBytes,
}

impl From<ProgressId> for gix_features::progress::Id {
    fn from(v: ProgressId) -> Self {
        match v {
            ProgressId::HashPackDataBytes => *b"PTHP",
            ProgressId::HashPackIndexBytes => *b"PTHI",
            ProgressId::CollectSortedIndexEntries => *b"PTCE",
            ProgressId::TreeFromOffsetsObjects => *b"PTDI",
            ProgressId::DecodedObjects => *b"PTRO",
            ProgressId::DecodedBytes => *b"PTDB",
        }
    }
}

/// Traversal with index
impl index::File {
    /// Iterate through all _decoded objects_ in the given `pack` and handle them with a `Processor`, using an index to reduce waste
    /// at the cost of memory.
    ///
    /// For more details, see the documentation on the [`traverse()`][index::File::traverse()] method.
    pub fn traverse_with_index<Processor, E>(
        &self,
        pack: &crate::data::File,
        mut processor: Processor,
        progress: &mut dyn DynNestedProgress,
        should_interrupt: &AtomicBool,
        Options { check, thread_limit }: Options,
    ) -> Result<Outcome, Error<E>>
    where
        Processor: FnMut(gix_object::Kind, &[u8], &index::Entry, &dyn gix_features::progress::Progress) -> Result<(), E>
            + Send
            + Clone,
        E: std::error::Error + Send + Sync + 'static,
    {
        let (verify_result, traversal_result) = parallel::join(
            {
                let mut pack_progress = progress.add_child_with_id(
                    format!(
                        "Hash of pack '{}'",
                        pack.path().file_name().expect("pack has filename").to_string_lossy()
                    ),
                    ProgressId::HashPackDataBytes.into(),
                );
                let mut index_progress = progress.add_child_with_id(
                    format!(
                        "Hash of index '{}'",
                        self.path.file_name().expect("index has filename").to_string_lossy()
                    ),
                    ProgressId::HashPackIndexBytes.into(),
                );
                move || {
                    let res =
                        self.possibly_verify(pack, check, &mut pack_progress, &mut index_progress, should_interrupt);
                    if res.is_err() {
                        should_interrupt.store(true, Ordering::SeqCst);
                    }
                    res
                }
            },
            || -> Result<_, Error<_>> {
                let sorted_entries = index_entries_sorted_by_offset_ascending(
                    self,
                    &mut progress.add_child_with_id(
                        "collecting sorted index".into(),
                        ProgressId::CollectSortedIndexEntries.into(),
                    ),
                ); /* Pack Traverse Collect sorted Entries */
                let tree = crate::cache::delta::Tree::from_offsets_in_pack(
                    pack.path(),
                    sorted_entries.into_iter().map(Entry::from),
                    &|e| e.index_entry.pack_offset,
                    &|id| self.lookup(id).map(|idx| self.pack_offset_at_index(idx)),
                    &mut progress.add_child_with_id("indexing".into(), ProgressId::TreeFromOffsetsObjects.into()),
                    should_interrupt,
                    self.object_hash,
                )?;
                let mut outcome = digest_statistics(tree.traverse(
                    |slice, pack| pack.entry_slice(slice),
                    pack,
                    pack.pack_end() as u64,
                    move |data,
                          progress,
                          traverse::Context {
                              entry: pack_entry,
                              entry_end,
                              decompressed: bytes,
                              level,
                          }| {
                        let object_kind = pack_entry.header.as_kind().expect("non-delta object");
                        data.level = level;
                        data.decompressed_size = pack_entry.decompressed_size;
                        data.object_kind = object_kind;
                        data.compressed_size = entry_end - pack_entry.data_offset;
                        data.object_size = bytes.len() as u64;
                        let result = index::traverse::process_entry(
                            check,
                            object_kind,
                            bytes,
                            &data.index_entry,
                            || {
                                // TODO: Fix this - we overwrite the header of 'data' which also changes the computed entry size,
                                // causing index and pack to seemingly mismatch. This is surprising, and should be done differently.
                                // debug_assert_eq!(&data.index_entry.pack_offset, &pack_entry.pack_offset());
                                gix_features::hash::crc32(
                                    pack.entry_slice(data.index_entry.pack_offset..entry_end)
                                        .expect("slice pointing into the pack (by now data is verified)"),
                                )
                            },
                            progress,
                            &mut processor,
                        );
                        match result {
                            Err(err @ Error::PackDecode { .. }) if !check.fatal_decode_error() => {
                                progress.info(format!("Ignoring decode error: {err}"));
                                Ok(())
                            }
                            res => res,
                        }
                    },
                    traverse::Options {
                        object_progress: Box::new(
                            progress.add_child_with_id("Resolving".into(), ProgressId::DecodedObjects.into()),
                        ),
                        size_progress:
                            &mut progress.add_child_with_id("Decoding".into(), ProgressId::DecodedBytes.into()),
                        thread_limit,
                        should_interrupt,
                        object_hash: self.object_hash,
                    },
                )?);
                outcome.pack_size = pack.data_len() as u64;
                Ok(outcome)
            },
        );
        Ok(Outcome {
            actual_index_checksum: verify_result?,
            statistics: traversal_result?,
        })
    }
}

struct Entry {
    index_entry: crate::index::Entry,
    object_kind: gix_object::Kind,
    object_size: u64,
    decompressed_size: u64,
    compressed_size: u64,
    level: u16,
}

impl From<crate::index::Entry> for Entry {
    fn from(index_entry: crate::index::Entry) -> Self {
        Entry {
            index_entry,
            level: 0,
            object_kind: gix_object::Kind::Tree,
            object_size: 0,
            decompressed_size: 0,
            compressed_size: 0,
        }
    }
}

fn digest_statistics(traverse::Outcome { roots, children }: traverse::Outcome<Entry>) -> index::traverse::Statistics {
    let mut res = index::traverse::Statistics::default();
    let average = &mut res.average;
    for item in roots.iter().chain(children.iter()) {
        res.total_compressed_entries_size += item.data.compressed_size;
        res.total_decompressed_entries_size += item.data.decompressed_size;
        res.total_object_size += item.data.object_size;
        *res.objects_per_chain_length
            .entry(u32::from(item.data.level))
            .or_insert(0) += 1;

        average.decompressed_size += item.data.decompressed_size;
        average.compressed_size += item.data.compressed_size as usize;
        average.object_size += item.data.object_size;
        average.num_deltas += u32::from(item.data.level);
        use gix_object::Kind::*;
        match item.data.object_kind {
            Blob => res.num_blobs += 1,
            Tree => res.num_trees += 1,
            Tag => res.num_tags += 1,
            Commit => res.num_commits += 1,
        }
    }

    let num_nodes = roots.len() + children.len();
    average.decompressed_size /= num_nodes as u64;
    average.compressed_size /= num_nodes;
    average.object_size /= num_nodes as u64;
    average.num_deltas /= num_nodes as u32;

    res
}
