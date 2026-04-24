use std::{cmp::Ordering, sync::atomic::AtomicBool, time::Instant};

use gix_features::progress::{Count, DynNestedProgress, Progress};

use crate::{exact_vec, index, multi_index::File};

///
pub mod integrity {
    use crate::multi_index::EntryIndex;

    /// Returned by [`multi_index::File::verify_integrity()`][crate::multi_index::File::verify_integrity()].
    #[derive(thiserror::Error, Debug)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Object {id} should be at pack-offset {expected_pack_offset} but was found at {actual_pack_offset}")]
        PackOffsetMismatch {
            id: gix_hash::ObjectId,
            expected_pack_offset: u64,
            actual_pack_offset: u64,
        },
        #[error(transparent)]
        MultiIndexChecksum(#[from] crate::multi_index::verify::checksum::Error),
        #[error(transparent)]
        IndexIntegrity(#[from] crate::index::verify::integrity::Error),
        #[error(transparent)]
        BundleInit(#[from] crate::bundle::init::Error),
        #[error("Counted {actual} objects, but expected {expected} as per multi-index")]
        UnexpectedObjectCount { actual: usize, expected: usize },
        #[error("{id} wasn't found in the index referenced in the multi-pack index")]
        OidNotFound { id: gix_hash::ObjectId },
        #[error("The object id at multi-index entry {index} wasn't in order")]
        OutOfOrder { index: EntryIndex },
        #[error("The fan at index {index} is out of order as it's larger then the following value.")]
        Fan { index: usize },
        #[error("The multi-index claims to have no objects")]
        Empty,
        #[error("The multi-index path '{path}' has no parent directory")]
        InvalidPath { path: std::path::PathBuf },
        #[error("Interrupted")]
        Interrupted,
    }

    /// Returned by [`multi_index::File::verify_integrity()`][crate::multi_index::File::verify_integrity()].
    pub struct Outcome {
        /// The computed checksum of the multi-index which matched the stored one.
        pub actual_index_checksum: gix_hash::ObjectId,
        /// The for each entry in [`index_names()`][super::File::index_names()] provide the corresponding pack traversal outcome.
        pub pack_traverse_statistics: Vec<crate::index::traverse::Statistics>,
    }

    /// The progress ids used in [`multi_index::File::verify_integrity()`][crate::multi_index::File::verify_integrity()].
    ///
    /// Use this information to selectively extract the progress of interest in case the parent application has custom visualization.
    #[derive(Debug, Copy, Clone)]
    pub enum ProgressId {
        /// The amount of bytes read to verify the multi-index checksum.
        ChecksumBytes,
        /// The amount of objects whose offset has been checked.
        ObjectOffsets,
    }

    impl From<ProgressId> for gix_features::progress::Id {
        fn from(v: ProgressId) -> Self {
            match v {
                ProgressId::ChecksumBytes => *b"MVCK",
                ProgressId::ObjectOffsets => *b"MVOF",
            }
        }
    }
}

///
pub mod checksum {
    /// Returned by [`multi_index::File::verify_checksum()`][crate::multi_index::File::verify_checksum()].
    pub type Error = crate::verify::checksum::Error;
}

impl<T> File<T>
where
    T: crate::FileData,
{
    /// Validate that our [`checksum()`][File::checksum()] matches the actual contents
    /// of this index file, and return it if it does.
    pub fn verify_checksum(
        &self,
        progress: &mut dyn Progress,
        should_interrupt: &AtomicBool,
    ) -> Result<gix_hash::ObjectId, checksum::Error> {
        crate::verify::checksum_on_disk_or_mmap(
            self.path(),
            &self.data,
            self.checksum(),
            self.object_hash,
            progress,
            should_interrupt,
        )
    }

    /// Similar to [`verify_integrity()`][File::verify_integrity()] but without any deep inspection of objects.
    ///
    /// Instead we only validate the contents of the multi-index itself.
    pub fn verify_integrity_fast(
        &self,
        progress: &mut dyn DynNestedProgress,
        should_interrupt: &AtomicBool,
    ) -> Result<gix_hash::ObjectId, integrity::Error> {
        self.verify_integrity_inner(
            progress,
            should_interrupt,
            false,
            index::verify::integrity::Options::default(),
        )
        .map_err(|err| match err {
            index::traverse::Error::Processor(err) => err,
            _ => unreachable!("BUG: no other error type is possible"),
        })
        .map(|o| o.actual_index_checksum)
    }

    /// Similar to [`crate::Bundle::verify_integrity()`] but checks all contained indices and their packs.
    ///
    /// Note that it's considered a failure if an index doesn't have a corresponding pack.
    pub fn verify_integrity<C, F>(
        &self,
        progress: &mut dyn DynNestedProgress,
        should_interrupt: &AtomicBool,
        options: index::verify::integrity::Options<F>,
    ) -> Result<integrity::Outcome, index::traverse::Error<integrity::Error>>
    where
        C: crate::cache::DecodeEntry,
        F: Fn() -> C + Send + Clone,
    {
        self.verify_integrity_inner(progress, should_interrupt, true, options)
    }

    fn verify_integrity_inner<C, F>(
        &self,
        progress: &mut dyn DynNestedProgress,
        should_interrupt: &AtomicBool,
        deep_check: bool,
        options: index::verify::integrity::Options<F>,
    ) -> Result<integrity::Outcome, index::traverse::Error<integrity::Error>>
    where
        C: crate::cache::DecodeEntry,
        F: Fn() -> C + Send + Clone,
    {
        let parent = self.path.parent().ok_or_else(|| {
            index::traverse::Error::Processor(integrity::Error::InvalidPath {
                path: self.path.clone(),
            })
        })?;

        let actual_index_checksum = self
            .verify_checksum(
                &mut progress.add_child_with_id(
                    format!("{}: checksum", self.path.display()),
                    integrity::ProgressId::ChecksumBytes.into(),
                ),
                should_interrupt,
            )
            .map_err(integrity::Error::from)
            .map_err(index::traverse::Error::Processor)?;

        if let Some(first_invalid) = crate::verify::fan(&self.fan) {
            return Err(index::traverse::Error::Processor(integrity::Error::Fan {
                index: first_invalid,
            }));
        }

        if self.num_objects == 0 {
            return Err(index::traverse::Error::Processor(integrity::Error::Empty));
        }

        let mut pack_traverse_statistics = Vec::new();

        let operation_start = Instant::now();
        let mut total_objects_checked = 0;
        let mut pack_ids_and_offsets = exact_vec(self.num_objects as usize);
        {
            let order_start = Instant::now();
            let mut progress = progress.add_child_with_id("checking oid order".into(), gix_features::progress::UNKNOWN);
            progress.init(
                Some(self.num_objects as usize),
                gix_features::progress::count("objects"),
            );

            for entry_index in 0..(self.num_objects - 1) {
                let lhs = self.oid_at_index(entry_index);
                let rhs = self.oid_at_index(entry_index + 1);

                if rhs.cmp(lhs) != Ordering::Greater {
                    return Err(index::traverse::Error::Processor(integrity::Error::OutOfOrder {
                        index: entry_index,
                    }));
                }
                let (pack_id, _) = self.pack_id_and_pack_offset_at_index(entry_index);
                pack_ids_and_offsets.push((pack_id, entry_index));
                progress.inc();
            }
            {
                let entry_index = self.num_objects - 1;
                let (pack_id, _) = self.pack_id_and_pack_offset_at_index(entry_index);
                pack_ids_and_offsets.push((pack_id, entry_index));
            }
            // sort by pack-id to allow handling all indices matching a pack while its open.
            pack_ids_and_offsets.sort_by_key(|l| l.0);
            progress.show_throughput(order_start);
        };

        progress.init(
            Some(self.num_indices as usize),
            gix_features::progress::count("indices"),
        );

        let mut pack_ids_slice = pack_ids_and_offsets.as_slice();

        for (pack_id, index_file_name) in self.index_names.iter().enumerate() {
            progress.set_name(index_file_name.display().to_string());
            progress.inc();

            let mut bundle = None;
            let index;
            let index_path = parent.join(index_file_name);
            let index = if deep_check {
                bundle = crate::Bundle::at(index_path, self.object_hash)
                    .map_err(integrity::Error::from)
                    .map_err(index::traverse::Error::Processor)?
                    .into();
                bundle.as_ref().map(|b| &b.index).expect("just set")
            } else {
                index = Some(
                    index::File::at(index_path, self.object_hash)
                        .map_err(|err| integrity::Error::BundleInit(crate::bundle::init::Error::Index(err)))
                        .map_err(index::traverse::Error::Processor)?,
                );
                index.as_ref().expect("just set")
            };

            let slice_end = pack_ids_slice.partition_point(|e| e.0 == pack_id as crate::data::Id);
            let multi_index_entries_to_check = &pack_ids_slice[..slice_end];
            {
                let offset_start = Instant::now();
                let mut offsets_progress = progress.add_child_with_id(
                    "verify object offsets".into(),
                    integrity::ProgressId::ObjectOffsets.into(),
                );
                offsets_progress.init(
                    Some(pack_ids_and_offsets.len()),
                    gix_features::progress::count("objects"),
                );
                pack_ids_slice = &pack_ids_slice[slice_end..];

                for entry_id in multi_index_entries_to_check.iter().map(|e| e.1) {
                    let oid = self.oid_at_index(entry_id);
                    let (_, expected_pack_offset) = self.pack_id_and_pack_offset_at_index(entry_id);
                    let entry_in_bundle_index = index.lookup(oid).ok_or_else(|| {
                        index::traverse::Error::Processor(integrity::Error::OidNotFound { id: oid.to_owned() })
                    })?;
                    let actual_pack_offset = index.pack_offset_at_index(entry_in_bundle_index);
                    if actual_pack_offset != expected_pack_offset {
                        return Err(index::traverse::Error::Processor(
                            integrity::Error::PackOffsetMismatch {
                                id: oid.to_owned(),
                                expected_pack_offset,
                                actual_pack_offset,
                            },
                        ));
                    }
                    offsets_progress.inc();
                }

                if should_interrupt.load(std::sync::atomic::Ordering::Relaxed) {
                    return Err(index::traverse::Error::Processor(integrity::Error::Interrupted));
                }
                offsets_progress.show_throughput(offset_start);
            }

            total_objects_checked += multi_index_entries_to_check.len();

            if let Some(bundle) = bundle {
                progress.set_name(format!("Validating {}", index_file_name.display()));
                let crate::bundle::verify::integrity::Outcome {
                    actual_index_checksum: _,
                    pack_traverse_outcome,
                } = bundle
                    .verify_integrity(progress, should_interrupt, options.clone())
                    .map_err(|err| {
                        use index::traverse::Error::*;
                        match err {
                            Processor(err) => Processor(integrity::Error::IndexIntegrity(err)),
                            IndexVerify(err) => IndexVerify(err),
                            Tree(err) => Tree(err),
                            TreeTraversal(err) => TreeTraversal(err),
                            PackVerify(err) => PackVerify(err),
                            PackDecode { id, offset, source } => PackDecode { id, offset, source },
                            PackMismatch(err) => PackMismatch(err),
                            EntryType(err) => EntryType(err),
                            PackObjectVerify { offset, source } => PackObjectVerify { offset, source },
                            Crc32Mismatch {
                                expected,
                                actual,
                                offset,
                                kind,
                            } => Crc32Mismatch {
                                expected,
                                actual,
                                offset,
                                kind,
                            },
                            Interrupted => Interrupted,
                        }
                    })?;
                pack_traverse_statistics.push(pack_traverse_outcome);
            }
        }

        assert_eq!(
            self.num_objects as usize, total_objects_checked,
            "BUG: our slicing should allow to visit all objects"
        );

        progress.set_name("Validating multi-pack".into());
        progress.show_throughput(operation_start);

        Ok(integrity::Outcome {
            actual_index_checksum,
            pack_traverse_statistics,
        })
    }
}
