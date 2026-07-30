use std::ops::Deref;

use gix_hash::oid;

use super::find::Error;
use crate::{
    find::Header,
    store::{find::error::DeltaBaseRecursion, handle, load_index},
};

impl<S> super::Handle<S>
where
    S: Deref<Target = super::Store> + Clone,
{
    pub(crate) fn try_header_inner<'b>(
        &'b self,
        mut id: &'b gix_hash::oid,
        inflate: &mut gix_zlib::Inflate,
        snapshot: &mut load_index::Snapshot,
        recursion: Option<DeltaBaseRecursion<'_>>,
    ) -> Result<Option<Header>, Error> {
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
                'indices: for (idx, index) in snapshot.indices.iter_mut().enumerate() {
                    if let Some(handle::index_lookup::Outcome {
                        object_index:
                            handle::IndexForObjectInPack {
                                pack_index,
                                pack_offset,
                            },
                        index_file,
                        pack: possibly_pack,
                        slot,
                        slot_id,
                    }) = index.lookup(id)
                    {
                        let pack = match possibly_pack {
                            Some(pack) => pack,
                            None => match self.store.load_pack(slot, slot_id, pack_index, &index_file, marker)? {
                                Some(pack) => {
                                    *possibly_pack = Some(pack);
                                    possibly_pack.as_deref().expect("just put it in")
                                }
                                None => {
                                    // The pack wasn't available anymore so we are supposed to try another round with a fresh index
                                    match self.store.load_one_index(self.index_ctx(marker))? {
                                        Some(new_snapshot) => {
                                            *snapshot = new_snapshot;
                                            self.clear_cache();
                                            continue 'outer;
                                        }
                                        None => continue 'indices,
                                    }
                                }
                            },
                        };
                        let entry = pack.entry(pack_offset)?;
                        let res = match pack.decode_header(entry, inflate, &|id| {
                            index_file.pack_offset_by_id(id).and_then(|pack_offset| {
                                pack.entry(pack_offset)
                                    .ok()
                                    .map(gix_pack::data::decode::header::ResolvedBase::InPack)
                            })
                        }) {
                            Ok(header) => Ok(header.into()),
                            Err(gix_pack::data::decode::Error::DeltaBaseUnresolved(base_id)) => {
                                // Only with multi-pack indices it's allowed to jump to refer to other packs within this
                                // multi-pack. Otherwise this would constitute a thin pack which is only allowed in transit.
                                // However, if we somehow end up with that, we will resolve it safely, even though we could
                                // avoid handling this case and error instead.
                                let hdr = self
                                    .try_header_inner(
                                        &base_id,
                                        inflate,
                                        snapshot,
                                        recursion
                                            .map(DeltaBaseRecursion::inc_depth)
                                            .or_else(|| DeltaBaseRecursion::new(id).into()),
                                    )
                                    .map_err(|err| Error::DeltaBaseLookup {
                                        err: Box::new(err),
                                        base_id,
                                        id: id.to_owned(),
                                    })?
                                    .ok_or_else(|| Error::DeltaBaseMissing {
                                        base_id,
                                        id: id.to_owned(),
                                    })?;
                                let handle::index_lookup::Outcome {
                                    object_index:
                                        handle::IndexForObjectInPack {
                                            pack_index: _,
                                            pack_offset,
                                        },
                                    index_file,
                                    pack: possibly_pack,
                                    ..
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
                                            panic!("could not find object {id} in any index after looking up one of its base objects {base_id}")
                                        })
                                    }
                                };
                                let pack = possibly_pack
                                    .as_ref()
                                    .expect("pack to still be available like just now");
                                let entry = pack.entry(pack_offset)?;
                                pack.decode_header(entry, inflate, &|id| {
                                    index_file
                                        .pack_offset_by_id(id)
                                        .and_then(|pack_offset| {
                                            pack.entry(pack_offset)
                                                .ok()
                                                .map(gix_pack::data::decode::header::ResolvedBase::InPack)
                                        })
                                        .or_else(|| {
                                            (id == base_id).then(|| {
                                                gix_pack::data::decode::header::ResolvedBase::OutOfPack {
                                                    kind: hdr.kind(),
                                                    num_deltas: hdr.num_deltas(),
                                                }
                                            })
                                        })
                                })
                                .map(Into::into)
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
                    return lodb.try_header(id).map(|opt| opt.map(Into::into)).map_err(Into::into);
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
}

impl<S> crate::Header for super::Handle<S>
where
    S: Deref<Target = super::Store> + Clone,
{
    fn try_header(&self, id: &oid) -> Result<Option<Header>, gix_object::find::Error> {
        let mut snapshot = self.snapshot.borrow_mut();
        let mut inflate = self.inflate.borrow_mut();
        self.try_header_inner(id, &mut inflate, &mut snapshot, None)
            .map_err(|err| Box::new(err) as _)
    }
}
