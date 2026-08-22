use std::sync::atomic::{AtomicBool, Ordering};

use gix_error::{ErrorExt, ResultExt, message};
use gix_features::{
    progress::Progress,
    threading::{self, OwnShared},
};

use crate::{
    cache::delta::{
        traverse::{Context, Error, interrupted, out_of_memory, util::ItemSliceSync},
        tree::Item,
    },
    data,
    data::EntryRange,
};

mod node {
    use crate::cache::delta::{traverse::util::ItemSliceSync, tree::Item};

    /// A node in a delta tree, with exclusive access to its item data.
    pub(crate) struct Node<'a, T: Send> {
        // SAFETY INVARIANT: see Node::new(). That function is the only one used
        // to create or modify these fields.
        item: &'a mut Item<T>,
        child_items: &'a ItemSliceSync<'a, Item<T>>,
    }

    impl<'a, T: Send> Node<'a, T> {
        /// SAFETY: `item.children` must uniquely reference elements in `child_items` that no other live item does.
        /// All child items must uphold the same invariant.
        #[expect(unsafe_code)]
        pub(super) unsafe fn new(item: &'a mut Item<T>, child_items: &'a ItemSliceSync<'a, Item<T>>) -> Self {
            Node { item, child_items }
        }

        /// Return the pack byte range used to resolve this entry's header and compressed data.
        pub fn entry_slice(&self) -> crate::data::EntryRange {
            self.item.offset..self.item.next_offset
        }

        /// Return the data associated with this node.
        pub fn data(&mut self) -> &mut T {
            &mut self.item.data
        }

        /// Return true if this node is a base for other deltas.
        pub fn has_children(&self) -> bool {
            !self.item.children().is_empty()
        }

        pub fn add_children(&mut self, children: impl IntoIterator<Item = u32>) {
            self.item.extend_children(children);
        }

        /// Transform this node into an iterator over its children.
        pub fn into_child_iter(self) -> impl Iterator<Item = Node<'a, T>> + 'a {
            let children = self.child_items;
            #[expect(unsafe_code)]
            self.item.children().iter().map(move |&index| {
                // SAFETY: Tree guarantees that each child index belongs to exactly one parent.
                let item = unsafe { children.get_mut(index as usize) };
                // SAFETY: The child inherits the same uniqueness guarantee.
                unsafe { Node::new(item, children) }
            })
        }
    }
}

use node::Node;

fn attach_ref_delta_children<T: Send>(
    node: &mut Node<'_, T>,
    entry: &data::Entry,
    decompressed: &[u8],
    ref_delta_children: Option<&super::SharedRefDeltaChildren>,
    object_hash: gix_hash::Kind,
) -> Result<(), Error> {
    let Some(ref_delta_children) = ref_delta_children else {
        return Ok(());
    };
    // Avoid hashing every remaining object once all pending ref-deltas have found their bases.
    if threading::lock(ref_delta_children).is_empty() {
        return Ok(());
    }

    let kind = entry.header.as_kind().expect("a fully resolved object has a base kind");
    let id = gix_object::compute_hash(object_hash, kind, decompressed)
        .or_raise_erased(|| message("Failed to hash an object while resolving in-pack ref-deltas"))?;
    if let Some(children) = threading::lock(ref_delta_children).remove(&id) {
        node.add_children(children);
    }
    Ok(())
}

/// A parsed entry and its decompressed bytes, ready to serve as a delta base.
struct ResolvedBase {
    /// The pack entry, with a delta header replaced by its resolved object kind.
    entry: data::Entry,
    /// The pack offset immediately after the entry.
    entry_end: u64,
    /// The fully resolved object bytes.
    bytes: Vec<u8>,
}

/// A resolved base shared by sibling work items.
///
/// [`OwnShared`] uses an `Arc` for parallel builds and an `Rc` otherwise. Once all siblings have released their clones,
/// the task holding the sole reference can use [`OwnShared::try_unwrap()`] to recover the base and reuse its `Vec`
/// allocation as scratch space.
type SharedResolvedBase = OwnShared<ResolvedBase>;

