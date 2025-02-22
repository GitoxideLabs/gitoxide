use std::{io, sync::atomic::AtomicBool};

pub use error::Error;
use gix_features::progress::{self, prodash::DynNestedProgress, Count, Progress};

use crate::cache::delta::{traverse, Tree};

mod error;

pub(crate) struct TreeEntry {
    pub id: gix_hash::ObjectId,
    pub crc32: u32,
}

/// Information gathered while executing [`write_data_iter_to_stream()`][crate::index::File::write_data_iter_to_stream]
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Outcome {
    /// The version of the verified index
    pub index_version: crate::index::Version,
    /// The verified checksum of the verified index
    pub index_hash: gix_hash::ObjectId,

    /// The hash of the '.pack' file, also found in its trailing bytes
    pub data_hash: gix_hash::ObjectId,
    /// The amount of objects that were verified, always the amount of objects in the pack.
    pub num_objects: u32,
}

/// The progress ids used in [`write_data_iter_from_stream()`][crate::index::File::write_data_iter_to_stream()].
///
/// Use this information to selectively extract the progress of interest in case the parent application has custom visualization.
#[derive(Debug, Copy, Clone)]
pub enum ProgressId {
    /// Counts the amount of objects that were index thus far.
    IndexObjects,
    /// The amount of bytes that were decompressed while decoding pack entries.
    ///
    /// This is done to determine entry boundaries.
    DecompressedBytes,
    /// The amount of objects whose hashes were computed.
    ///
    /// This is done by decoding them, which typically involves decoding delta objects.
    ResolveObjects,
    /// The amount of bytes that were decoded in total, as the sum of all bytes to represent all resolved objects.
    DecodedBytes,
    /// The amount of bytes written to the index file.
    IndexBytesWritten,
}

impl From<ProgressId> for gix_features::progress::Id {
    fn from(v: ProgressId) -> Self {
        match v {
            ProgressId::IndexObjects => *b"IWIO",
            ProgressId::DecompressedBytes => *b"IWDB",
            ProgressId::ResolveObjects => *b"IWRO",
            ProgressId::DecodedBytes => *b"IWDB",
            ProgressId::IndexBytesWritten => *b"IWBW",
        }
    }
}

