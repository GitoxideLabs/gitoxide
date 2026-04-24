use filetime::FileTime;

use crate::{entry, extension, Entry, State, Version};

mod entries;
///
pub mod header;

mod error {
    use crate::{decode, extension};
    use std::collections::TryReserveError;

    /// The error returned by [`State::from_bytes()`][crate::State::from_bytes()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        Header(#[from] decode::header::Error),
        #[error("Could not hash index data")]
        Hasher(#[from] gix_hash::hasher::Error),
        #[error("Index data would require more memory than can be reserved")]
        OutOfMemory,
        #[error("Could not parse entry at index {index}")]
        Entry { index: u32 },
        #[error("Mandatory extension wasn't implemented or malformed.")]
        Extension(#[from] extension::decode::Error),
        #[error("Index trailer should have been {expected} bytes long, but was {actual}")]
        UnexpectedTrailerLength { expected: usize, actual: usize },
        #[error("Shared index checksum mismatch")]
        Verify(#[from] gix_hash::verify::Error),
    }

    impl From<TryReserveError> for Error {
        #[cold]
        fn from(_: TryReserveError) -> Self {
            Self::OutOfMemory
        }
    }
}
pub use error::Error;
use gix_features::parallel::InOrderIter;

use crate::util::read_u32;

/// Options to define how to decode an index state [from bytes][State::from_bytes()].
#[derive(Debug, Default, Clone, Copy)]
pub struct Options {
    /// If Some(_), we are allowed to use more than one thread. If Some(N), use no more than N threads. If Some(0)|None, use as many threads
    /// as there are logical cores.
    ///
    /// This applies to loading extensions in parallel to entries if the common EOIE extension is available.
    /// It also allows to use multiple threads for loading entries if the IEOT extension is present.
    pub thread_limit: Option<usize>,
    /// The minimum size in bytes to load extensions in their own thread, assuming there is enough `num_threads` available.
    /// If set to 0, for example, extensions will always be read in their own thread if enough threads are available.
    pub min_extension_block_in_bytes_for_threading: usize,
    /// Set the expected hash of this index if we are read as part of a `link` extension.
    ///
    /// We will abort reading this file if it doesn't match.
    pub expected_checksum: Option<gix_hash::ObjectId>,
    /// Configure the maximum size of a single allocation caused by untrusted on-disk index data.
    ///
    /// Use `None` to disable the limit, which is also the default.
    pub alloc_limit_bytes: Option<usize>,
}

impl State {
    /// Decode an index state from `data` and store `timestamp` in the resulting instance for pass-through, assuming `object_hash`
    /// to be used through the file. Also return the stored hash over all bytes in `data` or `None` if none was written due to `index.skipHash`.
    pub fn from_bytes(
        data: &[u8],
        timestamp: FileTime,
        object_hash: gix_hash::Kind,
        _options @ Options {
            thread_limit,
            min_extension_block_in_bytes_for_threading,
            expected_checksum,
            alloc_limit_bytes,
        }: Options,
    ) -> Result<(Self, Option<gix_hash::ObjectId>), Error> {
        let _span = gix_features::trace::detail!("gix_index::State::from_bytes()", options = ?_options);
        let (version, num_entries, post_header_data) = header::decode(data, object_hash)?;
        let start_of_extensions = extension::end_of_index_entry::decode(data, object_hash)?;
        if num_entries as usize > entries::max_entries_possible(data.len(), start_of_extensions, object_hash, version) {
            return Err(header::Error::Corrupt("Declared entry count exceeds possible entries for file size").into());
        }

        let mut num_threads = gix_features::parallel::num_threads(thread_limit);
        let path_backing_buffer_size = entries::estimate_path_storage_requirements_in_bytes(
            num_entries,
            data.len(),
            start_of_extensions,
            object_hash,
            version,
        );
        ensure_in_alloc_limit(
            (num_entries as usize)
                .checked_mul(std::mem::size_of::<Entry>())
                .ok_or(Error::OutOfMemory)?,
            alloc_limit_bytes,
        )?;
        ensure_in_alloc_limit(path_backing_buffer_size, alloc_limit_bytes)?;

        let (entries, ext, data) = match start_of_extensions {
            Some(offset) if num_threads > 1 => {
                let extensions_data = &data[offset..];
                let index_offsets_table = extension::index_entry_offset_table::find(extensions_data, object_hash);
                let (entries_res, ext_res) = gix_features::parallel::threads(|scope| {
                    let extension_loading =
                        (extensions_data.len() > min_extension_block_in_bytes_for_threading).then({
                            num_threads -= 1;
                            || {
                                gix_features::parallel::build_thread()
                                    .name("gix-index.from_bytes.load-extensions".into())
                                    .spawn_scoped(scope, || {
                                        extension::decode::all(extensions_data, object_hash, alloc_limit_bytes)
                                    })
                                    .expect("valid name")
                            }
                        });
                    let entries_res = match index_offsets_table {
                        Some(entry_offsets) => {
                            let chunk_size = (entry_offsets.len() as f32 / num_threads as f32).ceil() as usize;
                            let entry_offsets_chunked = entry_offsets.chunks(chunk_size);
                            let num_chunks = entry_offsets_chunked.len();
                            let mut threads = Vec::with_capacity(num_chunks);
                            for (id, chunks) in entry_offsets_chunked.enumerate() {
                                let chunks = chunks.to_vec();
                                threads.push(
                                    gix_features::parallel::build_thread()
                                        .name(format!("gix-index.from_bytes.read-entries.{id}"))
                                        .spawn_scoped(scope, move || {
                                            let num_entries_for_chunks =
                                                chunks.iter().map(|c| c.num_entries).sum::<u32>() as usize;
                                            let mut entries = vec_with_capacity(num_entries_for_chunks)?;
                                            let path_backing_buffer_size_for_chunks =
                                                entries::estimate_path_storage_requirements_in_bytes(
                                                    num_entries_for_chunks as u32,
                                                    data.len() / num_chunks,
                                                    start_of_extensions.map(|ofs| ofs / num_chunks),
                                                    object_hash,
                                                    version,
                                                );
                                            let mut path_backing =
                                                vec_with_capacity(path_backing_buffer_size_for_chunks)?;
                                            let mut is_sparse = false;
                                            for offset in chunks {
                                                let (
                                                    entries::Outcome {
                                                        is_sparse: chunk_is_sparse,
                                                    },
                                                    _data,
                                                ) = entries::chunk(
                                                    &data[offset.from_beginning_of_file as usize..],
                                                    &mut entries,
                                                    &mut path_backing,
                                                    offset.num_entries,
                                                    object_hash,
                                                    version,
                                                )?;
                                                is_sparse |= chunk_is_sparse;
                                            }
                                            Ok::<_, Error>((
                                                id,
                                                EntriesOutcome {
                                                    entries,
                                                    path_backing,
                                                    is_sparse,
                                                },
                                            ))
                                        })
                                        .expect("valid name"),
                                );
                            }
                            let mut results =
                                InOrderIter::from(threads.into_iter().map(|thread| thread.join().unwrap()));
                            let mut acc = results.next().expect("have at least two results, one per thread");
                            // We explicitly don't adjust the reserve in acc and rather allow for more copying
                            // to happens as vectors grow to keep the peak memory size low.
                            // NOTE: one day, we might use a memory pool for paths. We could encode the block of memory
                            //       in some bytes in the path offset. That way there is more indirection/slower access
                            //       to the path, but it would save time here.
                            //       As it stands, `git` is definitely more efficient at this and probably uses less memory too.
                            //       Maybe benchmarks can tell if that is noticeable later at 200/400GB/s memory bandwidth, or maybe just
                            //       100GB/s on a single core.
                            while let (Ok(lhs), Some(res)) = (acc.as_mut(), results.next()) {
                                match res {
                                    Ok(mut rhs) => {
                                        lhs.is_sparse |= rhs.is_sparse;
                                        let ofs = lhs.path_backing.len();
                                        lhs.path_backing.append(&mut rhs.path_backing);
                                        lhs.entries.extend(rhs.entries.into_iter().map(|mut e| {
                                            e.path.start += ofs;
                                            e.path.end += ofs;
                                            e
                                        }));
                                    }
                                    Err(err) => {
                                        acc = Err(err);
                                    }
                                }
                            }
                            acc.map(|acc| (acc, &data[data.len() - object_hash.len_in_bytes()..]))
                        }
                        None => entries(
                            post_header_data,
                            path_backing_buffer_size,
                            num_entries,
                            object_hash,
                            version,
                        ),
                    };
                    let ext_res = extension_loading.map_or_else(
                        || extension::decode::all(extensions_data, object_hash, alloc_limit_bytes),
                        |thread| thread.join().unwrap(),
                    );
                    (entries_res, ext_res)
                });
                let (ext, data) = ext_res?;
                (entries_res?.0, ext, data)
            }
            None | Some(_) => {
                let (entries, data) = entries(
                    post_header_data,
                    path_backing_buffer_size,
                    num_entries,
                    object_hash,
                    version,
                )?;
                let (ext, data) = extension::decode::all(data, object_hash, alloc_limit_bytes)?;
                (entries, ext, data)
            }
        };

        if data.len() != object_hash.len_in_bytes() {
            return Err(Error::UnexpectedTrailerLength {
                expected: object_hash.len_in_bytes(),
                actual: data.len(),
            });
        }

        let checksum = gix_hash::ObjectId::from_bytes_or_panic(data);
        let checksum = (!checksum.is_null()).then_some(checksum);
        if let Some((expected_checksum, actual_checksum)) = expected_checksum.zip(checksum) {
            actual_checksum.verify(&expected_checksum)?;
        }
        let EntriesOutcome {
            entries,
            path_backing,
            mut is_sparse,
        } = entries;
        let extension::decode::Outcome {
            tree,
            link,
            resolve_undo,
            untracked,
            fs_monitor,
            is_sparse: is_sparse_from_ext, // a marker is needed in case there are no directories
            end_of_index,
            offset_table,
        } = ext;
        is_sparse |= is_sparse_from_ext;

        Ok((
            State {
                object_hash,
                timestamp,
                version,
                entries,
                path_backing,
                is_sparse,

                end_of_index_at_decode_time: end_of_index,
                offset_table_at_decode_time: offset_table,
                tree,
                link,
                resolve_undo,
                untracked,
                fs_monitor,
            },
            checksum,
        ))
    }
}

struct EntriesOutcome {
    pub entries: Vec<Entry>,
    pub path_backing: Vec<u8>,
    pub is_sparse: bool,
}

fn vec_with_capacity<T>(capacity: usize) -> Result<Vec<T>, Error> {
    let mut vec = Vec::new();
    vec.try_reserve(capacity).map_err(|_| Error::OutOfMemory)?;
    Ok(vec)
}

fn entries(
    post_header_data: &[u8],
    path_backing_buffer_size: usize,
    num_entries: u32,
    object_hash: gix_hash::Kind,
    version: Version,
) -> Result<(EntriesOutcome, &[u8]), Error> {
    let mut entries = vec_with_capacity(num_entries as usize)?;
    let mut path_backing = vec_with_capacity(path_backing_buffer_size)?;
    entries::chunk(
        post_header_data,
        &mut entries,
        &mut path_backing,
        num_entries,
        object_hash,
        version,
    )
    .map(|(entries::Outcome { is_sparse }, data): (entries::Outcome, &[u8])| {
        (
            EntriesOutcome {
                entries,
                path_backing,
                is_sparse,
            },
            data,
        )
    })
}

pub(crate) fn stat(data: &[u8]) -> Option<(entry::Stat, &[u8])> {
    let (ctime_secs, data) = read_u32(data)?;
    let (ctime_nsecs, data) = read_u32(data)?;
    let (mtime_secs, data) = read_u32(data)?;
    let (mtime_nsecs, data) = read_u32(data)?;
    let (dev, data) = read_u32(data)?;
    let (ino, data) = read_u32(data)?;
    let (uid, data) = read_u32(data)?;
    let (gid, data) = read_u32(data)?;
    let (size, data) = read_u32(data)?;
    Some((
        entry::Stat {
            mtime: entry::stat::Time {
                secs: ctime_secs,
                nsecs: ctime_nsecs,
            },
            ctime: entry::stat::Time {
                secs: mtime_secs,
                nsecs: mtime_nsecs,
            },
            dev,
            ino,
            uid,
            gid,
            size,
        },
        data,
    ))
}

fn ensure_in_alloc_limit(size: usize, alloc_limit_bytes: Option<usize>) -> Result<(), Error> {
    if alloc_limit_bytes.is_some_and(|limit| size > limit) {
        return Err(Error::OutOfMemory);
    }
    Ok(())
}
