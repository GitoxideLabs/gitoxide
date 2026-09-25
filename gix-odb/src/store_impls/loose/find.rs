use gix_error::Result;
use std::{cmp::Ordering, collections::HashSet};

use gix_error::{ErrorExt, Exn, ExnResult, Message, ResultExt, allocation_failure, allocation_limit, corruption};

use crate::store_impls::loose::{HEADER_MAX_SIZE, Store, hash_path};

/// Object lookup
impl Store {
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
    ) -> std::result::Result<Option<crate::store::prefix::lookup::Outcome>, crate::loose::iter::Error> {
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
    /// Failures include [metadata](gix_error::Error::metadata()) `path` (native path), the loose object file.
    pub fn try_find<'a>(&self, id: &gix_hash::oid, out: &'a mut Vec<u8>) -> Result<Option<gix_object::Data<'a>>> {
        debug_assert_eq!(self.object_hash, id.kind());
        (self
            .find_inner(id, out)
            .or_raise_erased(|| Message::new("Could not read loose object").with("path", self.object_path(id))))
        .map_err(Into::into)
    }

    /// Return only the decompressed size of the object and its kind without fully reading it into memory as tuple of `(size, kind)`.
    /// Returns `None` if `id` does not exist in the database.
    /// Failures include [metadata](gix_error::Error::metadata()) `path` (native path), the loose object file.
    pub fn try_header(&self, id: &gix_hash::oid) -> Result<Option<(u64, gix_object::Kind)>> {
        let path = hash_path(id, self.path.clone());
        let context = || Message::new("Could not read loose object header").with("path", path.as_path());
        let map = match self.map_loose_object(&path).or_raise_erased(context)? {
            Some(map) => map,
            None => return Ok(None),
        };
        let mut header = [0_u8; HEADER_MAX_SIZE];
        let mut inflate = gix_zlib::Inflate::default();
        let (status, _consumed_in, consumed_out) = inflate.once(&map, &mut header).or_raise_erased(context)?;

        if status == gix_zlib::Status::BufError {
            return Err(corruption(
                "Could not read loose object header: the zlib status indicated an error, status was 'BufError'",
            )
            .with("path", path.as_path())
            .raise()
            .into());
        }
        let (kind, size, _header_size) =
            gix_object::decode::loose_header(&header[..consumed_out]).or_raise_erased(context)?;
        Ok(Some((size, kind)))
    }

    /// Decode and allocation failures retain [metadata](gix_error::Error::metadata()) `size` (requested bytes) or
    /// `actual` and `expected`
    /// (inflated bytes); allocation limits also report `limit`. All counts are unsigned.
    fn find_inner<'a>(&self, id: &gix_hash::oid, out: &'a mut Vec<u8>) -> ExnResult<Option<gix_object::Data<'a>>> {
        let path = hash_path(id, self.path.clone());
        let map = match self.map_loose_object(&path)? {
            Some(map) => map,
            None => return Ok(None),
        };
        let mut header = [0_u8; HEADER_MAX_SIZE];

        let mut inflate = gix_zlib::Inflate::default();
        let (status, consumed_in, consumed_out) = inflate.once(&map, &mut header).or_erased()?;
        if status == gix_zlib::Status::BufError {
            return Err(corruption("The zlib status indicated an error, status was 'BufError'").raise_erased());
        }

        let (kind, size, header_size) = gix_object::decode::loose_header(&header[..consumed_out]).or_erased()?;
        self.ensure_in_alloc_limit(size)?;
        let allocation = || allocation_error(size);
        let size_usize = usize::try_from(size).or_raise_erased(|| {
            allocation_failure("Cannot store loose object in memory: the object size cannot be represented in memory")
                .with("size", size)
        })?;
        let decompressed_body_prefix_len = consumed_out
            .checked_sub(header_size)
            .ok_or_else(|| size_mismatch(consumed_out as u64, header_size as u64).erased())?;

        if decompressed_body_prefix_len > size_usize
            || (status == gix_zlib::Status::StreamEnd && decompressed_body_prefix_len != size_usize)
        {
            return Err(size_mismatch(decompressed_body_prefix_len as u64, size).erased());
        }

        // If the first inflate already reached the end of the stream, the fixed-size `header` buffer
        // contains the complete decompressed object, so we can skip a second streaming inflate pass.
        out.clear();
        out.try_reserve(size_usize).or_raise_erased(allocation)?;
        if status == gix_zlib::Status::StreamEnd {
            out.extend_from_slice(&header[header_size..consumed_out]);
        } else {
            out.resize(size_usize, 0);
            out[..decompressed_body_prefix_len].copy_from_slice(&header[header_size..consumed_out]);

            let mut input = &map[consumed_in..];
            let num_decompressed_bytes = gix_zlib::stream::inflate::read(
                &mut input,
                &mut inflate.state,
                &mut out[decompressed_body_prefix_len..],
            )
            .or_erased()?;

            if num_decompressed_bytes as u64 + decompressed_body_prefix_len as u64 != size {
                return Err(size_mismatch(
                    num_decompressed_bytes as u64 + decompressed_body_prefix_len as u64,
                    size,
                )
                .erased());
            }
        }
        Ok(Some(gix_object::Data {
            kind,
            object_hash: id.kind(),
            data: out,
        }))
    }

    /// Allocation-limit failures include [metadata](gix_error::Error::metadata()) `size` and `limit` (unsigned byte
    /// counts).
    fn ensure_in_alloc_limit(&self, size: u64) -> ExnResult {
        if let Some(limit) = self.alloc_limit_bytes.filter(|limit| size > *limit as u64) {
            return Err(allocation_limit(
                "Cannot store loose object in memory: the object exceeds the configured allocation limit",
            )
            .with("size", size)
            .with("limit", limit)
            .raise_erased());
        }
        Ok(())
    }

    fn map_loose_object(&self, path: &std::path::Path) -> ExnResult<Option<memmap2::Mmap>> {
        let map = match mmap::read_only(path) {
            Ok(map) => map,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err.raise_erased()),
        };
        if map.is_empty() {
            return Err(gix_error::corruption("Empty loose object file").raise_erased());
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

/// The raised error's [metadata](gix_error::Error::metadata()) `size` (unsigned bytes) identifies the requested
/// loose-object allocation.
fn allocation_error(size: u64) -> Message {
    Message::new("Cannot store loose object in memory").with("size", size)
}

/// Report invalid inflation sizes in [metadata](gix_error::Error::metadata()) `actual` and `expected` (unsigned byte
/// counts).
fn size_mismatch(actual: u64, expected: u64) -> Exn<Message> {
    corruption("Loose object size mismatch: invalid size of inflated loose object")
        .with("actual", actual)
        .with("expected", expected)
        .raise()
}
