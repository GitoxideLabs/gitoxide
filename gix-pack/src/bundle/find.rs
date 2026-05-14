use gix_features::zlib;

impl crate::Bundle {
    /// Find an object with the given [`ObjectId`](gix_hash::ObjectId) and place its data into `out`.
    /// `inflate` is used to decompress objects, and will be reset before first use, but not after the last use.
    ///
    /// [`cache`](crate::cache::DecodeEntry) is used to accelerate the lookup.
    ///
    /// **Note** that ref deltas are automatically resolved within this pack only, which makes this implementation unusable
    /// for thin packs, which by now are expected to be resolved already.
    pub fn find<'a>(
        &self,
        id: &gix_hash::oid,
        out: &'a mut Vec<u8>,
        inflate: &mut zlib::Inflate,
        cache: &mut dyn crate::cache::DecodeEntry,
    ) -> Result<Option<(gix_object::Data<'a>, crate::data::entry::Location)>, crate::data::decode::Error> {
        let idx = match self.index.lookup(id) {
            Some(idx) => idx,
            None => return Ok(None),
        };
        self.get_object_by_index(idx, out, inflate, cache).map(Some)
    }

    /// Special-use function to get an object given an index previously returned from
    /// [index::File::](crate::index::File::lookup()).
    /// `inflate` is used to decompress objects, and will be reset before first use, but not after the last use.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    pub fn get_object_by_index<'a>(
        &self,
        idx: u32,
        out: &'a mut Vec<u8>,
        inflate: &mut zlib::Inflate,
        cache: &mut dyn crate::cache::DecodeEntry,
    ) -> Result<(gix_object::Data<'a>, crate::data::entry::Location), crate::data::decode::Error> {
        let ofs = self.index.pack_offset_at_index(idx);
        let pack_entry = self.pack.entry(ofs)?;
        let header_size = pack_entry.header_size();
        self.pack
            .decode_entry(
                pack_entry,
                out,
                inflate,
                &|id, _out| {
                    let idx = self.index.lookup(id)?;
                    self.pack
                        .entry(self.index.pack_offset_at_index(idx))
                        .ok()
                        .map(crate::data::decode::entry::ResolvedBase::InPack)
                },
                cache,
            )
            .map(move |r| {
                (
                    gix_object::Data {
                        kind: r.kind,
                        data: out.as_slice(),
                        object_hash: self.pack.object_hash(),
                    },
                    crate::data::entry::Location {
                        pack_id: self.pack.id,
                        pack_offset: ofs,
                        entry_size: r.compressed_size + header_size,
                    },
                )
            })
    }
}
