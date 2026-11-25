use std::path::{Path, PathBuf};

use gix_error::{Result, ResultExt, bail, message};

use crate::{cache, data};

/// Instantiation
impl data::File<crate::MMap> {
    /// Try opening a data file at the given `path`.
    ///
    /// The `object_hash` is a way to read (and write) the same file format with different hashes, as the hash kind
    /// isn't stored within the file format itself.
    ///
    /// Untrusted input may require substantial memory; use a separate process with OS resource limits
    /// when memory exhaustion must not terminate the application.
    pub fn at(path: impl AsRef<Path>, object_hash: gix_hash::Kind) -> Result<Self> {
        Self::at_inner(path.as_ref(), object_hash)
    }

    fn at_inner(path: &Path, object_hash: gix_hash::Kind) -> Result<Self> {
        let data = crate::mmap::read_only(path)
            .or_raise(|| message!("Could not open pack data file at \"{}\"", path.display()))?;
        Self::from_data(data, path.to_owned(), object_hash)
    }
}

impl<T> data::File<T>
where
    T: crate::FileData,
{
    /// Instantiate a data file from `data` as assumed to be read or memory-mapped from `path`.
    ///
    /// Untrusted input may require substantial memory; use a separate process with OS resource limits
    /// when memory exhaustion must not terminate the application.
    pub fn from_data(data: T, path: PathBuf, object_hash: gix_hash::Kind) -> Result<Self> {
        let hash_len = object_hash.len_in_bytes();
        let pack_len = data.len();
        let id = crc32fast::hash(path.as_os_str().to_string_lossy().as_bytes());
        if pack_len < data::header::SIZE + hash_len {
            bail!("Pack data of size {pack_len} is too small for even an empty pack with shortest hash".corrupted());
        }
        let (kind, num_objects) = data::header::decode(
            &data[..data::header::SIZE]
                .try_into()
                .expect("enough data after previous check"),
        )?;
        if num_objects as u64 > (pack_len - data::header::SIZE - hash_len) as u64 {
            bail!("Declared object count exceeds possible entries for pack size".corrupted());
        }
        Ok(Self {
            data,
            path,
            id,
            cache_id: cache::Id::allocate(),
            version: kind,
            num_objects,
            object_hash,
        })
    }
}
