use std::{
    cmp::Ordering,
    collections::HashSet,
    fs,
    io::{self, Read},
    path::PathBuf,
};

use gix_features::zlib;

use crate::store_impls::loose::{hash_path, Store, HEADER_MAX_SIZE};

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

pub(crate) struct StreamReader {
    file: fs::File,
    inflate: zlib::Decompress,
    compressed: [u8; 8192],
    compressed_pos: usize,
    compressed_len: usize,
    pending: Vec<u8>,
    pending_pos: usize,
    done: bool,
}

impl StreamReader {
    fn new(file: fs::File) -> Self {
        Self {
            file,
            inflate: zlib::Decompress::default(),
            compressed: [0; 8192],
            compressed_pos: 0,
            compressed_len: 0,
            pending: Vec::new(),
            pending_pos: 0,
            done: false,
        }
    }

    fn read_header(mut self, path: &std::path::Path) -> Result<(gix_object::Kind, u64, Self), Error> {
        let mut decompressed = [0u8; HEADER_MAX_SIZE + 8192];
        let mut produced_total = 0usize;
        loop {
            if self.compressed_pos == self.compressed_len {
                self.compressed_len = self.file.read(&mut self.compressed).map_err(|source| Error::Io {
                    source,
                    action: "read",
                    path: path.to_owned(),
                })?;
                self.compressed_pos = 0;
            }

            let input = &self.compressed[self.compressed_pos..self.compressed_len];
            let eof = input.is_empty();
            let before_in = self.inflate.total_in();
            let before_out = self.inflate.total_out();
            let flush = if eof {
                zlib::FlushDecompress::Finish
            } else {
                zlib::FlushDecompress::None
            };
            let status = self
                .inflate
                .decompress(input, &mut decompressed[produced_total..], flush)
                .map_err(|source| Error::DecompressFile {
                    source: source.into(),
                    path: path.to_owned(),
                })?;
            let consumed = (self.inflate.total_in() - before_in) as usize;
            let produced = (self.inflate.total_out() - before_out) as usize;
            self.compressed_pos += consumed;
            produced_total += produced;

            if let Ok((kind, size, header_size)) = gix_object::decode::loose_header(&decompressed[..produced_total]) {
                self.pending = decompressed[header_size..produced_total].to_vec();
                return Ok((kind, size, self));
            }

            if produced_total == decompressed.len() {
                return Err(Error::Decode(
                    gix_object::decode::LooseHeaderDecodeError::InvalidHeader {
                        message: "loose object header exceeded the supported maximum size",
                    },
                ));
            }

            match status {
                zlib::Status::StreamEnd => {
                    return Err(Error::Decode(
                        gix_object::decode::LooseHeaderDecodeError::InvalidHeader {
                            message: "loose object header terminated before it could be parsed",
                        },
                    ))
                }
                zlib::Status::Ok | zlib::Status::BufError if eof => {
                    return Err(Error::Decode(
                        gix_object::decode::LooseHeaderDecodeError::InvalidHeader {
                            message: "loose object header terminated before it could be parsed",
                        },
                    ));
                }
                zlib::Status::Ok | zlib::Status::BufError if consumed != 0 || produced != 0 => {}
                zlib::Status::Ok | zlib::Status::BufError => {
                    return Err(Error::DecompressFile {
                        source: zlib::inflate::Error::Status(zlib::Status::BufError),
                        path: path.to_owned(),
                    });
                }
            }
        }
    }
}

impl io::Read for StreamReader {
    fn read(&mut self, mut out: &mut [u8]) -> io::Result<usize> {
        let mut total_written = 0usize;

        if self.pending_pos < self.pending.len() {
            let pending = &self.pending[self.pending_pos..];
            let count = pending.len().min(out.len());
            out[..count].copy_from_slice(&pending[..count]);
            self.pending_pos += count;
            total_written += count;
            out = &mut out[count..];
            if self.pending_pos == self.pending.len() {
                self.pending.clear();
                self.pending_pos = 0;
            }
            if out.is_empty() {
                return Ok(total_written);
            }
        }

        loop {
            if self.done {
                return Ok(total_written);
            }
            if self.compressed_pos == self.compressed_len {
                self.compressed_len = self.file.read(&mut self.compressed)?;
                self.compressed_pos = 0;
            }

            let input = &self.compressed[self.compressed_pos..self.compressed_len];
            let eof = input.is_empty();
            let before_in = self.inflate.total_in();
            let before_out = self.inflate.total_out();
            let flush = if eof {
                zlib::FlushDecompress::Finish
            } else {
                zlib::FlushDecompress::None
            };
            let status = self
                .inflate
                .decompress(input, out, flush)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "corrupt deflate stream"))?;
            let consumed = (self.inflate.total_in() - before_in) as usize;
            let produced = (self.inflate.total_out() - before_out) as usize;
            self.compressed_pos += consumed;
            total_written += produced;

            match status {
                zlib::Status::StreamEnd => {
                    self.done = true;
                    return Ok(total_written);
                }
                zlib::Status::Ok | zlib::Status::BufError if produced != 0 => return Ok(total_written),
                zlib::Status::Ok | zlib::Status::BufError if eof => {
                    self.done = true;
                    return Ok(total_written);
                }
                zlib::Status::Ok | zlib::Status::BufError if consumed != 0 => {}
                zlib::Status::Ok | zlib::Status::BufError => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "deflate stream made no progress",
                    ));
                }
            }
        }
    }
}

