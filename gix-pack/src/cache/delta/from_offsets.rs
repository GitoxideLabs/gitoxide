use gix_error::Result;
use std::{
    fs, io,
    io::{BufRead, Read, Seek, SeekFrom},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

use gix_error::{ErrorExt, OptionExt, ResultExt, bail, cancelled, message};
use gix_utils::progress::{self, Progress};

use crate::{cache::delta::Tree, data};

/// Generate tree from certain input
impl<T> Tree<T> {
    /// Create a new `Tree` from any data sorted by offset, ascending as returned by the `data_sorted_by_offsets` iterator.
    /// * `get_pack_offset(item: &T) -> data::Offset` is a function returning the pack offset of the given item, which can be used
    ///   for obtaining the objects entry within the pack.
    /// * `pack_path` is the path to the pack file itself and from which to read the entry data, which is a pack file matching the offsets
    ///   returned by `get_pack_offset(…)`.
    /// * `progress` is used to track progress when creating the tree.
    /// * `resolve_in_pack_id(&gix_hash::oid) -> Result<Option<data::Offset>>` takes an object ID and tries to resolve it to an offset
    ///   within this pack. `Err` aborts the operation with the resolver's error unchanged; `Ok(None)` aborts with a missing-base error.
    ///   The callback is only called for ref-deltas, which refer to their base by object ID instead of an encoded offset.
    /// * `alloc_limit_bytes` limits the requested total size of each of the two item buffers (roots and children)
    ///   independently, including during growth, rather than just the added bytes. Together these buffers can request
    ///   up to twice the limit. This is not a total tree-memory budget: child-index lists, bookkeeping, allocations
    ///   owned by `T`, and allocator overhead are not counted, and reallocation can temporarily retain old buffers.
    ///   `None` disables the limit.
    ///
    /// # Ref-delta bases
    ///
    /// This constructor requires every base to have an offset in this pack and an item in `data_sorted_by_offsets`.
    /// Forward references are supported: children whose bases occur later are attached before traversal.
    /// Unlike streaming indexing, this constructor does not defer base-ID lookup until objects are decoded or insert
    /// external bases. To complete thin packs, use the streaming indexing path with `data::input::LookupRefDeltaObjectsIter`.
    ///
    /// Note that the sort order is ascending. The given pack file path must match the provided offsets.
    #[expect(clippy::too_many_arguments)]
    pub fn from_offsets_in_pack(
        pack_path: &std::path::Path,
        data_sorted_by_offsets: impl Iterator<Item = T>,
        get_pack_offset: &dyn Fn(&T) -> data::Offset,
        resolve_in_pack_id: &dyn Fn(&gix_hash::oid) -> Result<Option<data::Offset>>,
        progress: &mut dyn Progress,
        should_interrupt: &AtomicBool,
        object_hash: gix_hash::Kind,
        alloc_limit_bytes: Option<usize>,
    ) -> Result<Self> {
        let mut r = io::BufReader::with_capacity(
            8192 * 8, // this value directly corresponds to performance, 8k (default) is about 4x slower than 64k
            fs::File::open(pack_path).or_raise(|| message("open pack path"))?,
        );

        let anticipated_num_objects = data_sorted_by_offsets
            .size_hint()
            .1
            .inspect(|&num_objects| {
                progress.init(Some(num_objects), progress::count("objects"));
            })
            .unwrap_or_default();
        let mut tree = Tree::with_capacity(anticipated_num_objects, alloc_limit_bytes)?;

        {
            // safety check - assure ourselves it's a pack we can handle
            let mut buf = [0u8; data::header::SIZE];
            r.read_exact(&mut buf)
                .or_raise(|| message("reading header buffer with at least 12 bytes failed - pack file truncated?"))?;
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
            let entry = crate::data::Entry::from_read(&mut r, pack_offset, hash_len)
                .or_raise(|| message("EOF while parsing header"))?;
            previous_cursor_position = Some(pack_offset + entry.header_size() as u64);

            use crate::data::entry::Header::*;
            match entry.header {
                Tree | Blob | Commit | Tag => {
                    tree.add_root(pack_offset, data)?;
                }
                RefDelta { base_id } => {
                    let base_pack_offset = resolve_in_pack_id(base_id.as_ref())?.ok_or_raise(|| {
                        message!("Base object {base_id} was not found in this pack; offset-based tree construction requires in-pack bases")
                            .not_found()
                    })?;
                    tree.add_child(base_pack_offset, pack_offset, data)?;
                }
                OfsDelta { base_distance } => {
                    let Some(base_pack_offset) =
                        crate::data::entry::Header::verified_base_pack_offset(pack_offset, base_distance)
                    else {
                        bail!(gix_error::corruption(format!(
                            "OFS_DELTA base distance {base_distance} is invalid for pack offset {pack_offset}"
                        )));
                    };
                    tree.add_child(base_pack_offset, pack_offset, data)?;
                }
            }
            progress.inc();
            if idx % 10_000 == 0 && should_interrupt.load(Ordering::SeqCst) {
                bail!(cancelled("Interrupted"));
            }
        }

        progress.show_throughput(then);
        Ok(tree)
    }

    fn advance_cursor_to_pack_offset(
        r: &mut io::BufReader<fs::File>,
        pack_offset: u64,
        previous_offset: u64,
    ) -> Result {
        let bytes_to_skip: u64 = pack_offset
            .checked_sub(previous_offset)
            .ok_or_raise(|| gix_error::corruption("Pack index offsets overlap an entry header"))?;
        if bytes_to_skip == 0 {
            return Ok(());
        }
        let buf = r.fill_buf().or_raise(|| message("skip bytes"))?;
        if buf.is_empty() {
            // This means we have reached the end of file and can't make progress anymore, before we have satisfied our need
            // for more
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "ran out of bytes before reading desired amount of bytes",
            )
            .and_raise(message("index file is damaged or corrupt")));
        }
        if bytes_to_skip <= u64::try_from(buf.len()).expect("sensible buffer size") {
            // SAFETY: bytes_to_skip <= buf.len() <= usize::MAX
            r.consume(bytes_to_skip as usize);
        } else {
            r.seek(SeekFrom::Start(pack_offset))
                .or_raise(|| message("seek to next entry"))?;
        }
        Ok(())
    }
}
