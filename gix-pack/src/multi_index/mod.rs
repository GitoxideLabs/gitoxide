use std::path::PathBuf;

use crate::MMap;

/// Known multi-index file versions
#[derive(Default, PartialEq, Eq, Ord, PartialOrd, Debug, Hash, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[expect(missing_docs)]
pub enum Version {
    #[default]
    V1 = 1,
}

/// An index into our [`File::index_names()`] array yielding the name of the index and by implication, its pack file.
pub type PackIndex = u32;

/// The type for referring to indices of an entry within the index file.
pub type EntryIndex = u32;

/// Represents an entry within a multi index file, effectively mapping object [`IDs`][gix_hash::ObjectId] to pack data
/// files and the offset within.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Entry {
    /// The ID of the object.
    pub oid: gix_hash::ObjectId,
    /// The offset to the object's header in the pack data file.
    pub pack_offset: crate::data::Offset,
    /// The index of the pack matching our [`File::index_names()`] slice.
    pub pack_index: PackIndex,
}

/// A representation of an index file for multiple packs at the same time, typically stored in a file
/// named 'multi-pack-index'.
pub struct File<T = MMap> {
    data: T,
    path: std::path::PathBuf,
    version: Version,
    hash_len: usize,
    object_hash: gix_hash::Kind,
    /// The amount of pack files contained within
    num_indices: u32,
    num_objects: u32,
    /// If `Some(limit)`, the maximum size of a single allocation caused by user-controlled on-disk pack data.
    alloc_limit_bytes: Option<usize>,

    fan: [u32; 256],
    index_names: Vec<PathBuf>,
    lookup_ofs: usize,
    offsets_ofs: usize,
    large_offsets: Option<std::ops::Range<usize>>,
}

///
pub mod write;
pub use write::function::write_from_index_paths;

///
mod access;

///
pub mod verify;

///
pub mod chunk;

///
pub mod init;
