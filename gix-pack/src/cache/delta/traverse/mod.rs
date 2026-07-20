use std::{collections::TryReserveError, sync::atomic::AtomicBool};

use gix_features::{
    progress::{self, DynNestedProgress, Progress},
    threading,
    threading::{Mutable, OwnShared},
};

use crate::{
    cache::delta::{Item, Tree, traverse::util::ItemSliceSync},
    data::EntryRange,
};

mod resolve;
pub(crate) mod util;

/// Shared access to ref-delta child indices awaiting a resolved base, keyed by its object ID.
pub(super) type SharedRefDeltaChildren = OwnShared<Mutable<super::tree::RefDeltaChildren>>;

/// Returned by [`Tree::traverse()`]
#[derive(Debug)]
#[allow(missing_docs)]
pub enum Error {
    ZlibInflate {
        source: gix_zlib::inflate::Error,
        message: &'static str,
    },
    ResolveFailed {
        pack_offset: u64,
    },
    EntryType(crate::data::entry::decode::Error),
    Inspect(Box<dyn std::error::Error + Send + Sync>),
    Interrupted,
    OutOfMemory,
    OutOfPackRefDelta {
        /// The base's offset which was from a resolved ref-delta that didn't actually get added to the tree
        base_pack_offset: crate::data::Offset,
    },
    UnresolvedRefDelta {
        /// The id named by one or more unresolved ref-delta entries.
        base_id: gix_hash::ObjectId,
    },
    ObjectHash(gix_hash::hasher::Error),
    SpawnThread(std::io::Error),
    Delta(crate::data::delta::apply::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ZlibInflate { message, .. } => f.write_str(message),
            Error::ResolveFailed { pack_offset } => write!(
                f,
                "The resolver failed to obtain the pack entry bytes for the entry at {pack_offset}"
            ),
            Error::EntryType(err) => std::fmt::Display::fmt(err, f),
            Error::Inspect(_) => f.write_str("One of the object inspectors failed"),
            Error::Interrupted => f.write_str("Interrupted"),
            Error::OutOfMemory => f.write_str("Entry too large to fit in memory"),
            Error::OutOfPackRefDelta { base_pack_offset } => write!(
                f,
                "The base at {base_pack_offset} was referred to by a ref-delta, but it was never added to the tree as if the pack was still thin."
            ),
            Error::UnresolvedRefDelta { base_id } => {
                write!(f, "The ref-delta base object {base_id} could not be found")
            }
            Error::ObjectHash(_) => f.write_str("Failed to hash an object while resolving in-pack ref-deltas"),
            Error::SpawnThread(_) => f.write_str("Failed to spawn thread when switching to work-stealing mode"),
            Error::Delta(err) => std::fmt::Display::fmt(err, f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::ZlibInflate { source, .. } => Some(source),
            Error::EntryType(err) => err.source(),
            Error::Inspect(err) => Some(&**err),
            Error::ObjectHash(err) => Some(err),
            Error::SpawnThread(err) => Some(err),
            Error::Delta(err) => err.source(),
            Error::ResolveFailed { .. }
            | Error::Interrupted
            | Error::OutOfMemory
            | Error::OutOfPackRefDelta { .. }
            | Error::UnresolvedRefDelta { .. } => None,
        }
    }
}

