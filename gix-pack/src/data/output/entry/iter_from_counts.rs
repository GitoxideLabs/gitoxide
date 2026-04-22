pub(crate) mod function {
    use std::{cmp::Ordering, sync::Arc};

    use gix_features::{
        parallel,
        parallel::SequenceId,
        progress::{
            prodash::{Count, DynNestedProgress},
            Progress,
        },
    };
    use gix_hash::ObjectId;

    use super::{reduce, util, Error, Mode, Options, Outcome, ProgressId};
    use crate::data::output;

    /// Given a known list of object `counts`, calculate entries ready to be put into a data pack.
    ///
    /// This allows objects to be written quite soon without having to wait for the entire pack to be built in memory.
    /// A chunk of objects is held in memory and compressed using DEFLATE, and serve the output of this iterator.
    /// That way slow writers will naturally apply back pressure, and communicate to the implementation that more time can be
    /// spent compressing objects.
    ///
    /// * `counts`
    ///   * A list of previously counted objects to add to the pack. Duplication checks are not performed, no object is expected to be duplicated.
    /// * `progress`
    ///   * a way to obtain progress information
    /// * `options`
    ///   * more configuration
    ///
    /// _Returns_ the checksum of the pack
    ///
    /// ## Discussion
    ///
    /// ### Advantages
    ///
    /// * Begins writing immediately and supports back-pressure.
    /// * Abstract over object databases and how input is provided.
    ///
    /// ### Disadvantages
    ///
    /// * ~~currently there is no way to easily write the pack index, even though the state here is uniquely positioned to do
    ///   so with minimal overhead (especially compared to `gix index-from-pack`)~~ Probably works now by chaining Iterators
    ///   or keeping enough state to write a pack and then generate an index with recorded data.
    ///
    pub fn iter_from_counts<Find>(
        mut counts: Vec<output::Count>,
        db: Find,
        mut progress: Box<dyn DynNestedProgress + 'static>,
        Options {
            version,
            mode,
            allow_thin_pack,
            thread_limit,
            chunk_size,
        }: Options,
    ) -> impl Iterator<Item = Result<(SequenceId, Vec<output::Entry>), Error>>
           + parallel::reduce::Finalize<Reduce = reduce::Statistics<Error>>
    where
        Find: crate::Find + Send + Clone + 'static,
    {
        assert!(
            matches!(version, crate::data::Version::V2),
            "currently we can only write version 2"
        );
        let (chunk_size, thread_limit, _) =
            parallel::optimize_chunk_size_and_thread_limit(chunk_size, Some(counts.len()), thread_limit, None);
        {
            let progress = Arc::new(parking_lot::Mutex::new(
                progress.add_child_with_id("resolving".into(), ProgressId::ResolveCounts.into()),
            ));
            progress.lock().init(None, gix_features::progress::count("counts"));
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
                            use crate::data::output::count::PackLocation::*;
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
        match mode {
            Mode::PackCopyAndBaseObjects => {
                let counts_range_by_pack_id = rearrange_counts_by_pack_id(&mut counts, &mut progress);
                let sorted_counts = Arc::new(counts);
                let progress = Arc::new(parking_lot::Mutex::new(progress));
                let chunks = util::ChunkRanges::new(chunk_size, sorted_counts.len());

                parallel::reduce::Stepwise::new(
                    chunks.enumerate(),
                    thread_limit,
                    {
                        let progress = Arc::clone(&progress);
                        move |n| {
                            (
                                Vec::new(), // object data buffer
                                progress
                                    .lock()
                                    .add_child_with_id(format!("thread {n}"), gix_features::progress::UNKNOWN),
                            )
                        }
                    },
                    {
                        let sorted_counts = Arc::clone(&sorted_counts);
                        move |(chunk_id, chunk_range): (SequenceId, std::ops::Range<usize>), (buf, progress)| {
                            let mut out = Vec::new();
                            let chunk = &sorted_counts[chunk_range];
                            let mut stats = Outcome::default();
                            let mut latest_pack_mapping = None;
                            progress.init(Some(chunk.len()), gix_features::progress::count("objects"));

                            for count in chunk.iter() {
                                out.push(match count
                                    .entry_pack_location
                                    .as_ref()
                                    .and_then(|l| db.entry_by_location(l).map(|pe| (l, pe)))
                                {
                                    // Existing in a pack
                                    Some((location, pack_entry)) => {
                                        // Unset latest_pack_offsets_to_id if outside the pack range
                                        if let Some((cached_pack_id, _)) = &latest_pack_mapping {
                                            if *cached_pack_id != location.pack_id {
                                                latest_pack_mapping = None;
                                            }
                                        }

                                        // Params for pack finding
                                        let (base_index_offset, counts_in_pack) = {
                                            let index = counts_range_by_pack_id
                                                .binary_search_by_key(&location.pack_id, |e| e.0)
                                                .expect("pack-id always present");
                                            let pack_range = counts_range_by_pack_id[index].1.clone();
                                            (pack_range.start, &sorted_counts[pack_range])
                                        };

                                        // First try to find existing entry in existing packs
                                        if let Some(entry) = output::Entry::from_pack_entry(
                                            pack_entry,
                                            count,
                                            counts_in_pack,
                                            base_index_offset,
                                            allow_thin_pack.then_some({
                                                |pack_id, base_offset| {
                                                    let (cached_pack_id, offsets_oid_mapping) = latest_pack_mapping
                                                        .get_or_insert_with(|| {
                                                            db.pack_offsets_and_oid(pack_id)
                                                                .map(|mut v| {
                                                                    v.sort_by_key(|e| e.0);
                                                                    (pack_id, v)
                                                                })
                                                                .expect("pack used for counts is still available")
                                                        });
                                                    debug_assert_eq!(*cached_pack_id, pack_id);

                                                    stats.ref_delta_objects += 1;
                                                    offsets_oid_mapping
                                                        .binary_search_by_key(&base_offset, |e| e.0)
                                                        .ok()
                                                        .map(|idx| offsets_oid_mapping[idx].1)
                                                }
                                            }),
                                            version,
                                        ) {
                                            stats.objects_copied_from_pack += 1;
                                            entry
                                        }
                                        // Fallback to find in Object Database
                                        // TODO: useless decompress then compress here
                                        else if let Some((obj, _location)) =
                                            db.try_find(&count.id, buf).map_err(Error::Find)?
                                        {
                                            stats.decoded_and_recompressed_objects += 1;
                                            output::Entry::from_data(count, &obj)
                                        }
                                        // If both missing, return Entry::invalid
                                        else {
                                            stats.missing_objects += 1;
                                            Ok(output::Entry::invalid())
                                        }
                                    }
                                    // Existing as a loose object
                                    None => match db.try_find(&count.id, buf).map_err(Error::Find)? {
                                        Some((obj, _location)) => {
                                            stats.decoded_and_recompressed_objects += 1;
                                            output::Entry::from_data(count, &obj)
                                        }
                                        None => {
                                            stats.missing_objects += 1;
                                            Ok(output::Entry::invalid())
                                        }
                                    },
                                }?);
                                progress.inc();
                            }
                            Ok((chunk_id, out, stats))
                        }
                    },
                    reduce::Statistics::default(),
                )
            }
            Mode::CustomizedDeltaTopo(topo) => {
                let counts_range_by_pack_id = rearrange_counts_by_pack_id(&mut counts, &mut progress);
                let sorted_counts = Arc::new(counts);
                let progress = Arc::new(parking_lot::Mutex::new(progress));
                let chunks = util::ChunkRanges::new(chunk_size, sorted_counts.len());

                parallel::reduce::Stepwise::new(
                    chunks.enumerate(),
                    thread_limit,
                    {
                        let progress = Arc::clone(&progress);
                        move |n| {
                            (
                                Vec::new(), // object data buffer
                                progress
                                    .lock()
                                    .add_child_with_id(format!("thread {n}"), gix_features::progress::UNKNOWN),
                            )
                        }
                    },
                    {
                        let sorted_counts = Arc::clone(&sorted_counts);
                        move |(chunk_id, chunk_range): (SequenceId, std::ops::Range<usize>), (buf, progress)| {
                            enum ToDo {
                                /// For packed objects
                                Dedelta { target: ObjectId, source: ObjectId },
                                /// For loose objects
                                Decompress(ObjectId),
                                /// For reuse delta
                                ReuseDelta(Entry),
                            }
                            let mut out = Vec::new();
                            let mut to_delta_buf = Vec::new();
                            let chunk = &sorted_counts[chunk_range];
                            let mut stats = Outcome::default();
                            let mut latest_pack_mapping = None;
                            progress.init(Some(chunk.len()), gix_features::progress::count("objects"));

                            for count in chunk.iter() {
                                let delta_target = count.id;
                                let delta_source = topo.get(&delta_target);

                                out.push(match count
                                    .entry_pack_location
                                    .as_ref()
                                    .and_then(|l| db.entry_by_location(l).map(|pe| (l, pe)))
                                {
                                    // Existing in a pack
                                    Some((location, pack_entry)) => {
                                        // Unset latest_pack_offsets_to_id if outside the pack range
                                        if let Some((cached_pack_id, _)) = &latest_pack_mapping {
                                            if *cached_pack_id != location.pack_id {
                                                latest_pack_mapping = None;
                                            }
                                        }

                                        // Params for pack finding
                                        let (base_index_offset, counts_in_pack) = {
                                            let index = counts_range_by_pack_id
                                                .binary_search_by_key(&location.pack_id, |e| e.0)
                                                .expect("pack-id always present");
                                            let pack_range = counts_range_by_pack_id[index].1.clone();
                                            (pack_range.start, &sorted_counts[pack_range])
                                        };

                                        // Try to reuse delta
                                        if let Some(entry) = output::Entry::from_pack_entry(
                                            pack_entry,
                                            count,
                                            counts_in_pack,
                                            base_index_offset,
                                            allow_thin_pack.then_some({
                                                |pack_id, base_offset| {
                                                    let (cached_pack_id, offsets_oid_mapping) = latest_pack_mapping
                                                        .get_or_insert_with(|| {
                                                            db.pack_offsets_and_oid(pack_id)
                                                                .map(|mut v| {
                                                                    v.sort_by_key(|e| e.0);
                                                                    (pack_id, v)
                                                                })
                                                                .expect("pack used for counts is still available")
                                                        });
                                                    debug_assert_eq!(*cached_pack_id, pack_id);

                                                    stats.ref_delta_objects += 1;
                                                    offsets_oid_mapping
                                                        .binary_search_by_key(&base_offset, |e| e.0)
                                                        .ok()
                                                        .map(|idx| offsets_oid_mapping[idx].1)
                                                }
                                            }),
                                            version,
                                        ) {
                                            stats.objects_copied_from_pack += 1;
                                            entry.map(|entry| {
                                                use super::super::Kind;

                                                match entry.kind {
                                                    Kind::DeltaRef { object_index } => {
                                                        let current_delta_source = {
                                                            let pack_location =
                                                                count.entry_pack_location.as_ref().expect("packed");
                                                            let (_, offsets_oid_mapping) = latest_pack_mapping
                                                                .get_or_insert_with(|| {
                                                                    db.pack_offsets_and_oid(pack_location.pack_id)
                                                                        .map(|mut v| {
                                                                            v.sort_by_key(|e| e.0);
                                                                            (pack_location.pack_id, v)
                                                                        })
                                                                        .expect(
                                                                            "pack used for counts is still available",
                                                                        )
                                                                });
                                                            offsets_oid_mapping
                                                                .binary_search_by_key(&pack_location.pack_offset, |e| {
                                                                    e.0
                                                                })
                                                                .ok()
                                                                .map(|idx| offsets_oid_mapping[idx].1)
                                                                .expect("pack offset is valid")
                                                        };
                                                        if let Some(delta_source) = delta_source {
                                                            // Reuse delta
                                                            if *delta_source == current_delta_source {
                                                                stats.objects_copied_from_pack += 1;
                                                                ToDo::ReuseDelta(entry)
                                                            } else {
                                                                ToDo::Dedelta {
                                                                    target: entry.id,
                                                                    source: current_delta_source,
                                                                }
                                                            }
                                                        } else {
                                                            ToDo::Dedelta {
                                                                target: entry.id,
                                                                source: current_delta_source,
                                                            }
                                                        }
                                                    }
                                                    Kind::Base(kind) => ToDo::Decompress(entry.id),
                                                    Kind::DeltaOid { id } => ToDo::Dedelta {
                                                        target: entry.id,
                                                        source: id,
                                                    },
                                                }
                                            })
                                        } else {
                                            Ok(ToDo::Decompress(count.id))
                                        }
                                    }
                                    // Existing as a loose object
                                    None => Ok(ToDo::Decompress(count.id)),
                                }?);
                                progress.inc();
                            }
                            let out = todo!();
                            Ok((chunk_id, out, stats))
                        }
                    },
                    reduce::Statistics::default(),
                )
            }
        }
    }

    fn rearrange_counts_by_pack_id(
        counts: &mut Vec<output::Count>,
        progress: &mut Box<dyn DynNestedProgress + 'static>,
    ) -> Vec<(u32, std::ops::Range<usize>)> {
        let mut progress = progress.add_child_with_id("sorting".into(), ProgressId::SortEntries.into());
        progress.init(Some(counts.len()), gix_features::progress::count("counts"));
        let start = std::time::Instant::now();

        use crate::data::output::count::PackLocation::*;
        counts.sort_by(|lhs, rhs| match (&lhs.entry_pack_location, &rhs.entry_pack_location) {
            (LookedUp(None), LookedUp(None)) => Ordering::Equal,
            (LookedUp(Some(_)), LookedUp(None)) => Ordering::Greater,
            (LookedUp(None), LookedUp(Some(_))) => Ordering::Less,
            (LookedUp(Some(lhs)), LookedUp(Some(rhs))) => lhs
                .pack_id
                .cmp(&rhs.pack_id)
                .then(lhs.pack_offset.cmp(&rhs.pack_offset)),
            (_, _) => unreachable!("counts were resolved beforehand"),
        });

        let mut index: Vec<(u32, std::ops::Range<usize>)> = Vec::new();
        let mut chunks_pack_start = counts.partition_point(|e| e.entry_pack_location.is_none());
        let mut slice = &counts[chunks_pack_start..];
        while !slice.is_empty() {
            let current_pack_id = slice[0].entry_pack_location.as_ref().expect("packed object").pack_id;
            let pack_end = slice
                .partition_point(|e| e.entry_pack_location.as_ref().expect("packed object").pack_id == current_pack_id);
            index.push((current_pack_id, chunks_pack_start..chunks_pack_start + pack_end));
            slice = &slice[pack_end..];
            chunks_pack_start += pack_end;
        }

        progress.set(counts.len());
        progress.show_throughput(start);

        index
    }
}

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
}

