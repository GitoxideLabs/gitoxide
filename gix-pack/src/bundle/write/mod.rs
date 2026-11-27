use gix_error::Result;
use std::{
    io::{self, Write},
    marker::PhantomData,
    path::Path,
    sync::{Arc, atomic::AtomicBool},
};

use gix_error::{ResultExt, bail, message};
use gix_tempfile::{AutoRemove, ContainingDirectory};
use gix_utils::interrupt;
use gix_utils::progress;
use gix_utils::progress::Progress;

use crate::data;

use gix_utils::progress::prodash::DynNestedProgress;

mod publish;
pub use publish::{Prepared, Published};
use publish::{StagedFiles, WriteOutcome};
mod types;
use types::{LockWriter, PassThrough, SharedTempFile};
pub use types::{Options, Outcome};

/// The progress ids used in [`write_to_directory()`][crate::Bundle::write_to_directory()].
///
/// Use this information to selectively extract the progress of interest in case the parent application has custom visualization.
#[derive(Debug, Copy, Clone)]
pub enum ProgressId {
    /// The amount of bytes read from the input pack data file.
    ReadPackBytes,
    /// A root progress counting logical steps towards an index file on disk.
    ///
    /// Underneath will be more progress information related to actually producing the index.
    IndexingSteps(PhantomData<crate::index::write::ProgressId>),
}

impl From<ProgressId> for gix_utils::progress::Id {
    fn from(v: ProgressId) -> Self {
        match v {
            ProgressId::ReadPackBytes => *b"BWRB",
            ProgressId::IndexingSteps(_) => *b"BWCI",
        }
    }
}

impl crate::Bundle {
    /// Given a `pack` data stream, write it along with a generated index into the `directory` if `Some` or discard all output if `None`.
    ///
    /// In the latter case, the functionality provided here is more a kind of pack data stream validation.
    /// Missing destination directories are created according to `options.shared_repository_permissions`.
    /// Use [`Self::prepare_to_directory()`] to inspect the bundle and require an exclusive marker before publishing it.
    ///
    /// * `progress` provides detailed progress information which can be discarded with [`gix_utils::progress::Discard`].
    /// * `should_interrupt` is checked regularly and when true, the whole operation will stop.
    /// * `thin_pack_base_object_lookup` retrieves external bases to insert into the output pack, completing a thin pack.
    ///   Ref-deltas whose bases are not found through this lookup are resolved in-pack during indexing, including forward
    ///   references. A base available neither through the lookup nor in the pack causes the operation to fail.
    /// * `object_hash` specifies the hash to use for writing the bundle.
    /// * `options` further configure how the task is performed.
    ///
    /// # Note
    ///
    /// * the resulting pack may be empty, that is, contains zero objects in some situations. This is a valid reply by a server and should
    ///   be accounted for.
    ///   - Empty packs always have the same name and not handling this case will result in at most one superfluous pack.
    pub fn write_to_directory(
        pack: &mut dyn io::BufRead,
        directory: Option<&Path>,
        progress: &mut dyn DynNestedProgress,
        should_interrupt: &AtomicBool,
        thin_pack_base_object_lookup: Option<impl gix_object::Find>,
        object_hash: gix_hash::Kind,
        options: Options,
    ) -> Result<Outcome> {
        let _span = gix_trace::coarse!("gix_pack::Bundle::write_to_directory()");
        Self::prepare_write(
            pack,
            directory,
            progress,
            should_interrupt,
            thin_pack_base_object_lookup,
            object_hash,
            options,
        )?
        .publish_immediately()
    }

