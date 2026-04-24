use std::sync::atomic::{AtomicBool, Ordering};

use gix_features::{
    parallel::{self, in_parallel_if},
    progress::{self, Count, DynNestedProgress, Progress},
    threading::{lock, Mutable, OwnShared},
    zlib,
};

use super::{Error, Reducer};
use crate::{
    data, exact_vec, index,
    index::{traverse::Outcome, util},
};

/// Traversal options for [`index::File::traverse_with_lookup()`]
pub struct Options<F> {
    /// If `Some`, only use the given number of threads. Otherwise, the number of threads to use will be selected based on
    /// the number of available logical cores.
    pub thread_limit: Option<usize>,
    /// The kinds of safety checks to perform.
    pub check: index::traverse::SafetyCheck,
    /// A function to create a pack cache
    pub make_pack_lookup_cache: F,
}

impl Default for Options<fn() -> crate::cache::Never> {
    fn default() -> Self {
        Options {
            check: Default::default(),
            thread_limit: None,
            make_pack_lookup_cache: || crate::cache::Never,
        }
    }
}

/// The progress ids used in [`index::File::traverse_with_lookup()`].
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
    /// The amount of objects which were decoded by brute-force.
    DecodedObjects,
}

impl From<ProgressId> for gix_features::progress::Id {
    fn from(v: ProgressId) -> Self {
        match v {
            ProgressId::HashPackDataBytes => *b"PTHP",
            ProgressId::HashPackIndexBytes => *b"PTHI",
            ProgressId::CollectSortedIndexEntries => *b"PTCE",
            ProgressId::DecodedObjects => *b"PTRO",
        }
    }
}

/// Verify and validate the content of the index file
impl<T> index::File<T>
where
    T: crate::FileData + Sync,
{
    /// Iterate through all _decoded objects_ in the given `pack` and handle them with a `Processor` using a cache to reduce the amount of
    /// waste while decoding objects.
    ///
    /// For more details, see the documentation on the [`traverse()`][index::File::traverse()] method.
    pub fn traverse_with_lookup<C, Processor, E, F, D>(
        &self,
        mut processor: Processor,
        pack: &data::File<D>,
        progress: &mut dyn DynNestedProgress,
        should_interrupt: &AtomicBool,
        Options {
            thread_limit,
            check,
            make_pack_lookup_cache,
        }: Options<F>,
    ) -> Result<Outcome, Error<E>>
    where
        C: crate::cache::DecodeEntry,
        E: std::error::Error + Send + Sync + 'static,
        Processor: FnMut(gix_object::Kind, &[u8], &index::Entry, &dyn Progress) -> Result<(), E> + Send + Clone,
        F: Fn() -> C + Send + Clone,
        D: crate::FileData + Send + Sync,
    {
        let (verify_result, traversal_result) = parallel::join(
            {
                let mut pack_progress = progress.add_child_with_id(
                    format!("Hash of pack '{}'", crate::source_name(pack.path())),
                    ProgressId::HashPackDataBytes.into(),
                );
                let mut index_progress = progress.add_child_with_id(
                    format!("Hash of index '{}'", crate::source_name(&self.path)),
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
            || {
                let index_entries = util::index_entries_sorted_by_offset_ascending(
                    self,
                    &mut progress.add_child_with_id(
                        "collecting sorted index".into(),
                        ProgressId::CollectSortedIndexEntries.into(),
                    ),
                );

                let (chunk_size, thread_limit, available_cores) =
                    parallel::optimize_chunk_size_and_thread_limit(1000, Some(index_entries.len()), thread_limit, None);
                let there_are_enough_entries_to_process = || index_entries.len() > chunk_size * available_cores;
                let input_chunks = index_entries.chunks(chunk_size);
                let reduce_progress = OwnShared::new(Mutable::new({
                    let mut p = progress.add_child_with_id("Traversing".into(), ProgressId::DecodedObjects.into());
                    p.init(Some(self.num_objects() as usize), progress::count("objects"));
                    p
                }));
                let state_per_thread = {
                    let reduce_progress = reduce_progress.clone();
                    move |index| {
                        (
                            make_pack_lookup_cache(),
                            Vec::with_capacity(2048), // decode buffer
                            zlib::Inflate::default(),
                            lock(&reduce_progress)
                                .add_child_with_id(format!("thread {index}"), gix_features::progress::UNKNOWN), // per thread progress
                        )
                    }
                };

                in_parallel_if(
                    there_are_enough_entries_to_process,
                    input_chunks,
                    thread_limit,
                    state_per_thread,
                    move |entries: &[index::Entry],
                          (cache, buf, inflate, progress)|
                          -> Result<Vec<data::decode::entry::Outcome>, Error<_>> {
                        progress.init(
                            Some(entries.len()),
                            gix_features::progress::count_with_decimals("objects", 2),
                        );
                        let mut stats = exact_vec(entries.len());
                        progress.set(0);
                        for index_entry in entries.iter() {
                            let result = self.decode_and_process_entry(
                                check,
                                pack,
                                cache,
                                buf,
                                inflate,
                                progress,
                                index_entry,
                                &mut processor,
                            );
                            progress.inc();
                            let stat = match result {
                                Err(err @ Error::PackDecode { .. }) if !check.fatal_decode_error() => {
                                    progress.info(format!("Ignoring decode error: {err}"));
                                    continue;
                                }
                                res => res,
                            }?;
                            stats.push(stat);
                            if should_interrupt.load(Ordering::Relaxed) {
                                break;
                            }
                        }
                        Ok(stats)
                    },
                    Reducer::from_progress(reduce_progress, pack.data_len(), check, should_interrupt),
                )
            },
        );
        Ok(Outcome {
            actual_index_checksum: verify_result?,
            statistics: traversal_result?,
        })
    }
}
