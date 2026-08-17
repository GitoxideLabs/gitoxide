use std::{
    fs, io,
    io::{BufRead, Read, Seek, SeekFrom},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

use gix_features::progress::{self, Progress};

use crate::{cache::delta::Tree, data};

/// Returned by [`Tree::from_offsets_in_pack()`]
#[derive(thiserror::Error, Debug)]
#[allow(missing_docs)]
pub enum Error {
    #[error("{message}")]
    Io { source: io::Error, message: &'static str },
    #[error(transparent)]
    Header(#[from] crate::data::header::decode::Error),
    #[error("Could find object with id {id} in this pack. Thin packs are not supported")]
    UnresolvedRefDelta { id: gix_hash::ObjectId },
    #[error(transparent)]
    Tree(#[from] crate::cache::delta::Error),
    #[error("Interrupted")]
    Interrupted,
}

const PACK_HEADER_LEN: usize = 12;

/// Generate tree from certain input
impl<T> Tree<T> {
    /// Create a new `Tree` from any data sorted by offset, ascending as returned by the `data_sorted_by_offsets` iterator.
    /// * `get_pack_offset(item: &T) -> data::Offset` is a function returning the pack offset of the given item, which can be used
    ///   for obtaining the objects entry within the pack.
    /// * `pack_path` is the path to the pack file itself and from which to read the entry data, which is a pack file matching the offsets
    ///   returned by `get_pack_offset(…)`.
    /// * `progress` is used to track progress when creating the tree.
    /// * `resolve_in_pack_id(gix_hash::oid) -> Option<data::Offset>` takes an object ID and tries to resolve it to an object within this pack if
    ///   possible. Failing to do so aborts the operation, and this function is not expected to be called in usual packs. It's a theoretical
    ///   possibility though as old packs might have referred to their objects using the 20 bytes hash, instead of their encoded offset from the base.
    ///
    /// Note that the sort order is ascending. The given pack file path must match the provided offsets.
    pub fn from_offsets_in_pack(
        pack_path: &std::path::Path,
        data_sorted_by_offsets: impl Iterator<Item = T>,
        get_pack_offset: &dyn Fn(&T) -> data::Offset,
        resolve_in_pack_id: &dyn Fn(&gix_hash::oid) -> Option<data::Offset>,
        progress: &mut dyn Progress,
        should_interrupt: &AtomicBool,
        object_hash: gix_hash::Kind,
    ) -> Result<Self, Error> {
        let mut r = io::BufReader::with_capacity(
            8192 * 8, // this value directly corresponds to performance, 8k (default) is about 4x slower than 64k
            fs::File::open(pack_path).map_err(|err| Error::Io {
                source: err,
                message: "open pack path",
            })?,
        );

        let anticipated_num_objects = data_sorted_by_offsets
            .size_hint()
            .1
            .inspect(|&num_objects| {
                progress.init(Some(num_objects), progress::count("objects"));
            })
            .unwrap_or_default();
        let mut tree = Tree::with_capacity(anticipated_num_objects)?;

        {
            // safety check - assure ourselves it's a pack we can handle
            let mut buf = [0u8; PACK_HEADER_LEN];
            r.read_exact(&mut buf).map_err(|err| Error::Io {
                source: err,
                message: "reading header buffer with at least 12 bytes failed - pack file truncated?",
            })?;
            crate::data::header::decode(&buf)?;
        }

        let then = Instant::now();

        let mut previous_cursor_position = None::<u64>;

        let hash_len = object_hash.len_in_bytes();
        for (idx, data) in data_sorted_by_offsets.enumerate() {
            let pack_offset = get_pack_offset(&data);
            if let Some(previous_offset) = previous_cursor_position {
                Self::advance_cursor_to_pack_offset(&mut r, pack_offset, previous_offset)?;
            }
            let entry = crate::data::Entry::from_read(&mut r, pack_offset, hash_len).map_err(|err| Error::Io {
                source: err,
                message: "EOF while parsing header",
            })?;
            previous_cursor_position = Some(pack_offset + entry.header_size() as u64);

            use crate::data::entry::Header::*;
            match entry.header {
                Tree | Blob | Commit | Tag => {
                    tree.add_root(pack_offset, data)?;
                }
                RefDelta { base_id } => {
                    resolve_in_pack_id(base_id.as_ref())
                        .ok_or(Error::UnresolvedRefDelta { id: base_id })
                        .and_then(|base_pack_offset| {
                            tree.add_child(base_pack_offset, pack_offset, data).map_err(Into::into)
                        })?;
                }
                OfsDelta { base_distance } => {
                    let base_pack_offset = pack_offset
                        .checked_sub(base_distance)
                        .expect("in bound distance for deltas");
                    tree.add_child(base_pack_offset, pack_offset, data)?;
                }
            }
            progress.inc();
            if idx % 10_000 == 0 && should_interrupt.load(Ordering::SeqCst) {
                return Err(Error::Interrupted);
            }
        }

        progress.show_throughput(then);
        Ok(tree)
    }

    fn advance_cursor_to_pack_offset(
        r: &mut io::BufReader<fs::File>,
        pack_offset: u64,
        previous_offset: u64,
    ) -> Result<(), Error> {
        let bytes_to_skip: u64 = pack_offset
            .checked_sub(previous_offset)
            .expect("continuously ascending pack offsets");
        if bytes_to_skip == 0 {
            return Ok(());
        }
        let buf = r.fill_buf().map_err(|err| Error::Io {
            source: err,
            message: "skip bytes",
        })?;
        if buf.is_empty() {
            // This means we have reached the end of file and can't make progress anymore, before we have satisfied our need
            // for more
            return Err(Error::Io {
                source: io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "ran out of bytes before reading desired amount of bytes",
                ),
                message: "index file is damaged or corrupt",
            });
        }
        if bytes_to_skip <= u64::try_from(buf.len()).expect("sensible buffer size") {
            // SAFETY: bytes_to_skip <= buf.len() <= usize::MAX
            r.consume(bytes_to_skip as usize);
        } else {
            r.seek(SeekFrom::Start(pack_offset)).map_err(|err| Error::Io {
                source: err,
                message: "seek to next entry",
            })?;
        }
        Ok(())
    }
}
