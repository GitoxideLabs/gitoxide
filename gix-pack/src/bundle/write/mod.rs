use std::{
    io,
    io::Write,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
};

use gix_features::{interrupt, progress, progress::Progress};
use gix_tempfile::{AutoRemove, ContainingDirectory};

use crate::data;

mod error;
pub use error::Error;
use gix_features::progress::prodash::DynNestedProgress;

mod types;
use types::{LockWriter, PassThrough};
pub use types::{Options, Outcome};

use crate::bundle::write::types::SharedTempFile;

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

impl From<ProgressId> for gix_features::progress::Id {
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
    ///
    /// * `progress` provides detailed progress information which can be discarded with [`gix_features::progress::Discard`].
    /// * `should_interrupt` is checked regularly and when true, the whole operation will stop.
    /// * `thin_pack_base_object_lookup` If set, we expect to see a thin-pack with objects that reference their base object by object id which is
    ///   expected to exist in the object database the bundle is contained within.
    ///   `options` further configure how the task is performed.
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
        options: Options,
    ) -> Result<Outcome, Error> {
        let _span = gix_features::trace::coarse!("gix_pack::Bundle::write_to_directory()");
        let mut read_progress = progress.add_child_with_id("read pack".into(), ProgressId::ReadPackBytes.into());
        read_progress.init(None, progress::bytes());
        let pack = progress::Read {
            inner: pack,
            progress: progress::ThroughputOnDrop::new(read_progress),
        };

        let object_hash = options.object_hash;
        let data_file = Arc::new(parking_lot::Mutex::new(io::BufWriter::with_capacity(
            64 * 1024,
            match directory.as_ref() {
                Some(directory) => gix_tempfile::new(directory, ContainingDirectory::Exists, AutoRemove::Tempfile)?,
                None => gix_tempfile::new(std::env::temp_dir(), ContainingDirectory::Exists, AutoRemove::Tempfile)?,
            },
        )));
        let (pack_entries_iter, pack_version): (
            Box<dyn Iterator<Item = Result<data::input::Entry, data::input::Error>>>,
            _,
        ) = match thin_pack_base_object_lookup {
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
                );
                let pack_version = pack_entries_iter.inner.version();
                let pack_entries_iter = data::input::EntriesToBytesIter::new(
                    pack_entries_iter,
                    LockWriter {
                        writer: data_file.clone(),
                    },
                    pack_version,
                    gix_hash::Kind::Sha1, // Thin packs imply a pack being transported, and there we only ever know SHA1 at the moment.
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
        let WriteOutcome {
            outcome,
            data_path,
            index_path,
            keep_path,
        } = crate::Bundle::inner_write(
            directory,
            progress,
            options,
            data_file,
            pack_entries_iter,
            should_interrupt,
            pack_version,
        )?;

        Ok(Outcome {
            index: outcome,
            object_hash,
            pack_version,
            data_path,
            index_path,
            keep_path,
        })
    }

    /// Equivalent to [`write_to_directory()`][crate::Bundle::write_to_directory()] but offloads reading of the pack into its own thread, hence the `Send + 'static'` bounds.
    ///
    /// # Note
    ///
    /// As it sends portions of the input to a thread it requires the 'static lifetime for the interrupt flags. This can only
    /// be satisfied by a static `AtomicBool` which is only suitable for programs that only run one of these operations at a time
    /// or don't mind that all of them abort when the flag is set.
    pub fn write_to_directory_eagerly(
        pack: Box<dyn io::Read + Send + 'static>,
        pack_size: Option<u64>,
        directory: Option<impl AsRef<Path>>,
        progress: &mut dyn DynNestedProgress,
        should_interrupt: &'static AtomicBool,
        thin_pack_base_object_lookup: Option<impl gix_object::Find + Send + 'static>,
        options: Options,
    ) -> Result<Outcome, Error> {
        let _span = gix_features::trace::coarse!("gix_pack::Bundle::write_to_directory_eagerly()");
        let mut read_progress = progress.add_child_with_id("read pack".into(), ProgressId::ReadPackBytes.into()); /* Bundle Write Read pack Bytes*/
        read_progress.init(pack_size.map(|s| s as usize), progress::bytes());
        let pack = progress::Read {
            inner: pack,
            progress: progress::ThroughputOnDrop::new(read_progress),
        };

        let data_file = Arc::new(parking_lot::Mutex::new(io::BufWriter::new(match directory.as_ref() {
            Some(directory) => gix_tempfile::new(directory, ContainingDirectory::Exists, AutoRemove::Tempfile)?,
            None => gix_tempfile::new(std::env::temp_dir(), ContainingDirectory::Exists, AutoRemove::Tempfile)?,
        })));
        let object_hash = options.object_hash;
        let eight_pages = 4096 * 8;
        let (pack_entries_iter, pack_version): (
            Box<dyn Iterator<Item = Result<data::input::Entry, data::input::Error>> + Send + 'static>,
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
            gix_features::parallel::EagerIterIf::new(move || num_objects > 25_000, pack_entries_iter, 5_000, 5);

        let WriteOutcome {
            outcome,
            data_path,
            index_path,
            keep_path,
        } = crate::Bundle::inner_write(
            directory,
            progress,
            options,
            data_file,
            Box::new(pack_entries_iter),
            should_interrupt,
            pack_version,
        )?;

        Ok(Outcome {
            index: outcome,
            object_hash,
            pack_version,
            data_path,
            index_path,
            keep_path,
        })
    }

    fn inner_write<'a>(
        directory: Option<impl AsRef<Path>>,
        progress: &mut dyn DynNestedProgress,
        Options {
            thread_limit,
            iteration_mode: _,
            index_version: index_kind,
            object_hash,
        }: Options,
        data_file: SharedTempFile,
        mut pack_entries_iter: Box<dyn Iterator<Item = Result<data::input::Entry, data::input::Error>> + 'a>,
        should_interrupt: &AtomicBool,
        pack_version: data::Version,
    ) -> Result<WriteOutcome, Error> {
        let mut indexing_progress = progress.add_child_with_id(
            "create index file".into(),
            ProgressId::IndexingSteps(Default::default()).into(),
        );
        Ok(match directory {
            Some(directory) => {
                let directory = directory.as_ref();
                let mut index_file = gix_tempfile::new(directory, ContainingDirectory::Exists, AutoRemove::Tempfile)?;

                let outcome = crate::index::File::write_data_iter_to_stream(
                    index_kind,
                    {
                        let data_file = Arc::clone(&data_file);
                        move || new_pack_file_resolver(data_file)
                    },
                    &mut pack_entries_iter,
                    thread_limit,
                    &mut indexing_progress,
                    &mut index_file,
                    should_interrupt,
                    object_hash,
                    pack_version,
                )?;
                drop(pack_entries_iter);

                if outcome.num_objects == 0 {
                    WriteOutcome {
                        outcome,
                        data_path: None,
                        index_path: None,
                        keep_path: None,
                    }
                } else {
                    let data_path = directory.join(format!("pack-{}.pack", outcome.data_hash.to_hex()));
                    let index_path = data_path.with_extension("idx");
                    let keep_path = if data_path.is_file() {
                        // avoid trying to overwrite existing files, we know they have the same content
                        // and this is likely to fail on Windows as negotiation opened the pack.
                        None
                    } else {
                        let keep_path = data_path.with_extension("keep");

                        std::fs::write(&keep_path, b"")?;
                        Arc::try_unwrap(data_file)
                            .expect("only one handle left after pack was consumed")
                            .into_inner()
                            .into_inner()
                            .map_err(|err| Error::from(err.into_error()))?
                            .persist(&data_path)?;
                        Some(keep_path)
                    };
                    if !index_path.is_file() {
                        index_file
                            .persist(&index_path)
                            .map_err(|err| {
                                gix_features::trace::warn!("pack file at \"{}\" is retained despite failing to move the index file into place. You can use plumbing to make it usable.",data_path.display());
                                err
                            })?;
                    }
                    WriteOutcome {
                        outcome,
                        data_path: Some(data_path),
                        index_path: Some(index_path),
                        keep_path,
                    }
                }
            }
            None => WriteOutcome {
                outcome: crate::index::File::write_data_iter_to_stream(
                    index_kind,
                    move || new_pack_file_resolver(data_file),
                    &mut pack_entries_iter,
                    thread_limit,
                    &mut indexing_progress,
                    &mut io::sink(),
                    should_interrupt,
                    object_hash,
                    pack_version,
                )?,
                data_path: None,
                index_path: None,
                keep_path: None,
            },
        })
    }
}

fn resolve_entry(range: data::EntryRange, mapped_file: &memmap2::Mmap) -> Option<&[u8]> {
    mapped_file.get(range.start as usize..range.end as usize)
}

#[allow(clippy::type_complexity)] // cannot typedef impl Fn
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

struct WriteOutcome {
    outcome: crate::index::write::Outcome,
    data_path: Option<PathBuf>,
    index_path: Option<PathBuf>,
    keep_path: Option<PathBuf>,
}