    /// Read and index a pack in temporary files in `directory` without publishing it.
    ///
    /// The returned [`Prepared`] bundle can be inspected before [publishing](Prepared::publish) it.
    /// Dropping it discards the temporary files. An empty pack returns `None`.
    /// Supply a reader scoped to the pack since this method can read ahead.
    /// The `options.iteration_mode` must be [`data::input::Mode::Verify`] for safety, after all this is for staging
    /// received pack on the server side.
    /// Like [`Self::write_to_directory()`], this supports completing thin packs using a base object lookup.
    pub fn prepare_to_directory(
        pack: &mut dyn io::BufRead,
        directory: &Path,
        progress: &mut dyn DynNestedProgress,
        should_interrupt: &AtomicBool,
        thin_pack_base_object_lookup: Option<impl gix_object::Find>,
        object_hash: gix_hash::Kind,
        options: Options,
    ) -> Result<Option<Prepared>> {
        let _span = gix_trace::coarse!("gix_pack::Bundle::prepare_to_directory()");
        if options.iteration_mode != data::input::Mode::Verify {
            bail!(message("Preparing a pack for publication requires verifying the input stream").validation());
        }
        let directory = std::path::absolute(directory).or_raise(|| message("Could not resolve pack destination"))?;
        let alloc_limit_bytes = options.alloc_limit_bytes;
        Self::prepare_write(
            pack,
            Some(&directory),
            progress,
            should_interrupt,
            thin_pack_base_object_lookup,
            object_hash,
            options,
        )?
        .into_prepared(alloc_limit_bytes)
    }

    fn prepare_write(
        pack: &mut dyn io::BufRead,
        directory: Option<&Path>,
        progress: &mut dyn DynNestedProgress,
        should_interrupt: &AtomicBool,
        thin_pack_base_object_lookup: Option<impl gix_object::Find>,
        object_hash: gix_hash::Kind,
        options: Options,
    ) -> Result<WriteOutcome> {
        let mut read_progress = progress.add_child_with_id("read pack".into(), ProgressId::ReadPackBytes.into());
        read_progress.init(None, progress::bytes());
        let pack = progress::Read {
            inner: pack,
            progress: progress::ThroughputOnDrop::new(read_progress),
        };

        let data_file = Arc::new(parking_lot::Mutex::new(io::BufWriter::with_capacity(
            64 * 1024,
            match directory.as_ref() {
                Some(directory) => new_pack_file(directory, options.shared_repository_permissions)
                    .or_raise(|| message("Could not create temporary pack file"))?,
                None => gix_tempfile::new(std::env::temp_dir(), ContainingDirectory::Exists, AutoRemove::Tempfile)
                    .or_raise(|| message("Could not create temporary pack file"))?,
            },
        )));
        let (pack_entries_iter, pack_version): (Box<dyn Iterator<Item = Result<data::input::Entry>>>, _) =
            match thin_pack_base_object_lookup {
                Some(thin_pack_lookup) => {
                    let pack = interrupt::Read {
                        inner: pack,
                        should_interrupt,
                    };
                    let buffered_pack = io::BufReader::new(pack);
                    let pack_entries_iter = data::input::LookupRefDeltaObjectsIter::new(
                        data::input::BytesToEntriesIter::new_from_header(
                            buffered_pack,
                            options.iteration_mode,
                            data::input::EntryDataMode::KeepAndCrc32,
                            object_hash,
                        )?,
                        thin_pack_lookup,
                        options.compression,
                    );
                    let pack_version = pack_entries_iter.inner.version();
                    let pack_entries_iter = data::input::EntriesToBytesIter::new(
                        pack_entries_iter,
                        LockWriter {
                            writer: data_file.clone(),
                        },
                        pack_version,
                        object_hash,
                    );
                    (Box::new(pack_entries_iter), pack_version)
                }
                None => {
                    let pack = PassThrough {
                        reader: interrupt::Read {
                            inner: pack,
                            should_interrupt,
                        },
                        writer: Some(data_file.clone()),
                    };
                    // This buf-reader is required to assure we call 'read()' in order to fill the (extra) buffer. Otherwise all the counting
                    // we do with the wrapped pack reader doesn't work as it does not expect anyone to call BufRead functions directly.
                    // However, this is exactly what's happening in the ZipReader implementation that is eventually used.
                    // The performance impact of this is probably negligible, compared to all the other work that is done anyway :D.
                    let buffered_pack = io::BufReader::new(pack);
                    let pack_entries_iter = data::input::BytesToEntriesIter::new_from_header(
                        buffered_pack,
                        options.iteration_mode,
                        data::input::EntryDataMode::Crc32,
                        object_hash,
                    )?;
                    let pack_version = pack_entries_iter.version();
                    (Box::new(pack_entries_iter), pack_version)
                }
            };
        Self::inner_write(
            directory,
            progress,
            object_hash,
            options,
            data_file,
            pack_entries_iter,
            should_interrupt,
            pack_version,
        )
    }

