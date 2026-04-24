//! a pack data file
use crate::MMap;
use std::path::Path;

/// The offset to an entry into the pack data file, relative to its beginning.
pub type Offset = u64;

/// An identifier to uniquely identify all packs loaded within a known context or namespace.
pub type Id = u32;

/// An representing an full- or delta-object within a pack
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Entry {
    /// The entry's header
    pub header: entry::Header,
    /// The decompressed size of the entry in bytes.
    ///
    /// Note that for non-delta entries this will be the size of the object itself.
    pub decompressed_size: u64,
    /// absolute offset to compressed object data in the pack, just behind the entry's header
    pub data_offset: Offset,
}

mod file;
pub use file::{decode, verify, Header};
///
pub mod header;

///
pub mod init {
    pub use super::header::decode::Error;
}

///
pub mod entry;

///
#[cfg(feature = "streaming-input")]
pub mod input;

/// Utilities to encode pack data entries and write them to a `Write` implementation to resemble a pack data file.
#[cfg(feature = "generate")]
pub mod output;

/// A slice into a pack file denoting a pack entry.
///
/// An entry can be decoded into an object.
pub type EntryRange = std::ops::Range<Offset>;

/// Supported versions of a pack data file
#[derive(Default, PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum Version {
    #[default]
    V2,
    V3,
}

/// A pack data file
pub struct File<T = MMap> {
    data: T,
    path: std::path::PathBuf,
    /// A value to represent this pack uniquely when used with cache lookup, or a way to identify this pack by its location on disk.
    /// The same location on disk should yield the same id.
    ///
    /// These must be unique per pack and must be stable, that is they don't change if the pack doesn't change.
    /// If the same id is assigned (or reassigned) to different packs, pack creation or cache access will fail in hard-to-debug ways.
    ///
    /// This value is controlled by the owning object store, which can use it in whichever way it wants as long as the above constraints are met.
    pub id: Id,
    version: Version,
    num_objects: u32,
    /// The size of the hash contained within. This is entirely determined by the caller, and repositories have to know which hash to use
    /// based on their configuration.
    hash_len: usize,
    object_hash: gix_hash::Kind,
    /// The maximum size of a single allocation caused by user-controlled on-disk pack data.
    ///
    /// If `None`, no additional limit is enforced.
    alloc_limit_bytes: Option<usize>,
}

/// Information about the pack data file itself
impl<T> File<T>
where
    T: crate::FileData,
{
    /// The pack data version of this file
    pub fn version(&self) -> Version {
        self.version
    }
    /// The number of objects stored in this pack data file
    pub fn num_objects(&self) -> u32 {
        self.num_objects
    }
    /// The length of all mapped data, including the pack header and the pack trailer
    pub fn data_len(&self) -> usize {
        self.data.len()
    }
    /// The kind of hash we use internally.
    pub fn object_hash(&self) -> gix_hash::Kind {
        self.object_hash
    }
    /// The maximum size of a single allocation caused by user-controlled on-disk pack data.
    ///
    /// A value of `None` means no additional limit is enforced.
    pub fn alloc_limit_bytes(&self) -> Option<usize> {
        self.alloc_limit_bytes
    }
    /// The position of the byte one past the last pack entry, or in other terms, the first byte of the trailing hash.
    pub fn pack_end(&self) -> usize {
        self.data.len() - self.hash_len
    }

    /// The path to the pack data file on disk
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the pack data at the given slice if its range is contained in the mapped pack data
    pub fn entry_slice(&self, slice: EntryRange) -> Option<&[u8]> {
        let entry_end: usize = slice.end.try_into().expect("end of pack fits into usize");
        let entry_start = slice.start as usize;
        self.data.get(entry_start..entry_end)
    }

    /// Returns the CRC32 of the pack data indicated by `pack_offset` and the `size` of the mapped data.
    ///
    /// _Note:_ finding the right size is only possible by decompressing
    /// the pack entry beforehand, or by using the (to be sorted) offsets stored in an index file.
    ///
    /// # Panics
    ///
    /// If `pack_offset` or `size` are pointing to a range outside of the mapped pack data.
    pub fn entry_crc32(&self, pack_offset: Offset, size: usize) -> u32 {
        let pack_offset: usize = pack_offset.try_into().expect("pack_size fits into usize");
        gix_features::hash::crc32(&self.data[pack_offset..pack_offset + size])
    }
}

///
pub mod delta;
