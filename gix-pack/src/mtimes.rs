//! Read and write Git's version-1 `.mtimes` sidecar files.
//!
//! Entries store epoch seconds in pack-index order. They preserve individual unreachable
//! objects' ages when those objects are repacked, so repacking does not restart their grace period.

use gix_error::{Result, ResultExt, bail, message};
use std::io::Write;

const HEADER: usize = 12;

/// A validated, borrowed modification-time table for a particular pack.
#[derive(Clone, Copy, Debug)]
pub struct Table<'a> {
    times: &'a [u8],
    pack_checksum: &'a gix_hash::oid,
}

impl<'a> Table<'a> {
    /// Decode `data` for the pack identified by `pack_checksum`, with `num_objects` index entries.
    ///
    /// Validate the exact length, format version, hash kind, pack identity, and sidecar checksum.
    /// This does not validate the corresponding pack or index; callers must do that separately.
    pub fn from_bytes(data: &'a [u8], num_objects: u32, pack_checksum: &gix_hash::oid) -> Result<Self> {
        let hash_len = pack_checksum.kind().len_in_bytes();
        let expected = usize::try_from(num_objects)
            .ok()
            .and_then(|count| count.checked_mul(4))
            .and_then(|bytes| bytes.checked_add(HEADER + 2 * hash_len));
        if expected != Some(data.len()) || data.get(..4) != Some(b"MTME") {
            bail!(message("Invalid cruft modification-time table size or signature").corrupted());
        }
        let version = crate::read_u32(&data[4..8]);
        if version != 1 {
            bail!(message!("Unsupported cruft modification-time version {version}").unsupported());
        }
        let hash_id = crate::read_u32(&data[8..12]);
        if hash_id != hash_identifier(pack_checksum.kind())? {
            bail!(message("Cruft modification-time table uses the wrong object format").corrupted());
        }
        let footer = data.len() - 2 * hash_len;
        let recorded_pack = gix_hash::oid::try_from_bytes(&data[footer..footer + hash_len])?;
        if recorded_pack != pack_checksum {
            bail!(message("Cruft modification-time table belongs to a different pack").corrupted());
        }
        let checksum_offset = data.len() - hash_len;
        let recorded_checksum = gix_hash::oid::try_from_bytes(&data[checksum_offset..])?;
        let mut hasher = gix_hash::hasher(pack_checksum.kind());
        hasher.update(&data[..checksum_offset]);
        hasher.try_finalize()?.verify(recorded_checksum)?;
        Ok(Self {
            times: &data[HEADER..footer],
            pack_checksum: recorded_pack,
        })
    }

    /// The checksum of the pack to which these times belong.
    pub fn pack_checksum(&self) -> &gix_hash::oid {
        self.pack_checksum
    }

    /// Return the epoch seconds for the entry at `index` in pack-index order, or `None` if out of bounds.
    pub fn get(&self, index: u32) -> Option<u32> {
        let start = usize::try_from(index).ok()?.checked_mul(4)?;
        self.times.get(start..start.checked_add(4)?).map(crate::read_u32)
    }

    /// Iterate over epoch seconds in pack-index order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = u32> + '_ {
        self.times
            .as_chunks::<4>()
            .0
            .iter()
            .map(|bytes| u32::from_be_bytes(*bytes))
    }
}

/// Write a version-1 table of epoch seconds in pack-index order and return its checksum.
///
/// `pack_checksum` identifies the associated pack and determines the object format.
/// The caller must supply exactly one entry per indexed object. Errors can leave partial output.
pub fn write(times: &[u32], pack_checksum: &gix_hash::oid, mut output: impl Write) -> Result<gix_hash::ObjectId> {
    u32::try_from(times.len()).or_raise(|| message("A cruft table cannot contain more than 2^32 - 1 entries"))?;
    let mut hasher = gix_hash::hasher(pack_checksum.kind());
    let mut write = |bytes: &[u8]| -> Result<()> {
        output
            .write_all(bytes)
            .or_raise(|| message("Could not write cruft modification times"))?;
        hasher.update(bytes);
        Ok(())
    };
    write(b"MTME")?;
    write(&1u32.to_be_bytes())?;
    write(&hash_identifier(pack_checksum.kind())?.to_be_bytes())?;
    for time in times {
        write(&time.to_be_bytes())?;
    }
    write(pack_checksum.as_bytes())?;
    let checksum = hasher.try_finalize()?;
    output
        .write_all(checksum.as_slice())
        .or_raise(|| message("Could not write cruft modification-time checksum"))?;
    Ok(checksum)
}

fn hash_identifier(kind: gix_hash::Kind) -> Result<u32> {
    match kind {
        #[cfg(feature = "sha1")]
        gix_hash::Kind::Sha1 => Ok(1),
        #[cfg(feature = "sha256")]
        gix_hash::Kind::Sha256 => Ok(2),
        _ => bail!(message!("Unsupported cruft table object format {kind}").unsupported()),
    }
}
