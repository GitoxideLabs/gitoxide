use std::{
    mem::size_of,
    path::{Path, PathBuf},
};

use gix_error::{OptionExt, Result, ResultExt, bail, corruption, message};

use crate::index::{self, FAN_LEN, V2_SIGNATURE, Version};

const N32_SIZE: usize = size_of::<u32>();

/// Instantiation
impl index::File<crate::MMap> {
    /// Open the pack index file at the given `path`.
    ///
    /// The `object_hash` is a way to read (and write) the same file format with different hashes, as the hash kind
    /// isn't stored within the file format itself.
    ///
    /// Checks the table layout on open and checks individual offset references as they are read.
    pub fn at(path: impl AsRef<Path>, object_hash: gix_hash::Kind) -> Result<Self> {
        Self::at_inner(path.as_ref(), object_hash)
    }

    fn at_inner(path: &Path, object_hash: gix_hash::Kind) -> Result<Self> {
        let data = crate::mmap::read_only(path)
            .or_raise(|| message!("Could not open pack index file at \"{}\"", path.display()))?;
        Self::from_data(data, path.to_owned(), object_hash)
    }
}

impl<T> index::File<T>
where
    T: crate::FileData,
{
    /// Instantiate an index file from `data` as assumed to be read or memory-mapped from `path`.
    /// Checks the table layout on open and checks individual offset references as they are read.
    pub fn from_data(data: T, path: PathBuf, object_hash: gix_hash::Kind) -> Result<Self> {
        let idx_len = data.len();
        let hash_len = object_hash.len_in_bytes();

        let footer_size = hash_len * 2;
        if idx_len < FAN_LEN * N32_SIZE + footer_size {
            bail!("Pack index of size {idx_len} is too small for even an empty index".corrupted());
        }
        let (kind, fan, num_objects) = {
            let (kind, d) = {
                let (sig, d) = data.split_at(V2_SIGNATURE.len());
                if sig == V2_SIGNATURE {
                    (Version::V2, d)
                } else {
                    (Version::V1, &data[..])
                }
            };
            let d = {
                if let Version::V2 = kind {
                    let (vd, dr) = d.split_at(N32_SIZE);
                    let version = crate::read_u32(vd);
                    if version != Version::V2 as u32 {
                        bail!("Unsupported index version: {version})".unsupported());
                    }
                    dr
                } else {
                    d
                }
            };
            let (fan, bytes_read) = read_fan(d);
            let (_, _d) = d.split_at(bytes_read);
            let num_objects = fan[FAN_LEN - 1];

            (kind, fan, num_objects)
        };
        validate_fan(&fan)?;
        validate_layout(idx_len, kind, num_objects, hash_len)?;
        Ok(Self {
            data,
            path,
            version: kind,
            num_objects,
            fan,
            hash_len,
            object_hash,
        })
    }
}

fn read_fan(d: &[u8]) -> ([u32; FAN_LEN], usize) {
    assert!(d.len() >= FAN_LEN * N32_SIZE);

    let mut fan = [0; FAN_LEN];
    for (c, f) in d.as_chunks::<N32_SIZE>().0.iter().zip(fan.iter_mut()) {
        *f = crate::read_u32(c);
    }
    (fan, FAN_LEN * N32_SIZE)
}

fn validate_fan(fan: &[u32; FAN_LEN]) -> Result {
    if !crate::fan_is_monotonically_increasing(fan) {
        bail!(corruption("Pack index fan-out table must be monotonically increasing"));
    }
    Ok(())
}

fn validate_layout(len: usize, kind: Version, num_objects: u32, hash_len: usize) -> Result {
    let (header, entry_bytes) = match kind {
        Version::V1 => (FAN_LEN * N32_SIZE, hash_len + N32_SIZE),
        Version::V2 => (V2_SIGNATURE.len() + (1 + FAN_LEN) * N32_SIZE, hash_len + 2 * N32_SIZE),
    };
    let fixed_bytes = (num_objects as usize)
        .checked_mul(entry_bytes)
        .and_then(|entries| entries.checked_add(header + 2 * hash_len))
        .ok_or_raise(|| corruption("Pack index size overflowed while locating its tables"))?;
    let tail = len.checked_sub(fixed_bytes).ok_or_raise(|| {
        message!("Pack index of size {len} is too small for {num_objects} objects in version {kind:?}").corrupted()
    })?;
    let valid = match kind {
        Version::V1 => tail == 0,
        Version::V2 => tail.is_multiple_of(size_of::<u64>()) && tail / size_of::<u64>() <= num_objects as usize,
    };
    if !valid {
        bail!("Invalid pack index size {len} for {num_objects} objects in version {kind:?}".corrupted());
    }
    Ok(())
}