/// A schedulable delta-tree node.
///
/// Work items can move between workers because each [`Node`] grants exclusive access to one item, while siblings
/// share their parent only through an immutable [`SharedResolvedBase`].
struct WorkItem<'a, T: Send> {
    /// The traversal level, with roots at level `0`.
    level: u16,
    /// The exclusive handle to the tree item being resolved.
    node: Node<'a, T>,
    /// The resolved parent's entry and bytes needed to apply this node's delta, or `None` for roots.
    parent: Option<SharedResolvedBase>,
}

/// Resolve all delta trees from a shared, lock-free work-stealing pool.
/// It's `unsafe` as there is safety-constraints on `items` and `child_items`.
///
/// SAFETY: `items` and `child_items` must originate from the same [`crate::cache::delta::Tree`].
#[expect(clippy::too_many_arguments, unsafe_code)]
#[deny(unsafe_op_in_unsafe_fn)]
pub(super) unsafe fn all<T, F, MBFN, R>(
    items: &mut [Item<T>],
    child_items: &ItemSliceSync<'_, Item<T>>,
    thread_limit: Option<usize>,
    num_objects: usize,
    objects: gix_features::progress::StepShared,
    size: gix_features::progress::StepShared,
    progress: &dyn Progress,
    resolve: F,
    resolve_data: &R,
    modify_base: MBFN,
    ref_delta_children: Option<super::SharedRefDeltaChildren>,
    object_hash: gix_hash::Kind,
    alloc_limit_bytes: Option<usize>,
    should_interrupt: &AtomicBool,
) -> Result<(), Error>
where
    T: Send,
    R: Send + Sync,
    F: for<'r> Fn(EntryRange, &'r R) -> Option<&'r [u8]> + Send + Clone,
    MBFN: FnMut(&mut T, &dyn Progress, Context<'_>) -> Result<(), gix_error::Exn> + Send + Clone,
{
    let work = items
        .iter_mut()
        .map(|item| {
            // SAFETY: Required from the caller, and each root item is unique.
            #[expect(unsafe_code)]
            let node = unsafe { Node::new(item, child_items) };
            WorkItem {
                level: 0,
                node,
                parent: None,
            }
        })
        .collect::<Vec<_>>();

    #[cfg(feature = "parallel")]
    {
        resolve_parallel(
            gix_features::parallel::num_threads(thread_limit).min(num_objects),
            work,
            objects,
            size,
            progress,
            resolve,
            resolve_data,
            modify_base,
            ref_delta_children,
            object_hash,
            alloc_limit_bytes,
            should_interrupt,
        )
    }
    #[cfg(not(feature = "parallel"))]
    {
        let _ = thread_limit;
        let _ = num_objects;
        resolve_serial(
            work,
            objects,
            size,
            progress,
            resolve,
            resolve_data,
            modify_base,
            ref_delta_children,
            object_hash,
            alloc_limit_bytes,
            should_interrupt,
        )
    }
}

/// Resolve all work on the current thread using `work` as a LIFO stack.
///
/// `work` initially contains only roots. Resolving a node adds a [`WorkItem`] for each child to the same stack consumed by
/// the loop, so processing continues through dynamically scheduled descendants, including ref-delta children attached
/// during resolution, until both they and the remaining roots are exhausted. LIFO order makes children of the current
/// node run before roots that were already waiting.
#[cfg(not(feature = "parallel"))]
#[expect(clippy::too_many_arguments)]
fn resolve_serial<T, F, MBFN, R>(
    mut work: Vec<WorkItem<'_, T>>,
    objects: gix_features::progress::StepShared,
    size: gix_features::progress::StepShared,
    progress: &dyn Progress,
    resolve: F,
    resolve_data: &R,
    mut modify_base: MBFN,
    ref_delta_children: Option<super::SharedRefDeltaChildren>,
    object_hash: gix_hash::Kind,
    alloc_limit_bytes: Option<usize>,
    should_interrupt: &AtomicBool,
) -> Result<(), Error>
where
    T: Send,
    R: Send + Sync,
    F: for<'r> Fn(EntryRange, &'r R) -> Option<&'r [u8]> + Send + Clone,
    MBFN: FnMut(&mut T, &dyn Progress, Context<'_>) -> Result<(), gix_error::Exn> + Send + Clone,
{
    let mut delta_bytes = Vec::new();
    let mut fully_resolved_delta_bytes = Vec::new();
    let mut inflate = gix_zlib::Inflate::default();
    while let Some(task) = work.pop() {
        if should_interrupt.load(Ordering::Relaxed) {
            return Err(interrupted());
        }
        resolve_task(
            task,
            &mut delta_bytes,
            &mut fully_resolved_delta_bytes,
            &mut inflate,
            progress,
            &resolve,
            resolve_data,
            &mut modify_base,
            ref_delta_children.as_ref(),
            object_hash,
            alloc_limit_bytes,
            &objects,
            &size,
            |child| work.push(child),
        )?;
    }
    Ok(())
}

/// Resolve work in parallel with per-worker LIFO queues and work stealing.
///
/// Roots start in a shared queue. Each worker prefers children on its own queue, then steals from peer queues, and finally
/// takes another root. This favors completing active trees before starting more roots.
///
/// Newly discovered children are scheduled on the current worker, where idle peers can steal them and help with an active
/// tree. A worker that temporarily finds no task yields while other work is queued or in progress, since it may expose
/// more descendants, and exits only when no work remains.
#[cfg(feature = "parallel")]
#[expect(clippy::too_many_arguments)]
fn resolve_parallel<T, F, MBFN, R>(
    num_threads: usize,
    work: Vec<WorkItem<'_, T>>,
    objects: gix_features::progress::StepShared,
    size: gix_features::progress::StepShared,
    progress: &dyn Progress,
    resolve: F,
    resolve_data: &R,
    modify_base: MBFN,
    ref_delta_children: Option<super::SharedRefDeltaChildren>,
    object_hash: gix_hash::Kind,
    alloc_limit_bytes: Option<usize>,
    should_interrupt: &AtomicBool,
) -> Result<(), Error>
where
    T: Send,
    R: Send + Sync,
    F: for<'r> Fn(EntryRange, &'r R) -> Option<&'r [u8]> + Send + Clone,
    MBFN: FnMut(&mut T, &dyn Progress, Context<'_>) -> Result<(), gix_error::Exn> + Send + Clone,
{
    use std::sync::atomic::AtomicUsize;

    if num_threads == 0 {
        return Ok(());
    }
    let roots = crossbeam_deque::Injector::new();
    let remaining = AtomicUsize::new(work.len());
    for task in work {
        roots.push(task);
    }
    let workers: Vec<_> = (0..num_threads).map(|_| crossbeam_deque::Worker::new_lifo()).collect();
    let stealers: Vec<_> = workers.iter().map(crossbeam_deque::Worker::stealer).collect();
    let abort = AtomicBool::new(false);

    gix_features::parallel::threads(|scope| {
        let mut handles = Vec::with_capacity(num_threads);
        for (tid, worker) in workers.into_iter().enumerate() {
            let result = gix_features::parallel::build_thread()
                .name(format!("gix-pack.traverse_deltas.{tid}"))
                .spawn_scoped(scope, {
                    let stealers = &stealers;
                    let roots = &roots;
                    let remaining = &remaining;
                    let abort = &abort;
                    let objects = &objects;
                    let size = &size;
                    let resolve = resolve.clone();
                    let mut modify_base = modify_base.clone();
                    let ref_delta_children = ref_delta_children.clone();
                    move || {
                        // Make sure we never deadlock because a panicking worker can't update `remaining` anymore.
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            let mut delta_bytes = Vec::new();
                            let mut fully_resolved_delta_bytes = Vec::new();
                            let mut inflate = gix_zlib::Inflate::default();
                            loop {
                                if abort.load(Ordering::Relaxed) {
                                    return Ok(());
                                }
                                if should_interrupt.load(Ordering::Relaxed) {
                                    abort.store(true, Ordering::Relaxed);
                                    return Err(interrupted());
                                }
                                let Some(task) = steal(&worker, stealers, roots) else {
                                    if remaining.load(Ordering::Acquire) == 0 {
                                        return Ok(());
                                    }
                                    std::thread::yield_now();
                                    continue;
                                };

                                let task_result = resolve_task(
                                    task,
                                    &mut delta_bytes,
                                    &mut fully_resolved_delta_bytes,
                                    &mut inflate,
                                    progress,
                                    &resolve,
                                    resolve_data,
                                    &mut modify_base,
                                    ref_delta_children.as_ref(),
                                    object_hash,
                                    alloc_limit_bytes,
                                    objects,
                                    size,
                                    |child| {
                                        remaining.fetch_add(1, Ordering::Release);
                                        worker.push(child);
                                    },
                                );
                                remaining.fetch_sub(1, Ordering::AcqRel);
                                if let Err(err) = task_result {
                                    abort.store(true, Ordering::Relaxed);
                                    return Err(err);
                                }
                            }
                        }));
                        if result.is_err() {
                            abort.store(true, Ordering::Relaxed);
                        }
                        result.unwrap_or_else(|payload| std::panic::resume_unwind(payload))
                    }
                });
            match result {
                Ok(handle) => handles.push(handle),
                Err(err) => {
                    abort.store(true, Ordering::Relaxed);
                    for handle in handles {
                        if let Err(payload) = handle.join() {
                            std::panic::resume_unwind(payload);
                        }
                    }
                    return Err(err
                        .and_raise(message("Failed to spawn thread when switching to work-stealing mode"))
                        .erased());
                }
            }
        }

        let mut error = None;
        for handle in handles {
            match handle.join() {
                Ok(Err(err)) if error.is_none() => error = Some(err),
                Ok(_) => {}
                Err(payload) => std::panic::resume_unwind(payload),
            }
        }
        error.map_or(Ok(()), Err)
    })
}