/// Various ways of writing an index file from pack entries
impl crate::index::File {
    /// Write information about `entries` as obtained from a pack data file into a pack index file via the `out` stream.
    /// The resolver produced by `make_resolver` must resolve pack entries from the same pack data file that produced the
    /// `entries` iterator.
    ///
    /// * `kind` is the version of pack index to produce, use [`crate::index::Version::default()`] if in doubt.
    /// * `tread_limit` is used for a parallel tree traversal for obtaining object hashes with optimal performance.
    /// * `root_progress` is the top-level progress to stay informed about the progress of this potentially long-running
    ///   computation.
    /// * `object_hash` defines what kind of object hash we write into the index file.
    /// * `pack_version` is the version of the underlying pack for which `entries` are read. It's used in case none of these objects are provided
    ///   to compute a pack-hash.
    ///
    /// # Remarks
    ///
    /// * neither in-pack nor out-of-pack Ref Deltas are supported here, these must have been resolved beforehand.
    /// * `make_resolver()` will only be called after the iterator stopped returning elements and produces a function that
    ///   provides all bytes belonging to a pack entry writing them to the given mutable output `Vec`.
    ///   It should return `None` if the entry cannot be resolved from the pack that produced the `entries` iterator, causing
    ///   the write operation to fail.
    #[allow(clippy::too_many_arguments)]
    pub fn write_data_iter_to_stream<F, F2, R>(
        version: crate::index::Version,
        make_resolver: F,
        entries: &mut dyn Iterator<Item = Result<crate::data::input::Entry, crate::data::input::Error>>,
        thread_limit: Option<usize>,
        root_progress: &mut dyn DynNestedProgress,
        out: &mut dyn io::Write,
        should_interrupt: &AtomicBool,
        object_hash: gix_hash::Kind,
        pack_version: crate::data::Version,
    ) -> Result<Outcome, Error>
    where
        F: FnOnce() -> io::Result<(F2, R)>,
        R: Send + Sync,
        F2: for<'r> Fn(crate::data::EntryRange, &'r R) -> Option<&'r [u8]> + Send + Clone,
    {
        if version != crate::index::Version::default() {
            return Err(Error::Unsupported(version));
        }
        let mut num_objects: usize = 0;
        let mut last_seen_trailer = None;
        let (anticipated_num_objects, upper_bound) = entries.size_hint();
        let worst_case_num_objects_after_thin_pack_resolution = upper_bound.unwrap_or(anticipated_num_objects);
        let mut tree = Tree::with_capacity(worst_case_num_objects_after_thin_pack_resolution)?;
        let indexing_start = std::time::Instant::now();

        root_progress.init(Some(4), progress::steps());
        let mut objects_progress = root_progress.add_child_with_id("indexing".into(), ProgressId::IndexObjects.into());
        objects_progress.init(Some(anticipated_num_objects), progress::count("objects"));
        let mut decompressed_progress =
            root_progress.add_child_with_id("decompressing".into(), ProgressId::DecompressedBytes.into());
        decompressed_progress.init(None, progress::bytes());
        let mut pack_entries_end: u64 = 0;

        for entry in entries {
            let crate::data::input::Entry {
                header,
                pack_offset,
                crc32,
                header_size,
                compressed: _,
                compressed_size,
                decompressed_size,
                trailer,
            } = entry?;

            decompressed_progress.inc_by(decompressed_size as usize);

            let entry_len = u64::from(header_size) + compressed_size;
            pack_entries_end = pack_offset + entry_len;

            let crc32 = crc32.expect("crc32 to be computed by the iterator. Caller assures correct configuration.");

            use crate::data::entry::Header::*;
            match header {
                Tree | Blob | Commit | Tag => {
                    tree.add_root(
                        pack_offset,
                        TreeEntry {
                            id: object_hash.null(),
                            crc32,
                        },
                    )?;
                }
                RefDelta { .. } => return Err(Error::IteratorInvariantNoRefDelta),
                OfsDelta { base_distance } => {
                    let base_pack_offset =
                        crate::data::entry::Header::verified_base_pack_offset(pack_offset, base_distance).ok_or(
                            Error::IteratorInvariantBaseOffset {
                                pack_offset,
                                distance: base_distance,
                            },
                        )?;
                    tree.add_child(
                        base_pack_offset,
                        pack_offset,
                        TreeEntry {
                            id: object_hash.null(),
                            crc32,
                        },
                    )?;
                }
            }
            last_seen_trailer = trailer;
            num_objects += 1;
            objects_progress.inc();
        }
        let num_objects: u32 = num_objects
            .try_into()
            .map_err(|_| Error::IteratorInvariantTooManyObjects(num_objects))?;

        objects_progress.show_throughput(indexing_start);
        decompressed_progress.show_throughput(indexing_start);
        drop(objects_progress);
        drop(decompressed_progress);

        root_progress.inc();

        let (resolver, pack) = make_resolver()?;
        let sorted_pack_offsets_by_oid = {
            let traverse::Outcome { roots, children } = tree.traverse(
                resolver,
                &pack,
                pack_entries_end,
                |data,
                 _progress,
                 traverse::Context {
                     entry,
                     decompressed: bytes,
                     ..
                 }| {
                    modify_base(data, entry, bytes, version.hash());
                    Ok::<_, Error>(())
                },
                traverse::Options {
                    object_progress: Box::new(
                        root_progress.add_child_with_id("Resolving".into(), ProgressId::ResolveObjects.into()),
                    ),
                    size_progress: &mut root_progress
                        .add_child_with_id("Decoding".into(), ProgressId::DecodedBytes.into()),
                    thread_limit,
                    should_interrupt,
                    object_hash,
                },
            )?;
            root_progress.inc();

            let mut items = roots;
            items.extend(children);
            {
                let _progress =
                    root_progress.add_child_with_id("sorting by id".into(), gix_features::progress::UNKNOWN);
                items.sort_by_key(|e| e.data.id);
            }

            root_progress.inc();
            items
        };

        let pack_hash = match last_seen_trailer {
            Some(ph) => ph,
            None if num_objects == 0 => {
                let header = crate::data::header::encode(pack_version, 0);
                let mut hasher = gix_features::hash::hasher(object_hash);
                hasher.update(&header);
                gix_hash::ObjectId::from(hasher.digest())
            }
            None => return Err(Error::IteratorInvariantTrailer),
        };
        let index_hash = crate::index::encode::write_to(
            out,
            sorted_pack_offsets_by_oid,
            &pack_hash,
            version,
            &mut root_progress.add_child_with_id("writing index file".into(), ProgressId::IndexBytesWritten.into()),
        )?;
        root_progress.show_throughput_with(
            indexing_start,
            num_objects as usize,
            progress::count("objects").expect("unit always set"),
            progress::MessageLevel::Success,
        );
        Ok(Outcome {
            index_version: version,
            index_hash,
            data_hash: pack_hash,
            num_objects,
        })
    }
}

fn modify_base(entry: &mut TreeEntry, pack_entry: &crate::data::Entry, decompressed: &[u8], hash: gix_hash::Kind) {
    let object_kind = pack_entry.header.as_kind().expect("base object as source of iteration");
    let id = gix_object::compute_hash(hash, object_kind, decompressed);
    entry.id = id;
}
