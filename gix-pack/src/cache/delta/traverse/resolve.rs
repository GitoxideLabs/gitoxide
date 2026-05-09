use std::sync::atomic::{AtomicBool, Ordering};

use gix_features::{progress::Progress, zlib};

use crate::{
    cache::delta::{
        Item,
        traverse::{Context, Error, util::ItemSliceSync},
    },
    data,
    data::EntryRange,
};

mod root {
    use crate::cache::delta::{Item, traverse::util::ItemSliceSync};

    /// An item returned by `iter_root_chunks`, allowing access to the `data` stored alongside nodes in a [`Tree`].
    pub(crate) struct Node<'a, T: Send> {
        // SAFETY INVARIANT: see Node::new(). That function is the only one used
        // to create or modify these fields.
        item: &'a mut Item<T>,
        child_items: &'a ItemSliceSync<'a, Item<T>>,
    }

    impl<'a, T: Send> Node<'a, T> {
        /// SAFETY: `item.children` must uniquely reference elements in child_items that no other currently alive
        /// item does. All child_items must also have unique children, unless the child_item is itself `item`,
        /// in which case no other live item should reference it in its `item.children`.
        ///
        /// This safety invariant can be reliably upheld by making sure `item` comes from a Tree and `child_items`
        /// was constructed using that Tree's child_items. This works since Tree has this invariant as well: all
        /// child_items are referenced at most once (really, exactly once) by a node in the tree.
        ///
        /// Note that this invariant is a bit more relaxed than that on `deltas()`, because this function can be called
        /// for traversal within a child item, which happens in into_child_iter()
        #[allow(unsafe_code)]
        pub(super) unsafe fn new(item: &'a mut Item<T>, child_items: &'a ItemSliceSync<'a, Item<T>>) -> Self {
            Node { item, child_items }
        }
    }

    impl<'a, T: Send> Node<'a, T> {
        /// Returns the slice into the data pack at which the pack entry is located.
        pub fn entry_slice(&self) -> crate::data::EntryRange {
            self.item.offset..self.item.next_offset
        }

        /// Returns the node data associated with this node.
        pub fn data(&mut self) -> &mut T {
            &mut self.item.data
        }

        /// Transform this `Node` into an iterator over its children.
        ///
        /// Children are `Node`s referring to pack entries whose base object is this pack entry.
        pub fn into_child_iter(self) -> impl Iterator<Item = Node<'a, T>> + 'a {
            let children = self.child_items;
            #[allow(unsafe_code)]
            self.item.children().iter().map(move |&index| {
                // SAFETY: Due to the invariant on new(), we can rely on these indices
                // being unique.
                let item = unsafe { children.get_mut(index as usize) };
                // SAFETY: Since every child_item is also required to uphold the uniqueness guarantee,
                // creating a Node with one of the child_items that we are allowed access to is still fine.
                unsafe { Node::new(item, children) }
            })
        }
    }
}

/// (delta depth, node to resolve, parent's resolved bytes `None` for level-0 roots)
type WorkItem<'a, T> = (
    u16,
    root::Node<'a, T>,
    Option<std::sync::Arc<(data::Entry, u64, Vec<u8>)>>,
);

struct WorkQueue<'a, T: Send> {
    items: std::sync::Mutex<Vec<WorkItem<'a, T>>>,
    cvar: std::sync::Condvar,
    /// Indicator that items are still being processed
    in_progress: std::sync::atomic::AtomicUsize,
}

