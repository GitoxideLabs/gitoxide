//! The standard object store which should fit all needs.
use std::{cell::RefCell, ops::Deref};

use gix_features::zlib;

use crate::Store;

/// This effectively acts like a handle but exists to be usable from the actual `crate::Handle` implementation which adds caches on top.
/// Each store is quickly cloned and contains thread-local state for shared packs.
pub struct Handle<S>
where
    S: Deref<Target = Store> + Clone,
{
    pub(crate) store: S,
    /// Defines what happens when there is no more indices to load.
    pub refresh: RefreshMode,
    /// The maximum recursion depth for resolving ref-delta base objects, that is objects referring to other objects within
    /// a pack.
    /// Recursive loops are possible only in purposefully crafted packs.
    /// This value doesn't have to be huge as in typical scenarios, these kind of objects are rare and chains supposedly are
    /// even more rare.
    pub max_recursion_depth: usize,

    /// If true, replacements will not be performed even if these are available.
    pub ignore_replacements: bool,

    pub(crate) token: Option<handle::Mode>,
    snapshot: RefCell<load_index::Snapshot>,
    inflate: RefCell<zlib::Inflate>,
    packed_object_count: RefCell<Option<u64>>,
}

/// Decide what happens when all indices are loaded.
#[derive(Default, Clone, Copy)]
pub enum RefreshMode {
    /// Check for new or changed pack indices (and pack data files) when the last known index is loaded.
    /// During runtime we will keep pack indices stable by never reusing them, however, there is the option for
    /// clearing internal caches which is likely to change pack ids and it will trigger unloading of packs as they are missing on disk.
    #[default]
    AfterAllIndicesLoaded,
    /// Use this if you expect a lot of missing objects that shouldn't trigger refreshes even after all packs are loaded.
    /// This comes at the risk of not learning that the packs have changed in the mean time.
    Never,
}

impl RefreshMode {
    /// Set this refresh mode to never refresh.
    pub fn never(&mut self) {
        *self = RefreshMode::Never;
    }
}

///
pub mod find;

///
pub mod prefix;

mod header;
mod stream;

///
pub mod iter;

///
pub mod write;

///
pub mod init;

pub(crate) mod types;
pub use types::Metrics;

pub(crate) mod handle;

///
pub mod load_index;

///
pub mod verify;

mod load_one;

mod metrics;

mod access;

///
pub mod structure;