/// Object lookup
impl Store {
    const OPEN_ACTION: &'static str = "open";

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
                    }
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
        match self.find_inner(id, out) {
            Ok(obj) => Ok(Some(obj)),
            Err(err) => match err {
                Error::Io {
                    source: err,
                    action,
                    path,
                } => {
                    if action == Self::OPEN_ACTION && err.kind() == std::io::ErrorKind::NotFound {
                        Ok(None)
                    } else {
                        Err(Error::Io {
                            source: err,
                            action,
                            path,
                        })
                    }
                }
                err => Err(err),
            },
        }
    }

    /// Return the object identified by `id` as a decoded byte stream.
    ///
    /// Returns `Ok(None)` if there is no such object.
    pub fn try_find_stream(&self, id: &gix_hash::oid) -> Result<Option<crate::find::Stream>, Error> {
        debug_assert_eq!(self.object_hash, id.kind());
        let path = hash_path(id, self.path.clone());
        let file = match fs::File::open(&path) {
            Ok(file) => file,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                return Err(Error::Io {
                    source,
                    action: Self::OPEN_ACTION,
                    path,
                })
            }
        };

        let (kind, size, reader) = StreamReader::new(file).read_header(&path)?;
        Ok(Some(crate::find::Stream::from_loose(kind, size, reader)))
    }

    /// Return only the decompressed size of the object and its kind without fully reading it into memory as tuple of `(size, kind)`.
    /// Returns `None` if `id` does not exist in the database.
    pub fn try_header(&self, id: &gix_hash::oid) -> Result<Option<(u64, gix_object::Kind)>, Error> {
        const BUF_SIZE: usize = 256;
        let mut buf = [0_u8; BUF_SIZE];
        let path = hash_path(id, self.path.clone());

        let mut inflate = zlib::Inflate::default();
        let mut istream = match fs::File::open(&path) {
            Ok(f) => f,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => {
                return Err(Error::Io {
                    source: err,
                    action: Self::OPEN_ACTION,
                    path,
                })
            }
        };

        let (compressed_buf, _) = buf.split_at_mut(BUF_SIZE - HEADER_MAX_SIZE);
        let bytes_read = istream.read(compressed_buf).map_err(|e| Error::Io {
            source: e,
            action: "read",
            path: path.to_owned(),
        })?;
        let (compressed_buf, header_buf) = buf.split_at_mut(bytes_read);
        let (status, _consumed_in, consumed_out) =
            inflate
                .once(compressed_buf, header_buf)
                .map_err(|e| Error::DecompressFile {
                    source: e,
                    path: path.to_owned(),
                })?;

        if status == zlib::Status::BufError {
            return Err(Error::DecompressFile {
                source: zlib::inflate::Error::Status(status),
                path,
            });
        }
        let (kind, size, _header_size) = gix_object::decode::loose_header(&header_buf[..consumed_out])?;
        Ok(Some((size, kind)))
    }

    fn find_inner<'a>(&self, id: &gix_hash::oid, buf: &'a mut Vec<u8>) -> Result<gix_object::Data<'a>, Error> {
        let path = hash_path(id, self.path.clone());

        let mut inflate = zlib::Inflate::default();
        let ((status, consumed_in, consumed_out), bytes_read) = {
            let mut istream = fs::File::open(&path).map_err(|e| Error::Io {
                source: e,
                action: Self::OPEN_ACTION,
                path: path.to_owned(),
            })?;

            buf.clear();
            let bytes_read = istream.read_to_end(buf).map_err(|e| Error::Io {
                source: e,
                action: "read",
                path: path.to_owned(),
            })?;
            buf.resize(bytes_read + HEADER_MAX_SIZE, 0);
            let (input, output) = buf.split_at_mut(bytes_read);
            (
                inflate
                    .once(&input[..bytes_read], output)
                    .map_err(|e| Error::DecompressFile {
                        source: e,
                        path: path.to_owned(),
                    })?,
                bytes_read,
            )
        };
        if status == zlib::Status::BufError {
            return Err(Error::DecompressFile {
                source: zlib::inflate::Error::Status(status),
                path,
            });
        }

        let decompressed_start = bytes_read;
        let (kind, size, header_size) =
            gix_object::decode::loose_header(&buf[decompressed_start..decompressed_start + consumed_out])?;

        if status == zlib::Status::StreamEnd {
            let decompressed_body_bytes_sans_header =
                decompressed_start + header_size..decompressed_start + consumed_out;

            if consumed_out as u64 != size + header_size as u64 {
                return Err(Error::SizeMismatch {
                    expected: size + header_size as u64,
                    actual: consumed_out as u64,
                    path,
                });
            }
            buf.copy_within(decompressed_body_bytes_sans_header, 0);
        } else {
            let new_len = bytes_read as u64 + size + header_size as u64;
            buf.resize(new_len.try_into().map_err(|_| Error::OutOfMemory { size: new_len })?, 0);
            {
                let (input, output) = buf.split_at_mut(bytes_read);
                let num_decompressed_bytes = zlib::stream::inflate::read(
                    &mut &input[consumed_in..],
                    &mut inflate.state,
                    &mut output[consumed_out..],
                )
                .map_err(|e| Error::Io {
                    source: e,
                    action: "deflate",
                    path: path.to_owned(),
                })?;
                if num_decompressed_bytes as u64 + consumed_out as u64 != size + header_size as u64 {
                    return Err(Error::SizeMismatch {
                        expected: size + header_size as u64,
                        actual: num_decompressed_bytes as u64 + consumed_out as u64,
                        path,
                    });
                }
            };
            buf.copy_within(decompressed_start + header_size.., 0);
        }
        buf.resize(
            size.try_into()
                .expect("BUG: here the size is already confirmed to fit into memory"),
            0,
        );
        Ok(gix_object::Data { kind, data: buf })
    }
}
