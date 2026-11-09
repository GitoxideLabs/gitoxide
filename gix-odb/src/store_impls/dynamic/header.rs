use gix_error::Result;
use std::ops::Deref;

use gix_error::{OptionExt, ResultExt, bail, not_found};

use gix_hash::oid;

use crate::{
    find::Header,
    store::{
        find::{DeltaBaseRecursion, delta_base_lookup_error, delta_base_recursion_limit_error},
        handle, load_index,
    },
};

impl<S> super::Handle<S>
where
    S: Deref<Target = super::Store> + Clone,
{
    /// Delta resolution failures include [metadata](gix_error::Error::metadata()) `object_id` and `base_id` (hex text).
    /// Recursion limits include `object_id` (hex text) and `max_depth` (unsigned).
    pub(crate) fn try_header_inner<'b>(
        &'b self,
        mut id: &'b gix_hash::oid,
        inflate: &mut gix_zlib::Inflate,
        snapshot: &mut load_index::Snapshot,
        recursion: Option<DeltaBaseRecursion<'_>>,
    ) -> Result<Option<Header>> {
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
                        let res = pack.decode_header(entry, inflate, &|id| {
                            index_file
                                .pack_offset_by_id(id)?
                                .map(|pack_offset| {
                                    pack.entry(pack_offset)
                                        .map(gix_pack::data::decode::header::ResolvedBase::InPack)
                                })
                                .transpose()
                        });
                        let res = match res {
                            Ok(header) => Ok(header.into()),
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
                                let context = || delta_base_lookup_error(&base_id, id);
                                let hdr = self
                                    .try_header_inner(
                                        &base_id,
                                        inflate,
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
                                    })?;
                                // A refreshed snapshot may reuse pack ids; retry through normal pack loading.
                                if snapshot.marker.generation != marker.generation
                                    || snapshot.marker.state_id != marker.state_id
                                {
                                    continue 'outer;
                                }
                                // Base lookup can reorder indices; keep the original pack and delta representation.
                                idx = match snapshot.indices.iter().position(|index| index.id == pack_id.index) {
                                    Some(idx) => idx,
                                    None => continue 'outer,
                                };
                                let Some(handle::index_lookup::Outcome {
                                    index_file,
                                    pack: Some(pack),
                                    ..
                                }) = snapshot.indices[idx].lookup(id)?
                                else {
                                    continue 'outer;
                                };
                                let entry = pack.entry(pack_offset)?;
                                pack.decode_header(entry, inflate, &|id| match index_file.pack_offset_by_id(id)? {
                                    Some(pack_offset) => pack
                                        .entry(pack_offset)
                                        .map(gix_pack::data::decode::header::ResolvedBase::InPack)
                                        .map(Some),
                                    None => Ok((id == base_id).then(|| {
                                        gix_pack::data::decode::header::ResolvedBase::OutOfPack {
                                            kind: hdr.kind(),
                                            num_deltas: hdr.num_deltas(),
                                        }
                                    })),
                                })
                                .map(Into::into)
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
                    return lodb.try_header(id).map(|opt| opt.map(Into::into));
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
    fn try_header(&self, id: &oid) -> Result<Option<Header>> {
        let mut snapshot = self.snapshot.borrow_mut();
        let mut inflate = self.inflate.borrow_mut();
        self.try_header_inner(id, &mut inflate, &mut snapshot, None)
    }
}
