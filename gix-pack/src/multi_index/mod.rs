use std::path::PathBuf;

use crate::MMap;

/// Known multi-index file versions
#[derive(Default, PartialEq, Eq, Ord, PartialOrd, Debug, Hash, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum Version {
    #[default]
    V1 = 1,
}

/// An index into our [`File::index_names()`] array yielding the name of the index and by implication, its pack file.
pub type PackIndex = u32;

/// The type for referring to indices of an entry within the index file.
pub type EntryIndex = u32;

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

    fan: [u32; 256],
    index_names: Vec<PathBuf>,
    lookup_ofs: usize,
    offsets_ofs: usize,
    large_offsets_ofs: Option<usize>,
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