mod reduce {
    use std::marker::PhantomData;

    use gix_features::{parallel, parallel::SequenceId};

    use super::Outcome;
    use crate::data::output;

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
                self.total.aggregate(stats);
                (cid, entries)
            })
        }

        fn finalize(self) -> Result<Self::Output, Self::Error> {
            Ok(self.total)
        }
    }
}

mod types {
    use gix_hash::ObjectId;
    use gix_hashtable::sync::ObjectIdMap;

    use crate::data::output::entry;

    /// Information gathered during the run of [`iter_from_counts()`][crate::data::output::entry::iter_from_counts()].
    #[derive(Default, PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct Outcome {
        /// The amount of fully decoded objects. These are the most expensive as they are fully decoded.
        pub decoded_and_recompressed_objects: usize,
        /// The amount of objects that could not be located despite them being mentioned during iteration
        pub missing_objects: usize,
        /// The amount of base or delta objects that could be copied directly from the pack. These are cheapest as they
        /// only cost a memory copy for the most part.
        pub objects_copied_from_pack: usize,
        /// The amount of objects that ref to their base as ref-delta, an indication for a thin back being created.
        pub ref_delta_objects: usize,
    }

    impl Outcome {
        pub(in crate::data::output::entry) fn aggregate(
            &mut self,
            Outcome {
                decoded_and_recompressed_objects: decoded_objects,
                missing_objects,
                objects_copied_from_pack,
                ref_delta_objects,
            }: Self,
        ) {
            self.decoded_and_recompressed_objects += decoded_objects;
            self.missing_objects += missing_objects;
            self.objects_copied_from_pack += objects_copied_from_pack;
            self.ref_delta_objects += ref_delta_objects;
        }
    }

    /// The way the iterator operates.
    #[derive(Debug)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub enum Mode {
        /// Copy base objects and deltas from packs, while non-packed objects will be treated as base objects
        /// (i.e. without trying to delta compress them). This is a fast way of obtaining a back while benefiting
        /// from existing pack compression and spending the smallest possible time on compressing unpacked objects at
        /// the cost of bandwidth.
        PackCopyAndBaseObjects,
        /// Determine whether an object is a base or a delta based on topological relationships.
        /// Key object refers to delta target, value object refers to delta source.
        /// Treat objects missing in keys as base objects.
        /// If the required delta does not exist, it will be computed.
        CustomizedDeltaTopo(std::collections::HashMap<ObjectId, ObjectId>),
    }

    /// Configuration options for the pack generation functions provided in [`iter_from_counts()`][crate::data::output::entry::iter_from_counts()].
    #[derive(Debug)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct Options {
        /// The amount of threads to use at most when resolving the pack. If `None`, all logical cores are used.
        pub thread_limit: Option<usize>,
        /// The algorithm to produce a pack
        pub mode: Mode,
        /// If set, the resulting back can have deltas that refer to an object which is not in the pack. This can happen
        /// if the initial counted objects do not contain an object that an existing packed delta refers to, for example, because
        /// it wasn't part of the iteration, for instance when the iteration was performed on tree deltas or only a part of the
        /// commit graph. Please note that thin packs are not valid packs at rest, thus they are only valid for packs in transit.
        ///
        /// If set to false, delta objects will be decompressed and recompressed as base objects.
        pub allow_thin_pack: bool,
        /// The amount of objects per chunk or unit of work to be sent to threads for processing
        /// TODO: could this become the window size?
        pub chunk_size: usize,
        /// The pack data version to produce for each entry
        pub version: crate::data::Version,
    }

    impl Default for Options {
        fn default() -> Self {
            Options {
                thread_limit: None,
                mode: Mode::PackCopyAndBaseObjects,
                allow_thin_pack: false,
                chunk_size: 10,
                version: Default::default(),
            }
        }
    }

    /// The error returned by the pack generation function [`iter_from_counts()`][crate::data::output::entry::iter_from_counts()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        Find(gix_object::find::Error),
        #[error(transparent)]
        NewEntry(#[from] entry::Error),
    }

    /// The progress ids used in [`write_to_directory()`][crate::Bundle::write_to_directory()].
    ///
    /// Use this information to selectively extract the progress of interest in case the parent application has custom visualization.
    #[derive(Debug, Copy, Clone)]
    pub enum ProgressId {
        /// The amount of [`Count`][crate::data::output::Count] objects which are resolved to their pack location.
        ResolveCounts,
        /// Layout pack entries for placement into a pack (by pack-id and by offset).
        SortEntries,
    }

    impl From<ProgressId> for gix_features::progress::Id {
        fn from(v: ProgressId) -> Self {
            match v {
                ProgressId::ResolveCounts => *b"ECRC",
                ProgressId::SortEntries => *b"ECSE",
            }
        }
    }
}
pub use types::{Error, Mode, Options, Outcome, ProgressId};
