use std::{cmp::Ordering, collections::HashSet, io, path::PathBuf};

use gix_features::zlib;

use crate::store_impls::loose::{HEADER_MAX_SIZE, Store, hash_path};

/// Returned by [`Store::try_find()`]
#[derive(thiserror::Error, Debug)]
#[allow(missing_docs)]
pub enum Error {
    #[error("decompression of loose object at '{path}' failed")]
    DecompressFile {
        source: zlib::inflate::Error,
        path: PathBuf,
    },
    #[error("file at '{path}' showed invalid size of inflated data, expected {expected}, got {actual}")]
    SizeMismatch { actual: u64, expected: u64, path: PathBuf },
    #[error(transparent)]
    Decode(#[from] gix_object::decode::LooseHeaderDecodeError),
    #[error("Cannot store {size} in memory as it's not representable")]
    OutOfMemory { size: u64 },
    #[error("Could not {action} data at '{path}'")]
    Io {
        source: std::io::Error,
        action: &'static str,
        path: PathBuf,
    },
}

/// Object lookup
impl Store {
    const OPEN_OR_MAP_ACTION: &'static str = "open or map";

    /// Returns true if the given id is contained in our repository.
    pub fn contains(&self, id: &gix_hash::oid) -> bool {
        debug_assert_eq!(self.object_hash, id.kind());
        hash_path(id, self.path.clone()).is_file()
    }

    /// Given a `prefix`, find an object that matches it uniquely within this loose object
    /// database as `Ok(Some(Ok(<oid>)))`.
    /// If there is more than one object matching the object `Ok(Some(Err(()))` is returned.
    ///
    /// Finally, if no object matches, the return value is `Ok(None)`.
    ///
    /// The outer `Result` is to indicate errors during file system traversal.
    ///
    /// Pass `candidates` to obtain the set of all object ids matching `prefix`, with the same return value as
    /// one would have received if it remained `None`.
    pub fn lookup_prefix(
        &self,
        prefix: gix_hash::Prefix,
        mut candidates: Option<&mut HashSet<gix_hash::ObjectId>>,
    ) -> Result<Option<crate::store::prefix::lookup::Outcome>, crate::loose::iter::Error> {
        let single_directory_iter = crate::loose::Iter {
            inner: gix_features::fs::walkdir_new(
                &self.path.join(prefix.as_oid().to_hex_with_len(2).to_string()),
                gix_features::fs::walkdir::Parallelism::Serial,
                false,
            )
            .min_depth(1)
            .max_depth(1)
            .follow_links(false)
            .into_iter(),
            hash_hex_len: prefix.as_oid().kind().len_in_hex(),
        };
        let mut candidate = None;
        for oid in single_directory_iter {
            let oid = match oid {
                Ok(oid) => oid,
                Err(err) => {
                    return match err.io_error() {
                        Some(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
                        None | Some(_) => Err(err),
                    };
                }
            };
            if prefix.cmp_oid(&oid) == Ordering::Equal {
                match &mut candidates {
                    Some(candidates) => {
                        candidates.insert(oid);
                    }
                    None => {
                        if candidate.is_some() {
                            return Ok(Some(Err(())));
                        }
                        candidate = Some(oid);
                    }
                }
            }
        }

        match &mut candidates {
            Some(candidates) => match candidates.len() {
                0 => Ok(None),
                1 => Ok(candidates.iter().next().copied().map(Ok)),
                _ => Ok(Some(Err(()))),
            },
            None => Ok(candidate.map(Ok)),
        }
    }

    /// Return the object identified by the given [`ObjectId`][gix_hash::ObjectId] if present in this database,
    /// writing its raw data into the given `out` buffer.
    ///
    /// Returns `Err` if there was an error locating or reading the object. Returns `Ok<None>` if
    /// there was no such object.
    pub fn try_find<'a>(
        &self,
        id: &gix_hash::oid,
        out: &'a mut Vec<u8>,
    ) -> Result<Option<gix_object::Data<'a>>, Error> {
        debug_assert_eq!(self.object_hash, id.kind());
        self.find_inner(id, out)
    }

    /// Return only the decompressed size of the object and its kind without fully reading it into memory as tuple of `(size, kind)`.
    /// Returns `None` if `id` does not exist in the database.
    pub fn try_header(&self, id: &gix_hash::oid) -> Result<Option<(u64, gix_object::Kind)>, Error> {
        let path = hash_path(id, self.path.clone());
        let map = match self.map_loose_object(&path)? {
            Some(map) => map,
            None => return Ok(None),
        };
        let mut header = [0_u8; HEADER_MAX_SIZE];
        let mut inflate = zlib::Inflate::default();
        let (status, _consumed_in, consumed_out) =
            inflate.once(&map, &mut header).map_err(|e| Error::DecompressFile {
                source: e,
                path: path.to_owned(),
            })?;

        if status == zlib::Status::BufError {
            return Err(Error::DecompressFile {
                source: zlib::inflate::Error::Status(status),
                path,
            });
        }
        let (kind, size, _header_size) = gix_object::decode::loose_header(&header[..consumed_out])?;
        Ok(Some((size, kind)))
    }

