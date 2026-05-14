use std::sync::atomic::{AtomicBool, Ordering};

use gix_features::{
    parallel::in_parallel_with_slice,
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

/// Returned by [`Tree::traverse()`]
#[derive(thiserror::Error, Debug)]
#[allow(missing_docs)]
pub enum Error {
    #[error("{message}")]
    ZlibInflate {
        source: gix_features::zlib::inflate::Error,
        message: &'static str,
    },
    #[error("The resolver failed to obtain the pack entry bytes for the entry at {pack_offset}")]
    ResolveFailed { pack_offset: u64 },
    #[error(transparent)]
    EntryType(#[from] crate::data::entry::decode::Error),
    #[error("One of the object inspectors failed")]
    Inspect(#[from] Box<dyn std::error::Error + Send + Sync>),
    #[error("Interrupted")]
    Interrupted,
    #[error(
        "The base at {base_pack_offset} was referred to by a ref-delta, but it was never added to the tree as if the pack was still thin."
    )]
    OutOfPackRefDelta {
        /// The base's offset which was from a resolved ref-delta that didn't actually get added to the tree
        base_pack_offset: crate::data::Offset,
    },
    #[error("Failed to spawn thread when switching to work-stealing mode")]
    SpawnThread(#[from] std::io::Error),
    #[error(transparent)]
    Delta(#[from] crate::data::delta::apply::Error),
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
        let object_progress = OwnShared::new(Mutable::new(object_progress));

        let start = std::time::Instant::now();
        let (mut root_items, mut child_items_vec) = self.take_root_and_child();
        let child_items = ItemSliceSync::new(&mut child_items_vec);
        let child_items = &child_items;
        in_parallel_with_slice(
            &mut root_items,
            thread_limit,
            {
                {
                    let object_progress = object_progress.clone();
                    move |thread_index| resolve::State {
                        delta_bytes: Vec::<u8>::with_capacity(4096),
                        fully_resolved_delta_bytes: Vec::<u8>::with_capacity(4096),
                        progress: Box::new(
                            threading::lock(&object_progress).add_child(format!("thread {thread_index}")),
                        ),
                        resolve: resolve.clone(),
                        modify_base: inspect_object.clone(),
                        child_items,
                    }
                }
            },
            {
                move |node, state, threads_left, should_interrupt| {
                    // SAFETY: This invariant is upheld since `child_items` and `node` come from the same Tree.
                    // This means we can rely on Tree's invariant that node.children will be the only `children` array in
                    // for nodes in this tree that will contain any of those children.
                    #[allow(unsafe_code)]
                    unsafe {
                        resolve::deltas(
                            object_counter.clone(),
                            size_counter.clone(),
                            node,
                            state,
                            resolve_data,
                            object_hash.len_in_bytes(),
                            threads_left,
                            should_interrupt,
                        )
                    }
                }
            },
            || (!should_interrupt.load(Ordering::Relaxed)).then(|| std::time::Duration::from_millis(50)),
            |_| (),
        )?;

        threading::lock(&object_progress).show_throughput(start);
        size_progress.show_throughput(start);

        Ok(Outcome {
            roots: root_items,
            children: child_items_vec,
        })
    }
}
