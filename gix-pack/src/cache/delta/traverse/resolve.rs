use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicBool, AtomicIsize, Ordering},
};

use gix_features::{progress::Progress, threading};

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
        #[expect(unsafe_code)]
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

        pub fn add_children(&mut self, children: impl IntoIterator<Item = u32>) {
            self.item.extend_children(children);
        }

        /// Transform this `Node` into an iterator over its children.
        ///
        /// Children are `Node`s referring to pack entries whose base object is this pack entry.
        pub fn into_child_iter(self) -> impl Iterator<Item = Node<'a, T>> + 'a {
            let children = self.child_items;
            #[expect(unsafe_code)]
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

fn attach_ref_delta_children<T: Send>(
    node: &mut root::Node<'_, T>,
    entry: &data::Entry,
    decompressed: &[u8],
    ref_delta_children: Option<&super::SharedRefDeltaChildren>,
    object_hash: gix_hash::Kind,
) -> Result<(), Error> {
    let Some(ref_delta_children) = ref_delta_children else {
        return Ok(());
    };
    if threading::lock(ref_delta_children).is_empty() {
        return Ok(());
    }

    let kind = entry.header.as_kind().expect("a fully resolved object has a base kind");
    let id = gix_object::compute_hash(object_hash, kind, decompressed)?;
    if let Some(children) = threading::lock(ref_delta_children).remove(&id) {
        node.add_children(children);
    }
    Ok(())
}

pub(super) struct State<'items, F, MBFN, T: Send> {
    pub delta_bytes: Vec<u8>,
    pub fully_resolved_delta_bytes: Vec<u8>,
    pub progress: Box<dyn Progress>,
    pub resolve: F,
    pub modify_base: MBFN,
    pub child_items: &'items ItemSliceSync<'items, Item<T>>,
    pub ref_delta_children: Option<super::SharedRefDeltaChildren>,
}

