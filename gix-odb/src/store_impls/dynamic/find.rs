use std::ops::Deref;

use gix_pack::cache::DecodeEntry;

use crate::store::{handle, load_index};

pub(crate) mod error {
    use crate::{loose, pack};

    /// Returned by [`Handle::try_find()`][gix_pack::Find::try_find()]
    #[derive(thiserror::Error, Debug)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("An error occurred while obtaining an object from the loose object store")]
        Loose(#[from] loose::find::Error),
        #[error("An error occurred while obtaining an object from the packed object store")]
        Pack(#[from] pack::data::decode::Error),
        #[error(transparent)]
        LoadIndex(#[from] crate::store::load_index::Error),
        #[error(transparent)]
        LoadPack(#[from] std::io::Error),
        #[error(transparent)]
        EntryType(#[from] gix_pack::data::entry::decode::Error),
        #[error("Reached recursion limit of {} while resolving ref delta bases for {}", .max_depth, .id)]
        DeltaBaseRecursionLimit {
            /// the maximum recursion depth we encountered.
            max_depth: usize,
            /// The original object to lookup
            id: gix_hash::ObjectId,
        },
        #[error("The base object {} could not be found but is required to decode {}", .base_id, .id)]
        DeltaBaseMissing {
            /// the id of the base object which failed to lookup
            base_id: gix_hash::ObjectId,
            /// The original object to lookup
            id: gix_hash::ObjectId,
        },
        #[error("An error occurred when looking up a ref delta base object {} to decode {}", .base_id, .id)]
        DeltaBaseLookup {
            #[source]
            err: Box<Self>,
            /// the id of the base object which failed to lookup
            base_id: gix_hash::ObjectId,
            /// The original object to lookup
            id: gix_hash::ObjectId,
        },
    }

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

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn error_size() {
            let actual = std::mem::size_of::<Error>();
            assert!(actual <= 88, "{actual} <= 88: should not grow without us noticing");
        }
    }
}
pub use error::Error;
use gix_features::zlib;

use crate::store::types::PackId;

impl<S> super::Handle<S>
where
    S: Deref<Target = super::Store> + Clone,
{
    fn try_find_cached_inner<'a, 'b>(
        &'b self,
        mut id: &'b gix_hash::oid,
        buffer: &'a mut Vec<u8>,
        inflate: &mut zlib::Inflate,
        pack_cache: &mut dyn DecodeEntry,
        snapshot: &mut load_index::Snapshot,
        recursion: Option<error::DeltaBaseRecursion<'_>>,
    ) -> Result<Option<(gix_object::Data<'a>, Option<gix_pack::data::entry::Location>)>, Error> {
        if let Some(r) = recursion {
            if r.depth >= self.max_recursion_depth {
                return Err(Error::DeltaBaseRecursionLimit {
                    max_depth: self.max_recursion_depth,
                    id: r.original_id.to_owned(),
                });
            }
        } else if !self.ignore_replacements {
            if let Ok(pos) = self
                .store
                .replacements
                .binary_search_by(|(map_this, _)| map_this.as_ref().cmp(id))
            {
                id = self.store.replacements[pos].1.as_ref();
            }
        }

        'outer: loop {
            {
                let marker = snapshot.marker;
                for (idx, index) in snapshot.indices.iter_mut().enumerate() {
                    if let Some(handle::index_lookup::Outcome {
                        object_index: handle::IndexForObjectInPack { pack_id, pack_offset },
                        index_file,
                        pack: possibly_pack,
                    }) = index.lookup(id)
                    {
                        let pack = match possibly_pack {
                            Some(pack) => pack,
                            None => match self.store.load_pack(pack_id, marker)? {
                                Some(pack) => {
                                    *possibly_pack = Some(pack);
                                    possibly_pack.as_deref().expect("just put it in")
                                }
                                None => {
                                    // The pack wasn't available anymore so we are supposed to try another round with a fresh index
                                    match self.store.load_one_index(self.refresh, snapshot.marker)? {
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
                                let pack_offset = index_file.pack_offset_by_id(id)?;
                                pack.entry(pack_offset)
                                    .ok()
                                    .map(gix_pack::data::decode::entry::ResolvedBase::InPack)
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
                            Err(gix_pack::data::decode::Error::DeltaBaseUnresolved(base_id)) => {
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
                                let obj_kind = self
                                    .try_find_cached_inner(
                                        &base_id,
                                        &mut buf,
                                        inflate,
                                        pack_cache,
                                        snapshot,
                                        recursion
                                            .map(error::DeltaBaseRecursion::inc_depth)
                                            .or_else(|| error::DeltaBaseRecursion::new(id).into()),
                                    )
                                    .map_err(|err| Error::DeltaBaseLookup {
                                        err: Box::new(err),
                                        base_id,
                                        id: id.to_owned(),
                                    })?
                                    .ok_or_else(|| Error::DeltaBaseMissing {
                                        base_id,
                                        id: id.to_owned(),
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
                                } = match snapshot.indices[idx].lookup(id) {
                                    Some(res) => res,
                                    None => {
                                        let mut out = None;
                                        for index in &mut snapshot.indices {
                                            out = index.lookup(id);
                                            if out.is_some() {
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
                                    &|id, out| {
                                        index_file
                                            .pack_offset_by_id(id)
                                            .and_then(|pack_offset| {
                                                pack.entry(pack_offset)
                                                    .ok()
                                                    .map(gix_pack::data::decode::entry::ResolvedBase::InPack)
                                            })
                                            .or_else(|| {
                                                (id == base_id).then(|| {
                                                    out.resize(buf.len(), 0);
                                                    out.copy_from_slice(buf.as_slice());
                                                    gix_pack::data::decode::entry::ResolvedBase::OutOfPack {
                                                        kind: obj_kind,
                                                        end: out.len(),
                                                    }
                                                })
                                            })
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
                            Err(err) => Err(err),
                        }?;

                        if idx != 0 {
                            snapshot.indices.swap(0, idx);
                        }
                        return Ok(Some(res));
                    }
                }
            }

            for lodb in snapshot.loose_dbs.iter() {
                // TODO: remove this double-lookup once the borrow checker allows it.
                if lodb.contains(id) {
                    return lodb
                        .try_find(id, buffer)
                        .map(|obj| obj.map(|obj| (obj, None)))
                        .map_err(Into::into);
                }
            }

            match self.store.load_one_index(self.refresh, snapshot.marker)? {
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
}

impl<S> gix_pack::Find for super::Handle<S>
where
    S: Deref<Target = super::Store> + Clone,
{
    // TODO: probably make this method fallible, but that would mean its own error type.
    fn contains(&self, id: &gix_hash::oid) -> bool {
        let mut snapshot = self.snapshot.borrow_mut();
        loop {
            for (idx, index) in snapshot.indices.iter().enumerate() {
                if index.contains(id) {
                    if idx != 0 {
                        snapshot.indices.swap(0, idx);
                    }
                    return true;
                }
            }

            for lodb in snapshot.loose_dbs.iter() {
                if lodb.contains(id) {
                    return true;
                }
            }

            match self.store.load_one_index(self.refresh, snapshot.marker) {
                Ok(Some(new_snapshot)) => {
                    *snapshot = new_snapshot;
                    self.clear_cache();
                }
                Ok(None) => return false, // nothing more to load, or our refresh mode doesn't allow disk refreshes
                Err(_) => return false, // something went wrong, nothing we can communicate here with this trait. TODO: Maybe that should change?
            }
        }
    }

    fn try_find_cached<'a>(
        &self,
        id: &gix_hash::oid,
        buffer: &'a mut Vec<u8>,
        pack_cache: &mut dyn DecodeEntry,
    ) -> Result<Option<(gix_object::Data<'a>, Option<gix_pack::data::entry::Location>)>, gix_object::find::Error> {
        let mut snapshot = self.snapshot.borrow_mut();
        let mut inflate = self.inflate.borrow_mut();
        self.try_find_cached_inner(id, buffer, &mut inflate, pack_cache, &mut snapshot, None)
            .map_err(|err| Box::new(err) as _)
    }

    fn location_by_oid(&self, id: &gix_hash::oid, buf: &mut Vec<u8>) -> Option<gix_pack::data::entry::Location> {
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
                    }) = index.lookup(id)
                    {
                        let pack = match possibly_pack {
                            Some(pack) => pack,
                            None => match self.store.load_pack(pack_id, marker).ok()? {
                                Some(pack) => {
                                    *possibly_pack = Some(pack);
                                    possibly_pack.as_deref().expect("just put it in")
                                }
                                None => {
                                    // The pack wasn't available anymore so we are supposed to try another round with a fresh index
                                    match self.store.load_one_index(self.refresh, snapshot.marker).ok()? {
                                        Some(new_snapshot) => {
                                            *snapshot = new_snapshot;
                                            self.clear_cache();
                                            continue 'outer;
                                        }
                                        None => {
                                            // nothing new in the index, kind of unexpected to not have a pack but to also
                                            // to have no new index yet. We set the new index before removing any slots, so
                                            // this should be observable.
                                            return None;
                                        }
                                    }
                                }
                            },
                        };
                        let entry = pack.entry(pack_offset).ok()?;
                        // This allocation is driven by on-disk pack metadata, so keep it aligned with
                        // `gix_pack::data::File::with_alloc_limit_bytes()`.
                        let size: usize = entry.decompressed_size.try_into().ok()?;
                        if pack.alloc_limit_bytes().is_some_and(|limit| size > limit) {
                            return None;
                        }
                        buf.resize(size, 0);
                        assert_eq!(pack.id, pack_id.to_intrinsic_pack_id(), "both ids must always match");

                        let res = pack
                            .decompress_entry(&entry, &mut inflate, buf)
                            .ok()
                            .map(|entry_size_past_header| gix_pack::data::entry::Location {
                                pack_id: pack.id,
                                pack_offset,
                                entry_size: entry.header_size() + entry_size_past_header,
                            });

                        if idx != 0 {
                            snapshot.indices.swap(0, idx);
                        }
                        return res;
                    }
                }
            }

            match self.store.load_one_index(self.refresh, snapshot.marker).ok()? {
                Some(new_snapshot) => {
                    *snapshot = new_snapshot;
                    self.clear_cache();
                }
                None => return None,
            }
        }
    }

    fn pack_offsets_and_oid(&self, pack_id: u32) -> Option<Vec<(u64, gix_hash::ObjectId)>> {
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
                        return Some(iter.map(|e| (e.pack_offset, e.oid)).collect());
                    }
                }
            }

            match self.store.load_one_index(self.refresh, snapshot.marker).ok()? {
                Some(new_snapshot) => {
                    drop(snapshot);
                    *self.snapshot.borrow_mut() = new_snapshot;
                }
                None => return None,
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
    fn try_find<'a>(
        &self,
        id: &gix_hash::oid,
        buffer: &'a mut Vec<u8>,
    ) -> Result<Option<gix_object::Data<'a>>, gix_object::find::Error> {
        gix_pack::Find::try_find(self, id, buffer).map(|t| t.map(|t| t.0))
    }
}

impl<S> gix_object::FindHeader for super::Handle<S>
where
    S: Deref<Target = super::Store> + Clone,
{
    fn try_header(&self, id: &gix_hash::oid) -> Result<Option<gix_object::Header>, gix_object::find::Error> {
        let mut snapshot = self.snapshot.borrow_mut();
        let mut inflate = self.inflate.borrow_mut();
        self.try_header_inner(id, &mut inflate, &mut snapshot, None)
            .map(|maybe_header| {
                maybe_header.map(|hdr| gix_object::Header {
                    kind: hdr.kind(),
                    size: hdr.size(),
                })
            })
            .map_err(|err| Box::new(err) as _)
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
