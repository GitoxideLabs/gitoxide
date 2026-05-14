use gix_hash::ObjectId;

use crate::data::{entry::Header, input};

/// An iterator to resolve thin packs on the fly.
pub struct LookupRefDeltaObjectsIter<I, Find> {
    /// The inner iterator whose entries we will resolve.
    pub inner: I,
    lookup: Find,
    /// The cached delta to provide next time we are called, it's the delta to go with the base we just resolved in its place.
    next_delta: Option<input::Entry>,
    /// Fuse to stop iteration after first missing object.
    error: bool,
    /// The overall pack-offset we accumulated thus far. Each inserted entry offsets all following
    /// objects by its length. We need to determine exactly where the object was inserted to see if its affected at all.
    inserted_entry_length_at_offset: Vec<Change>,
    /// The sum of all entries added so far, as a cache to avoid recomputation
    inserted_entries_length_in_bytes: i64,
    buf: Vec<u8>,
}

impl<I, Find> LookupRefDeltaObjectsIter<I, Find>
where
    I: Iterator<Item = Result<input::Entry, input::Error>>,
    Find: gix_object::Find,
{
    /// Create a new instance wrapping `iter` and using `lookup` as function to retrieve objects that will serve as bases
    /// for ref deltas seen while traversing `iter`.
    pub fn new(iter: I, lookup: Find) -> Self {
        LookupRefDeltaObjectsIter {
            inner: iter,
            lookup,
            error: false,
            inserted_entry_length_at_offset: Vec::new(),
            inserted_entries_length_in_bytes: 0,
            next_delta: None,
            buf: Vec::new(),
        }
    }

    fn shifted_pack_offset(&self, pack_offset: u64) -> u64 {
        let new_ofs = pack_offset as i64 + self.inserted_entries_length_in_bytes;
        new_ofs.try_into().expect("offset value is never becomes negative")
    }

    /// positive `size_change` values mean an object grew or was more commonly, was inserted. Negative values
    /// mean the object shrunk, usually because there header changed from ref-deltas to ofs deltas.
    fn track_change(&mut self, shifted_pack_offset: u64, pack_offset: u64, size_change: i64, oid: Option<ObjectId>) {
        if size_change == 0 {
            return;
        }
        self.inserted_entry_length_at_offset.push(Change {
            shifted_pack_offset,
            pack_offset,
            size_change_in_bytes: size_change,
            oid: oid.unwrap_or_else(||
                // NOTE: this value acts as sentinel and the actual hash kind doesn't matter.
                gix_hash::Kind::shortest().null()),
        });
        self.inserted_entries_length_in_bytes += size_change;
    }

    fn shift_entry_and_point_to_base_by_offset(&mut self, entry: &mut input::Entry, base_distance: u64) {
        let pack_offset = entry.pack_offset;
        entry.pack_offset = self.shifted_pack_offset(pack_offset);
        entry.header = Header::OfsDelta { base_distance };
        let previous_header_size = entry.header_size;
        entry.header_size = entry.header.size(entry.decompressed_size) as u16;

        let change = i64::from(entry.header_size) - i64::from(previous_header_size);
        entry.crc32 = Some(entry.compute_crc32());
        self.track_change(entry.pack_offset, pack_offset, change, None);
    }
}

