use std::{
    io::Write,
    sync::atomic::{AtomicBool, Ordering},
};

use gix_object::FindExt;

use crate::{
    ObjectId, Repository, Result,
    error::{ResultExt, bail, message},
};

/// Information about a complete pack written by [`Repository::write_pack()`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// The checksum identifying the pack, also written as its trailer.
    pub pack_hash: ObjectId,
    /// The number of distinct objects in the pack.
    pub num_objects: u32,
    /// The number of bytes written, including the header and trailer.
    pub bytes_written: u64,
}

impl Repository {
    /// Write a self-contained version-2 pack containing the given `object_ids` to `output`.
    ///
    /// Duplicate IDs are removed, preserving their first occurrence. Each object's checksum is
    /// verified before compression, and replacement refs are ignored. Objects are compressed using `pack.compression`,
    /// falling back to `core.compression`; no deltas or external bases are used. This uses memory
    /// proportional to the number of object IDs and the largest object, without buffering the pack.
    ///
    /// This does not traverse object relationships or install a pack or index in the repository.
    /// For reachable objects, obtain IDs from [`gix_traverse::object::BreadthFirst`]. The caller
    /// is responsible for retaining or removing `output`; failures can leave a partial pack.
    /// Cancellation is checked while collecting IDs and between objects.
    pub fn write_pack(
        &self,
        object_ids: impl IntoIterator<Item = ObjectId>,
        output: impl Write,
        should_interrupt: &AtomicBool,
    ) -> Result<Outcome> {
        let object_hash = self.object_hash();
        let compression = self.pack_compression()?;
        let mut seen = gix_hashtable::HashSet::default();
        let mut unique = Vec::new();
        check_interrupt(should_interrupt)?;
        for object_id in object_ids {
            check_interrupt(should_interrupt)?;
            if object_id.kind() != object_hash || object_id.is_null() {
                bail!(message("Pack object IDs must be non-null and use the repository's object format").validation());
            }
            if seen.insert(object_id) {
                unique.push(object_id);
            }
        }
        let num_objects =
            u32::try_from(unique.len()).or_raise(|| message("A pack cannot contain more than 2^32 - 1 objects"))?;
        let mut objects = self.objects.clone();
        objects.ignore_replacements = true;
        let mut buffer = Vec::new();
        let entries = unique.into_iter().map(|object_id| {
            check_interrupt(should_interrupt)?;
            let data = objects
                .find(&object_id, &mut buffer)
                .or_raise(|| message!("Could not read object {object_id} for packing"))?;
            data.verify_checksum(&object_id)?;
            Ok(vec![gix_pack::data::output::Entry::from_data(
                &gix_pack::data::output::Count::from_data(object_id, None),
                &data,
                compression,
            )?])
        });
        let mut pack = gix_pack::data::output::bytes::FromEntriesIter::new(
            entries,
            output,
            num_objects,
            gix_pack::data::Version::V2,
            object_hash,
        );
        let mut bytes_written = 0;
        for written in pack.by_ref() {
            bytes_written += written?;
        }
        Ok(Outcome {
            pack_hash: pack.digest().expect("a completed pack has a checksum"),
            num_objects,
            bytes_written,
        })
    }
}

fn check_interrupt(should_interrupt: &AtomicBool) -> Result<()> {
    if should_interrupt.load(Ordering::Relaxed) {
        bail!(message("Pack generation interrupted").cancelled());
    }
    Ok(())
}