impl<'a, T: Send> WorkQueue<'a, T> {
    fn new(nodes: Vec<WorkItem<'a, T>>) -> Self {
        WorkQueue {
            items: std::sync::Mutex::new(nodes),
            cvar: std::sync::Condvar::new(),
            in_progress: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Blocks until an item is available, returns `None` only when all work is done.
    fn pop(&self) -> Option<WorkItem<'a, T>> {
        let mut guard = self.items.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        loop {
            if let Some(v) = guard.pop() {
                self.in_progress.fetch_add(1, Ordering::SeqCst);
                return Some(v);
            }
            if self.in_progress.load(Ordering::SeqCst) == 0 {
                return None;
            }
            // Nothing available yet but work is still in-progress — wait for a push or finish.
            guard = self.cvar.wait(guard).unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }

    /// Push a batch of items and wake waiting workers.
    fn push_items(&self, children: Vec<WorkItem<'a, T>>, num_threads: usize) {
        if children.is_empty() {
            return;
        }
        let n = children.len();
        let mut guard = self.items.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.extend(children);
        drop(guard);
        for _ in 0..n.min(num_threads) {
            self.cvar.notify_one();
        }
    }

    /// Mark one item done. Must be called for every `pop`, including error paths.
    fn finish_item(&self) {
        let _guard = self.items.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        self.in_progress.fetch_sub(1, Ordering::SeqCst);
        self.cvar.notify_all();
    }
}

/// Resolve all delta objects in parallel using a work-stealing queue.
///
/// SAFETY: `item.children` must uniquely reference elements in child_items that no other currently alive
/// item does. All child_items must also have unique children.
/// This safety invariant can be reliably upheld by making sure `item` comes from a Tree and `child_items`
/// was constructed using that Tree's child_items. This works since Tree has this invariant as well: all
/// child_items are referenced at most once (really, exactly once) by a node in the tree.
#[allow(clippy::too_many_arguments, unsafe_code)]
#[deny(unsafe_op_in_unsafe_fn)]
pub(super) unsafe fn resolve_all_nodes<T, F, MBFN, E, R>(
    items: &mut [Item<T>],
    child_items: &ItemSliceSync<'_, Item<T>>,
    thread_limit: Option<usize>,
    objects: gix_features::progress::StepShared,
    size: gix_features::progress::StepShared,
    progress: &dyn Progress,
    resolve: F,
    resolve_data: &R,
    modify_base: MBFN,
    hash_len: usize,
    should_interrupt: &AtomicBool,
) -> Result<(), Error>
where
    T: Send,
    R: Send + Sync,
    F: for<'r> Fn(EntryRange, &'r R) -> Option<&'r [u8]> + Send + Clone,
    MBFN: FnMut(&mut T, &dyn Progress, Context<'_>) -> Result<(), E> + Send + Clone,
    E: std::error::Error + Send + Sync + 'static,
{
    let num_threads = gix_features::parallel::num_threads(thread_limit);
    let nodes = items
        .iter_mut()
        .map(|item| (0u16, unsafe { root::Node::new(item, child_items) }, None))
        .collect();
    deltas_mt(
        num_threads,
        objects,
        size,
        progress,
        nodes,
        resolve,
        resolve_data,
        modify_base,
        hash_len,
        should_interrupt,
    )
}

#[allow(clippy::too_many_arguments)]
fn deltas_mt<T, F, MBFN, E, R>(
    num_threads: usize,
    objects: gix_features::progress::StepShared,
    size: gix_features::progress::StepShared,
    progress: &dyn Progress,
    nodes: Vec<WorkItem<'_, T>>,
    resolve: F,
    resolve_data: &R,
    modify_base: MBFN,
    hash_len: usize,
    should_interrupt: &AtomicBool,
) -> Result<(), Error>
where
    T: Send,
    R: Send + Sync,
    F: for<'r> Fn(EntryRange, &'r R) -> Option<&'r [u8]> + Send + Clone,
    MBFN: FnMut(&mut T, &dyn Progress, Context<'_>) -> Result<(), E> + Send + Clone,
    E: std::error::Error + Send + Sync + 'static,
{
    let queue = WorkQueue::new(nodes);

    gix_features::parallel::threads(|s| -> Result<(), Error> {
        let threads = (0..num_threads)
            .map(|tid| {
                gix_features::parallel::build_thread()
                    .name(format!("gix-pack.traverse_deltas.{tid}"))
                    .spawn_scoped(s, {
                        let queue = &queue;
                        let resolve = resolve.clone();
                        let mut modify_base = modify_base.clone();
                        let objects = &objects;
                        let size = &size;

                        move || -> Result<(), Error> {
                            let mut fully_resolved_delta_bytes = Vec::new();
                            let mut delta_bytes = Vec::new();
                            let mut inflate = zlib::Inflate::default();
                            let mut decompress_from_resolver =
                                |slice: EntryRange, out: &mut Vec<u8>| -> Result<(data::Entry, u64), Error> {
                                    let bytes = resolve(slice.clone(), resolve_data).ok_or(Error::ResolveFailed {
                                        pack_offset: slice.start,
                                    })?;
                                    let entry = data::Entry::from_bytes(bytes, slice.start, hash_len)?;
                                    let compressed = &bytes[entry.header_size()..];
                                    let decompressed_len = entry.decompressed_size as usize;
                                    decompress_all_at_once_with(&mut inflate, compressed, decompressed_len, out)?;
                                    Ok((entry, slice.end))
                                };

                            while let Some((level, mut base, parent_arc)) = queue.pop() {
                                let result: Result<(), Error> = (|| {
                                    if should_interrupt.load(Ordering::Relaxed) {
                                        return Err(Error::Interrupted);
                                    }

                                    let base_arc: std::sync::Arc<(data::Entry, u64, Vec<u8>)> =
                                        if let Some(parent) = parent_arc {
                                            let (mut entry, entry_end) =
                                                decompress_from_resolver(base.entry_slice(), &mut delta_bytes)?;
                                            let (base_size, consumed) = data::delta::decode_header_size(&delta_bytes)?;
                                            let mut header_ofs = consumed;
                                            assert_eq!(
                                                parent.2.len(),
                                                base_size as usize,
                                                "recorded base size in delta does not match the actual one"
                                            );
                                            let (result_size, consumed) =
                                                data::delta::decode_header_size(&delta_bytes[consumed..])?;
                                            header_ofs += consumed;
                                            fully_resolved_delta_bytes.resize(result_size as usize, 0);
                                            data::delta::apply(
                                                &parent.2,
                                                &mut fully_resolved_delta_bytes,
                                                &delta_bytes[header_ofs..],
                                            )?;
                                            // FIXME: this actually invalidates the "pack_offset()" computation
                                            entry.header = parent.0.header; // inherit real object type
                                            std::sync::Arc::new((
                                                entry,
                                                entry_end,
                                                std::mem::take(&mut fully_resolved_delta_bytes),
                                            ))
                                        } else {
                                            let mut buf = Vec::new();
                                            let (entry, entry_end) =
                                                decompress_from_resolver(base.entry_slice(), &mut buf)?;
                                            std::sync::Arc::new((entry, entry_end, buf))
                                        };

                                    let (base_entry, entry_end, base_bytes) = &*base_arc;
                                    modify_base(
                                        base.data(),
                                        progress,
                                        Context {
                                            entry: base_entry,
                                            entry_end: *entry_end,
                                            decompressed: base_bytes,
                                            level,
                                        },
                                    )
                                    .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>)?;
                                    objects.fetch_add(1, Ordering::Relaxed);
                                    size.fetch_add(base_bytes.len(), Ordering::Relaxed);

                                    let children = base
                                        .into_child_iter()
                                        .map(|child| (level + 1, child, Some(std::sync::Arc::clone(&base_arc))))
                                        .collect();
                                    queue.push_items(children, num_threads);

                                    Ok(())
                                })();

                                queue.finish_item();
                                result?;
                            }
                            Ok(())
                        }
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;

        for thread in threads {
            match thread.join() {
                Ok(Ok(())) => {}
                Ok(Err(err)) => return Err(err),
                Err(payload) => std::panic::resume_unwind(payload),
            }
        }
        Ok(())
    })
}

fn decompress_all_at_once_with(
    inflate: &mut zlib::Inflate,
    b: &[u8],
    decompressed_len: usize,
    out: &mut Vec<u8>,
) -> Result<(), Error> {
    out.resize(decompressed_len, 0);
    inflate.reset();
    inflate.once(b, out).map_err(|err| Error::ZlibInflate {
        source: err,
        message: "Failed to decompress entry",
    })?;
    Ok(())
}
