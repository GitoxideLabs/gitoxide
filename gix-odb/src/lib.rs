//! Git stores all of its data as _Objects_, which are data along with a hash over all data. Thus it's an
//! object store indexed by the signature of data itself with inherent deduplication: the same data will have the same hash,
//! and thus occupy the same space within the store.
//!
//! There is only one all-round object store, also known as the [`Store`], as it supports ~~everything~~ most of what git has to offer.
//!
//! * loose object reading and writing
//! * access to packed objects
//! * multiple loose objects and pack locations as gathered from `alternates` files.
//!
//! ## Write And Read Loose Objects
//!
//! ```
//! # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
//! use gix_object::{FindExt, Write};
//!
//! let (_dir, odb) = doctest::empty_store()?;
//! let id = odb.write_buf(gix_object::Kind::Blob, b"hello")?;
//!
//! let mut buf = Vec::new();
//! let object = odb.find(&id, &mut buf)?;
//! assert_eq!(object.kind, gix_object::Kind::Blob);
//! assert_eq!(object.data, b"hello");
//! # Ok(()) }
//! ```
//!
//! ## Inspect Headers Without Decoding The Object
//!
//! ```
//! # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
//! use gix_object::Write;
//! use gix_odb::HeaderExt;
//!
//! let (_dir, odb) = doctest::empty_store()?;
//! let id = odb.write_buf(gix_object::Kind::Blob, b"hello")?;
//!
//! let header = odb.header(&id)?;
//! assert_eq!(header.kind(), gix_object::Kind::Blob);
//! assert_eq!(header.size(), 5);
//! # Ok(()) }
//! ```
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs, unsafe_code)]

use std::{
    cell::RefCell,
    path::PathBuf,
    sync::{Arc, atomic::AtomicUsize},
};

use arc_swap::ArcSwap;
use gix_features::{threading::OwnShared, zlib::stream::deflate};
pub use gix_pack as pack;

mod store_impls;
pub use store_impls::{dynamic as store, loose};

pub mod alternate;

/// A way to access objects along with pre-configured thread-local caches for packed base objects as well as objects themselves.
///
/// By default, no cache will be used.
pub struct Cache<S> {
    /// The inner provider of trait implementations we use in conjunction with our caches.
    ///
    /// For calling methods on `inner`, prefer to make use of auto-dereferencing, i.e. `cache.inner_method()` instead of `cache.inner.inner_method()`.
    inner: S,
    // TODO: have single-threaded code-paths also for pack-creation (entries from counts) so that we can use OwnShared here
    //       instead of Arc. However, it's probably not that important as these aren't called often.
    new_pack_cache: Option<Arc<cache::NewPackCacheFn>>,
    new_object_cache: Option<Arc<cache::NewObjectCacheFn>>,
    pack_cache: Option<RefCell<Box<cache::PackCache>>>,
    object_cache: Option<RefCell<Box<cache::ObjectCache>>>,
}

///
pub mod cache;

///
/// It can optionally compress the content, similarly to what would happen when using a [`loose::Store`].
///
#[derive(Clone)]
pub struct Sink {
    compressor: Option<RefCell<deflate::Write<std::io::Sink>>>,
    object_hash: gix_hash::Kind,
}

/// Create a new [`Sink`] with compression disabled.
pub fn sink(object_hash: gix_hash::Kind) -> Sink {
    Sink {
        compressor: None,
        object_hash,
    }
}

///
pub mod memory;

mod sink;

///
pub mod find;

/// An object database equivalent to `/dev/null`, dropping all objects stored into it.
mod traits;

pub use traits::{Header, HeaderExt};

/// A thread-local handle to access any object.
pub type Handle = Cache<store::Handle<OwnShared<Store>>>;
/// A thread-local handle to access any object, but thread-safe and independent of the actual type of `OwnShared` or feature toggles in `gix-features`.
pub type HandleArc = Cache<store::Handle<Arc<Store>>>;

use store::types;

/// The object store for use in any applications with support for auto-updates in the light of changes to the object database.
///
/// ### Features
///
/// - entirely lazy, creating an instance does no disk IO at all if [`Slots::Given`][store::init::Slots::Given] is used.
/// - multi-threaded lazy-loading of indices and packs
/// - per-thread pack and object caching avoiding cache trashing.
/// - most-recently-used packs are always first for speedups if objects are stored in the same pack, typical for packs organized by
///   commit graph and object age.
/// - lock-free reading for perfect scaling across all cores, and changes to it don't affect readers as long as these don't want to
///   enter the same branch.
/// - sync with the state on disk if objects aren't found to catch up with changes if an object seems to be missing.
///    - turn off the behaviour above for all handles if objects are expected to be missing due to spare checkouts.
pub struct Store {
    /// The central write lock without which the slotmap index can't be changed.
    write: parking_lot::Mutex<()>,

    /// The source directory from which all content is loaded, and the central write lock for use when a directory refresh is needed.
    pub(crate) path: PathBuf,

    /// The current working directory at the time this store was instantiated. It becomes relevant when resolving alternate paths
    /// when re-reading the store configuration on updates when an object was missed.
    /// Keeping it here helps to assure consistency even while a process changes its CWD.
    pub(crate) current_dir: PathBuf,

    /// A set of replacements that given a source OID return a destination OID. The vector is sorted.
    pub(crate) replacements: Vec<(gix_hash::ObjectId, gix_hash::ObjectId)>,

    /// A list of indices keeping track of which slots are filled with data. These are usually, but not always, consecutive.
    pub(crate) index: ArcSwap<types::SlotMapIndex>,

    /// The below state acts like a slot-map with each slot is mutable when the write lock is held, but readable independently of it.
    /// This allows multiple file to be loaded concurrently if there is multiple handles requesting to load packs or additional indices.
    /// The map is static and cannot change.
    /// It's read often and changed rarely.
    pub(crate) files: Vec<types::MutableIndexAndPack>,

    /// The amount of handles that would prevent us from unloading packs or indices
    pub(crate) num_handles_stable: AtomicUsize,
    /// The amount of handles that don't affect our ability to compact our internal data structures or unload packs or indices.
    pub(crate) num_handles_unstable: AtomicUsize,

    /// The amount of times we re-read the disk state to consolidate our in-memory representation.
    pub(crate) num_disk_state_consolidation: AtomicUsize,
    /// If true, we are allowed to use multi-pack indices and they must have the `object_hash` or be ignored.
    use_multi_pack_index: bool,
    /// The hash kind to use for some operations
    object_hash: gix_hash::Kind,
    /// The maximum size of a single allocation caused by user-controlled on-disk pack data.
    alloc_limit_bytes: Option<usize>,
}

/// Create a new cached handle to the object store with support for additional options.
///
/// `replacements` is an iterator over pairs of old and new object ids for replacement support.
/// This means that when asking for object `X`, one will receive object `X-replaced` given an iterator like `Some((X, X-replaced))`.
pub fn at_opts(
    objects_dir: impl Into<PathBuf>,
    replacements: impl IntoIterator<Item = (gix_hash::ObjectId, gix_hash::ObjectId)>,
    options: store::init::Options,
) -> std::io::Result<Handle> {
    let handle = OwnShared::new(Store::at_opts(
        objects_dir.into(),
        &mut replacements.into_iter(),
        options,
    )?)
    .to_handle();
    Ok(Cache::from(handle))
}

/// Create a new cached handle to the object store.
pub fn at(objects_dir: impl Into<PathBuf>) -> std::io::Result<Handle> {
    at_opts(objects_dir, Vec::new(), Default::default())
}
