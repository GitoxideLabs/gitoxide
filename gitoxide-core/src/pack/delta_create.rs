use std::{collections::HashMap, io, path::Path, time::Instant};

use anyhow::anyhow;
use gix::{
    Count, NestedProgress, Progress, hash, hash::ObjectId, interrupt, odb::pack, parallel::InOrderIter,
    prelude::Finalize, progress,
};

use crate::OutputFormat;

/// A general purpose context for many operations provided here
///
/// NOTE: copied from create.rs, but removed `expansion` and `thin` fields
pub struct Context<W> {
    /// If `Some(threads)`, use this amount of `threads` to accelerate the counting phase at the cost of losing
    /// determinism as the order of objects during expansion changes with multiple threads unless no expansion is performed.
    /// In the latter case, this flag has no effect.
    /// If `None`, counting will only use one thread and thus yield the same sequence of objects in any case.
    pub nondeterministic_thread_count: Option<usize>,
    /// If set, don't use more than this amount of threads.
    /// Otherwise, usually use as many threads as there are logical cores.
    /// A value of 0 is interpreted as no-limit
    pub thread_limit: Option<usize>,
    /// If set, statistics about the operation will be written to the output stream.
    pub statistics: Option<OutputFormat>,
    /// The size of the cache storing fully decoded delta objects. This can greatly speed up pack decoding by reducing the length of delta
    /// chains. Note that caches also incur a cost and poorly used caches may reduce overall performance.
    /// This is a total, shared among all threads if `thread_limit` permits.
    ///
    /// If 0, the cache is disabled entirely.
    pub pack_cache_size_in_bytes: usize,
    /// The size of the cache to store full objects by their ID, bypassing any lookup in the object database.
    /// Note that caches also incur a cost and poorly used caches may reduce overall performance.
    /// This is a total, shared among all threads if `thread_limit` permits.
    ///
    /// If 0, the cache is disabled entirely.
    pub object_cache_size_in_bytes: usize,
    /// The output stream for use of additional information
    pub out: W,
}

