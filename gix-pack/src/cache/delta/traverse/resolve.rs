use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicBool, AtomicIsize, Ordering},
};

use gix_features::{progress::Progress, threading, zlib};

use crate::{
    cache::delta::{
        traverse::{util::ItemSliceSync, Context, Error},
        Item,
    },
    data,
    data::EntryRange,
};

mod root {
    use crate::cache::delta::{traverse::util::ItemSliceSync, Item};

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
        /// Returns the offset into the pack at which the `Node`s data is located.
        pub fn offset(&self) -> u64 {
            self.item.offset
        }

        /// Returns the slice into the data pack at which the pack entry is located.
        pub fn entry_slice(&self) -> crate::data::EntryRange {
            self.item.offset..self.item.next_offset
        }

        /// Returns the node data associated with this node.
        pub fn data(&mut self) -> &mut T {
            &mut self.item.data
        }

        /// Returns true if this node has children, e.g. is not a leaf in the tree.
        pub fn has_children(&self) -> bool {
            !self.item.children().is_empty()
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

pub(super) struct State<'items, F, MBFN, T: Send> {
    pub delta_bytes: Vec<u8>,
    pub fully_resolved_delta_bytes: Vec<u8>,
    pub progress: Box<dyn Progress>,
    pub resolve: F,
    pub modify_base: MBFN,
    pub child_items: &'items ItemSliceSync<'items, Item<T>>,
}

/// SAFETY: `item.children` must uniquely reference elements in child_items that no other currently alive
/// item does. All child_items must also have unique children.
///
/// This safety invariant can be reliably upheld by making sure `item` comes from a Tree and `child_items`
/// was constructed using that Tree's child_items. This works since Tree has this invariant as well: all
/// child_items are referenced at most once (really, exactly once) by a node in the tree.
#[allow(clippy::too_many_arguments, unsafe_code)]
#[deny(unsafe_op_in_unsafe_fn)] // this is a big function, require unsafe for the one small unsafe op we have
pub(super) unsafe fn deltas<T, F, MBFN, E, R>(
    objects: gix_features::progress::StepShared,
    size: gix_features::progress::StepShared,
    item: &mut Item<T>,
    State {
        delta_bytes,
        fully_resolved_delta_bytes,
        progress,
        resolve,
        modify_base,
        child_items,
    }: &mut State<'_, F, MBFN, T>,
    resolve_data: &R,
    hash_len: usize,
    threads_left: &AtomicIsize,
    should_interrupt: &AtomicBool,
) -> Result<(), Error>
where
    T: Send,
    R: Send + Sync,
    F: for<'r> Fn(EntryRange, &'r R) -> Option<&'r [u8]> + Send + Clone,
    MBFN: FnMut(&mut T, &dyn Progress, Context<'_>) -> Result<(), E> + Send + Clone,
    E: std::error::Error + Send + Sync + 'static,
{
    let mut decompressed_bytes_by_pack_offset = BTreeMap::new();
    let mut inflate = zlib::Inflate::default();
    let mut decompress_from_resolver = |slice: EntryRange, out: &mut Vec<u8>| -> Result<(data::Entry, u64), Error> {
        let bytes = resolve(slice.clone(), resolve_data).ok_or(Error::ResolveFailed {
            pack_offset: slice.start,
        })?;
        let entry = data::Entry::from_bytes(bytes, slice.start, hash_len)?;
        let compressed = &bytes[entry.header_size()..];
        let decompressed_len = entry.decompressed_size as usize;
        decompress_all_at_once_with(&mut inflate, compressed, decompressed_len, out)?;
        Ok((entry, slice.end))
    };

    // each node is a base, and its children always start out as deltas which become a base after applying them.
    // These will be pushed onto our stack until all are processed
    let root_level = 0;
    // SAFETY: This invariant is required from the caller
    #[allow(unsafe_code)]
    let root_node = unsafe { root::Node::new(item, child_items) };
    let mut nodes: Vec<_> = vec![(root_level, root_node)];
    while let Some((level, mut base)) = nodes.pop() {
        if should_interrupt.load(Ordering::Relaxed) {
            return Err(Error::Interrupted);
        }
        let (base_entry, entry_end, base_bytes) = if level == root_level {
            let mut buf = Vec::new();
            let (a, b) = decompress_from_resolver(base.entry_slice(), &mut buf)?;
            (a, b, buf)
        } else {
            decompressed_bytes_by_pack_offset
                .remove(&base.offset())
                .expect("we store the resolved delta buffer when done")
        };

        // anything done here must be repeated further down for leaf-nodes.
        // This way we avoid retaining their decompressed memory longer than needed (they have no children,
        // thus their memory can be released right away, using 18% less peak memory on the linux kernel).
        {
            modify_base(
                base.data(),
                progress,
                Context {
                    entry: &base_entry,
                    entry_end,
                    decompressed: &base_bytes,
                    level,
                },
            )
            .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>)?;
            objects.fetch_add(1, Ordering::Relaxed);
            size.fetch_add(base_bytes.len(), Ordering::Relaxed);
        }

        for mut child in base.into_child_iter() {
            let (mut child_entry, entry_end) = decompress_from_resolver(child.entry_slice(), delta_bytes)?;
            let (base_size, consumed) = data::delta::decode_header_size(delta_bytes);
            let mut header_ofs = consumed;
            assert_eq!(
                base_bytes.len(),
                base_size as usize,
                "recorded base size in delta does match the actual one"
            );
            let (result_size, consumed) = data::delta::decode_header_size(&delta_bytes[consumed..]);
            header_ofs += consumed;

            fully_resolved_delta_bytes.resize(result_size as usize, 0);
            data::delta::apply(&base_bytes, fully_resolved_delta_bytes, &delta_bytes[header_ofs..]);

            // FIXME: this actually invalidates the "pack_offset()" computation, which is not obvious to consumers
            //        at all
            child_entry.header = base_entry.header; // assign the actual object type, instead of 'delta'
            if child.has_children() {
                decompressed_bytes_by_pack_offset.insert(
                    child.offset(),
                    (child_entry, entry_end, std::mem::take(fully_resolved_delta_bytes)),
                );
                nodes.push((level + 1, child));
            } else {
                modify_base(
                    child.data(),
                    &progress,
                    Context {
                        entry: &child_entry,
                        entry_end,
                        decompressed: fully_resolved_delta_bytes,
                        level: level + 1,
                    },
                )
                .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>)?;
                objects.fetch_add(1, Ordering::Relaxed);
                size.fetch_add(base_bytes.len(), Ordering::Relaxed);
            }
        }

        // After the first round, see if we can use additional threads, and if so we enter multi-threaded mode.
        // In it we will keep using new threads as they become available while using this thread for coordination.
        // We optimize for a low memory footprint as we are likely to get here if long delta-chains with large objects are involved.
        // Try to avoid going into threaded mode if there isn't more than one unit of work anyway.
        if nodes.len() > 1 {
            if let Ok(initial_threads) =
                threads_left.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |threads_available| {
                    (threads_available > 0).then_some(0)
                })
            {
                // Assure no memory is held here.
                *delta_bytes = Vec::new();
                *fully_resolved_delta_bytes = Vec::new();
                return deltas_mt(
                    initial_threads,
                    decompressed_bytes_by_pack_offset,
                    objects,
                    size,
                    &progress,
                    nodes,
                    resolve.clone(),
                    resolve_data,
                    modify_base.clone(),
                    hash_len,
                    threads_left,
                    should_interrupt,
                );
            }
        }
    }

    Ok(())
}

