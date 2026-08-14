//! An object database storing each object in a zlib compressed file with its hash in the path
/// The maximum size that an object header can have. `git2` says 64, and `git` says 32 but also mentions it can be larger.
const HEADER_MAX_SIZE: usize = 64;
use std::path::{Path, PathBuf};

use gix_features::fs;

/// Options for use in [`Store::at_opts()`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    /// The maximum size of a single allocation caused by user-controlled loose object data.
    ///
    /// If `None`, no additional limit is enforced.
    pub alloc_limit_bytes: Option<usize>,
    /// The compression level to use when writing loose objects.
    ///
    /// Git uses [`Compression::BEST_SPEED`](gix_zlib::Compression::BEST_SPEED) unless configured otherwise with
    /// `core.looseCompression` or `core.compression`.
    pub compression: gix_zlib::Compression,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            alloc_limit_bytes: None,
            compression: gix_zlib::Compression::BEST_SPEED,
        }
    }
}

/// A database for reading and writing objects to disk, one file per object.
#[derive(Clone, PartialEq, Eq)]
pub struct Store {
    /// The directory in which objects are stored, containing 256 folders representing the hashes first byte.
    pub(crate) path: PathBuf,
    /// The kind of hash we should assume during iteration and when writing new objects.
    pub(crate) object_hash: gix_hash::Kind,
    /// The maximum size of a single allocation caused by user-controlled loose object data.
    pub(crate) alloc_limit_bytes: Option<usize>,
    /// The compression level to use when writing loose objects.
    pub(crate) compression: gix_zlib::Compression,
}

/// Initialization
impl Store {
    /// Initialize the object database with the `objects_directory` containing the hexadecimal
    /// first byte subdirectories, which in turn contain all loose objects.
    ///
    /// In a git repository, this would be `.git/objects`.
    ///
    /// Use `object_hash` to specify the db’s hash.
    pub fn at(objects_directory: impl Into<PathBuf>, object_hash: gix_hash::Kind) -> Store {
        Self::at_opts(objects_directory, object_hash, Default::default())
    }

    /// Initialize the object database with the `objects_directory` containing the hexadecimal
    /// first byte subdirectories, which in turn contain all loose objects.
    ///
    /// In a git repository, this would be `.git/objects`.
    ///
    /// Use `object_hash` to specify the db’s hash and `options` to configure allocation limits and
    /// compression level.
    pub fn at_opts(objects_directory: impl Into<PathBuf>, object_hash: gix_hash::Kind, options: Options) -> Store {
        let Options {
            alloc_limit_bytes,
            compression,
        } = options;
        Store {
            path: objects_directory.into(),
            object_hash,
            alloc_limit_bytes,
            compression,
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