/// NOTE: copied from create.rs, but:
/// - rewrite `input` transform
/// - use rewriten `iter_from_counts`
pub fn delta_create<W, P>(
    repository_path: impl AsRef<Path>,
    input: impl io::BufRead + Send + 'static,
    output_directory: Option<impl AsRef<Path>>,
    mut progress: P,
    Context {
        nondeterministic_thread_count,
        thread_limit,
        statistics,
        pack_cache_size_in_bytes,
        object_cache_size_in_bytes,
        mut out,
        ..
    }: Context<W>,
) -> anyhow::Result<()>
where
    W: std::io::Write,
    P: NestedProgress,
    P::SubProgress: 'static,
{
    let repo = gix::discover(repository_path)?.into_sync();
    progress.init(Some(2), progress::steps());
    let make_cancellation_err = || anyhow!("Cancelled by user");
    let mut topo = HashMap::new();
    let parsed_input: Vec<Result<(ObjectId, Option<ObjectId>), Box<dyn std::error::Error + Send + Sync>>> = {
        let mut progress = progress.add_child("iterating");
        progress.init(None, progress::count("objects"));
        input
            .lines()
            .map(|line| {
                line.map_err(|err| Box::new(err) as Box<_>).and_then(|line| {
                    let hex2oid = |hex: &str| {
                        ObjectId::from_hex(hex.as_bytes())
                            .map_err(Into::<Box<dyn std::error::Error + Send + Sync>>::into)
                    };
                    if let Some((target, source)) = line.split_once(' ') {
                        Ok((hex2oid(target)?, Some(hex2oid(source)?)))
                    } else {
                        Ok((hex2oid(&line)?, None))
                    }
                })
            })
            .inspect(move |_| progress.inc())
            .collect()
    };
    for res in &parsed_input {
        if let Ok((target, Some(source))) = res {
            topo.insert(target.clone(), source.clone());
        }
    }
    let mut handle = repo.objects.into_shared_arc().to_cache_arc();
    let mut input: Box<dyn Iterator<Item = Result<ObjectId, Box<dyn std::error::Error + Send + Sync>>> + Send> =
        Box::new(parsed_input.into_iter().map(|res| res.map(|(target, _)| target)));

    let mut stats = Statistics::default();
    let chunk_size = 1000; // What's a good value for this?
    let counts = {
        let mut progress = progress.add_child("counting");
        progress.init(None, progress::count("objects"));
        let may_use_multiple_threads = nondeterministic_thread_count.is_some();
        let thread_limit = if may_use_multiple_threads {
            nondeterministic_thread_count.or(thread_limit)
        } else {
            Some(1)
        };
        if nondeterministic_thread_count.is_some() && !may_use_multiple_threads {
            progress.fail("Cannot use multi-threaded counting in tree-diff object expansion mode as it may yield way too many objects.".into());
        }
        let (_, _, thread_count) = gix::parallel::optimize_chunk_size_and_thread_limit(50, None, thread_limit, None);
        let progress = progress::ThroughputOnDrop::new(progress);

        {
            // Maybe should disable cache in some cases
            handle.set_pack_cache(move || {
                Box::new(pack::cache::lru::MemoryCappedHashmap::new(
                    pack_cache_size_in_bytes / thread_count,
                ))
            });
            handle.set_object_cache(move || {
                Box::new(pack::cache::object::MemoryCappedHashmap::new(
                    object_cache_size_in_bytes / thread_count,
                ))
            });
        }
        handle.prevent_pack_unload();
        handle.ignore_replacements = true;
        let input_object_expansion = pack::data::output::count::objects::ObjectExpansion::AsIs;
        let (mut counts, count_stats) = if may_use_multiple_threads {
            pack::data::output::count::objects(
                handle.clone(),
                input,
                &progress,
                &interrupt::IS_INTERRUPTED,
                pack::data::output::count::objects::Options {
                    thread_limit,
                    chunk_size,
                    input_object_expansion,
                },
            )?
        } else {
            pack::data::output::count::objects_unthreaded(
                &handle,
                &mut input,
                &progress,
                &interrupt::IS_INTERRUPTED,
                input_object_expansion,
            )?
        };
        stats.counts = count_stats;
        counts.shrink_to_fit();
        counts
    };

    progress.inc();
    let num_objects = counts.len();
    let mut in_order_entries = {
        let progress = progress.add_child("creating entries");
        InOrderIter::from(iter_from_counts::iter_from_counts(
            counts,
            topo,
            handle,
            Box::new(progress),
            iter_from_counts::Options {
                thread_limit,
                chunk_size,
                version: Default::default(),
            },
        ))
    };

    let mut entries_progress = progress.add_child("consuming");
    entries_progress.init(Some(num_objects), progress::count("entries"));
    let mut write_progress = progress.add_child("writing");
    write_progress.init(None, progress::bytes());
    let start = Instant::now();

    let mut named_tempfile_store: Option<tempfile::NamedTempFile> = None;
    let mut sink_store: std::io::Sink;
    let (mut pack_file, output_directory): (&mut dyn std::io::Write, Option<_>) = match output_directory {
        Some(dir) => {
            named_tempfile_store = Some(tempfile::NamedTempFile::new_in(dir.as_ref())?);
            (named_tempfile_store.as_mut().expect("packfile just set"), Some(dir))
        }
        None => {
            sink_store = std::io::sink();
            (&mut sink_store, None)
        }
    };
    let mut interruptible_output_iter = interrupt::Iter::new(
        pack::data::output::bytes::FromEntriesIter::new(
            in_order_entries.by_ref().inspect(|e| {
                if let Ok(entries) = e {
                    entries_progress.inc_by(entries.len());
                }
            }),
            &mut pack_file,
            num_objects as u32,
            pack::data::Version::default(),
            hash::Kind::default(),
        ),
        make_cancellation_err,
    );
    for io_res in interruptible_output_iter.by_ref() {
        let written = io_res??;
        write_progress.inc_by(written as usize);
    }

    let hash = interruptible_output_iter
        .into_inner()
        .digest()
        .expect("iteration is done");
    let pack_name = format!("{hash}.pack");
    if let (Some(pack_file), Some(dir)) = (named_tempfile_store.take(), output_directory) {
        pack_file.persist(dir.as_ref().join(pack_name))?;
    } else {
        writeln!(out, "{pack_name}")?;
    }
    stats.entries = in_order_entries.inner.finalize()?;

    write_progress.show_throughput(start);
    entries_progress.show_throughput(start);

    if let Some(format) = statistics {
        print(stats, format, out)?;
    }
    progress.inc();
    Ok(())
}