/// Resolve and visit the complete delta subtree rooted at `item`.
///
/// The root entry is read and inflated through `resolve`. Each child entry is inflated as delta instructions and applied
/// to its resolved parent's bytes. The resulting object inherits its parent's object kind and is passed to `modify_base`;
/// objects which are themselves bases retain their bytes until their children have been processed.
///
/// After resolving an object, its ID is computed if ref-deltas are waiting for a matching base. Those children are then
/// attached to the object and traversed like children whose bases were known by pack offset.
///
/// # Safety
///
/// This safety invariant can be reliably upheld by making sure `item` comes from a Tree and `child_items`
/// was constructed using that Tree's child_items. This works since Tree has this invariant as well: all
/// child_items are referenced at most once (really, exactly once) by a node in the tree.
///
/// So: `item.children` must contain valid, unique indices into `child_items`, and every child item must recursively uphold the
/// same rule. No index may be reachable from another live root or child. This gives each worker exclusive access to every
/// `Item<T>` it obtains through [`ItemSliceSync`], despite that type using a shared raw pointer internally.
/// The caller can satisfy this contract by taking `item` and `child_items` from the same
/// [`Tree`](crate::cache::delta::Tree), whose construction maintains the single-parent invariant.
#[expect(clippy::too_many_arguments, unsafe_code)]
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
        ref_delta_children,
    }: &mut State<'_, F, MBFN, T>,
    resolve_data: &R,
    object_hash: gix_hash::Kind,
    alloc_limit_bytes: Option<usize>,
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
    let mut inflate = gix_zlib::Inflate::default();
    let mut decompress_from_resolver = |slice: EntryRange, out: &mut Vec<u8>| -> Result<(data::Entry, u64), Error> {
        let bytes = resolve(slice.clone(), resolve_data).ok_or(Error::ResolveFailed {
            pack_offset: slice.start,
        })?;
        let entry = data::Entry::from_bytes(bytes, slice.start, object_hash)?;
        let compressed = &bytes[entry.header_size()..];
        let decompressed_len = decoded_size_limited(entry.decompressed_size, alloc_limit_bytes)?;
        decompress_all_at_once_with(&mut inflate, compressed, decompressed_len, out, alloc_limit_bytes)?;
        Ok((entry, slice.end))
    };

    // each node is a base, and its children always start out as deltas which become a base after applying them.
    // These will be pushed onto our stack until all are processed
    let root_level = 0;
    // SAFETY: This invariant is required from the caller
    #[expect(unsafe_code)]
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
        attach_ref_delta_children(
            &mut base,
            &base_entry,
            &base_bytes,
            ref_delta_children.as_ref(),
            object_hash,
        )?;

        for mut child in base.into_child_iter() {
            let (mut child_entry, entry_end) = decompress_from_resolver(child.entry_slice(), delta_bytes)?;
            let (base_size, consumed) = data::delta::decode_header_size(delta_bytes)?;
            let base_size = decoded_size_limited(base_size, alloc_limit_bytes)?;
            let mut header_ofs = consumed;
            if base_bytes.len() != base_size {
                return Err(data::delta::apply::Error::Corrupt {
                    message: "delta base size does not match base object size",
                }
                .into());
            }
            let (result_size, consumed) = data::delta::decode_header_size(&delta_bytes[consumed..])?;
            let result_size = decoded_size_limited(result_size, alloc_limit_bytes)?;
            header_ofs += consumed;

            resize_with_limit(fully_resolved_delta_bytes, result_size, alloc_limit_bytes)?;
            data::delta::apply(&base_bytes, fully_resolved_delta_bytes, &delta_bytes[header_ofs..])?;

            // FIXME: this actually invalidates the "pack_offset()" computation, which is not obvious to consumers
            //        at all
            child_entry.header = base_entry.header; // assign the actual object type, instead of 'delta'
            attach_ref_delta_children(
                &mut child,
                &child_entry,
                fully_resolved_delta_bytes,
                ref_delta_children.as_ref(),
                object_hash,
            )?;
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
                    ref_delta_children.clone(),
                    object_hash,
                    alloc_limit_bytes,
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
#[expect(clippy::too_many_arguments)]
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
    ref_delta_children: Option<super::SharedRefDeltaChildren>,
    object_hash: gix_hash::Kind,
    alloc_limit_bytes: Option<usize>,
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
                        let ref_delta_children = ref_delta_children.clone();
                        let objects = &objects;
                        let size = &size;

                        move || -> Result<(), Error> {
                            let mut fully_resolved_delta_bytes = Vec::new();
                            let mut delta_bytes = Vec::new();
                            let mut inflate = gix_zlib::Inflate::default();
                            let mut decompress_from_resolver =
                                |slice: EntryRange, out: &mut Vec<u8>| -> Result<(data::Entry, u64), Error> {
                                    let bytes = resolve(slice.clone(), resolve_data).ok_or(Error::ResolveFailed {
                                        pack_offset: slice.start,
                                    })?;
                                    let entry = data::Entry::from_bytes(bytes, slice.start, object_hash)?;
                                    let compressed = &bytes[entry.header_size()..];
                                    let decompressed_len =
                                        decoded_size_limited(entry.decompressed_size, alloc_limit_bytes)?;
                                    decompress_all_at_once_with(
                                        &mut inflate,
                                        compressed,
                                        decompressed_len,
                                        out,
                                        alloc_limit_bytes,
                                    )?;
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
                                attach_ref_delta_children(
                                    &mut base,
                                    &base_entry,
                                    &base_bytes,
                                    ref_delta_children.as_ref(),
                                    object_hash,
                                )?;

                                for mut child in base.into_child_iter() {
                                    let (mut child_entry, entry_end) =
                                        decompress_from_resolver(child.entry_slice(), &mut delta_bytes)?;
                                    let (base_size, consumed) = data::delta::decode_header_size(&delta_bytes)?;
                                    let base_size = decoded_size_limited(base_size, alloc_limit_bytes)?;
                                    let mut header_ofs = consumed;
                                    if base_bytes.len() != base_size {
                                        return Err(data::delta::apply::Error::Corrupt {
                                            message: "delta base size does not match base object size",
                                        }
                                        .into());
                                    }
                                    let (result_size, consumed) =
                                        data::delta::decode_header_size(&delta_bytes[consumed..])?;
                                    let result_size = decoded_size_limited(result_size, alloc_limit_bytes)?;
                                    header_ofs += consumed;

                                    resize_with_limit(&mut fully_resolved_delta_bytes, result_size, alloc_limit_bytes)?;
                                    data::delta::apply(
                                        &base_bytes,
                                        &mut fully_resolved_delta_bytes,
                                        &delta_bytes[header_ofs..],
                                    )?;

                                    // FIXME: this actually invalidates the "pack_offset()" computation, which is not obvious to consumers
                                    //        at all
                                    child_entry.header = base_entry.header; // assign the actual object type, instead of 'delta'
                                    attach_ref_delta_children(
                                        &mut child,
                                        &child_entry,
                                        &fully_resolved_delta_bytes,
                                        ref_delta_children.as_ref(),
                                        object_hash,
                                    )?;
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
            #[allow(
                clippy::redundant_closure_for_method_calls,
                reason = "the closure supports both real and serial scoped thread handles"
            )]
            if threads.iter().any(|thread| thread.is_finished()) {
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
    inflate: &mut gix_zlib::Inflate,
    b: &[u8],
    decompressed_len: usize,
    out: &mut Vec<u8>,
    alloc_limit_bytes: Option<usize>,
) -> Result<(), Error> {
    resize_with_limit(out, decompressed_len, alloc_limit_bytes)?;
    inflate.reset();
    inflate.once(b, out).map_err(|err| Error::ZlibInflate {
        source: err,
        message: "Failed to decompress entry",
    })?;
    Ok(())
}

fn decoded_size_limited(size: u64, alloc_limit_bytes: Option<usize>) -> Result<usize, Error> {
    let size: usize = size.try_into().map_err(|_| Error::OutOfMemory)?;
    if alloc_limit_bytes.is_some_and(|limit| size > limit) {
        return Err(Error::OutOfMemory);
    }
    Ok(size)
}

fn resize_with_limit(out: &mut Vec<u8>, len: usize, alloc_limit_bytes: Option<usize>) -> Result<(), Error> {
    if alloc_limit_bytes.is_some_and(|limit| len > limit) {
        return Err(Error::OutOfMemory);
    }
    out.try_reserve(len.saturating_sub(out.len()))?;
    out.resize(len, 0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{io::Write, sync::atomic::AtomicBool};

    use gix_features::progress;

    use crate::{
        cache::delta::{Tree, traverse},
        data,
    };

    #[test]
    fn traversal_rejects_declared_decompressed_size_over_alloc_limit() {
        let mut pack = Vec::new();
        let root_offset = append_entry(&mut pack, data::entry::Header::Blob, 1, b"");
        let mut tree = Tree::with_capacity(1).expect("capacity is small");
        tree.add_root(root_offset, ()).expect("offsets are increasing");

        let err = traverse_with_limit(tree, &pack).expect_err("entry size exceeds the allocation cap");

        assert!(
            matches!(err, traverse::Error::OutOfMemory),
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

        assert!(
            matches!(err, traverse::Error::OutOfMemory),
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

        assert!(
            matches!(err, traverse::Error::OutOfMemory),
            "delta result sizes above the cap must be rejected before resizing the output buffer"
        );
    }

    fn traverse_with_limit(tree: Tree<()>, pack: &Vec<u8>) -> Result<(), traverse::Error> {
        let should_interrupt = AtomicBool::new(false);
        let mut size_progress = progress::Discard;
        tree.traverse(
            |slice, pack| pack.get(slice.start as usize..slice.end as usize),
            pack,
            pack.len() as u64,
            |(), _progress, _context| Ok::<_, std::io::Error>(()),
            traverse::Options {
                object_progress: Box::new(progress::Discard),
                size_progress: &mut size_progress,
                thread_limit: Some(1),
                should_interrupt: &should_interrupt,
                object_hash: gix_hash::Kind::Sha1,
                alloc_limit_bytes: Some(0),
            },
        )
        .map(|_| ())
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
