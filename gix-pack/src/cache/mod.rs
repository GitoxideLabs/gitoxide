use std::ops::DerefMut;

use gix_object::Kind;

/// A trait to model putting objects at a given pack `offset` into a cache, and fetching them.
///
/// It is used to speed up [pack traversals][crate::index::File::traverse()].
pub trait DecodeEntry {
    /// Store a fully decoded object at `offset` of `kind` with `compressed_size` and `data` in the cache.
    ///
    /// It is up to the cache implementation whether that actually happens or not.
    fn put(&mut self, pack_id: u32, offset: u64, data: &[u8], kind: gix_object::Kind, compressed_size: usize);
    /// Attempt to fetch the object at `offset` and store its decoded bytes in `out`, as previously stored with [`DecodeEntry::put()`], and return
    /// its (object `kind`, `decompressed_size`)
    fn get(&mut self, pack_id: u32, offset: u64, out: &mut Vec<u8>) -> Option<(gix_object::Kind, usize)>;
}

/// A cache that stores nothing and retrieves nothing, thus it _never_ caches.
#[derive(Default)]
pub struct Never;

impl DecodeEntry for Never {
    fn put(&mut self, _pack_id: u32, _offset: u64, _data: &[u8], _kind: gix_object::Kind, _compressed_size: usize) {}
    fn get(&mut self, _pack_id: u32, _offset: u64, _out: &mut Vec<u8>) -> Option<(gix_object::Kind, usize)> {
        None
    }
}

impl<T: DecodeEntry + ?Sized> DecodeEntry for Box<T> {
    fn put(&mut self, pack_id: u32, offset: u64, data: &[u8], kind: Kind, compressed_size: usize) {
        self.deref_mut().put(pack_id, offset, data, kind, compressed_size);
    }

    fn get(&mut self, pack_id: u32, offset: u64, out: &mut Vec<u8>) -> Option<(Kind, usize)> {
        self.deref_mut().get(pack_id, offset, out)
    }
}

/// A way of storing and retrieving entire objects to and from a cache.
pub trait Object {
    /// Put the object going by `id` of `kind` with `data` into the cache.
    fn put(&mut self, id: gix_hash::ObjectId, kind: gix_object::Kind, data: &[u8]);

    /// Try to retrieve the object named `id` and place its data into `out` if available and return `Some(kind)` if found.
    fn get(&mut self, id: &gix_hash::ObjectId, out: &mut Vec<u8>) -> Option<gix_object::Kind>;
}

/// Various implementations of [`DecodeEntry`] using least-recently-used algorithms.
#[cfg(any(feature = "pack-cache-lru-dynamic", feature = "pack-cache-lru-static"))]
pub mod lru;

pub mod object;

/// Index-less parallel pack traversal.
///
/// Build a delta [`delta::Tree`] directly from a streaming pack scan with
/// [`delta::Tree::from_offsets_in_pack()`] — no pre-built `.idx` — then resolve every
/// object in parallel with [`delta::Tree::traverse()`]. This is the index-less companion
/// to the idx-verified [`crate::index::File::traverse_with_index()`]; [`delta::traverse::Context`] and
/// [`delta::traverse::Options`] configure the traversal.
///
/// ```no_run
/// use gix_pack::cache::delta::Tree;
/// use std::sync::atomic::AtomicBool;
///
/// # fn build_index_less(
/// #     pack_path: &std::path::Path,
/// #     offsets: Vec<gix_pack::data::Offset>,
/// # ) -> Result<(), gix_pack::cache::delta::from_offsets::Error> {
/// // Build the delta tree straight from the pack — no `.idx` required:
/// let tree = Tree::from_offsets_in_pack(
///     pack_path,
///     offsets.into_iter(),
///     &|offset| *offset,
///     &|_id| None,                     // self-contained pack: no ref-delta lookups
///     &mut gix_features::progress::Discard,
///     &AtomicBool::default(),
///     gix_hash::Kind::default(),
/// )?;
/// // `tree.traverse(resolve, &pack, pack_end, inspect, Options { .. })` then
/// // resolves every object in parallel across threads.
/// let _ = tree;
/// # Ok(()) }
/// ```
pub mod delta;

/// Replaces content of the given `Vec` with the slice. The vec will have the same length
/// as the slice. The vec can be either `&mut Vec` or `Vec`.
/// Returns `None` if no memory could be allocated.
#[cfg(any(
    feature = "pack-cache-lru-static",
    feature = "pack-cache-lru-dynamic",
    feature = "object-cache-dynamic"
))]
fn set_vec_to_slice<V: std::borrow::BorrowMut<Vec<u8>>>(mut vec: V, source: &[u8]) -> Option<V> {
    let out = vec.borrow_mut();
    out.clear();
    out.try_reserve(source.len()).ok()?;
    out.extend_from_slice(source);
    Some(vec)
}
