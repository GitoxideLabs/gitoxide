use std::{
    io::Write,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    ObjectId, Repository, Result,
    error::{ResultExt, bail, cancelled, ensure, message, not_found, validation},
};
use gix_pack::{FindExt, data::output};
use gix_utils::progress::{Count as _, Progress as _};

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
    /// Write a self-contained version-2 pack containing exactly `object_ids` to `out`.
    ///
    /// Duplicate IDs are included once. Referenced objects are not added implicitly: callers must select
    /// the complete object set they need. Replacement refs are ignored so each packed object's bytes
    /// retain the requested identity. Each object's checksum is verified before packing. In-memory
    /// objects and the implicit empty tree are included without persisting them.
    /// Existing compressed objects and deltas are reused where possible; deltas whose bases aren't
    /// selected are recompressed as complete objects. `pack.compression` controls recompression.
    ///
    /// Invalid IDs, missing or corrupt objects, and cancellation detected while counting produce no output.
    /// Later read, write, or cancellation failures may leave partial output; callers should use a
    /// temporary file when publishing a pack. No index or repository files are written.
    pub fn write_pack<P>(
        &self,
        object_ids: impl IntoIterator<Item = ObjectId>,
        out: impl Write,
        mut progress: P,
        should_interrupt: &AtomicBool,
    ) -> Result<Outcome>
    where
        P: gix_utils::progress::NestedProgress,
        P::SubProgress: 'static,
    {
        let check_interrupt = || -> Result<()> {
            if should_interrupt.load(Ordering::Relaxed) {
                bail!(cancelled("Pack creation interrupted"));
            }
            Ok(())
        };
        check_interrupt()?;
        let mut repo = self.clone();
        repo.make_paths_absolute()?;
        let mut objects = repo
            .objects
            .into_arc()
            .or_raise(|| message("Could not prepare the pack object database"))?;
        objects.ignore_replacements = true;
        objects.prevent_pack_unload();
        let mut seen = gix_hashtable::HashSet::default();
        let mut counts = Vec::new();
        let mut buffer = Vec::new();
        let mut counting = progress.add_child("counting pack objects");
        counting.init(None, gix_utils::progress::count("objects"));
        for id in object_ids {
            check_interrupt()?;
            ensure!(
                id.kind() == self.object_hash() && !id.is_null(),
                validation("Pack object IDs must be non-null and use the repository's hash")
            );
            if !seen.insert(id) {
                continue;
            }
            if id.is_empty_tree() {
                // Repository lookup synthesizes this tree; give the pack writer the same view.
                let mut memory = objects.take_object_memory().unwrap_or_default();
                memory.insert(id, (gix_object::Kind::Tree, Vec::new()));
                objects.set_object_memory(memory);
            }
            let (data, location) = objects
                .find(&id, &mut buffer)
                .or_raise(|| message!("Could not read object {id} for packing"))?;
            data.verify_checksum(&id)?;
            counts.push(output::Count::from_data(id, location));
            counting.inc();
        }
        let num_objects = u32::try_from(counts.len()).or_raise(|| validation("Pack has more than 2^32 - 1 objects"))?;
        let entries = output::entry::iter_from_counts(
            counts,
            objects,
            Box::new(progress.add_child("preparing pack entries")),
            output::entry::iter_from_counts::Options {
                compression: self.pack_compression()?,
                ..Default::default()
            },
        )?;
        let entries = gix_parallel::InOrderIter::from(entries).map(|entries| {
            let entries = entries?;
            ensure!(
                entries.iter().all(|entry| !entry.is_invalid()),
                not_found("An object disappeared while creating the pack")
            );
            Ok(entries)
        });
        let mut bytes =
            output::bytes::FromEntriesIter::new(entries, out, num_objects, Default::default(), self.object_hash());
        let mut written = progress.add_child("writing pack");
        written.init(None, gix_utils::progress::bytes());
        let mut bytes_written = 0;
        loop {
            check_interrupt()?;
            let Some(count) = bytes.next() else { break };
            let count = count?;
            bytes_written += count;
            written.inc_by(count as usize);
        }
        Ok(Outcome {
            pack_hash: bytes.digest().expect("successful iteration wrote the pack trailer"),
            num_objects,
            bytes_written,
        })
    }
}