/// Take local work or steal it from another worker or the shared root queue, in that order.
///
/// Work in peer queues belongs to already active trees and retains shared base buffers. Preferring it advances those
/// trees toward completion and releases their buffers before another root starts a new tree.
#[cfg(feature = "parallel")]
fn steal<T>(
    worker: &crossbeam_deque::Worker<T>,
    stealers: &[crossbeam_deque::Stealer<T>],
    roots: &crossbeam_deque::Injector<T>,
) -> Option<T> {
    if let Some(task) = worker.pop() {
        return Some(task);
    }
    loop {
        let mut retry = false;
        for stealer in stealers {
            match stealer.steal() {
                crossbeam_deque::Steal::Success(task) => return Some(task),
                crossbeam_deque::Steal::Retry => retry = true,
                crossbeam_deque::Steal::Empty => {}
            }
        }
        match roots.steal() {
            crossbeam_deque::Steal::Success(task) => return Some(task),
            crossbeam_deque::Steal::Retry => retry = true,
            crossbeam_deque::Steal::Empty => {}
        }
        if !retry {
            return None;
        }
    }
}

/// Resolve one work item using scratch buffers owned by the calling worker.
///
/// `delta_bytes` holds inflated delta instructions, while `fully_resolved_delta_bytes` receives the object produced by
/// applying them. Passing both by mutable reference preserves their allocations across tasks handled by the same worker.
///
/// A resolved delta that has children moves its output buffer into [`SharedResolvedBase`]. Once the last sibling releases
/// that base, [`OwnShared::try_unwrap()`] can recover its allocation; the largest available delta buffer is retained as
/// `fully_resolved_delta_bytes` for a later task. Root objects deliberately use a task-local buffer instead.
///
/// `push` separates discovering children from scheduling them. This function calls it with each child [`WorkItem`];
/// serial traversal pushes that item onto its `Vec`, while parallel traversal pushes it onto the current worker's deque
/// and updates the shared count of unfinished work.
#[expect(clippy::too_many_arguments)]
fn resolve_task<'a, T, F, MBFN, R>(
    WorkItem {
        level,
        mut node,
        parent,
    }: WorkItem<'a, T>,
    delta_bytes: &mut Vec<u8>,
    fully_resolved_delta_bytes: &mut Vec<u8>,
    inflate: &mut gix_zlib::Inflate,
    progress: &dyn Progress,
    resolve: &F,
    resolve_data: &R,
    modify_base: &mut MBFN,
    ref_delta_children: Option<&super::SharedRefDeltaChildren>,
    object_hash: gix_hash::Kind,
    alloc_limit_bytes: Option<usize>,
    objects: &gix_features::progress::StepShared,
    size: &gix_features::progress::StepShared,
    mut push: impl FnMut(WorkItem<'a, T>),
) -> Result<(), Error>
where
    T: Send,
    R: Send + Sync,
    F: for<'r> Fn(EntryRange, &'r R) -> Option<&'r [u8]> + Send,
    MBFN: FnMut(&mut T, &dyn Progress, Context<'_>) -> Result<(), gix_error::Exn> + Send,
{
    let is_root = parent.is_none();
    // Root buffers either become shared bases or are dropped after inspection. Keeping leaf-root allocations out of
    // worker scratch avoids retaining an occasionally huge capacity for the worker's lifetime.
    let mut root_bytes = Vec::new();
    let (entry, entry_end) = if let Some(parent) = parent.as_ref() {
        let (mut entry, entry_end) = decompress_from_resolver(
            node.entry_slice(),
            delta_bytes,
            inflate,
            resolve,
            resolve_data,
            object_hash,
            alloc_limit_bytes,
        )?;
        let (base_size, consumed) = data::delta::decode_header_size(delta_bytes).or_erased()?;
        let base_size = decoded_size_limited(base_size, alloc_limit_bytes)?;
        if parent.bytes.len() != base_size {
            return Err(data::delta::apply::Error::new(
                "Corrupt delta data: delta base size does not match base object size",
            )
            .raise_erased());
        }
        let (result_size, result_header_size) =
            data::delta::decode_header_size(&delta_bytes[consumed..]).or_erased()?;
        let result_size = decoded_size_limited(result_size, alloc_limit_bytes)?;
        resize_with_limit(fully_resolved_delta_bytes, result_size, alloc_limit_bytes)?;
        data::delta::apply(
            &parent.bytes,
            fully_resolved_delta_bytes,
            &delta_bytes[consumed + result_header_size..],
        )
        .or_erased()?;
        entry.header = parent.entry.header;
        (entry, entry_end)
    } else {
        decompress_from_resolver(
            node.entry_slice(),
            &mut root_bytes,
            inflate,
            resolve,
            resolve_data,
            object_hash,
            alloc_limit_bytes,
        )?
    };

    let resolved = ResolvedBase {
        entry,
        entry_end,
        bytes: if is_root {
            root_bytes
        } else {
            std::mem::take(fully_resolved_delta_bytes)
        },
    };
    attach_ref_delta_children(
        &mut node,
        &resolved.entry,
        &resolved.bytes,
        ref_delta_children,
        object_hash,
    )?;
    let has_children = node.has_children();
    inspect(&mut node, level, &resolved, progress, modify_base, objects, size)?;
    let mut reusable = if has_children {
        let resolved = OwnShared::new(resolved);
        for child in node.into_child_iter() {
            push(WorkItem {
                level: level + 1,
                node: child,
                parent: Some(OwnShared::clone(&resolved)),
            });
        }
        None
    } else if is_root {
        None
    } else {
        Some(resolved.bytes)
    };

    // This might be a leaf, while its base buffer now is also exclusively available,
    // and if so, keep the larger buffer.
    if let Some(parent) = parent {
        if let Ok(parent) = OwnShared::try_unwrap(parent) {
            if reusable
                .as_ref()
                .is_none_or(|reusable| parent.bytes.capacity() > reusable.capacity())
            {
                reusable = Some(parent.bytes);
            }
        }
    }
    if let Some(reusable) = reusable {
        *fully_resolved_delta_bytes = reusable;
    }
    fully_resolved_delta_bytes.clear();
    Ok(())
}

/// Hand one fully resolved node to the caller's inspector and record its progress.
///
/// `modify_base` receives mutable access to the node's associated data and a [`Context`] containing the parsed entry,
/// its end offset, the resolved object bytes, and its delta-tree level. Only a successful inspection increments the
/// object counter and the total number of resolved bytes; inspector errors abort traversal.
fn inspect<T, MBFN>(
    node: &mut Node<'_, T>,
    level: u16,
    resolved: &ResolvedBase,
    progress: &dyn Progress,
    modify_base: &mut MBFN,
    objects: &gix_features::progress::StepShared,
    size: &gix_features::progress::StepShared,
) -> Result<(), Error>
where
    T: Send,
    MBFN: FnMut(&mut T, &dyn Progress, Context<'_>) -> Result<(), gix_error::Exn> + Send,
{
    modify_base(
        node.data(),
        progress,
        Context {
            entry: &resolved.entry,
            entry_end: resolved.entry_end,
            decompressed: &resolved.bytes,
            level,
        },
    )
    .or_raise_erased(|| message("One of the object inspectors failed"))?;
    objects.fetch_add(1, Ordering::Relaxed);
    size.fetch_add(resolved.bytes.len(), Ordering::Relaxed);
    Ok(())
}

/// Resolve and decompress one pack entry into `out`, returning its parsed metadata and end offset.
///
/// `resolve` keeps traversal independent of pack storage by borrowing the bytes for `slice` from caller-owned
/// `resolve_data`, such as an in-memory buffer or mapped file.
fn decompress_from_resolver<F, R>(
    slice: EntryRange,
    out: &mut Vec<u8>,
    inflate: &mut gix_zlib::Inflate,
    resolve: &F,
    resolve_data: &R,
    object_hash: gix_hash::Kind,
    alloc_limit_bytes: Option<usize>,
) -> Result<(data::Entry, u64), Error>
where
    F: for<'r> Fn(EntryRange, &'r R) -> Option<&'r [u8]> + Send,
{
    let bytes = resolve(slice.clone(), resolve_data).ok_or_else(|| {
        gix_error::message!(
            "The resolver failed to obtain the pack entry bytes for the entry at {}",
            slice.start
        )
        .raise_erased()
    })?;
    let entry = data::Entry::from_bytes(bytes, slice.start, object_hash).or_erased()?;
    let compressed = &bytes[entry.header_size()..];
    let decompressed_len = decoded_size_limited(entry.decompressed_size, alloc_limit_bytes)?;
    decompress_all_at_once_with(inflate, compressed, decompressed_len, out, alloc_limit_bytes)?;
    Ok((entry, slice.end))
}

fn decompress_all_at_once_with(
    inflate: &mut gix_zlib::Inflate,
    b: &[u8],
    decompressed_len: usize,
    out: &mut Vec<u8>,
    alloc_limit_bytes: Option<usize>,
) -> Result<(), Error> {
    resize_with_limit(out, decompressed_len, alloc_limit_bytes)?;
    inflate.reset();
    inflate
        .once(b, out)
        .or_raise_erased(|| message("Failed to decompress entry"))?;
    Ok(())
}

fn decoded_size_limited(size: u64, alloc_limit_bytes: Option<usize>) -> Result<usize, Error> {
    let size: usize = size.try_into().map_err(|_| out_of_memory())?;
    if alloc_limit_bytes.is_some_and(|limit| size > limit) {
        return Err(out_of_memory());
    }
    Ok(size)
}

fn resize_with_limit(out: &mut Vec<u8>, len: usize, alloc_limit_bytes: Option<usize>) -> Result<(), Error> {
    if alloc_limit_bytes.is_some_and(|limit| len > limit) {
        return Err(out_of_memory());
    }
    out.try_reserve(len.saturating_sub(out.len()))
        .or_raise_erased(|| message("Entry too large to fit in memory"))?;
    out.resize(len, 0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        io::Write,
        sync::atomic::{AtomicBool, AtomicUsize, Ordering},
        time::Duration,
    };

    use gix_features::progress;

    use crate::{
        cache::delta::{Tree, traverse},
        data,
    };

    #[test]
    fn traversal_resolves_children_lazily() {
        let mut pack = Vec::new();
        let root_offset = append_entry(&mut pack, data::entry::Header::Blob, 1, b"A");
        let first_child = append_delta(&mut pack, root_offset, b'B');
        let second_child = append_delta(&mut pack, root_offset, b'C');
        let first_leaf = append_delta(&mut pack, first_child, b'D');
        let second_leaf = append_delta(&mut pack, second_child, b'E');

        let mut tree = Tree::with_capacity(5).expect("capacity is small");
        tree.add_root(root_offset, ()).expect("offsets are increasing");
        tree.add_child(root_offset, first_child, ())
            .expect("offsets are increasing");
        tree.add_child(root_offset, second_child, ())
            .expect("offsets are increasing");
        tree.add_child(first_child, first_leaf, ())
            .expect("offsets are increasing");
        tree.add_child(second_child, second_leaf, ())
            .expect("offsets are increasing");

        let resolve_calls = AtomicUsize::new(0);
        let calls_at_first_child = AtomicUsize::new(usize::MAX);
        traverse(
            tree,
            &pack,
            Some(1),
            None,
            |slice, pack| {
                resolve_calls.fetch_add(1, Ordering::Relaxed);
                pack.get(slice.start as usize..slice.end as usize)
            },
            |(), _progress, context| {
                if context.level == 1 {
                    calls_at_first_child.fetch_min(resolve_calls.load(Ordering::Relaxed), Ordering::Relaxed);
                }
                Ok::<_, gix_error::Exn>(())
            },
        )
        .expect("valid delta tree");

        assert_eq!(
            calls_at_first_child.load(Ordering::Relaxed),
            2,
            "the first child must be inspected before its siblings are materialized"
        );
    }

    #[test]
    fn traversal_parallelizes_children_of_one_root() {
        let mut pack = Vec::new();
        let root_offset = append_entry(&mut pack, data::entry::Header::Blob, 1, b"A");
        let child_offsets: Vec<_> = (b'B'..=b'I')
            .map(|byte| append_delta(&mut pack, root_offset, byte))
            .collect();

        let mut tree = Tree::with_capacity(1 + child_offsets.len()).expect("capacity is small");
        tree.add_root(root_offset, ()).expect("offsets are increasing");
        for child_offset in child_offsets {
            tree.add_child(root_offset, child_offset, ())
                .expect("offsets are increasing");
        }

        let active = AtomicUsize::new(0);
        let max_active = AtomicUsize::new(0);
        traverse(
            tree,
            &pack,
            Some(2),
            None,
            |slice, pack| pack.get(slice.start as usize..slice.end as usize),
            |(), _progress, context| {
                if context.level > 0 {
                    let now_active = active.fetch_add(1, Ordering::Relaxed) + 1;
                    max_active.fetch_max(now_active, Ordering::Relaxed);
                    std::thread::sleep(Duration::from_millis(20));
                    active.fetch_sub(1, Ordering::Relaxed);
                }
                Ok::<_, gix_error::Exn>(())
            },
        )
        .expect("valid delta tree");

        let expected = if cfg!(feature = "parallel") { 1 } else { 0 };
        assert!(
            max_active.load(Ordering::Relaxed) > expected,
            "idle workers must help with children of the last remaining root (if in parallel mode)"
        );
    }

    #[test]
    fn traversal_rejects_declared_decompressed_size_over_alloc_limit() {
        let mut pack = Vec::new();
        let root_offset = append_entry(&mut pack, data::entry::Header::Blob, 1, b"");
        let mut tree = Tree::with_capacity(1).expect("capacity is small");
        tree.add_root(root_offset, ()).expect("offsets are increasing");

        let err = traverse_with_limit(tree, &pack).expect_err("entry size exceeds the allocation cap");

        assert_eq!(
            err.to_string(),
            "Entry too large to fit in memory",
            "declared decompressed sizes above the cap must be rejected before allocation"
        );
    }

    #[test]
    fn traversal_rejects_delta_base_size_over_alloc_limit() {
        let mut pack = Vec::new();
        let root_offset = append_entry(&mut pack, data::entry::Header::Blob, 0, b"");

        let delta = [1, 0];
        let child_offset = pack.len() as u64;
        append_entry(
            &mut pack,
            data::entry::Header::OfsDelta {
                base_distance: child_offset - root_offset,
            },
            delta.len() as u64,
            &delta,
        );

        let mut tree = Tree::with_capacity(2).expect("capacity is small");
        tree.add_root(root_offset, ()).expect("offsets are increasing");
        tree.add_child(root_offset, child_offset, ())
            .expect("offsets are increasing");

        let err = traverse_with_limit(tree, &pack).expect_err("delta base size exceeds the allocation cap");

        assert_eq!(
            err.to_string(),
            "Entry too large to fit in memory",
            "delta base sizes above the cap must be rejected before comparing them with the decoded base"
        );
    }

    #[test]
    fn traversal_rejects_delta_result_size_over_alloc_limit() {
        let mut pack = Vec::new();
        let root_offset = append_entry(&mut pack, data::entry::Header::Blob, 0, b"");

        let delta = [0, 1, 1, b'A'];
        let child_offset = pack.len() as u64;
        append_entry(
            &mut pack,
            data::entry::Header::OfsDelta {
                base_distance: child_offset - root_offset,
            },
            delta.len() as u64,
            &delta,
        );

        let mut tree = Tree::with_capacity(2).expect("capacity is small");
        tree.add_root(root_offset, ()).expect("offsets are increasing");
        tree.add_child(root_offset, child_offset, ())
            .expect("offsets are increasing");

        let err = traverse_with_limit(tree, &pack).expect_err("delta result size exceeds the allocation cap");

        assert_eq!(
            err.to_string(),
            "Entry too large to fit in memory",
            "delta result sizes above the cap must be rejected before resizing the output buffer"
        );
    }

    fn traverse_with_limit(tree: Tree<()>, pack: &Vec<u8>) -> Result<(), traverse::Error> {
        traverse(
            tree,
            pack,
            Some(1),
            Some(0),
            |slice, pack| pack.get(slice.start as usize..slice.end as usize),
            |(), _progress, _context| Ok::<_, gix_error::Exn>(()),
        )
    }

    fn traverse<F, MBFN>(
        tree: Tree<()>,
        pack: &Vec<u8>,
        thread_limit: Option<usize>,
        alloc_limit_bytes: Option<usize>,
        resolve: F,
        inspect: MBFN,
    ) -> Result<(), traverse::Error>
    where
        F: for<'r> Fn(data::EntryRange, &'r Vec<u8>) -> Option<&'r [u8]> + Send + Clone,
        MBFN:
            FnMut(&mut (), &dyn progress::Progress, traverse::Context<'_>) -> Result<(), gix_error::Exn> + Send + Clone,
    {
        let should_interrupt = AtomicBool::new(false);
        let mut size_progress = progress::Discard;
        tree.traverse(
            resolve,
            pack,
            pack.len() as u64,
            inspect,
            traverse::Options {
                object_progress: Box::new(progress::Discard),
                size_progress: &mut size_progress,
                thread_limit,
                should_interrupt: &should_interrupt,
                object_hash: gix_hash::Kind::Sha1,
                alloc_limit_bytes,
            },
        )
        .map(|_| ())
    }

    fn append_delta(pack: &mut Vec<u8>, base_offset: data::Offset, byte: u8) -> data::Offset {
        let delta = [1, 1, 1, byte];
        let offset = pack.len() as data::Offset;
        append_entry(
            pack,
            data::entry::Header::OfsDelta {
                base_distance: offset - base_offset,
            },
            delta.len() as u64,
            &delta,
        )
    }

    fn append_entry(
        pack: &mut Vec<u8>,
        header: data::entry::Header,
        decompressed_size: u64,
        payload: &[u8],
    ) -> data::Offset {
        let offset = pack.len() as data::Offset;
        header
            .write_to(decompressed_size, pack)
            .expect("writing an entry header to memory succeeds");
        pack.extend(deflate(payload));
        offset
    }

    fn deflate(bytes: &[u8]) -> Vec<u8> {
        let mut out = gix_zlib::stream::deflate::Write::new(Vec::new(), gix_zlib::Compression::BEST_SPEED);
        out.write_all(bytes).expect("writing to deflater succeeds");
        out.flush().expect("flushing deflater succeeds");
        out.into_inner()
    }
}
