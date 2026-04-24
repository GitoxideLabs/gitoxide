//! An object database storing each object in a zlib compressed file with its hash in the path
/// The maximum size that an object header can have. `git2` says 64, and `git` says 32 but also mentions it can be larger.
const HEADER_MAX_SIZE: usize = 64;
use std::path::{Path, PathBuf};

use gix_features::fs;

/// A database for reading and writing objects to disk, one file per object.
#[derive(Clone, PartialEq, Eq)]
pub struct Store {
    /// The directory in which objects are stored, containing 256 folders representing the hashes first byte.
    pub(crate) path: PathBuf,
    /// The kind of hash we should assume during iteration and when writing new objects.
    pub(crate) object_hash: gix_hash::Kind,
    /// The maximum size of a single allocation caused by user-controlled loose object data.
    pub(crate) alloc_limit_bytes: Option<usize>,
}

/// Initialization
impl Store {
    /// Initialize the Db with the `objects_directory` containing the hexadecimal first byte subdirectories, which in turn
    /// contain all loose objects.
    ///
    /// In a git repository, this would be `.git/objects`.
    ///
    /// The `object_hash` determines which hash to use when writing, finding or iterating objects.
    ///
    /// `alloc_limit_bytes` limits allocations caused by loose object bodies declared on disk, useful for untrusted input.
    /// Use `None` to disable the limit.
    pub fn at(
        objects_directory: impl Into<PathBuf>,
        object_hash: gix_hash::Kind,
        alloc_limit_bytes: Option<usize>,
    ) -> Store {
        Store {
            path: objects_directory.into(),
            object_hash,
            alloc_limit_bytes,
        }
    }

    /// Return the path to our `objects` directory.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Return the kind of hash we would iterate and write.
    pub fn object_hash(&self) -> gix_hash::Kind {
        self.object_hash
    }
}

fn hash_path(id: &gix_hash::oid, mut root: PathBuf) -> PathBuf {
    let mut hex = gix_hash::Kind::hex_buf();
    let hex = id.hex_to_buf(hex.as_mut());
    root.push(&hex[..2]);
    root.push(&hex[2..]);
    root
}

///
pub mod find;
///
pub mod iter;
///
pub mod verify;

/// The type for an iterator over `Result<gix_hash::ObjectId, Error>)`
pub struct Iter {
    inner: fs::walkdir::DirEntryIter,
    hash_hex_len: usize,
}

///
pub mod write;
