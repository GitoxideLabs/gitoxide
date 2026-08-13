use std::{path::PathBuf, sync::Arc};

use arc_swap::ArcSwap;

use crate::{
    Store,
    store::types::{Catalog, MutableIndexAndPack, SlotMapIndex},
};

/// Options for use in [`Store::at_opts()`].
#[derive(Clone, Debug)]
pub struct Options {
    /// How to obtain a size for the slot map.
    pub slots: Slots,
    /// If false, no multi-pack indices will be used. If true, they will be used if their hash matches `object_hash`.
    pub use_multi_pack_index: bool,
    /// The maximum size of a single allocation caused by user-controlled on-disk pack data.
    ///
    /// If `None`, no additional limit is enforced.
    pub alloc_limit_bytes: Option<usize>,
    /// The current directory of the process at the time of instantiation.
    /// If unset, it will be retrieved using `gix_fs::current_dir(false)`.
    pub current_dir: Option<std::path::PathBuf>,
    /// The compression level to use when writing loose objects.
    ///
    /// Defaults to [`Compression::BEST_SPEED`](gix_zlib::Compression::BEST_SPEED), which is
    /// also what `git` uses unless configured otherwise with `core.looseCompression` or `core.compression`.
    pub loose_compression: gix_zlib::Compression,
    /// Deterministic synchronization hooks for tests of concurrent behavior.
    #[cfg(feature = "test-support")]
    pub debug: Option<debug::Options>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            slots: Default::default(),
            use_multi_pack_index: true,
            alloc_limit_bytes: None,
            current_dir: None,
            loose_compression: gix_zlib::Compression::BEST_SPEED,
            #[cfg(feature = "test-support")]
            debug: None,
        }
    }
}

/// Deterministic synchronization support for tests of concurrent object-database behavior.
#[cfg(feature = "test-support")]
pub mod debug {
    use std::{fmt, sync::Arc, time::Instant};

    /// The result of a filesystem-backed load attempt.
    #[non_exhaustive]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum LoadOutcome {
        /// The file was loaded successfully.
        Success,
        /// Loading the file failed.
        Failure,
    }

    /// A synchronization point in concurrent object-database processing.
    #[non_exhaustive]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum Point {
        /// An index slot was claimed for loading. No Store or slot lock is held.
        IndexLoadClaimed {
            /// The claimed slot-map index.
            slot: usize,
            /// The number of registered index loads, including this one.
            active_loads: u16,
        },
        /// A loader found all index slots claimed by other loaders. No Store or slot lock is held.
        IndexLoadWaiting,
        /// A failed-load check is about to acquire an index slot lock.
        IndexRetrySlotLocking {
            /// The index slot about to be locked.
            slot: usize,
        },
        /// Snapshot collection is waiting for an in-progress index load. No Store or slot lock is held.
        SnapshotWaitingForIndexLoad,
        /// The claimed index slot was locked, immediately before its file is loaded.
        ///
        /// The slot lock is held while the hook runs.
        IndexSlotLocked {
            /// The locked slot-map index.
            slot: usize,
        },
        /// An index load attempt finished and its result was stored.
        ///
        /// The slot lock is held while the hook runs.
        IndexLoadCompleted {
            /// The loaded slot-map index.
            slot: usize,
            /// Whether the load succeeded.
            outcome: LoadOutcome,
        },
        /// A pack loader is about to acquire its slot lock.
        PackSlotLocking {
            /// The pack's slot-map index.
            slot: usize,
            /// The pack index within a multi-pack index, or `None` for a standalone pack.
            pack_index: Option<gix_pack::multi_index::PackIndex>,
        },
        /// A pack loader acquired its slot lock, immediately before checking or loading the file.
        ///
        /// The slot lock is held while the hook runs.
        PackSlotLocked {
            /// The pack's slot-map index.
            slot: usize,
            /// The pack index within a multi-pack index, or `None` for a standalone pack.
            pack_index: Option<gix_pack::multi_index::PackIndex>,
        },
        /// A filesystem-backed pack load attempt finished.
        ///
        /// The slot lock is held while the hook runs. Cached outcomes do not emit this point.
        PackLoadCompleted {
            /// The pack's slot-map index.
            slot: usize,
            /// The pack index within a multi-pack index, or `None` for a standalone pack.
            pack_index: Option<gix_pack::multi_index::PackIndex>,
            /// Whether the load succeeded.
            outcome: LoadOutcome,
        },
        /// A refresh is about to acquire the Store write lock.
        RefreshLocking,
        /// A refresh read the shared completion counter and is about to snapshot the catalog.
        RefreshCompletionObserved,
        /// A refresh acquired the Store write lock.
        ///
        /// The Store write lock is held while the hook runs.
        RefreshLockAcquired,
        /// A refresh is about to inspect the object database on disk.
        ///
        /// The Store write lock is held while the hook runs.
        RefreshScanStarted,
        /// A refresh finished inspecting the object database on disk.
        ///
        /// The Store write lock is held while the hook runs.
        RefreshScanCompleted {
            /// Whether the refresh succeeded.
            outcome: LoadOutcome,
        },
        /// A new index state was published.
        ///
        /// The Store write lock is held while the hook runs.
        IndexStatePublished,
    }

    /// Configuration for deterministic synchronization hooks.
    #[derive(Clone)]
    pub struct Options {
        hook: Arc<dyn Fn(Point) + Send + Sync>,
        clock: Option<Arc<dyn Fn() -> Instant + Send + Sync>>,
    }

    impl Options {
        /// Create options that invoke `hook` at every configured synchronization point.
        pub fn new(hook: impl Fn(Point) + Send + Sync + 'static) -> Self {
            Options {
                hook: Arc::new(hook),
                clock: None,
            }
        }

        /// Use `clock` instead of the system monotonic clock for refresh throttling.
        pub fn with_clock(mut self, clock: impl Fn() -> Instant + Send + Sync + 'static) -> Self {
            self.clock = Some(Arc::new(clock));
            self
        }

        pub(crate) fn at(&self, point: Point) {
            (self.hook)(point);
        }

        pub(crate) fn now(&self) -> Instant {
            self.clock.as_ref().map_or_else(Instant::now, |clock| clock())
        }
    }

    impl fmt::Debug for Options {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Options").finish_non_exhaustive()
        }
    }
}

