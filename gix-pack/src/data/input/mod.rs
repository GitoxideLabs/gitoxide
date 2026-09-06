/// An item of the iteration produced by [`BytesToEntriesIter`]
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Entry {
    /// The header of a pack entry
    pub header: crate::data::entry::Header,
    /// The amount of bytes used to encode the `header`. `pack_offset + header_size` is the beginning of
    /// the compressed data in the pack.
    pub header_size: u16,
    /// The first byte of the entry at which the `header` can be read.
    pub pack_offset: u64,
    /// The bytes consumed while producing `decompressed`
    /// These do not contain the header, which makes it possible to easily replace a RefDelta with offset deltas
    /// when resolving thin packs.
    /// Depends on `CompressionMode` when the iterator is initialized.
    pub compressed: Option<Vec<u8>>,
    /// The amount of bytes the compressed portion of the entry takes, i.e. the portion behind the header.
    pub compressed_size: u64,
    /// The CRC32 over the complete entry, that is encoded header and compressed object data.
    /// Depends on `CompressionMode` when the iterator is initialized
    pub crc32: Option<u32>,
    /// The amount of decompressed bytes of the entry.
    pub decompressed_size: u64,
    /// Set for the last object in the iteration, providing the hash over all bytes of the iteration
    /// for use as trailer in a pack or to verify it matches the trailer.
    pub trailer: Option<gix_hash::ObjectId>,
}

mod entry;

mod types;
pub use types::{EntryDataMode, Mode};

mod bytes_to_entries;
pub use bytes_to_entries::BytesToEntriesIter;

mod lookup_ref_delta_objects;
pub use lookup_ref_delta_objects::LookupRefDeltaObjectsIter;

mod entries_to_bytes;
pub use entries_to_bytes::EntriesToBytesIter;
