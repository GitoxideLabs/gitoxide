use std::sync::atomic::AtomicBool;

use gix_features::{parallel, progress::Progress, zlib};

use crate::index;

mod reduce;
///
pub mod with_index;
///
pub mod with_lookup;
use reduce::Reducer;

mod error;
pub use error::Error;
use gix_features::progress::DynNestedProgress;

mod types;
pub use types::{Algorithm, ProgressId, SafetyCheck, Statistics};

/// Traversal options for [`index::File::traverse()`].
#[derive(Debug, Clone)]
pub struct Options<F> {
    /// The algorithm to employ.
    pub traversal: Algorithm,
    /// If `Some`, only use the given number of threads. Otherwise, the number of threads to use will be selected based on
    /// the number of available logical cores.
    pub thread_limit: Option<usize>,
    /// The kinds of safety checks to perform.
    pub check: SafetyCheck,
    /// A function to create a pack cache
    pub make_pack_lookup_cache: F,
}

impl Default for Options<fn() -> crate::cache::Never> {
    fn default() -> Self {
        Options {
            check: Default::default(),
            traversal: Default::default(),
            thread_limit: None,
            make_pack_lookup_cache: || crate::cache::Never,
        }
    }
}

/// The outcome of the [`traverse()`][index::File::traverse()] method.
pub struct Outcome {
    /// The checksum obtained when hashing the file, which matched the checksum contained within the file.
    pub actual_index_checksum: gix_hash::ObjectId,
    /// The statistics obtained during traversal.
    pub statistics: Statistics,
}

/// Traversal of pack data files using an index file
impl<T> index::File<T>
where
    T: crate::FileData + Sync,
{
    /// Iterate through all _decoded objects_ in the given `pack` and handle them with a `Processor`.
    /// The return value is (pack-checksum, [`Outcome`], `progress`), thus the pack traversal will always verify
    /// the whole packs checksum to assure it was correct. In case of bit-rod, the operation will abort early without
    /// verifying all objects using the [interrupt mechanism][gix_features::interrupt] mechanism.
    ///
    /// # Algorithms
    ///
    /// Using the [`Options::traversal`] field one can chose between two algorithms providing different tradeoffs. Both invoke
    /// `new_processor()` to create functions receiving decoded objects, their object kind, index entry and a progress instance to provide
    /// progress information.
    ///
    /// * [`Algorithm::DeltaTreeLookup`] builds an index to avoid any unnecessary computation while resolving objects, avoiding
    ///   the need for a cache entirely, rendering `new_cache()` unused.
    ///   One could also call [`traverse_with_index()`][index::File::traverse_with_index()] directly.
    /// * [`Algorithm::Lookup`] uses a cache created by `new_cache()` to avoid having to re-compute all bases of a delta-chain while
    ///   decoding objects.
    ///   One could also call [`traverse_with_lookup()`][index::File::traverse_with_lookup()] directly.
    ///
    /// Use [`thread_limit`][Options::thread_limit] to further control parallelism and [`check`][SafetyCheck] to define how much the passed
    /// objects shall be verified beforehand.
    pub fn traverse<C, Processor, E, F, D>(
        &self,
        pack: &crate::data::File<D>,
        progress: &mut dyn DynNestedProgress,
        should_interrupt: &AtomicBool,
        processor: Processor,
        Options {
            traversal,
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
        match traversal {
            Algorithm::Lookup => self.traverse_with_lookup(
                processor,
                pack,
                progress,
                should_interrupt,
                with_lookup::Options {
                    thread_limit,
                    check,
                    make_pack_lookup_cache,
                },
            ),
            Algorithm::DeltaTreeLookup => self.traverse_with_index(
                pack,
                processor,
                progress,
                should_interrupt,
                with_index::Options { check, thread_limit },
            ),
        }
    }

    fn possibly_verify<E, D>(
        &self,
        pack: &crate::data::File<D>,
        check: SafetyCheck,
        pack_progress: &mut dyn Progress,
        index_progress: &mut dyn Progress,
        should_interrupt: &AtomicBool,
    ) -> Result<gix_hash::ObjectId, Error<E>>
    where
        E: std::error::Error + Send + Sync + 'static,
        D: crate::FileData + Send + Sync,
    {
        Ok(if check.file_checksum() {
            pack.checksum()
                .verify(&self.pack_checksum())
                .map_err(Error::PackMismatch)?;
            let (pack_res, id) = parallel::join(
                move || pack.verify_checksum(pack_progress, should_interrupt),
                move || self.verify_checksum(index_progress, should_interrupt),
            );
            pack_res.map_err(Error::PackVerify)?;
            id.map_err(Error::IndexVerify)?
        } else {
            self.index_checksum()
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn decode_and_process_entry<C, E, D>(
        &self,
        check: SafetyCheck,
        pack: &crate::data::File<D>,
        cache: &mut C,
        buf: &mut Vec<u8>,
        inflate: &mut zlib::Inflate,
        progress: &mut dyn Progress,
        index_entry: &index::Entry,
        processor: &mut impl FnMut(gix_object::Kind, &[u8], &index::Entry, &dyn Progress) -> Result<(), E>,
    ) -> Result<crate::data::decode::entry::Outcome, Error<E>>
    where
        C: crate::cache::DecodeEntry,
        E: std::error::Error + Send + Sync + 'static,
        D: crate::FileData + Send + Sync,
    {
        let pack_entry = pack.entry(index_entry.pack_offset)?;
        let pack_entry_data_offset = pack_entry.data_offset;
        let entry_stats = pack
            .decode_entry(
                pack_entry,
                buf,
                inflate,
                &|id, _| {
                    let index = self.lookup(id)?;
                    pack.entry(self.pack_offset_at_index(index))
                        .ok()
                        .map(crate::data::decode::entry::ResolvedBase::InPack)
                },
                cache,
            )
            .map_err(|e| Error::PackDecode {
                source: e,
                id: index_entry.oid,
                offset: index_entry.pack_offset,
            })?;
        let object_kind = entry_stats.kind;
        let header_size = (pack_entry_data_offset - index_entry.pack_offset) as usize;
        let entry_len = header_size + entry_stats.compressed_size;

        process_entry(
            check,
            object_kind,
            buf,
            index_entry,
            || pack.entry_crc32(index_entry.pack_offset, entry_len),
            progress,
            processor,
        )?;
        Ok(entry_stats)
    }
}

#[allow(clippy::too_many_arguments)]
fn process_entry<E>(
    check: SafetyCheck,
    object_kind: gix_object::Kind,
    decompressed: &[u8],
    index_entry: &index::Entry,
    pack_entry_crc32: impl FnOnce() -> u32,
    progress: &dyn Progress,
    processor: &mut impl FnMut(gix_object::Kind, &[u8], &index::Entry, &dyn Progress) -> Result<(), E>,
) -> Result<(), Error<E>>
where
    E: std::error::Error + Send + Sync + 'static,
{
    if check.object_checksum() {
        gix_object::Data::new(decompressed, object_kind, index_entry.oid.kind())
            .verify_checksum(&index_entry.oid)
            .map_err(|source| Error::PackObjectVerify {
                offset: index_entry.pack_offset,
                source,
            })?;
        if let Some(desired_crc32) = index_entry.crc32 {
            let actual_crc32 = pack_entry_crc32();
            if actual_crc32 != desired_crc32 {
                return Err(Error::Crc32Mismatch {
                    actual: actual_crc32,
                    expected: desired_crc32,
                    offset: index_entry.pack_offset,
                    kind: object_kind,
                });
            }
        }
    }
    processor(object_kind, decompressed, index_entry, progress).map_err(Error::Processor)
}
