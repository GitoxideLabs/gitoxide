//! an index into the pack file

use crate::MMap;

/// The version of an index file
#[derive(Default, PartialEq, Eq, Ord, PartialOrd, Debug, Hash, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[expect(missing_docs)]
pub enum Version {
    V1 = 1,
    #[default]
    V2 = 2,
}

/// A way to indicate if a lookup, despite successful, was ambiguous or yielded exactly
/// one result in the particular index.
pub type PrefixLookupResult = Result<EntryIndex, ()>;

/// The type for referring to indices of an entry within the index file.
pub type EntryIndex = u32;

const FAN_LEN: usize = 256;

/// A representation of a pack index file
pub struct File<T = MMap> {
    data: T,
    path: std::path::PathBuf,
    version: Version,
    num_objects: u32,
    fan: [u32; FAN_LEN],
    hash_len: usize,
    object_hash: gix_hash::Kind,
}

/// Basic file information
impl<T> File<T>
where
    T: crate::FileData,
{
    /// The version of the pack index
    pub fn version(&self) -> Version {
        self.version
    }
    /// The path of the opened index file
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
    /// The amount of objects stored in the pack and index, as one past the highest entry index.
    pub fn num_objects(&self) -> EntryIndex {
        self.num_objects
    }
    /// The kind of hash we assume
    pub fn object_hash(&self) -> gix_hash::Kind {
        self.object_hash
    }
}

const V2_SIGNATURE: &[u8] = b"\xfftOc";
///
pub mod init;

pub(crate) mod access;
pub use access::Entry;

pub(crate) mod encode;
///
pub mod traverse;
mod util;
///
pub mod verify;
///
#[cfg(feature = "streaming-input")]
pub mod write;
#[cfg(feature = "streaming-input")]
pub use write::function::write_data_iter_to_stream;
