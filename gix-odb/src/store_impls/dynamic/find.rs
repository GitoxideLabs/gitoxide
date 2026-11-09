use gix_error::Result;
use std::ops::Deref;

use gix_error::{Message, OptionExt, ResultExt, bail, message, not_found};

use gix_pack::cache::DecodeEntry;

use crate::store::{handle, load_index};

#[derive(Copy, Clone)]
pub(crate) struct DeltaBaseRecursion<'a> {
    pub depth: usize,
    pub original_id: &'a gix_hash::oid,
}

impl<'a> DeltaBaseRecursion<'a> {
    pub fn new(id: &'a gix_hash::oid) -> Self {
        Self {
            original_id: id,
            depth: 0,
        }
    }
    pub fn inc_depth(mut self) -> Self {
        self.depth += 1;
        self
    }
}

use crate::store::types::PackId;

/// The raised error's [metadata](gix_error::Error::metadata()) `max_depth` (unsigned) and `object_id` (hex text) identify
/// the recursion limit and original object.
pub(super) fn delta_base_recursion_limit_error(max_depth: usize, object_id: &gix_hash::oid) -> Message {
    Message::new("Reached recursion limit while resolving ref delta bases")
        .with("max_depth", max_depth)
        .with("object_id", object_id.to_string())
}

/// The raised error's [metadata](gix_error::Error::metadata()) `base_id` and `object_id` (hex text) identify the delta
/// base and object being resolved.
pub(super) fn delta_base_lookup_error(base_id: &gix_hash::oid, object_id: &gix_hash::oid) -> Message {
    Message::new("Could not resolve delta base object")
        .with("base_id", base_id.to_string())
        .with("object_id", object_id.to_string())
}