    /// Equivalent to [`write_to_directory()`][crate::Bundle::write_to_directory()] but offloads reading of the pack into its own thread, hence the `Send + 'static'` bounds.
    ///
    /// # Note
    ///
    /// As it sends portions of the input to a thread it requires the 'static lifetime for the interrupt flags. This can only
    /// be satisfied by a static `AtomicBool` which is only suitable for programs that only run one of these operations at a time
    /// or don't mind that all of them abort when the flag is set.
    #[expect(clippy::too_many_arguments)]
    pub fn write_to_directory_eagerly(
        pack: Box<dyn io::Read + Send + 'static>,
        pack_size: Option<u64>,
        directory: Option<impl AsRef<Path>>,
        progress: &mut dyn DynNestedProgress,
        should_interrupt: &'static AtomicBool,
        thin_pack_base_object_lookup: Option<impl gix_object::Find + Send + 'static>,
        object_hash: gix_hash::Kind,
        options: Options,
    ) -> Result<Outcome> {
        let _span = gix_trace::coarse!("gix_pack::Bundle::write_to_directory_eagerly()");
        let directory = directory.as_ref().map(AsRef::as_ref);
        let mut read_progress = progress.add_child_with_id("read pack".into(), ProgressId::ReadPackBytes.into()); /* Bundle Write Read pack Bytes*/
        read_progress.init(pack_size.map(|s| s as usize), progress::bytes());
        let pack = progress::Read {
            inner: pack,
            progress: progress::ThroughputOnDrop::new(read_progress),
        };

        let data_file = Arc::new(parking_lot::Mutex::new(io::BufWriter::new(match directory.as_ref() {
            Some(directory) => new_pack_file(directory, options.shared_repository_permissions)
                .or_raise(|| message("Could not create temporary pack file"))?,
            None => gix_tempfile::new(std::env::temp_dir(), ContainingDirectory::Exists, AutoRemove::Tempfile)
                .or_raise(|| message("Could not create temporary pack file"))?,
        })));
        let eight_pages = 4096 * 8;
        let (pack_entries_iter, pack_version): (
            Box<dyn Iterator<Item = Result<data::input::Entry>> + Send + 'static>,
            _,
        ) = match thin_pack_base_object_lookup {
            Some(thin_pack_lookup) => {
                let pack = interrupt::Read {
                    inner: pack,
                    should_interrupt,
                };
                let buffered_pack = io::BufReader::with_capacity(eight_pages, pack);
                let pack_entries_iter = data::input::LookupRefDeltaObjectsIter::new(
                    data::input::BytesToEntriesIter::new_from_header(
                        buffered_pack,
                        options.iteration_mode,
                        data::input::EntryDataMode::KeepAndCrc32,
                        object_hash,
                    )?,
                    thin_pack_lookup,
                    options.compression,
                );
                let pack_kind = pack_entries_iter.inner.version();
                (Box::new(pack_entries_iter), pack_kind)
            }
            None => {
                let pack = PassThrough {
                    reader: interrupt::Read {
                        inner: pack,
                        should_interrupt,
                    },
                    writer: Some(data_file.clone()),
                };
                let buffered_pack = io::BufReader::with_capacity(eight_pages, pack);
                let pack_entries_iter = data::input::BytesToEntriesIter::new_from_header(
                    buffered_pack,
                    options.iteration_mode,
                    data::input::EntryDataMode::Crc32,
                    object_hash,
                )?;
                let pack_kind = pack_entries_iter.version();
                (Box::new(pack_entries_iter), pack_kind)
            }
        };
        let num_objects = pack_entries_iter.size_hint().0;
        let pack_entries_iter =
            gix_parallel::EagerIterIf::new(move || num_objects > 25_000, pack_entries_iter, 5_000, 5);

        Self::inner_write(
            directory,
            progress,
            object_hash,
            options,
            data_file,
            Box::new(pack_entries_iter),
            should_interrupt,
            pack_version,
        )?
        .publish_immediately()
    }