impl From<crate::data::entry::decode::Error> for Error {
    fn from(err: crate::data::entry::decode::Error) -> Self {
        Error::EntryType(err)
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for Error {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Error::Inspect(err)
    }
}

impl From<gix_hash::hasher::Error> for Error {
    fn from(err: gix_hash::hasher::Error) -> Self {
        Error::ObjectHash(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::SpawnThread(err)
    }
}

impl From<crate::data::delta::apply::Error> for Error {
    fn from(err: crate::data::delta::apply::Error) -> Self {
        Error::Delta(err)
    }
}

impl From<TryReserveError> for Error {
    #[cold]
    fn from(_: TryReserveError) -> Self {
        Self::OutOfMemory
    }
}

/// Additional context passed to the `inspect_object(…)` function of the [`Tree::traverse()`] method.
pub struct Context<'a> {
    /// The pack entry describing the object
    pub entry: &'a crate::data::Entry,
    /// The offset at which `entry` ends in the pack, useful to learn about the exact range of `entry` within the pack.
    pub entry_end: u64,
    /// The decompressed object itself, ready to be decoded.
    pub decompressed: &'a [u8],
    /// The depth at which this object resides in the delta-tree. It represents the number of base objects, with 0 indicating
    /// an 'undeltified' object, and higher values indicating delta objects with the given number of bases.
    pub level: u16,
}

/// Options for [`Tree::traverse()`].
pub struct Options<'a, 's> {
    /// is a progress instance to track progress for each object in the traversal.
    pub object_progress: Box<dyn DynNestedProgress>,
    /// is a progress instance to track the overall progress.
    pub size_progress: &'s mut dyn Progress,
    /// If `Some`, only use the given number of threads. Otherwise, the number of threads to use will be selected based on
    /// the number of available logical cores.
    pub thread_limit: Option<usize>,
    /// Abort the operation if the value is `true`.
    pub should_interrupt: &'a AtomicBool,
    /// specifies what kind of hashes we expect to be stored in oid-delta entries, which is viable to decoding them
    /// with the correct size.
    pub object_hash: gix_hash::Kind,
    /// If `Some`, rejects individual allocations above the given number of bytes while resolving decoded object and
    /// delta result buffers. `Some(0)` rejects all non-empty allocations.
    pub alloc_limit_bytes: Option<usize>,
}

/// The outcome of [`Tree::traverse()`]
pub struct Outcome<T> {
    /// The items that have no children in the pack, i.e. base objects.
    pub roots: Vec<Item<T>>,
    /// The items that children to a root object, i.e. delta objects.
    pub children: Vec<Item<T>>,
}

impl<T> Tree<T>
where
    T: Send,
{
    /// Traverse this tree of delta objects with a function `inspect_object` to process each object at will.
    ///
    /// * `should_run_in_parallel() -> bool` returns true if the underlying pack is big enough to warrant parallel traversal at all.
    /// * `resolve(EntrySlice, &mut Vec<u8>) -> Option<()>` resolves the bytes in the pack for the given `EntrySlice` and stores them in the
    ///   output vector. It returns `Some(())` if the object existed in the pack, or `None` to indicate a resolution error, which would abort the
    ///   operation as well.
    /// * `pack_entries_end` marks one-past-the-last byte of the last entry in the pack, as the last entries size would otherwise
    ///   be unknown as it's not part of the index file.
    /// * `inspect_object(node_data: &mut T, progress: Progress, context: Context<ThreadLocal State>) -> Result<(), CustomError>` is a function
    ///   running for each thread receiving fully decoded objects along with contextual information, which either succeeds with `Ok(())`
    ///   or returns a `CustomError`.
    ///   Note that `node_data` can be modified to allow storing maintaining computation results on a per-object basis. It should contain
    ///   its own mutable per-thread data as required.
    ///
    /// This method returns a vector of all tree items, along with their potentially modified custom node data.
    ///
    /// _Note_ that this method consumed the Tree to assure safe parallel traversal with mutation support.
    pub fn traverse<F, MBFN, E, R>(
        mut self,
        resolve: F,
        resolve_data: &R,
        pack_entries_end: u64,
        inspect_object: MBFN,
        Options {
            thread_limit,
            mut object_progress,
            size_progress,
            should_interrupt,
            object_hash,
            alloc_limit_bytes,
        }: Options<'_, '_>,
    ) -> Result<Outcome<T>, Error>
    where
        F: for<'r> Fn(EntryRange, &'r R) -> Option<&'r [u8]> + Send + Clone,
        R: Send + Sync,
        MBFN: FnMut(&mut T, &dyn Progress, Context<'_>) -> Result<(), E> + Send + Clone,
        E: std::error::Error + Send + Sync + 'static,
    {
        self.set_pack_entries_end_and_resolve_ref_offsets(pack_entries_end)?;

        let num_objects = self.num_items();
        let object_counter = {
            let progress = &mut object_progress;
            progress.init(Some(num_objects), progress::count("objects"));
            progress.counter()
        };
        size_progress.init(None, progress::bytes());
        let size_counter = size_progress.counter();
        let resolver_progress = object_progress.add_child("delta resolver".into());

        let start = std::time::Instant::now();
        let (mut root_items, mut child_items_vec, ref_delta_children) = self.take_root_child_and_refs();
        let ref_delta_children =
            (!ref_delta_children.is_empty()).then(|| OwnShared::new(Mutable::new(ref_delta_children)));
        let child_items = ItemSliceSync::new(&mut child_items_vec);
        // SAFETY: Both item slices come from the same Tree, whose child-index uniqueness invariant still holds.
        #[expect(unsafe_code)]
        unsafe {
            resolve::all(
                &mut root_items,
                &child_items,
                thread_limit,
                num_objects,
                object_counter,
                size_counter,
                &resolver_progress,
                resolve,
                resolve_data,
                inspect_object,
                ref_delta_children.clone(),
                object_hash,
                alloc_limit_bytes,
                should_interrupt,
            )?;
        }

        if let Some(ref_delta_children) = ref_delta_children {
            if let Some((base_id, _children)) = threading::lock(&ref_delta_children).first_key_value() {
                return Err(Error::UnresolvedRefDelta { base_id: *base_id });
            }
        }

        object_progress.show_throughput(start);
        size_progress.show_throughput(start);

        Ok(Outcome {
            roots: root_items,
            children: child_items_vec,
        })
    }
}
