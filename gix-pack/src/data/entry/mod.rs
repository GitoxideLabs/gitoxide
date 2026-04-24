use crate::data::Entry;

const _TYPE_EXT1: u8 = 0;
const COMMIT: u8 = 1;
const TREE: u8 = 2;
const BLOB: u8 = 3;
const TAG: u8 = 4;
const _TYPE_EXT2: u8 = 5;
const OFS_DELTA: u8 = 6;
const REF_DELTA: u8 = 7;

/// A way to uniquely identify the location of an entry within a pack bundle
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Location {
    /// The id of the pack containing the object. It's unique within its frame of reference which is the owning object database.
    pub pack_id: u32,
    /// The size of the entry of disk so that the range of bytes of the entry is `pack_offset..pack_offset + entry_size`.
    pub entry_size: usize,
    /// The start of the entry in the pack identified by `pack_id`.
    pub pack_offset: data::Offset,
}

impl Location {
    /// Compute a range suitable for lookup in pack data using the [`entry_slice()`][crate::data::File::entry_slice()] method.
    pub fn entry_range(&self, pack_offset: data::Offset) -> crate::data::EntryRange {
        pack_offset..pack_offset + self.entry_size as u64
    }
}

/// Access
impl Entry {
    /// Compute the pack offset to the base entry of the object represented by this entry, or
    /// return `None` if the distance would underflow or is invalid.
    pub fn checked_base_pack_offset(&self, distance: u64) -> Option<data::Offset> {
        let pack_offset = self.data_offset - self.header_size() as u64;
        Header::verified_base_pack_offset(pack_offset, distance)
    }

    /// Compute the pack offset to the base entry of the object represented by this entry.
    ///
    /// # Panics
    ///
    /// Panics if the `distance` will cause an underflow or is invalid.
    pub fn base_pack_offset(&self, distance: u64) -> data::Offset {
        self.checked_base_pack_offset(distance)
            .expect("in-bound distance of deltas")
    }
    /// The pack offset at which this entry starts
    pub fn pack_offset(&self) -> data::Offset {
        self.data_offset - self.header_size() as u64
    }
    /// The amount of bytes used to describe this entry in the pack. The header starts at [`Self::pack_offset()`]
    pub fn header_size(&self) -> usize {
        self.header.size(self.decompressed_size)
    }
}

///
pub mod decode;

mod header;
pub use header::Header;

use crate::data;
