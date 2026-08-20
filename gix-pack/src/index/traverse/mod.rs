use std::sync::atomic::AtomicBool;

use gix_error::{ErrorExt, ResultExt, message};
use gix_features::{parallel, progress::Progress};

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
    /// If `Some`, rejects individual allocations above the given number of bytes while resolving decoded object and
    /// delta result buffers during delta-tree traversal. `Some(0)` rejects all non-empty allocations.
    pub alloc_limit_bytes: Option<usize>,
    /// A function to create a pack cache
    pub make_pack_lookup_cache: F,
}

impl Default for Options<fn() -> crate::cache::Never> {
    fn default() -> Self {
        Options {
            check: Default::default(),
            traversal: Default::default(),
            thread_limit: None,
            alloc_limit_bytes: None,
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
    pub fn traverse<C, Processor, F, D>(
        &self,
        pack: &crate::data::File<D>,
        progress: &mut dyn DynNestedProgress,
        should_interrupt: &AtomicBool,
        processor: Processor,
        Options {
            traversal,
            thread_limit,
            check,
            alloc_limit_bytes,
            make_pack_lookup_cache,
        }: Options<F>,
    ) -> Result<Outcome, Error>
    where
        C: crate::cache::DecodeEntry,
        Processor:
            FnMut(gix_object::Kind, &[u8], &index::Entry, &dyn Progress) -> Result<(), gix_error::Exn> + Send + Clone,
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
                with_index::Options {
                    check,
                    thread_limit,
                    alloc_limit_bytes,
                },
            ),
        }
    }

    fn possibly_verify<D>(
        &self,
        pack: &crate::data::File<D>,
        check: SafetyCheck,
        pack_progress: &mut dyn Progress,
        index_progress: &mut dyn Progress,
        should_interrupt: &AtomicBool,
    ) -> Result<gix_hash::ObjectId, Error>
    where
        D: crate::FileData + Send + Sync,
    {
        Ok(if check.file_checksum() {
            pack.checksum()
                .verify(&self.pack_checksum())
                .or_raise_erased(|| gix_error::CorruptionError::new("Pack checksum differs from index"))?;
            let (pack_res, id) = parallel::join(
                move || pack.verify_checksum(pack_progress, should_interrupt),
                move || self.verify_checksum(index_progress, should_interrupt),
            );
            pack_res?;
            id?
        } else {
            self.index_checksum()
        })
    }

    #[expect(clippy::too_many_arguments)]
    fn decode_and_process_entry<C, D>(
        &self,
        check: SafetyCheck,
        pack: &crate::data::File<D>,
        cache: &mut C,
        buf: &mut Vec<u8>,
        inflate: &mut gix_zlib::Inflate,
        progress: &mut dyn Progress,
        index_entry: &index::Entry,
        processor: &mut impl FnMut(gix_object::Kind, &[u8], &index::Entry, &dyn Progress) -> Result<(), gix_error::Exn>,
    ) -> Result<Option<crate::data::decode::entry::Outcome>, Error>
    where
        C: crate::cache::DecodeEntry,
        D: crate::FileData + Send + Sync,
    {
        let pack_entry = pack.entry(index_entry.pack_offset).map_err(ErrorExt::raise_erased)?;
        let pack_entry_data_offset = pack_entry.data_offset;
        let entry_stats = match pack.decode_entry(
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
        ) {
            Ok(stats) => stats,
            Err(err) if !check.fatal_decode_error() => {
                progress.info(format!(
                    "Ignoring decode error for object {} at offset {}: {err}",
                    index_entry.oid, index_entry.pack_offset
                ));
                return Ok(None);
            }
            Err(err) => {
                return Err(err
                    .raise(message!(
                        "Object {} at offset {} could not be decoded",
                        index_entry.oid,
                        index_entry.pack_offset
                    ))
                    .erased());
            }
        };
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
        Ok(Some(entry_stats))
    }
}

fn process_entry(
    check: SafetyCheck,
    object_kind: gix_object::Kind,
    decompressed: &[u8],
    index_entry: &index::Entry,
    pack_entry_crc32: impl FnOnce() -> u32,
    progress: &dyn Progress,
    processor: &mut impl FnMut(gix_object::Kind, &[u8], &index::Entry, &dyn Progress) -> Result<(), gix_error::Exn>,
) -> Result<(), Error> {
    if check.object_checksum() {
        gix_object::Data::new(decompressed, object_kind, index_entry.oid.kind())
            .verify_checksum(&index_entry.oid)
            .or_raise_erased(|| {
                gix_error::CorruptionError::new(format!(
                    "Error verifying object at offset {} against checksum in the index file",
                    index_entry.pack_offset
                ))
            })?;
        if let Some(desired_crc32) = index_entry.crc32 {
            let actual_crc32 = pack_entry_crc32();
            if actual_crc32 != desired_crc32 {
                return Err(gix_error::CorruptionError::new(format!(
                    "The CRC32 of {object_kind} object at offset {} didn't match the checksum in the index file: expected {desired_crc32}, got {actual_crc32}",
                    index_entry.pack_offset
                ))
                .raise_erased());
            }
        }
    }
    processor(object_kind, decompressed, index_entry, progress)
}
