use crate::store_impl::packed;

impl AsRef<[u8]> for packed::Buffer {
    fn as_ref(&self) -> &[u8] {
        &self.data.as_ref()[self.offset..]
    }
}

impl AsRef<[u8]> for packed::Backing {
    fn as_ref(&self) -> &[u8] {
        match self {
            packed::Backing::InMemory(data) => data,
            packed::Backing::Mapped(map) => map,
        }
    }
}

///
pub mod open {
    use std::path::PathBuf;

    use crate::store_impl::packed;

    /// Initialization
    impl packed::Buffer {
        fn open_with_backing(
            backing: packed::Backing,
            path: PathBuf,
            object_hash: gix_hash::Kind,
        ) -> Result<Self, Error> {
            let (backing, offset) = {
                let (offset, sorted) = {
                    let mut input = backing.as_ref();
                    if *input.first().unwrap_or(&b' ') == b'#' {
                        let header = packed::decode::header(&mut input).map_err(|_| Error::HeaderParsing)?;
                        let offset = backing.as_ref().len() - input.len();
                        (offset, header.sorted)
                    } else {
                        (0, false)
                    }
                };

                if !sorted {
                    // this implementation is likely slower than what git does, but it's less code, too.
                    let mut entries =
                        packed::Iter::new(&backing.as_ref()[offset..], object_hash)?.collect::<Result<Vec<_>, _>>()?;
                    entries.sort_by_key(|e| e.name.as_bstr());
                    let mut serialized = Vec::<u8>::new();
                    for entry in entries {
                        serialized.extend_from_slice(entry.target);
                        serialized.push(b' ');
                        serialized.extend_from_slice(entry.name.as_bstr());
                        serialized.push(b'\n');
                        if let Some(object) = entry.object {
                            serialized.push(b'^');
                            serialized.extend_from_slice(object);
                            serialized.push(b'\n');
                        }
                    }
                    (Backing::InMemory(serialized), 0)
                } else {
                    (backing, offset)
                }
            };
            Ok(packed::Buffer {
                offset,
                data: backing,
                path,
                object_hash,
            })
        }

        /// Open the file at `path`, parsing object ids as `object_hash`, and map it into memory if the file size is larger
        /// than `use_memory_map_if_larger_than_bytes`.
        ///
        /// In order to allow fast lookups and optimizations, the contents of the packed refs must be sorted.
        /// If that's not the case, they will be sorted on the fly with the data being written into a memory buffer.
        pub fn open(
            path: PathBuf,
            use_memory_map_if_larger_than_bytes: u64,
            object_hash: gix_hash::Kind,
        ) -> Result<Self, Error> {
            let backing = if std::fs::metadata(&path)?.len() <= use_memory_map_if_larger_than_bytes {
                packed::Backing::InMemory(std::fs::read(&path)?)
            } else {
                packed::Backing::Mapped(
                    // SAFETY: we have to take the risk of somebody changing the file underneath. Git never writes into the same file.
                    #[expect(unsafe_code)]
                    unsafe {
                        memmap2::MmapOptions::new().map_copy_read_only(&std::fs::File::open(&path)?)?
                    },
                )
            };
            Self::open_with_backing(backing, path, object_hash)
        }

        /// Open a buffer from `bytes`, which is the content of a typical `packed-refs` file, parsing object ids as
        /// `object_hash`.
        ///
        /// In order to allow fast lookups and optimizations, the contents of the packed refs must be sorted.
        /// If that's not the case, they will be sorted on the fly.
        pub fn from_bytes(bytes: &[u8], object_hash: gix_hash::Kind) -> Result<Self, Error> {
            let backing = packed::Backing::InMemory(bytes.into());
            Self::open_with_backing(backing, PathBuf::from("<memory>"), object_hash)
        }
    }

    mod error {
        use crate::packed;

        /// The error returned by [`open()`][super::packed::Buffer::open()].
        #[derive(Debug)]
        #[expect(missing_docs)]
        pub enum Error {
            Iter(packed::iter::Error),
            HeaderParsing,
            Io(std::io::Error),
        }

        impl std::fmt::Display for Error {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Error::Iter(_) => f.write_str(
                        "The packed-refs file did not have a header or wasn't sorted and could not be iterated",
                    ),
                    Error::HeaderParsing => {
                        f.write_str("The header could not be parsed, even though first line started with '#'")
                    }
                    Error::Io(_) => f.write_str("The buffer could not be opened or read"),
                }
            }
        }

        impl std::error::Error for Error {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                match self {
                    Error::Iter(err) => Some(err),
                    Error::HeaderParsing => None,
                    Error::Io(err) => Some(err),
                }
            }
        }

        impl From<packed::iter::Error> for Error {
            fn from(err: packed::iter::Error) -> Self {
                Error::Iter(err)
            }
        }

        impl From<std::io::Error> for Error {
            fn from(err: std::io::Error) -> Self {
                Error::Io(err)
            }
        }
    }
    pub use error::Error;

    use crate::packed::Backing;
}