/// * `initial_threads` is the threads we may spawn, not accounting for our own thread which is still considered used by the parent
///   system. Since this thread will take a controlling function, we may spawn one more than that. In threaded mode, we will finish
///   all remaining work.
#[allow(clippy::too_many_arguments)]
fn deltas_mt<T, F, MBFN, E, R>(
    mut threads_to_create: isize,
    decompressed_bytes_by_pack_offset: BTreeMap<u64, (data::Entry, u64, Vec<u8>)>,
    objects: gix_features::progress::StepShared,
    size: gix_features::progress::StepShared,
    progress: &dyn Progress,
    nodes: Vec<(u16, root::Node<'_, T>)>,
    resolve: F,
    resolve_data: &R,
    modify_base: MBFN,
    hash_len: usize,
    threads_left: &AtomicIsize,
    should_interrupt: &AtomicBool,
) -> Result<(), Error>
where
    T: Send,
    R: Send + Sync,
    F: for<'r> Fn(EntryRange, &'r R) -> Option<&'r [u8]> + Send + Clone,
    MBFN: FnMut(&mut T, &dyn Progress, Context<'_>) -> Result<(), E> + Send + Clone,
    E: std::error::Error + Send + Sync + 'static,
{
    let nodes = gix_features::threading::Mutable::new(nodes);
    let decompressed_bytes_by_pack_offset = gix_features::threading::Mutable::new(decompressed_bytes_by_pack_offset);
    threads_to_create += 1; // ourselves
    let mut returned_ourselves = false;

    gix_features::parallel::threads(|s| -> Result<(), Error> {
        let mut threads = Vec::new();
        let poll_interval = std::time::Duration::from_millis(100);
        loop {
            for tid in 0..threads_to_create {
                let thread = gix_features::parallel::build_thread()
                    .name(format!("gix-pack.traverse_deltas.{tid}"))
                    .spawn_scoped(s, {
                        let nodes = &nodes;
                        let decompressed_bytes_by_pack_offset = &decompressed_bytes_by_pack_offset;
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

                            loop {
                                let (level, mut base) = match threading::lock(nodes).pop() {
                                    Some(v) => v,
                                    None => break,
                                };
                                if should_interrupt.load(Ordering::Relaxed) {
                                    return Err(Error::Interrupted);
                                }
                                let (base_entry, entry_end, base_bytes) = if level == 0 {
                                    let mut buf = Vec::new();
                                    let (a, b) = decompress_from_resolver(base.entry_slice(), &mut buf)?;
                                    (a, b, buf)
                                } else {
                                    threading::lock(decompressed_bytes_by_pack_offset)
                                        .remove(&base.offset())
                                        .expect("we store the resolved delta buffer when done")
                                };

                                // anything done here must be repeated further down for leaf-nodes.
                                // This way we avoid retaining their decompressed memory longer than needed (they have no children,
                                // thus their memory can be released right away, using 18% less peak memory on the linux kernel).
                                {
                                    modify_base(
                                        base.data(),
                                        progress,
                                        Context {
                                            entry: &base_entry,
                                            entry_end,
                                            decompressed: &base_bytes,
                                            level,
                                        },
                                    )
                                    .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>)?;
                                    objects.fetch_add(1, Ordering::Relaxed);
                                    size.fetch_add(base_bytes.len(), Ordering::Relaxed);
                                }

                                for mut child in base.into_child_iter() {
                                    let (mut child_entry, entry_end) =
                                        decompress_from_resolver(child.entry_slice(), &mut delta_bytes)?;
                                    let (base_size, consumed) = data::delta::decode_header_size(&delta_bytes);
                                    let mut header_ofs = consumed;
                                    assert_eq!(
                                        base_bytes.len(),
                                        base_size as usize,
                                        "recorded base size in delta does match the actual one"
                                    );
                                    let (result_size, consumed) =
                                        data::delta::decode_header_size(&delta_bytes[consumed..]);
                                    header_ofs += consumed;

                                    fully_resolved_delta_bytes.resize(result_size as usize, 0);
                                    data::delta::apply(
                                        &base_bytes,
                                        &mut fully_resolved_delta_bytes,
                                        &delta_bytes[header_ofs..],
                                    );

                                    // FIXME: this actually invalidates the "pack_offset()" computation, which is not obvious to consumers
                                    //        at all
                                    child_entry.header = base_entry.header; // assign the actual object type, instead of 'delta'
                                    if child.has_children() {
                                        threading::lock(decompressed_bytes_by_pack_offset).insert(
                                            child.offset(),
                                            (child_entry, entry_end, std::mem::take(&mut fully_resolved_delta_bytes)),
                                        );
                                        threading::lock(nodes).push((level + 1, child));
                                    } else {
                                        modify_base(
                                            child.data(),
                                            progress,
                                            Context {
                                                entry: &child_entry,
                                                entry_end,
                                                decompressed: &fully_resolved_delta_bytes,
                                                level: level + 1,
                                            },
                                        )
                                        .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>)?;
                                        objects.fetch_add(1, Ordering::Relaxed);
                                        size.fetch_add(base_bytes.len(), Ordering::Relaxed);
                                    }
                                }
                            }
                            Ok(())
                        }
                    })?;
                threads.push(thread);
            }
            if threads_left
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |threads_available: isize| {
                    (threads_available > 0).then(|| {
                        threads_to_create = threads_available.min(threading::lock(&nodes).len() as isize);
                        threads_available - threads_to_create
                    })
                })
                .is_err()
            {
                threads_to_create = 0;
            }

            // What we really want to do is either wait for one of our threads to go down
            // or for another scheduled thread to become available. Unfortunately we can't do that,
            // but may instead find a good way to set the polling interval instead of hard-coding it.
            std::thread::sleep(poll_interval);
            // Get out of threads are already starving or they would be starving soon as no work is left.
            //
            // Lint: ScopedJoinHandle is not the same depending on active features and is not exposed in some cases.
            #[allow(clippy::redundant_closure_for_method_calls)]
            if threads.iter().any(|t| t.is_finished()) {
                let mut running_threads = Vec::new();
                for thread in threads.drain(..) {
                    if thread.is_finished() {
                        match thread.join() {
                            Ok(Err(err)) => return Err(err),
                            Ok(Ok(())) => {
                                if !returned_ourselves {
                                    returned_ourselves = true;
                                } else {
                                    threads_left.fetch_add(1, Ordering::SeqCst);
                                }
                            }
                            Err(err) => {
                                std::panic::resume_unwind(err);
                            }
                        }
                    } else {
                        running_threads.push(thread);
                    }
                }
                if running_threads.is_empty() && threading::lock(&nodes).is_empty() {
                    break;
                }
                threads = running_threads;
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
