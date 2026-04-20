use std::{io::Seek, ops::Deref};

use gix_features::zlib;
use gix_hash::oid;
use gix_pack::cache::DecodeEntry;

use super::find::Error;
use crate::store::{find::error::DeltaBaseRecursion, handle, load_index};

impl<S> super::Handle<S>
where
    S: Deref<Target = super::Store> + Clone,
{
    pub(crate) fn try_find_stream_inner<'b>(
        &'b self,
        mut id: &'b gix_hash::oid,
        inflate: &mut zlib::Inflate,
        pack_cache: &mut dyn DecodeEntry,
        snapshot: &mut load_index::Snapshot,
        recursion: Option<DeltaBaseRecursion<'_>>,
    ) -> Result<Option<(crate::find::Stream, Option<gix_pack::data::entry::Location>)>, Error> {
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
                                None => match self.store.load_one_index(self.refresh, snapshot.marker)? {
                                    Some(new_snapshot) => {
                                        *snapshot = new_snapshot;
                                        self.clear_cache();
                                        continue 'outer;
                                    }
                                    None => return Ok(None),
                                },
                            },
                        };
                        let resolved_pack_id = pack.id;
                        let entry = pack.entry(pack_offset)?;
                        let header_size = entry.header_size();
                        let result = {
                            let mut scratch = Vec::new();
                            let mut temp = tempfile::tempfile()?;
                            let result = match pack.decode_entry_to_write(
                                entry,
                                &mut scratch,
                                inflate,
                                &mut temp,
                                &|id, _out| {
                                    index_file.pack_offset_by_id(id).and_then(|pack_offset| {
                                        pack.entry(pack_offset)
                                            .ok()
                                            .map(gix_pack::data::decode::entry::ResolvedBase::InPack)
                                    })
                                },
                                pack_cache,
                            ) {
                                Ok(outcome) => Ok((outcome, temp)),
                                Err(gix_pack::data::decode::Error::DeltaBaseUnresolved(base_id)) => {
                                    let mut buf = Vec::new();
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
                                                panic!(
                                                    "could not find object {id} in any index after looking up one of its base objects {base_id}"
                                                )
                                            })
                                        }
                                    };
                                    let pack = possibly_pack
                                        .as_ref()
                                        .expect("pack to still be available like just now");
                                    let entry = pack.entry(pack_offset)?;
                                    let mut scratch = Vec::new();
                                    let mut temp = tempfile::tempfile()?;
                                    pack.decode_entry_to_write(
                                        entry,
                                        &mut scratch,
                                        inflate,
                                        &mut temp,
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
                                    .map(|outcome| (outcome, temp))
                                }
                                Err(err) => Err(err),
                            }?;
                            result
                        };
                        let (outcome, mut temp) = result;
                        temp.rewind()?;
                        let res = (
                            crate::find::Stream::from_file(outcome.kind, outcome.object_size, temp),
                            Some(gix_pack::data::entry::Location {
                                pack_id: resolved_pack_id,
                                pack_offset,
                                entry_size: outcome.compressed_size + header_size,
                            }),
                        );

                        if idx != 0 {
                            snapshot.indices.swap(0, idx);
                        }
                        return Ok(Some(res));
                    }
                }
            }

            for lodb in snapshot.loose_dbs.iter() {
                if lodb.contains(id) {
                    return lodb
                        .try_find_stream(id)
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
}

impl<S> super::Handle<S>
where
    S: Deref<Target = super::Store> + Clone,
{
    /// Try to find the object identified by `id` in any backing store and return it as a readable stream,
    /// along with its pack location if it came from a pack.
    pub fn try_find_stream(
        &self,
        id: &oid,
        pack_cache: &mut dyn DecodeEntry,
    ) -> Result<Option<(crate::find::Stream, Option<gix_pack::data::entry::Location>)>, gix_object::find::Error> {
        let mut snapshot = self.snapshot.borrow_mut();
        let mut inflate = self.inflate.borrow_mut();
        self.try_find_stream_inner(id, &mut inflate, pack_cache, &mut snapshot, None)
            .map_err(|err| Box::new(err) as _)
    }
}
