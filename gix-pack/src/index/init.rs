use std::{
    mem::size_of,
    path::{Path, PathBuf},
};

use crate::index::{self, FAN_LEN, V2_SIGNATURE, Version};

/// Returned by [`index::File::at()`].
#[derive(thiserror::Error, Debug)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Could not open pack index file at '{path}'")]
    Io {
        source: std::io::Error,
        path: std::path::PathBuf,
    },
    #[error("{message}")]
    Corrupt { message: String },
    #[error("Unsupported index version: {version})")]
    UnsupportedVersion { version: u32 },
}

const N32_SIZE: usize = size_of::<u32>();

/// Instantiation
impl index::File<crate::MMap> {
    /// Open the pack index file at the given `path`.
    ///
    /// The `object_hash` is a way to read (and write) the same file format with different hashes, as the hash kind
    /// isn't stored within the file format itself.
    pub fn at(path: impl AsRef<Path>, object_hash: gix_hash::Kind) -> Result<Self, Error> {
        Self::at_inner(path.as_ref(), object_hash)
    }

    fn at_inner(path: &Path, object_hash: gix_hash::Kind) -> Result<Self, Error> {
        let data = crate::mmap::read_only(path).map_err(|source| Error::Io {
            source,
            path: path.to_owned(),
        })?;
        Self::from_data(data, path.to_owned(), object_hash)
    }
}

impl<T> index::File<T>
where
    T: crate::FileData,
{
    /// Instantiate an index file from `data` as assumed to be read or memory-mapped from `path`.
    pub fn from_data(data: T, path: PathBuf, object_hash: gix_hash::Kind) -> Result<Self, Error> {
        let idx_len = data.len();
        let hash_len = object_hash.len_in_bytes();

        let footer_size = hash_len * 2;
        if idx_len < FAN_LEN * N32_SIZE + footer_size {
            return Err(Error::Corrupt {
                message: format!("Pack index of size {idx_len} is too small for even an empty index"),
            });
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
                        return Err(Error::UnsupportedVersion { version });
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
        validate_size(&data, kind, num_objects, hash_len)?;
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
    for (c, f) in d.chunks_exact(N32_SIZE).zip(fan.iter_mut()) {
        *f = crate::read_u32(c);
    }
    (fan, FAN_LEN * N32_SIZE)
}

fn validate_fan(fan: &[u32; FAN_LEN]) -> Result<(), Error> {
    if !crate::fan_is_monotonically_increasing(fan) {
        return Err(Error::Corrupt {
            message: "Pack index fan-out table must be monotonically increasing".into(),
        });
    }
    Ok(())
}

fn validate_size(data: &[u8], kind: Version, num_objects: u32, hash_len: usize) -> Result<(), Error> {
    let num_objects = num_objects as usize;
    let footer_size = hash_len * 2;
    let expected_size = match kind {
        Version::V1 => FAN_LEN
            .checked_mul(N32_SIZE)
            .and_then(|size| size.checked_add(num_objects.checked_mul(N32_SIZE + hash_len)?))
            .and_then(|size| size.checked_add(footer_size))
            .ok_or_else(|| Error::Corrupt {
                message: "Pack index size overflowed while validating version 1 layout".into(),
            })?,
        Version::V2 => {
            let v2_header_size = V2_SIGNATURE.len() + N32_SIZE + FAN_LEN * N32_SIZE;
            let oid_bytes = num_objects.checked_mul(hash_len).ok_or_else(|| Error::Corrupt {
                message: "Pack index size overflowed while validating object ids".into(),
            })?;
            let table_bytes = num_objects.checked_mul(N32_SIZE).ok_or_else(|| Error::Corrupt {
                message: "Pack index size overflowed while validating 32-bit tables".into(),
            })?;
            let offset32_start = v2_header_size
                .checked_add(oid_bytes)
                .and_then(|size| size.checked_add(table_bytes))
                .ok_or_else(|| Error::Corrupt {
                    message: "Pack index size overflowed while locating 32-bit offsets".into(),
                })?;
            let offset32_end = offset32_start.checked_add(table_bytes).ok_or_else(|| Error::Corrupt {
                message: "Pack index size overflowed while locating 32-bit offsets".into(),
            })?;
            if offset32_end > data.len() {
                return Err(Error::Corrupt {
                    message: format!(
                        "Pack index of size {} is too small for {} objects in version 2",
                        data.len(),
                        num_objects
                    ),
                });
            }
            let (large_offsets, max_large_offset_index) = data[offset32_start..offset32_end]
                .chunks_exact(N32_SIZE)
                .filter_map(|offset| {
                    let offset = crate::read_u32(offset);
                    (offset & (1 << 31) != 0).then_some((offset ^ (1 << 31)) as usize)
                })
                .fold((0usize, 0usize), |(count, max_index), index| {
                    (count + 1, max_index.max(index))
                });
            v2_header_size
                .checked_add(oid_bytes)
                .and_then(|size| size.checked_add(table_bytes))
                .and_then(|size| size.checked_add(table_bytes))
                .and_then(|size| size.checked_add(large_offsets.checked_mul(size_of::<u64>())?))
                .and_then(|size| size.checked_add(footer_size))
                .ok_or_else(|| Error::Corrupt {
                    message: "Pack index size overflowed while validating version 2 layout".into(),
                })
                .and_then(|expected_size| {
                    if large_offsets > 0 && max_large_offset_index >= large_offsets {
                        return Err(Error::Corrupt {
                            message: format!(
                                "Pack index references large offset {max_large_offset_index}, but only {large_offsets} large offsets are present"
                            ),
                        });
                    }
                    Ok(expected_size)
                })?
        }
    };
    if data.len() != expected_size {
        // Aborting here is needed for protection against malformed inputs, or the offset access done later can panic
        // as it's done without explicit error handling.
        return Err(Error::Corrupt {
            message: format!(
                "Pack index size is incorrect, expected {expected_size} bytes for {num_objects} objects in version {kind:?}, but got {} bytes",
                data.len()
            ),
        });
    }
    Ok(())
}