    #[expect(clippy::too_many_arguments)]
    fn inner_write<'a>(
        directory: Option<&Path>,
        progress: &mut dyn DynNestedProgress,
        object_hash: gix_hash::Kind,
        Options {
            thread_limit,
            iteration_mode: _,
            index_version: index_kind,
            alloc_limit_bytes,
            compression: _,
            shared_repository_permissions,
        }: Options,
        data_file: SharedTempFile,
        mut pack_entries_iter: Box<dyn Iterator<Item = Result<data::input::Entry>> + 'a>,
        should_interrupt: &AtomicBool,
        pack_version: data::Version,
    ) -> Result<WriteOutcome> {
        let mut indexing_progress = progress.add_child_with_id(
            "create index file".into(),
            ProgressId::IndexingSteps(Default::default()).into(),
        );
        let mut index_file = directory
            .map(|directory| new_pack_file(directory, shared_repository_permissions))
            .transpose()
            .or_raise(|| message("Could not create temporary index file"))?;
        let mut sink = io::sink();
        let index_writer: &mut dyn Write = match index_file.as_mut() {
            Some(file) => file,
            None => &mut sink,
        };
        let index = crate::index::write_data_iter_to_stream(
            index_kind,
            || new_pack_file_resolver(Arc::clone(&data_file)),
            &mut pack_entries_iter,
            thread_limit,
            &mut indexing_progress,
            index_writer,
            should_interrupt,
            object_hash,
            alloc_limit_bytes,
            pack_version,
        )?;
        drop(pack_entries_iter);
        let files = match (directory, index_file) {
            (Some(directory), Some(index_file)) if index.num_objects != 0 => Some(StagedFiles {
                data_path: directory.join(format!("pack-{}.pack", index.data_hash.to_hex())),
                data_file: Arc::try_unwrap(data_file)
                    .expect("only one handle left after pack was consumed")
                    .into_inner()
                    .into_inner()
                    .or_error()?,
                index_file,
                shared_repository_permissions,
            }),
            _ => None,
        };
        Ok(WriteOutcome {
            index,
            object_hash,
            pack_version,
            files,
        })
    }
}
fn new_pack_file(
    directory: &Path,
    shared_repository_permissions: gix_fs::SharedPermissions,
) -> io::Result<gix_tempfile::Handle<gix_tempfile::handle::Writable>> {
    gix_fs::dir::create::all(directory, Default::default(), shared_repository_permissions)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        gix_tempfile::new_with_permissions(
            directory,
            ContainingDirectory::Exists,
            AutoRemove::Tempfile,
            std::fs::Permissions::from_mode(0o444),
        )
    }
    #[cfg(not(unix))]
    gix_tempfile::new(directory, ContainingDirectory::Exists, AutoRemove::Tempfile)
}

fn resolve_entry(range: data::EntryRange, mapped_file: &memmap2::Mmap) -> Option<&[u8]> {
    mapped_file.get(range.start as usize..range.end as usize)
}

#[expect(clippy::type_complexity)] // cannot typedef impl Fn
fn new_pack_file_resolver(
    data_file: SharedTempFile,
) -> io::Result<(
    impl Fn(data::EntryRange, &memmap2::Mmap) -> Option<&[u8]> + Send + Clone,
    memmap2::Mmap,
)> {
    let mut guard = data_file.lock();
    guard.flush()?;
    let mapped_file = crate::mmap::read_only(&guard.get_mut().with_mut(|f| f.path().to_owned())?)?;
    Ok((resolve_entry, mapped_file))
}
