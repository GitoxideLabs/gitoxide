use std::{
    borrow::Cow,
    mem::size_of,
    path::{Path, PathBuf},
};

use gix_error::{Result, ResultExt, bail, message};

use crate::index::{self, FAN_LEN, V2_SIGNATURE, Version};

fn corrupt(message: impl Into<Cow<'static, str>>) -> gix_error::Error {
    gix_error::corruption(message).into()
}

const N32_SIZE: usize = size_of::<u32>();

/// Instantiation
impl index::File<crate::MMap> {
    /// Open the pack index file at the given `path`.
    ///
    /// The `object_hash` is a way to read (and write) the same file format with different hashes, as the hash kind
    /// isn't stored within the file format itself.
    pub fn at(path: impl AsRef<Path>, object_hash: gix_hash::Kind) -> Result<Self> {
        index::LookupFile::at(path, object_hash)?.into_validated()
    }
}

impl index::LookupFile<crate::MMap> {
    /// Open an index for lookups without walking its offset table.
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
    pub fn from_data(data: T, path: PathBuf, object_hash: gix_hash::Kind) -> Result<Self> {
        index::LookupFile::from_data(data, path, object_hash)?.into_validated()
    }
}

impl<T: crate::FileData> index::LookupFile<T> {
    /// Open memory-backed index data for checked, on-demand offset lookups.
    pub fn from_data(data: T, path: PathBuf, object_hash: gix_hash::Kind) -> Result<Self> {
        let idx_len = data.len();
        let hash_len = object_hash.len_in_bytes();

        let footer_size = hash_len * 2;
        if idx_len < FAN_LEN * N32_SIZE + footer_size {
            return Err(corrupt(format!(
                "Pack index of size {idx_len} is too small for even an empty index"
            )));
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
            file: index::File {
                data,
                path,
                version: kind,
                num_objects,
                fan,
                hash_len,
                object_hash,
            },
            offset_validation: Default::default(),
        })
    }

    /// Check every offset reference once, returning an index suitable for full traversal.
    pub fn validated(&self) -> Result<&index::File<T>> {
        match self.offset_validation.get_or_init(|| self.check_offsets()) {
            Ok(()) => Ok(&self.file),
            Err(message) => Err(corrupt(message.clone())),
        }
    }

    fn check_offsets(&self) -> std::result::Result<(), String> {
        let file = &self.file;
        if file.version == Version::V2 {
            let end = file.offset_pack_offset64_v2();
            let offsets = file.data[file.offset_pack_offset_v2()..end].as_chunks::<N32_SIZE>().0;
            let large_offsets = (file.data.len() - end - 2 * file.hash_len) / size_of::<u64>();
            let (references, max_slot) = offsets
                .iter()
                .filter_map(|offset| {
                    let offset = crate::read_u32(offset);
                    (offset & (1 << 31) != 0).then_some((offset ^ (1 << 31)) as usize)
                })
                .fold((0usize, 0usize), |(count, max_slot), slot| {
                    (count + 1, max_slot.max(slot))
                });
            if references != 0 && max_slot >= large_offsets {
                return Err(format!(
                    "Pack index references large offset {max_slot}, but only {large_offsets} are present"
                ));
            }
            if references != large_offsets {
                return Err(format!(
                    "Pack index has {large_offsets} large offsets for {references} references"
                ));
            }
        }
        Ok(())
    }

    fn into_validated(self) -> Result<index::File<T>> {
        self.check_offsets().map_err(corrupt)?;
        Ok(self.file)
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
        return Err(corrupt("Pack index fan-out table must be monotonically increasing"));
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
        .ok_or_else(|| corrupt("Pack index size overflowed while locating its tables"))?;
    let tail = len.checked_sub(fixed_bytes).ok_or_else(|| {
        corrupt(format!(
            "Pack index of size {len} is too small for {num_objects} objects in version {kind:?}"
        ))
    })?;
    let valid = match kind {
        Version::V1 => tail == 0,
        Version::V2 => tail.is_multiple_of(size_of::<u64>()) && tail / size_of::<u64>() <= num_objects as usize,
    };
    if !valid {
        return Err(corrupt(format!(
            "Invalid pack index size {len} for {num_objects} objects in version {kind:?}"
        )));
    }
    Ok(())
}