    fn find_inner<'a>(&self, id: &gix_hash::oid, out: &'a mut Vec<u8>) -> Result<Option<gix_object::Data<'a>>, Error> {
        let path = hash_path(id, self.path.clone());
        let map = match self.map_loose_object(&path)? {
            Some(map) => map,
            None => return Ok(None),
        };
        let mut header = [0_u8; HEADER_MAX_SIZE];

        let mut inflate = zlib::Inflate::default();
        let (status, consumed_in, consumed_out) =
            inflate.once(&map, &mut header).map_err(|e| Error::DecompressFile {
                source: e,
                path: path.to_owned(),
            })?;
        if status == zlib::Status::BufError {
            return Err(Error::DecompressFile {
                source: zlib::inflate::Error::Status(status),
                path,
            });
        }

        let (kind, size, header_size) = gix_object::decode::loose_header(&header[..consumed_out])?;
        self.ensure_in_alloc_limit(size)?;
        let size_usize = usize::try_from(size).map_err(|_| Error::OutOfMemory { size })?;
        let decompressed_body_prefix_len = consumed_out.checked_sub(header_size).ok_or(Error::SizeMismatch {
            actual: consumed_out as u64,
            expected: header_size as u64,
            path: path.clone(),
        })?;

        if decompressed_body_prefix_len > size_usize {
            return Err(Error::SizeMismatch {
                expected: size,
                actual: decompressed_body_prefix_len as u64,
                path,
            });
        }

        // If the first inflate already reached the end of the stream, the fixed-size `header` buffer
        // contains the complete decompressed object. In that case we can avoid allocating the full
        // output buffer and a second streaming inflate pass.
        out.clear();
        if status == zlib::Status::StreamEnd {
            if consumed_out as u64 != size + header_size as u64 {
                return Err(Error::SizeMismatch {
                    expected: size + header_size as u64,
                    actual: consumed_out as u64,
                    path,
                });
            }
            out.extend_from_slice(&header[header_size..consumed_out]);
        } else {
            out.resize(size_usize, 0);
            out[..decompressed_body_prefix_len].copy_from_slice(&header[header_size..consumed_out]);

            let mut input = &map[consumed_in..];
            let num_decompressed_bytes =
                zlib::stream::inflate::read(&mut input, &mut inflate.state, &mut out[decompressed_body_prefix_len..])
                    .map_err(|e| Error::Io {
                    source: e,
                    action: "inflate",
                    path: path.to_owned(),
                })?;

            if num_decompressed_bytes as u64 + decompressed_body_prefix_len as u64 != size {
                return Err(Error::SizeMismatch {
                    expected: size,
                    actual: num_decompressed_bytes as u64 + decompressed_body_prefix_len as u64,
                    path,
                });
            }
        }
        Ok(Some(gix_object::Data {
            kind,
            object_hash: id.kind(),
            data: out,
        }))
    }

    fn ensure_in_alloc_limit(&self, size: u64) -> Result<(), Error> {
        if self.alloc_limit_bytes.is_some_and(|limit| size > limit as u64) {
            return Err(Error::OutOfMemory { size });
        }
        Ok(())
    }

    fn map_loose_object(&self, path: &std::path::Path) -> Result<Option<memmap2::Mmap>, Error> {
        let map = match mmap::read_only(path) {
            Ok(map) => map,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => {
                return Err(Error::Io {
                    action: Self::OPEN_OR_MAP_ACTION,
                    source: err,
                    path: path.to_owned(),
                });
            }
        };

        if map.is_empty() {
            return Err(Error::Io {
                source: io::Error::other("empty loose object file"),
                action: Self::OPEN_OR_MAP_ACTION,
                path: path.to_owned(),
            });
        }
        Ok(Some(map))
    }
}

mod mmap {
    use std::path::Path;

    pub fn read_only(path: &Path) -> std::io::Result<memmap2::Mmap> {
        let file = std::fs::File::open(path)?;
        // SAFETY: we have to take the risk of somebody changing the file underneath. Git never writes into the same file.
        #[allow(unsafe_code)]
        unsafe {
            memmap2::MmapOptions::new().map_copy_read_only(&file)
        }
    }
}