impl<S> super::Handle<S>
where
    S: Deref<Target = super::Store> + Clone,
{
    /// Delta resolution failures include [metadata](gix_error::Error::metadata()) `object_id` and `base_id` (hex text).
    /// Recursion limits include `object_id` (hex text) and `max_depth` (unsigned).
    fn try_find_cached_inner<'a, 'b>(
        &'b self,
        mut id: &'b gix_hash::oid,
        buffer: &'a mut Vec<u8>,
        inflate: &mut gix_zlib::Inflate,
        pack_cache: &mut dyn DecodeEntry,
        snapshot: &mut load_index::Snapshot,
        recursion: Option<DeltaBaseRecursion<'_>>,
    ) -> Result<Option<(gix_object::Data<'a>, Option<gix_pack::data::entry::Location>)>> {
        if let Some(r) = recursion {
            if r.depth >= self.max_recursion_depth {
                bail!(delta_base_recursion_limit_error(
                    self.max_recursion_depth,
                    r.original_id
                ));
            }
        } else if !self.ignore_replacements
            && let Ok(pos) = self
                .store
                .replacements
                .binary_search_by(|(map_this, _)| map_this.as_ref().cmp(id))
        {
            id = self.store.replacements[pos].1.as_ref();
        }

        'outer: loop {
            {
                let marker = snapshot.marker;
                for (mut idx, index) in snapshot.indices.iter_mut().enumerate() {
                    if let Some(handle::index_lookup::Outcome {
                        object_index: handle::IndexForObjectInPack { pack_id, pack_offset },
                        index_file,
                        pack: possibly_pack,
                    }) = index.lookup(id)?
                    {
                        let pack = match possibly_pack {
                            Some(pack) => pack,
                            None => match self.store.load_pack(pack_id, marker).or_error()? {
                                Some(pack) => {
                                    *possibly_pack = Some(pack);
                                    possibly_pack.as_deref().expect("just put it in")
                                }
                                None => {
                                    // The pack wasn't available anymore so we are supposed to try another round with a fresh index
                                    match self.store.load_one_index(self.index_ctx(snapshot.marker))? {
                                        Some(new_snapshot) => {
                                            *snapshot = new_snapshot;
                                            self.clear_cache();
                                            continue 'outer;
                                        }
                                        None => {
                                            // nothing new in the index, kind of unexpected to not have a pack but to also
                                            // to have no new index yet. We set the new index before removing any slots, so
                                            // this should be observable.
                                            return Ok(None);
                                        }
                                    }
                                }
                            },
                        };
                        let entry = pack.entry(pack_offset)?;
                        let header_size = entry.header_size();
                        let res = pack.decode_entry(
                            entry,
                            buffer,
                            inflate,
                            &|id, _out| {
                                index_file
                                    .pack_offset_by_id(id)?
                                    .map(|pack_offset| {
                                        pack.entry(pack_offset)
                                            .map(gix_pack::data::decode::entry::ResolvedBase::InPack)
                                    })
                                    .transpose()
                            },
                            pack_cache,
                        );

                        let res = match res {
                            Ok(r) => Ok((
                                gix_object::Data {
                                    kind: r.kind,
                                    object_hash: pack.object_hash(),
                                    data: buffer.as_slice(),
                                },
                                Some(gix_pack::data::entry::Location {
                                    pack_id: pack.id,
                                    pack_offset,
                                    entry_size: r.compressed_size + header_size,
                                }),
                            )),
                            Err(err) => {
                                let Some(base_id) = err
                                    .downcast_any_ref::<gix_pack::data::decode::DeltaBaseUnresolved>()
                                    .map(|err| err.0)
                                else {
                                    return Err(err);
                                };
                                // Only with multi-pack indices it's allowed to jump to refer to other packs within this
                                // multi-pack. Otherwise this would constitute a thin pack which is only allowed in transit.
                                // However, if we somehow end up with that, we will resolve it safely, even though we could
                                // avoid handling this case and error instead.

                                // Since this is a special case, we just allocate here to make it work. It's an actual delta-ref object
                                // which is sent by some servers that points to an object outside of the pack we are looking
                                // at right now. With the complexities of loading packs, we go into recursion here. Git itself
                                // doesn't do a cycle check, and we won't either but limit the recursive depth.
                                // The whole ordeal isn't as efficient as it could be due to memory allocation and
                                // later mem-copying when trying again.
                                let mut buf = Vec::new();
                                let context = || delta_base_lookup_error(&base_id, id);
                                let obj_kind = self
                                    .try_find_cached_inner(
                                        &base_id,
                                        &mut buf,
                                        inflate,
                                        pack_cache,
                                        snapshot,
                                        recursion
                                            .map(DeltaBaseRecursion::inc_depth)
                                            .or_else(|| DeltaBaseRecursion::new(id).into()),
                                    )
                                    .or_raise(context)?
                                    .ok_or_raise(|| {
                                        not_found("Could not resolve delta base object: delta base object is missing")
                                            .with("base_id", base_id.to_string())
                                            .with("object_id", id.to_string())
                                    })?
                                    .0
                                    .kind;
                                let handle::index_lookup::Outcome {
                                    object_index:
                                        handle::IndexForObjectInPack {
                                            pack_id: _,
                                            pack_offset,
                                        },
                                    index_file,
                                    pack: possibly_pack,
                                } = match snapshot.indices[idx].lookup(id)? {
                                    Some(res) => res,
                                    None => {
                                        let mut out = None;
                                        for (other_idx, index) in snapshot.indices.iter_mut().enumerate() {
                                            out = index.lookup(id)?;
                                            if out.is_some() {
                                                // Looking up the base moved indices, so the one that answers now is
                                                // the one to move to the front.
                                                idx = other_idx;
                                                break;
                                            }
                                        }

                                        out.unwrap_or_else(|| {
                                           panic!("could not find object {id} in any index after looking up one of its base objects {base_id}" )
                                       })
                                    }
                                };
                                let pack = possibly_pack
                                    .as_ref()
                                    .expect("pack to still be available like just now");
                                let entry = pack.entry(pack_offset)?;
                                let header_size = entry.header_size();
                                pack.decode_entry(
                                    entry,
                                    buffer,
                                    inflate,
                                    &|id, out| match index_file.pack_offset_by_id(id)? {
                                        Some(pack_offset) => pack
                                            .entry(pack_offset)
                                            .map(gix_pack::data::decode::entry::ResolvedBase::InPack)
                                            .map(Some),
                                        None => Ok((id == base_id).then(|| {
                                            out.resize(buf.len(), 0);
                                            out.copy_from_slice(buf.as_slice());
                                            gix_pack::data::decode::entry::ResolvedBase::OutOfPack {
                                                kind: obj_kind,
                                                end: out.len(),
                                            }
                                        })),
                                    },
                                    pack_cache,
                                )
                                .map(move |r| {
                                    (
                                        gix_object::Data {
                                            kind: r.kind,
                                            object_hash: pack.object_hash(),
                                            data: buffer.as_slice(),
                                        },
                                        Some(gix_pack::data::entry::Location {
                                            pack_id: pack.id,
                                            pack_offset,
                                            entry_size: r.compressed_size + header_size,
                                        }),
                                    )
                                })
                            }
                        }?;

                        snapshot.move_to_front(idx);
                        return Ok(Some(res));
                    }
                }
            }

            for lodb in snapshot.loose_dbs.iter() {
                // TODO: remove this double-lookup once the borrow checker allows it.
                if lodb.contains(id) {
                    return lodb.try_find(id, buffer).map(|obj| obj.map(|obj| (obj, None)));
                }
            }

            match self.store.load_one_index(self.index_ctx(snapshot.marker))? {
                Some(new_snapshot) => {
                    *snapshot = new_snapshot;
                    self.clear_cache();
                }
                None => return Ok(None),
            }
        }
    }

    pub(crate) fn clear_cache(&self) {
        self.packed_object_count.borrow_mut().take();
    }

    /// Like [`exists()`](gix_object::Exists::exists), but never look for packs that were added since the last refresh,
    /// whatever this handle's [refresh mode](super::RefreshMode) is. The indices that the store already knows are still loaded.
    ///
    /// Use it to avoid writing objects that already exist. A new object always misses, and in the default refresh
    /// mode each miss lists the pack directory again, which is slow with many packs. Here a miss costs at most a
    /// redundant write.
    pub fn exists_without_refresh(&self, id: &gix_hash::oid) -> bool {
        self.contains_with_refresh_mode(id, super::RefreshMode::Never)
    }

    fn contains_with_refresh_mode(&self, id: &gix_hash::oid, refresh_mode: super::RefreshMode) -> bool {
        let mut snapshot = self.snapshot.borrow_mut();
        loop {
            for (idx, index) in snapshot.indices.iter().enumerate() {
                if index.contains(id) {
                    snapshot.move_to_front(idx);
                    return true;
                }
            }

            for lodb in snapshot.loose_dbs.iter() {
                if lodb.contains(id) {
                    return true;
                }
            }

            match self.store.load_one_index(super::IndexCtx {
                refresh_mode,
                ..self.index_ctx(snapshot.marker)
            }) {
                Ok(Some(new_snapshot)) => {
                    *snapshot = new_snapshot;
                    self.clear_cache();
                }
                Ok(None) => return false, // nothing more to load, or our refresh mode doesn't allow disk refreshes
                Err(_) => return false, // something went wrong, nothing we can communicate here with this trait. TODO: Maybe that should change?
            }
        }
    }
}

