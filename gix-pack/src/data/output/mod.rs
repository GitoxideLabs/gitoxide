use gix_hash::ObjectId;

///
pub mod count;

/// An item representing a future Entry in the leanest way possible.
///
/// One can expect to have one of these in memory when building big objects, so smaller is better here.
/// They should contain everything of importance to generate a pack as fast as possible.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Count {
    /// The hash of the object to write
    pub id: ObjectId,
    /// A way to locate a pack entry in the object database, only available if the object is in a pack.
    pub entry_pack_location: count::PackLocation,
}

/// An entry to be written to a file.
///
/// Some of these will be in-flight and in memory while waiting to be written. Memory requirements depend on the amount of compressed
/// data they hold.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Entry {
    /// The hash of the object to write, or null for an [`invalid`](Self::is_invalid) entry.
    /// Invalid entries preserve input positions but are skipped by the pack writer.
    pub id: ObjectId,
    /// The kind of entry represented by `data`. It's used alongside with it to complete the pack entry
    /// at rest or in transit.
    pub kind: entry::Kind,
    /// The size in bytes needed once `data` gets decompressed
    pub decompressed_size: usize,
    /// The compressed data right behind the header
    pub compressed_data: Vec<u8>,
}

///
pub mod entry;

///
pub mod bytes;
