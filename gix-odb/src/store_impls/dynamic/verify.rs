use std::{
    ops::Deref,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

use gix_features::progress::{DynNestedProgress, MessageLevel, Progress};

use crate::{
    pack,
    store::verify::integrity::{IndexStatistics, SingleOrMultiStatistics},
    types::IndexAndPacks,
};

///
pub mod integrity {
    use std::{marker::PhantomData, path::PathBuf};

    use crate::pack;

    /// Options for use in [`Store::verify_integrity()`][crate::Store::verify_integrity()].
    pub type Options<F> = pack::index::verify::integrity::Options<F>;

    /// Returned by [`Store::verify_integrity()`][crate::Store::verify_integrity()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        MultiIndexIntegrity(#[from] pack::index::traverse::Error<pack::multi_index::verify::integrity::Error>),
        #[error(transparent)]
        IndexIntegrity(#[from] pack::index::traverse::Error<pack::index::verify::integrity::Error>),
        #[error(transparent)]
        IndexOpen(#[from] pack::index::init::Error),
        #[error(transparent)]
        LooseObjectStoreIntegrity(#[from] crate::loose::verify::integrity::Error),
        #[error(transparent)]
        MultiIndexOpen(#[from] pack::multi_index::init::Error),
        #[error(transparent)]
        PackOpen(#[from] pack::data::init::Error),
        #[error(transparent)]
        InitializeODB(#[from] crate::store::load_index::Error),
        #[error("The disk on state changed while performing the operation, and we observed the change.")]
        NeedsRetryDueToChangeOnDisk,
    }

    #[derive(Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Clone)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    /// Integrity information about loose object databases
    pub struct LooseObjectStatistics {
        /// The path to the root directory of the loose objects database
        pub path: PathBuf,
        /// The statistics created after verifying the loose object database.
        pub statistics: crate::loose::verify::integrity::Statistics,
    }

    #[derive(Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Clone)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    /// Traversal statistics of packs governed by single indices or multi-pack indices.
    #[allow(missing_docs)]
    pub enum SingleOrMultiStatistics {
        Single(pack::index::traverse::Statistics),
        Multi(Vec<(PathBuf, pack::index::traverse::Statistics)>),
    }

    /// Statistics gathered when traversing packs of various kinds of indices.
    #[derive(Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Clone)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct IndexStatistics {
        /// The path to the index or multi-pack index for which statistics were gathered.
        pub path: PathBuf,
        /// The actual statistics for the index at `path`.
        pub statistics: SingleOrMultiStatistics,
    }

    /// Returned by [`Store::verify_integrity()`][crate::Store::verify_integrity()].
    pub struct Outcome {
        /// Statistics for validated loose object stores.
        pub loose_object_stores: Vec<LooseObjectStatistics>,
        /// Pack traversal statistics for each index and their pack(s)
        pub index_statistics: Vec<IndexStatistics>,
    }

    /// The progress ids used in [`Store::verify_integrity()`][crate::Store::verify_integrity()].
    ///
    /// Use this information to selectively extract the progress of interest in case the parent application has custom visualization.
    #[derive(Debug, Copy, Clone)]
    pub enum ProgressId {
        /// Contains the path of the currently validated loose object database.
        VerifyLooseObjectDbPath,
        /// The root progress for all verification of an index. It doesn't contain any useful information itself.
        VerifyIndex(PhantomData<gix_pack::index::verify::integrity::ProgressId>),
        /// The root progress for all verification of a multi-index. It doesn't contain any useful information itself.
        VerifyMultiIndex(PhantomData<gix_pack::multi_index::verify::integrity::ProgressId>),
    }

    impl From<ProgressId> for gix_features::progress::Id {
        fn from(v: ProgressId) -> Self {
            match v {
                ProgressId::VerifyLooseObjectDbPath => *b"VISP",
                ProgressId::VerifyMultiIndex(_) => *b"VIMI",
                ProgressId::VerifyIndex(_) => *b"VISI",
            }
        }
    }
}

impl super::Store {
    /// Check the integrity of all objects as per the given `options`.
    ///
    /// Note that this will not force loading all indices or packs permanently, as we will only use the momentarily loaded disk state.
    /// This does, however, include all alternates.
    pub fn verify_integrity<C, F>(
        &self,
        progress: &mut dyn DynNestedProgress,
        should_interrupt: &AtomicBool,
        options: integrity::Options<F>,
    ) -> Result<integrity::Outcome, integrity::Error>
    where
        C: pack::cache::DecodeEntry,
        F: Fn() -> C + Send + Clone,
    {
        let _span = gix_features::trace::coarse!("gix_odb:Store::verify_integrity()");
        let mut index = self.index.load();
        if !index.is_initialized() {
            self.consolidate_with_disk_state(true, false)?;
            index = self.index.load();
            assert!(
                index.is_initialized(),
                "BUG: after consolidating successfully, we have an initialized index"
            );
        }

        progress.init(
            Some(index.slot_indices.len()),
            gix_features::progress::count("pack indices"),
        );
        let mut statistics = Vec::new();
        let index_check_message = |path: &std::path::Path| {
            format!(
                "Checking integrity: {}",
                path.file_name()
                    .map_or_else(Default::default, std::ffi::OsStr::to_string_lossy)
            )
        };
        gix_features::trace::detail!("verify indices").into_scope(|| {
            for slot_index in &index.slot_indices {
                let slot = &self.files[*slot_index];
                if slot.generation.load(Ordering::SeqCst) != index.generation {
                    return Err(integrity::Error::NeedsRetryDueToChangeOnDisk);
                }
                let files = slot.files.load();
                let files = Option::as_ref(&files).ok_or(integrity::Error::NeedsRetryDueToChangeOnDisk)?;

                let start = Instant::now();
                let (mut child_progress, num_objects, index_path) = match files {
                    IndexAndPacks::Index(bundle) => {
                        let index;
                        let index = match bundle.index.loaded() {
                            Some(index) => index.deref(),
                            None => {
                                index = pack::index::File::at(bundle.index.path(), self.object_hash)?;
                                &index
                            }
                        };
                        let pack;
                        let data = match bundle.data.loaded() {
                            Some(pack) => pack.deref(),
                            None => {
                                pack = pack::data::File::at(bundle.data.path(), self.object_hash)?
                                    .with_alloc_limit_bytes(self.alloc_limit_bytes);
                                &pack
                            }
                        };
                        let mut child_progress = progress.add_child_with_id(
                            "verify index".into(),
                            integrity::ProgressId::VerifyIndex(Default::default()).into(),
                        );
                        let outcome = index.verify_integrity(
                            Some(pack::index::verify::PackContext {
                                data,
                                options: options.clone(),
                            }),
                            &mut child_progress,
                            should_interrupt,
                        )?;
                        statistics.push(IndexStatistics {
                            path: bundle.index.path().to_owned(),
                            statistics: SingleOrMultiStatistics::Single(
                                outcome
                                    .pack_traverse_statistics
                                    .expect("pack provided so there are stats"),
                            ),
                        });
                        (child_progress, index.num_objects(), index.path().to_owned())
                    }
                    IndexAndPacks::MultiIndex(bundle) => {
                        let index;
                        let index = match bundle.multi_index.loaded() {
                            Some(index) => index.deref(),
                            None => {
                                index = pack::multi_index::File::at(bundle.multi_index.path(), self.alloc_limit_bytes)?;
                                &index
                            }
                        };
                        let mut child_progress = progress.add_child_with_id(
                            "verify multi-index".into(),
                            integrity::ProgressId::VerifyMultiIndex(Default::default()).into(),
                        );
                        let outcome = index.verify_integrity(&mut child_progress, should_interrupt, options.clone())?;

                        let index_dir = bundle.multi_index.path().parent().expect("file in a directory");
                        statistics.push(IndexStatistics {
                            path: Default::default(),
                            statistics: SingleOrMultiStatistics::Multi(
                                outcome
                                    .pack_traverse_statistics
                                    .into_iter()
                                    .zip(index.index_names())
                                    .map(|(statistics, index_name)| (index_dir.join(index_name), statistics))
                                    .collect(),
                            ),
                        });
                        (child_progress, index.num_objects(), index.path().to_owned())
                    }
                };

                child_progress.set_name(index_check_message(&index_path));
                child_progress.show_throughput_with(
                    start,
                    num_objects as usize,
                    gix_features::progress::count("objects").expect("set"),
                    MessageLevel::Success,
                );
                progress.inc();
            }
            Ok(())
        })?;

        progress.init(
            Some(index.loose_dbs.len()),
            gix_features::progress::count("loose object stores"),
        );
        let mut loose_object_stores = Vec::new();
        gix_features::trace::detail!("verify loose ODBs").into_scope(
            || -> Result<_, crate::loose::verify::integrity::Error> {
                for loose_db in &*index.loose_dbs {
                    let out = loose_db
                        .verify_integrity(
                            &mut progress.add_child_with_id(
                                loose_db.path().display().to_string(),
                                integrity::ProgressId::VerifyLooseObjectDbPath.into(),
                            ),
                            should_interrupt,
                        )
                        .map(|statistics| integrity::LooseObjectStatistics {
                            path: loose_db.path().to_owned(),
                            statistics,
                        })?;
                    loose_object_stores.push(out);
                }
                Ok(())
            },
        )?;

        Ok(integrity::Outcome {
            loose_object_stores,
            index_statistics: statistics,
        })
    }
}