/// NOTE: copied from create.rs
fn print(stats: Statistics, format: OutputFormat, out: impl std::io::Write) -> anyhow::Result<()> {
    match format {
        OutputFormat::Human => human_output(stats, out).map_err(Into::into),
        #[cfg(feature = "serde")]
        OutputFormat::Json => serde_json::to_writer_pretty(out, &stats).map_err(Into::into),
    }
}

/// NOTE: copied from create.rs
fn human_output(
    Statistics {
        counts:
            pack::data::output::count::objects::Outcome {
                input_objects,
                expanded_objects,
                decoded_objects,
                total_objects,
            },
        entries:
            pack::data::output::entry::iter_from_counts::Outcome {
                decoded_and_recompressed_objects,
                missing_objects,
                objects_copied_from_pack,
                ref_delta_objects,
            },
    }: Statistics,
    mut out: impl std::io::Write,
) -> std::io::Result<()> {
    let width = 30;
    writeln!(out, "counting phase")?;
    #[rustfmt::skip]
    writeln!(
        out,
        "\t{:<width$} {}\n\t{:<width$} {}\n\t{:<width$} {}\n\t{:<width$} {}",
        "input objects", input_objects,
        "expanded objects", expanded_objects,
        "decoded objects", decoded_objects,
        "total objects", total_objects,
        width = width
    )?;
    writeln!(out, "generation phase")?;
    #[rustfmt::skip]
    writeln!(
        out,
        "\t{:<width$} {}\n\t{:<width$} {}\n\t{:<width$} {}\n\t{:<width$} {}",
        "decoded and recompressed", decoded_and_recompressed_objects,
        "pack-to-pack copies", objects_copied_from_pack,
        "ref-delta-objects", ref_delta_objects,
        "missing objects", missing_objects,
        width = width
    )?;
    Ok(())
}

/// NOTE: copied from create.rs
#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct Statistics {
    counts: pack::data::output::count::objects::Outcome,
    entries: pack::data::output::entry::iter_from_counts::Outcome,
}

mod iter_from_counts {
    use std::{cmp::Ordering, collections::HashMap, io::Write, sync::Arc};

    use gix::{Count, Progress, hash::ObjectId, odb::pack, parallel, parallel::SequenceId, progress};

    use pack::data::output::{
        self,
        entry::iter_from_counts::{Error, Outcome, ProgressId},
    };