impl<S> gix_pack::Find for super::Handle<S>
where
    S: Deref<Target = super::Store> + Clone,
{
    // TODO: make this method fallible to propagate index loading failures.
    fn contains(&self, id: &gix_hash::oid) -> bool {
        self.contains_with_refresh_mode(id, self.refresh)
    }

    fn try_find_cached<'a>(
        &self,
        id: &gix_hash::oid,
        buffer: &'a mut Vec<u8>,
        pack_cache: &mut dyn DecodeEntry,
    ) -> Result<Option<(gix_object::Data<'a>, Option<gix_pack::data::entry::Location>)>> {
        let mut snapshot = self.snapshot.borrow_mut();
        let mut inflate = self.inflate.borrow_mut();
        self.try_find_cached_inner(id, buffer, &mut inflate, pack_cache, &mut snapshot, None)
    }

    fn location_by_oid(
        &self,
        id: &gix_hash::oid,
        buf: &mut Vec<u8>,
    ) -> Result<Option<gix_pack::data::entry::Location>> {
        assert!(
            matches!(self.token.as_ref(), Some(handle::Mode::KeepDeletedPacksAvailable)),
            "BUG: handle must be configured to `prevent_pack_unload()` before using this method"
        );

        assert!(
            self.store_ref().replacements.is_empty() || self.ignore_replacements,
            "Everything related to packing must not use replacements. These are not used here, but it should be turned off for good measure."
        );

        let mut snapshot = self.snapshot.borrow_mut();
        let mut inflate = self.inflate.borrow_mut();
        'outer: loop {
            {
                let marker = snapshot.marker;
                for (idx, index) in snapshot.indices.iter_mut().enumerate() {
                    if let Some(handle::index_lookup::Outcome {
                        object_index: handle::IndexForObjectInPack { pack_id, pack_offset },
                        index_file: _,
                        pack: possibly_pack,
                    }) = index.lookup(id)?
                    {
                        let pack = match possibly_pack {
                            Some(pack) => pack,
                            None => match self.store.load_pack(pack_id, marker).or_error()? {
                                Some(pack) => {
                                    *possibly_pack = Some(pack);
                                    possibly_pack.as_deref().expect("just put it in")
                                }
                                None => {
                                    // The pack wasn't available anymore so we are supposed to try another round with a fresh index
                                    match self.store.load_one_index(self.index_ctx(snapshot.marker))? {
                                        Some(new_snapshot) => {
                                            *snapshot = new_snapshot;
                                            self.clear_cache();
                                            continue 'outer;
                                        }
                                        None => {
                                            // nothing new in the index, kind of unexpected to not have a pack but to also
                                            // to have no new index yet. We set the new index before removing any slots, so
                                            // this should be observable.
                                            return Ok(None);
                                        }
                                    }
                                }
                            },
                        };
                        let entry = pack.entry(pack_offset)?;
                        // This allocation is driven by on-disk pack metadata, so keep it aligned with
                        // `gix_pack::data::File::with_alloc_limit_bytes()`.
                        let size = entry.decompressed_size;
                        if pack.alloc_limit_bytes.is_some_and(|limit| size > limit as u64) {
                            bail!("Cannot store pack entry in memory: allocation limit exceeded".allocation_limit());
                        }
                        let size_usize = usize::try_from(size).or_raise(|| {
                            message!("Cannot store pack entry of {size} bytes in memory: the entry size cannot be represented in memory")
                                .allocation_failure()
                        })?;
                        buf.clear();
                        buf.try_reserve(size_usize).or_raise(|| {
                            message!("Cannot store pack entry of {size} bytes in memory").allocation_failure()
                        })?;
                        buf.resize(size_usize, 0);
                        assert_eq!(pack.id, pack_id.to_intrinsic_pack_id(), "both ids must always match");

                        let entry_size_past_header = pack.decompress_entry(&entry, &mut inflate, buf)?;
                        let location = gix_pack::data::entry::Location {
                            pack_id: pack.id,
                            pack_offset,
                            entry_size: entry.header_size() + entry_size_past_header,
                        };

                        snapshot.move_to_front(idx);
                        return Ok(Some(location));
                    }
                }
            }

            {
                let Some(new_snapshot) = self.store.load_one_index(self.index_ctx(snapshot.marker))? else {
                    return Ok(None);
                };
                *snapshot = new_snapshot;
                self.clear_cache();
            }
        }
    }

    fn pack_offsets_and_oid(&self, pack_id: u32) -> Result<Option<Vec<(gix_pack::data::Offset, gix_hash::ObjectId)>>> {
        assert!(
            matches!(self.token.as_ref(), Some(handle::Mode::KeepDeletedPacksAvailable)),
            "BUG: handle must be configured to `prevent_pack_unload()` before using this method"
        );
        let pack_id = PackId::from_intrinsic_pack_id(pack_id);
        loop {
            let snapshot = self.snapshot.borrow();
            {
                for index in &snapshot.indices {
                    if let Some(iter) = index.iter(pack_id) {
                        return iter
                            .map(|e| e.map(|e| (e.pack_offset, e.oid)))
                            .collect::<Result<Vec<_>>>()
                            .map(Some);
                    }
                }
            }

            {
                let Some(new_snapshot) = self.store.load_one_index(self.index_ctx(snapshot.marker))? else {
                    return Ok(None);
                };
                drop(snapshot);
                *self.snapshot.borrow_mut() = new_snapshot;
            }
        }
    }

    fn entry_by_location(&self, location: &gix_pack::data::entry::Location) -> Option<gix_pack::find::Entry> {
        assert!(
            matches!(self.token.as_ref(), Some(handle::Mode::KeepDeletedPacksAvailable)),
            "BUG: handle must be configured to `prevent_pack_unload()` before using this method"
        );
        let pack_id = PackId::from_intrinsic_pack_id(location.pack_id);
        let mut snapshot = self.snapshot.borrow_mut();
        let marker = snapshot.marker;
        loop {
            {
                for index in &mut snapshot.indices {
                    if let Some(possibly_pack) = index.pack(pack_id) {
                        let pack = match possibly_pack {
                            Some(pack) => pack,
                            None => {
                                let pack = self.store.load_pack(pack_id, marker).ok()?.expect(
                                "BUG: pack must exist from previous call to location_by_oid() and must not be unloaded",
                            );
                                *possibly_pack = Some(pack);
                                possibly_pack.as_deref().expect("just put it in")
                            }
                        };
                        return pack
                            .entry_slice(location.entry_range(location.pack_offset))
                            .map(|data| gix_pack::find::Entry {
                                data: data.to_owned(),
                                version: pack.version(),
                            });
                    }
                }
            }

            snapshot.indices.insert(
                0,
                self.store
                    .index_by_id(pack_id, marker)
                    .expect("BUG: index must always be present, must not be unloaded or overwritten"),
            );
        }
    }
}