impl<I, Find> Iterator for LookupRefDeltaObjectsIter<I, Find>
where
    I: Iterator<Item = Result<input::Entry, input::Error>>,
    Find: gix_object::Find,
{
    type Item = Result<input::Entry, input::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.error {
            return None;
        }
        if let Some(delta) = self.next_delta.take() {
            return Some(Ok(delta));
        }
        match self.inner.next() {
            Some(Ok(mut entry)) => match entry.header {
                Header::RefDelta { base_id } => {
                    match self.inserted_entry_length_at_offset.iter().rfind(|e| e.oid == base_id) {
                        None => {
                            let base_entry = match self.lookup.try_find(&base_id, &mut self.buf).ok()? {
                                Some(obj) => {
                                    let current_pack_offset = entry.pack_offset;
                                    let mut entry = match input::Entry::from_data_obj(&obj, 0) {
                                        Ok(e) => e,
                                        Err(err) => return Some(Err(err)),
                                    };
                                    entry.pack_offset = self.shifted_pack_offset(current_pack_offset);
                                    self.track_change(
                                        entry.pack_offset,
                                        current_pack_offset,
                                        entry.bytes_in_pack() as i64,
                                        Some(base_id),
                                    );
                                    entry
                                }
                                None => {
                                    self.error = true;
                                    return Some(Err(input::Error::NotFound { object_id: base_id }));
                                }
                            };

                            {
                                self.shift_entry_and_point_to_base_by_offset(&mut entry, base_entry.bytes_in_pack());
                                self.next_delta = Some(entry);
                            }
                            Some(Ok(base_entry))
                        }
                        Some(base_entry) => {
                            let base_distance =
                                self.shifted_pack_offset(entry.pack_offset) - base_entry.shifted_pack_offset;
                            self.shift_entry_and_point_to_base_by_offset(&mut entry, base_distance);
                            Some(Ok(entry))
                        }
                    }
                }
                _ => {
                    if self.inserted_entries_length_in_bytes != 0 {
                        if let Header::OfsDelta { base_distance } = entry.header {
                            // We have to find the new distance based on the previous distance to the base, using the absolute
                            // pack offset computed from it as stored in `base_pack_offset`.
                            let base_pack_offset = entry
                                .pack_offset
                                .checked_sub(base_distance)
                                .expect("distance to be in range of pack");
                            match self
                                .inserted_entry_length_at_offset
                                .binary_search_by_key(&base_pack_offset, |c| c.pack_offset)
                            {
                                Ok(index) => {
                                    let index = {
                                        let maybe_index_of_actual_entry = index + 1;
                                        self.inserted_entry_length_at_offset
                                            .get(maybe_index_of_actual_entry)
                                            .and_then(|c| {
                                                (c.pack_offset == base_pack_offset)
                                                    .then_some(maybe_index_of_actual_entry)
                                            })
                                            .unwrap_or(index)
                                    };
                                    let new_distance = self
                                        .shifted_pack_offset(entry.pack_offset)
                                        .checked_sub(self.inserted_entry_length_at_offset[index].shifted_pack_offset)
                                        .expect("a base that is behind us in the pack");
                                    self.shift_entry_and_point_to_base_by_offset(&mut entry, new_distance);
                                }
                                Err(index) => {
                                    let change_since_offset = self.inserted_entry_length_at_offset[index..]
                                        .iter()
                                        .map(|c| c.size_change_in_bytes)
                                        .sum::<i64>();
                                    let new_distance: u64 = {
                                        (base_distance as i64 + change_since_offset)
                                            .try_into()
                                            .expect("it still points behind us")
                                    };
                                    self.shift_entry_and_point_to_base_by_offset(&mut entry, new_distance);
                                }
                            }
                        } else {
                            // Offset this entry by all changes (positive or negative) that we saw thus far.
                            entry.pack_offset = self.shifted_pack_offset(entry.pack_offset);
                        }
                    }
                    Some(Ok(entry))
                }
            },
            other => other,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (min, max) = self.inner.size_hint();
        max.map_or_else(|| (min * 2, None), |max| (min, Some(max * 2)))
    }
}

#[derive(Debug)]
struct Change {
    /// The original pack offset as mentioned in the entry we saw. This is used to find this as base object if deltas refer to it by
    /// old offset.
    pack_offset: u64,
    /// The new pack offset that is the shifted location of the pack entry in the pack.
    shifted_pack_offset: u64,
    /// The size change of the entry header, negative values denote shrinking, positive denote growing.
    size_change_in_bytes: i64,
    /// The object id of the entry responsible for the change, or null if it's an entry just for tracking an insertion.
    oid: ObjectId,
}