    /// Configuration options for the pack generation functions provided in [`iter_from_counts()`][crate::data::output::entry::iter_from_counts()].
    #[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct Options {
        /// The amount of threads to use at most when resolving the pack. If `None`, all logical cores are used.
        pub thread_limit: Option<usize>,
        /// The amount of objects per chunk or unit of work to be sent to threads for processing
        /// TODO: could this become the window size?
        pub chunk_size: usize,
        /// The pack data version to produce for each entry
        pub version: pack::data::Version,
    }

    // NOTE: copied from gix-pack/src/data/output/entry/iter_from_counts.rs,
    // but rewrote parameters in parallel::reduce::Stepwise::new
    pub fn iter_from_counts<Find>(
        mut counts: Vec<output::Count>,
        topo: HashMap<ObjectId, ObjectId>,
        db: Find,
        mut progress: Box<dyn gix::DynNestedProgress + 'static>,
        Options {
            thread_limit,
            chunk_size,
            version,
        }: Options,
    ) -> impl Iterator<Item = Result<(SequenceId, Vec<output::Entry>), Error>>
    + parallel::reduce::Finalize<Reduce = reduce::Statistics<Error>>
    where
        Find: pack::Find + Send + Clone + 'static,
    {
        assert!(
            matches!(version, pack::data::Version::V2),
            "currently we can only write version 2"
        );
        let (chunk_size, thread_limit, _) =
            parallel::optimize_chunk_size_and_thread_limit(chunk_size, Some(counts.len()), thread_limit, None);
        {
            let progress = Arc::new(parking_lot::Mutex::new(
                progress.add_child_with_id("resolving".into(), ProgressId::ResolveCounts.into()),
            ));
            progress.lock().init(None, progress::count("counts"));
            let enough_counts_present = counts.len() > 4_000;
            let start = std::time::Instant::now();
            parallel::in_parallel_if(
                || enough_counts_present,
                counts.chunks_mut(chunk_size),
                thread_limit,
                |_n| Vec::<u8>::new(),
                {
                    let progress = Arc::clone(&progress);
                    let db = db.clone();
                    move |chunk, buf| {
                        let chunk_size = chunk.len();
                        for count in chunk {
                            use pack::data::output::count::PackLocation::*;
                            match count.entry_pack_location {
                                LookedUp(_) => continue,
                                NotLookedUp => count.entry_pack_location = LookedUp(db.location_by_oid(&count.id, buf)),
                            }
                        }
                        progress.lock().inc_by(chunk_size);
                        Ok::<_, ()>(())
                    }
                },
                parallel::reduce::IdentityWithResult::<(), ()>::default(),
            )
            .expect("infallible - we ignore none-existing objects");
            progress.lock().show_throughput(start);
        }

        let sorted_counts = {
            topo_sort(counts.as_mut_slice(), &topo).expect("no loop in delta topo");
            Arc::new(counts)
        };
        let progress = Arc::new(parking_lot::Mutex::new(progress));
        let chunks = util::ChunkRanges::new(chunk_size, sorted_counts.len());

        let oid_index_mapping = Arc::new(
            sorted_counts
                .iter()
                .enumerate()
                .map(|(index, count)| (count.id, index))
                .collect::<std::collections::HashMap<_, _>>(),
        ); // TODO: rearrange delta solving order or lru to avoid cache peak
        parallel::reduce::Stepwise::new(
            chunks.enumerate(),
            thread_limit,
            {
                let progress = Arc::clone(&progress);
                move |n| {
                    (
                        // Cache entries object ID and offset for packs
                        std::collections::HashMap::<u32, Vec<(pack::data::Offset, ObjectId)>>::new(),
                        // buffer object data for target
                        Vec::new(),
                        // buffer object data for source
                        Vec::new(),
                        progress
                            .lock()
                            .add_child_with_id(format!("thread {n}"), progress::UNKNOWN),
                    )
                }
            },
            {
                let sorted_counts = Arc::clone(&sorted_counts);
                let oid_index_mapping = Arc::clone(&oid_index_mapping);
                move |(chunk_id, chunk_range): (SequenceId, std::ops::Range<usize>),
                      (pack_index_cache, buf_t, buf_s, progress)| {
                    let mut out = Vec::new();
                    let chunk = &sorted_counts[chunk_range];
                    let mut stats = Outcome::default();
                    progress.init(Some(chunk.len()), progress::count("objects"));

                    for count in chunk.iter() {
                        let oid = count.id;
                        let db_find_cached = |oid, buf| db.try_find(oid, buf).map_err(Error::Find);
                        let entry = if let Some(source_oid) = topo.get(&oid) {
                            let mut find_existing_delta = || -> Option<_> {
                                let (compressed_data, decompressed_size) = find_delta(
                                    count,
                                    &db,
                                    source_oid,
                                    |pack_id, base_offset| {
                                        let offsets_oid_mapping =
                                            pack_index_cache.entry(pack_id).or_insert_with(|| {
                                                db.pack_offsets_and_oid(pack_id)
                                                    .map(|mut v| {
                                                        v.sort_by_key(|e| e.0);
                                                        v
                                                    })
                                                    .expect("pack used for counts is still available")
                                            });
                                        offsets_oid_mapping
                                            .binary_search_by_key(&base_offset, |e| e.0)
                                            .ok()
                                            .map(|idx| offsets_oid_mapping[idx].1)
                                    },
                                    version,
                                )?;
                                Some(Ok(output::Entry {
                                    id: oid.to_owned(),
                                    kind: output::entry::Kind::DeltaRef {
                                        object_index: *oid_index_mapping
                                            .get(source_oid)
                                            .expect("all target and source objects should in ONE pack"),
                                    },
                                    decompressed_size,
                                    compressed_data,
                                }))
                            };
                            // Find existing delta
                            if let Some(entry) = find_existing_delta() {
                                stats.objects_copied_from_pack += 1;
                                entry
                            }
                            // Build delta
                            else if let Some((target, _)) = db_find_cached(&oid, buf_t)? {
                                if let Some((source, _)) = db_find_cached(source_oid, buf_s)? {
                                    let delta_data = delta_diff::diff(source.data, target.data)
                                        .map_err(|err| Error::NewEntry(std::io::Error::other(err).into()))?;
                                    let mut deflate = gix_features::zlib::stream::deflate::Write::new(Vec::new());
                                    std::io::copy(&mut delta_data.as_slice(), &mut deflate)
                                        .map_err(|e| Error::NewEntry(e.into()))?;
                                    deflate.flush().map_err(|e| Error::NewEntry(e.into()))?;
                                    let compressed_delta = deflate.into_inner();
                                    Ok(output::Entry {
                                        id: oid.to_owned(),
                                        kind: output::entry::Kind::DeltaRef {
                                            object_index: *oid_index_mapping
                                                .get(source_oid)
                                                .expect("all target and source objects should in ONE pack"), // TODO: allow ref delta in thin pack
                                        },
                                        decompressed_size: delta_data.len(),
                                        compressed_data: compressed_delta,
                                    })
                                } else {
                                    Ok(output::Entry::invalid())
                                }
                            } else {
                                Ok(output::Entry::invalid())
                            }
                        } else if let Some((data, _)) = db_find_cached(&oid, buf_t)? {
                            output::Entry::from_data(count, &data)
                        } else {
                            Ok(output::Entry::invalid())
                        }?;
                        out.push(entry);
                        progress.inc();
                    }
                    Ok((chunk_id, out, stats))
                }
            },
            reduce::Statistics::default(),
        )
    }

    /// Topological sort `counts` in place, parents first.
    /// If there is a loop, returns Err(usize), meaning how many ObjectID are in loops indicated in the `to_parent`.
    ///
    /// # Panics
    ///
    /// Panics if any ObjectId in `to_parent` is not present in `counts`, which would indicate
    /// that the parent-child relationship map references objects outside the given count set.
    fn topo_sort(
        counts: &mut [output::Count],
        to_parent: &std::collections::HashMap<ObjectId, ObjectId>,
    ) -> Result<(), usize> {
        use std::collections::HashMap;

        type CountIndex = usize;

        let n = counts.len();
        if n == 0 {
            return Ok(());
        }

        // Firstly sort `vertexes` as children first via Kahn method...
        let oid_to_idx: HashMap<ObjectId, CountIndex> = counts
            .iter()
            .enumerate()
            .map(|(idx, c)| (c.id.to_owned(), idx))
            .collect();
        let mut idx_to_child_count: HashMap<CountIndex, usize> = (0..n).map(|c| (c, 0)).collect();
        for (child, parent) in to_parent {
            let child = oid_to_idx
                .get(child)
                .expect("child ObjectId in to_parent should exist in counts");
            let parent = oid_to_idx
                .get(parent)
                .expect("parent ObjectId in to_parent should exist in counts");
            if idx_to_child_count.contains_key(child) {
                if let Some(count) = idx_to_child_count.get_mut(parent) {
                    *count += 1;
                }
            }
        }

        let mut stack: Vec<CountIndex> = (0..n)
            .filter(|idx| idx_to_child_count.get(idx) == Some(&0)) // Collect leaf vertices in count order.
            .collect();
        let mut sorted = Vec::with_capacity(n);
        while let Some(curr) = stack.pop() {
            if let Some(parent) = to_parent.get(&counts[curr].id) {
                let parent = oid_to_idx
                    .get(parent)
                    .expect("parent ObjectId in to_parent should exist in counts");
                if let Some(count) = idx_to_child_count.get_mut(parent) {
                    *count -= 1;
                    if *count == 0 {
                        stack.push(*parent);
                    }
                }
            }
            sorted.push(curr);
        }

        match sorted.len().cmp(&n) {
            Ordering::Less => Err(n - sorted.len()),
            Ordering::Equal => {
                // ...then reverse `vertexex`, and returns as parents first
                sorted.reverse();
                util::apply_permutation(counts, &sorted);
                Ok(())
            }
            Ordering::Greater => {
                unreachable!("sorted counts should less or equal than all counts")
            }
        }
    }

    /// Returns `(compressed_delta_data, decompressed_size)` if the pack entry is a delta pointing to `source_oid`.
    /// The compressed data is extracted as-is from the pack (no decompression/recompression round-trip).
    fn find_delta(
        count: &output::Count,
        db: &impl pack::Find,
        source_oid: &ObjectId,
        mut pack_offset_to_oid: impl FnMut(u32, u64) -> Option<ObjectId>,
        target_version: pack::data::Version,
    ) -> Option<(Vec<u8>, usize)> {
        let entry = count
            .entry_pack_location
            .as_ref()
            .and_then(|l| db.entry_by_location(l))?;

        if entry.version != target_version {
            return None;
        }

        let pack_offset_must_be_zero = 0;
        let pack_entry = pack::data::Entry::from_bytes(&entry.data, pack_offset_must_be_zero, count.id.kind()).ok()?;

        use pack::data::entry::Header::*;
        let source_matches = match pack_entry.header {
            OfsDelta { base_distance } => {
                let pack_location = count.entry_pack_location.as_ref().expect("packed");
                let base_offset = pack_location
                    .pack_offset
                    .checked_sub(base_distance)
                    .expect("pack-offset - distance is firmly within the pack");
                pack_offset_to_oid(pack_location.pack_id, base_offset)
            }
            RefDelta { base_id } => Some(base_id),
            _ => None,
        }
        .filter(|id| id == source_oid);

        if source_matches.is_none() {
            return None;
        }

        let compressed = entry.data[pack_entry.data_offset as usize..].to_vec();
        Some((compressed, pack_entry.decompressed_size as usize))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn topo_sort_keeps_unrelated_objects_in_count_order() {
            let ids = [
                "1111111111111111111111111111111111111111",
                "2222222222222222222222222222222222222222",
                "3333333333333333333333333333333333333333",
            ]
            .map(|hex| ObjectId::from_hex(hex.as_bytes()).expect("valid hex object id"));
            let mut counts = ids
                .iter()
                .map(|id| output::Count {
                    id: *id,
                    entry_pack_location: output::count::PackLocation::NotLookedUp,
                })
                .collect::<Vec<_>>();

            topo_sort(&mut counts, &HashMap::new()).expect("no cycle without parent relationships");

            assert_eq!(
                counts.iter().map(|count| count.id).collect::<Vec<_>>(),
                ids,
                "unrelated objects retain the input count order"
            );
        }
    }

    /// NOTE: except `apply_permutation`, copied from gix-pack/src/data/output/entry/iter_from_counts.rs
    mod util {
        #[derive(Clone)]
        pub struct ChunkRanges {
            cursor: usize,
            size: usize,
            len: usize,
        }

        impl ChunkRanges {
            pub fn new(size: usize, total: usize) -> Self {
                ChunkRanges {
                    cursor: 0,
                    size,
                    len: total,
                }
            }
        }

        impl Iterator for ChunkRanges {
            type Item = std::ops::Range<usize>;

            fn next(&mut self) -> Option<Self::Item> {
                if self.cursor >= self.len {
                    None
                } else {
                    let upper = (self.cursor + self.size).min(self.len);
                    let range = self.cursor..upper;
                    self.cursor = upper;
                    Some(range)
                }
            }
        }

        pub fn apply_permutation<T>(data: &mut [T], indices: &[usize]) {
            let n = data.len();

            // inverse transformation: indices[i] = j => indices[j] = i
            let mut inv = vec![0; n];
            for (i, &j) in indices.iter().enumerate() {
                inv[j] = i;
            }

            for i in 0..n {
                while inv[i] != i {
                    let target = inv[i];
                    data.swap(i, target);
                    inv.swap(i, target);
                }
            }
        }
    }
    /// NOTE: copied from gix-pack/src/data/output/entry/iter_from_counts.rs
    mod reduce {
        use std::marker::PhantomData;

        use super::Outcome;
        use super::pack::data::output;
        use super::{parallel, parallel::SequenceId};

        pub struct Statistics<E> {
            total: Outcome,
            _err: PhantomData<E>,
        }

        impl<E> Default for Statistics<E> {
            fn default() -> Self {
                Statistics {
                    total: Default::default(),
                    _err: PhantomData,
                }
            }
        }

        impl<Error> parallel::Reduce for Statistics<Error> {
            type Input = Result<(SequenceId, Vec<output::Entry>, Outcome), Error>;
            type FeedProduce = (SequenceId, Vec<output::Entry>);
            type Output = Outcome;
            type Error = Error;

            fn feed(&mut self, item: Self::Input) -> Result<Self::FeedProduce, Self::Error> {
                item.map(|(cid, entries, stats)| {
                    // Should reuse Outcome::aggregate, but it's private
                    self.total.decoded_and_recompressed_objects += stats.decoded_and_recompressed_objects;
                    self.total.missing_objects += stats.missing_objects;
                    self.total.objects_copied_from_pack += stats.objects_copied_from_pack;
                    self.total.ref_delta_objects += stats.ref_delta_objects;
                    (cid, entries)
                })
            }

            fn finalize(self) -> Result<Self::Output, Self::Error> {
                Ok(self.total)
            }
        }
    }

    mod delta_diff {
        use std::io::Write;

        /// Returned when failing to encode deltas.
        #[derive(thiserror::Error, Debug)]
        #[allow(missing_docs)]
        pub enum Error {
            #[error("Failed to write bytes: {0}")]
            IOError(std::io::Error),
            #[error("Too large offset in Copy instruction, should <= 0xffffffff, got {0}")]
            TooLargeOffset(usize),
            #[error("Too large size in Copy instruction, should <= 0x00ffffff, got {0}")]
            TooLargeSize(usize),
            #[error("Too large data in Add instruction, length should <= 127, got {0}")]
            TooLargeData(usize),
        }

        /// Delta instruction
        #[derive(Debug)]
        pub enum Instruction<'a> {
            /// Copy data from source
            Copy {
                /// Start position to copy
                offset: usize,
                /// Data length in bytes
                size: usize,
            },
            /// Insert bytes embedded in instruction
            Add {
                /// Data to insert
                data: &'a [u8],
            },
        }

        impl Instruction<'_> {
            /// Encode instruction to bytes.
            pub fn encode(self, mut writer: impl Write) -> Result<(), Error> {
                match self {
                    Self::Copy { offset, mut size } => {
                        let mut header = 0x80u8;
                        let mut buf = [0u8; 7];
                        let mut n = 0;

                        if size == 0x10000 {
                            size = 0;
                        } else if size > 0x00ffffff {
                            return Err(Error::TooLargeSize(size));
                        }
                        if offset > 0xffffffff {
                            return Err(Error::TooLargeOffset(offset));
                        }

                        for i in 0..4 {
                            let byte = (offset >> (i * 8)) as u8;
                            if byte != 0 {
                                header |= 1 << i;
                                buf[n] = byte;
                                n += 1;
                            }
                        }
                        for i in 0..3 {
                            let byte = (size >> (i * 8)) as u8;
                            if byte != 0 {
                                header |= 1 << (4 + i);
                                buf[n] = byte;
                                n += 1;
                            }
                        }

                        writer.write_all(&[header]).map_err(Error::IOError)?;
                        writer.write_all(&buf[..n]).map_err(Error::IOError)?;
                        Ok(())
                    }
                    Self::Add { data } => {
                        if data.len() > 127 {
                            return Err(Error::TooLargeData(data.len()));
                        }

                        let header = data.len() as u8;
                        writer.write_all(&[header]).map_err(Error::IOError)?;
                        writer.write_all(data).map_err(Error::IOError)?;
                        Ok(())
                    }
                }
            }
        }

        /// Encode a variable-length integer for the delta header (7 bits per byte, MSB = continuation).
        fn encode_delta_varint(mut value: usize, buf: &mut impl Write) -> Result<(), Error> {
            loop {
                let mut byte = (value & 0x7f) as u8;
                value >>= 7;
                if value > 0 {
                    byte |= 0x80;
                }
                buf.write_all(&[byte]).map_err(Error::IOError)?;
                if value == 0 {
                    break;
                }
            }
            Ok(())
        }

        /// Calculate delta from `source` to `target`, returning the instructions
        /// (header + instructions) as expected by the Git pack format.
        fn compute_delta<'a>(source: &[u8], target: &'a [u8]) -> Vec<Instruction<'a>> {
            let mut insts = Vec::new();

            let mut common_prefix_len: usize = 0;
            for (s, t) in source.iter().zip(target) {
                if s == t {
                    common_prefix_len += 1;
                } else {
                    break;
                }
            }
            let mut remaining_prefix_len = common_prefix_len;
            let mut offset = 0;
            while remaining_prefix_len > 0 {
                let size = remaining_prefix_len.min(0x00ff_ffff);
                insts.push(Instruction::Copy { offset, size });
                remaining_prefix_len -= size;
                offset += size;
            }

            for chunk in target[common_prefix_len..].chunks(127) {
                insts.push(Instruction::Add { data: chunk });
            }
            insts
        }

        /// Calculate delta from `source` to `target`, returning the instructions
        /// (header + instructions) as expected by the Git pack format.
        pub fn diff(source: &[u8], target: &[u8]) -> Result<Vec<u8>, Error> {
            let mut delta_data = Vec::new();
            encode_delta_varint(source.len(), &mut delta_data)?;
            encode_delta_varint(target.len(), &mut delta_data)?;
            for inst in compute_delta(source, target) {
                inst.encode(&mut delta_data)?;
            }
            Ok(delta_data)
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            fn common_prefix_larger_than_copy_limit_is_split() {
                let source = vec![b'a'; 0x0100_0001];
                let mut target = source.clone();
                target.extend_from_slice(b"tail");

                let instructions = compute_delta(&source, &target);

                assert!(
                    matches!(
                        instructions.as_slice(),
                        [
                            Instruction::Copy {
                                offset: 0,
                                size: 0x00ff_ffff,
                            },
                            Instruction::Copy {
                                offset: 0x00ff_ffff,
                                size: 2,
                            },
                            Instruction::Add { data: b"tail" },
                        ]
                    ),
                    "large common prefixes are represented as multiple valid copy instructions"
                );
                diff(&source, &target).expect("split copy instructions stay within the delta encoding limits");
            }
        }
    }
}