/// Configures the initial size and possible growth of the index slot map.
#[derive(Copy, Clone, Debug)]
pub enum Slots {
    /// The maximum number of indices the store can hold.
    /// This avoids an initial directory listing and provides an explicit resource bound.
    ///
    /// Note that this won't affect their packs, as each index can have one or more packs associated with it.
    Limit(u16),
    /// Start with `initial` slots without reading the object database and grow as needed.
    Growable {
        /// The number of slots allocated when opening the store.
        initial: u16,
    },
    /// Compute the initial number of slots from the disk state and grow as needed.
    AsNeededByDiskState {
        /// 1.0 means no safety, 1.1 means 10% more slots than needed
        multiplier: f32,
        /// The minimum number of slots to assume
        minimum: usize,
    },
}

impl Default for Slots {
    fn default() -> Self {
        Slots::Growable { initial: 32 }
    }
}

impl Store {
    /// Open the store at `objects_dir` (containing loose objects and `packs/`), which must only be a directory for
    /// the store to be created without any additional work being done.
    /// `slots` defines the initial capacity and whether it may grow, including indices from additional object
    /// databases reached through `alternates`.
    /// Note that the `slots` isn't used for packs, these are included with their multi-index or index respectively.
    /// For example, In a repository with 250m objects and geometric packing one would expect 27 index/pack pairs,
    /// or a single multi-pack index.
    /// `object_hash` is the hash expected in packs and used for loose object iteration and object writing.
    /// `replacements` is an iterator over pairs of old and new object ids for replacement support.
    /// This means that when asking for object `X`, one will receive object `X-replaced` given an iterator like `Some((X, X-replaced))`.
    pub fn at_opts(
        objects_dir: PathBuf,
        object_hash: gix_hash::Kind,
        replacements: &mut dyn Iterator<Item = (gix_hash::ObjectId, gix_hash::ObjectId)>,
        Options {
            slots,
            use_multi_pack_index,
            alloc_limit_bytes,
            current_dir,
            loose_compression,
            #[cfg(feature = "test-support")]
            debug,
        }: Options,
    ) -> std::io::Result<Self> {
        let _span = gix_features::trace::detail!("gix_odb::Store::at()");
        let current_dir = current_dir.map_or_else(
            || {
                // It's only used for real-pathing alternate paths and there it just needs to be consistent (enough).
                gix_fs::current_dir(false)
            },
            Ok,
        )?;
        if !objects_dir.is_dir() {
            return Err(std::io::Error::other(format!(
                "'{}' wasn't a directory",
                objects_dir.display()
            )));
        }
        let (slot_count, slot_limit) = match slots {
            Slots::Limit(n) => (n as usize, Some(n as usize)),
            Slots::Growable { initial } => (initial as usize, None),
            Slots::AsNeededByDiskState { multiplier, minimum } => {
                let mut db_paths =
                    crate::alternate::resolve(objects_dir.clone(), &current_dir).map_err(std::io::Error::other)?;
                db_paths.insert(0, objects_dir.clone());
                let num_slots =
                    Store::collect_indices_and_mtime_sorted_by_size(db_paths, None, None, alloc_limit_bytes)
                        .map_err(std::io::Error::other)?
                        .len();

                let candidate = ((num_slots as f32 * multiplier) as usize).max(minimum);
                (candidate, None)
            }
        };
        let mut replacements: Vec<_> = replacements.collect();
        replacements.sort_by_key(|a| a.0);

        Ok(Store {
            current_dir,
            write: Default::default(),
            replacements,
            path: objects_dir,
            catalog: ArcSwap::from_pointee(Catalog {
                index: Arc::new(SlotMapIndex::default()),
                slots: Arc::new(
                    std::iter::repeat_with(|| Arc::new(MutableIndexAndPack::default()))
                        .take(slot_count)
                        .collect(),
                ),
            }),
            slot_limit,
            use_multi_pack_index,
            object_hash,
            alloc_limit_bytes,
            loose_compression,
            #[cfg(feature = "test-support")]
            debug,
            num_handles: Default::default(),
            num_disk_state_consolidation: Default::default(),
            num_disk_state_consolidations_completed: Default::default(),
            last_disk_state_consolidation_error: Default::default(),
            last_successful_disk_state_consolidation: Default::default(),
        })
    }
}