impl<S> gix_object::Find for super::Handle<S>
where
    S: Deref<Target = super::Store> + Clone,
    Self: gix_pack::Find,
{
    fn try_find<'a>(&self, id: &gix_hash::oid, buffer: &'a mut Vec<u8>) -> Result<Option<gix_object::Data<'a>>> {
        gix_pack::Find::try_find(self, id, buffer).map(|t| t.map(|t| t.0))
    }
}

impl<S> gix_object::FindHeader for super::Handle<S>
where
    S: Deref<Target = super::Store> + Clone,
{
    fn try_header(&self, id: &gix_hash::oid) -> Result<Option<gix_object::Header>> {
        let mut snapshot = self.snapshot.borrow_mut();
        let mut inflate = self.inflate.borrow_mut();
        self.try_header_inner(id, &mut inflate, &mut snapshot, None)
            .map(|maybe_header| {
                maybe_header.map(|hdr| gix_object::Header {
                    kind: hdr.kind(),
                    size: hdr.size(),
                })
            })
    }
}

impl<S> gix_object::Exists for super::Handle<S>
where
    S: Deref<Target = super::Store> + Clone,
    Self: gix_pack::Find,
{
    fn exists(&self, id: &gix_hash::oid) -> bool {
        gix_pack::Find::contains(self, id)
    }
}
