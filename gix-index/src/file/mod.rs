#[derive(Clone)]
pub(crate) enum Snapshot {
    Checksum(gix_hash::ObjectId),
    // Git replaces the index by rename. Its open file still contains the bytes we read.
    #[cfg(not(windows))]
    Checksumless(std::sync::Arc<std::fs::File>),
    // Windows can refuse to replace an index with an open handle. Keep a digest of its bytes.
    #[cfg(windows)]
    Checksumless(gix_hash::ObjectId),
}

impl Snapshot {
    fn new(checksum: gix_hash::ObjectId, file: Option<std::fs::File>) -> Option<Self> {
        if !checksum.is_null() {
            return Some(Self::Checksum(checksum));
        }
        #[cfg(not(windows))]
        {
            file.map(|file| Self::Checksumless(file.into()))
        }
        #[cfg(windows)]
        {
            Some(Self::Checksumless(hash_index(file.as_ref()?, checksum.kind()).ok()?))
        }
    }

    fn matches(&self, path: &std::path::Path) -> std::io::Result<bool> {
        use std::io::{Read, Seek, SeekFrom};
        let mut file = std::fs::File::open(path)?;
        match self {
            Self::Checksum(expected) => {
                let mut actual = expected.kind().null();
                if file.seek(SeekFrom::End(-(actual.as_slice().len() as i64)))? < crate::decode::header::SIZE as u64 {
                    return Ok(false);
                }
                file.read_exact(actual.as_mut_slice())?;
                Ok(actual == *expected)
            }
            #[cfg(not(windows))]
            Self::Checksumless(original) => {
                Ok(original.metadata()?.len() == file.metadata()?.len() && *map_index(original)? == *map_index(&file)?)
            }
            #[cfg(windows)]
            Self::Checksumless(expected) => Ok(hash_index(&file, expected.kind())? == *expected),
        }
    }
}

#[cfg(windows)]
fn hash_index(file: &std::fs::File, kind: gix_hash::Kind) -> std::io::Result<gix_hash::ObjectId> {
    let mut hasher = gix_hash::hasher(kind);
    hasher.update(&map_index(file)?);
    hasher.try_finalize().map_err(std::io::Error::other)
}

fn map_index(file: &std::fs::File) -> std::io::Result<memmap2::Mmap> {
    // SAFETY: Git writers replace the locked index by rename and never modify its mapped bytes in place.
    #[expect(unsafe_code)]
    unsafe {
        memmap2::MmapOptions::new().map_copy_read_only(file)
    }
}

mod impls {
    use std::ops::{Deref, DerefMut};

    use crate::{File, State};

    impl Deref for File {
        type Target = State;

        fn deref(&self) -> &Self::Target {
            &self.state
        }
    }

    impl DerefMut for File {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.state
        }
    }
}

mod impl_ {
    use std::fmt::Formatter;

    use crate::{Entry, File, PathStorageRef, State};

    impl std::fmt::Debug for File {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            if f.alternate() {
                return f
                    .debug_struct("File")
                    .field("path", &self.path.display())
                    .field("checksum", &self.checksum())
                    .field("object_hash", &self.state.object_hash)
                    .field("timestamp", &self.state.timestamp)
                    .field("version", &self.state.version)
                    .field(
                        "entries",
                        &EntriesDebug {
                            entries: &self.state.entries,
                            path_backing: &self.state.path_backing,
                        },
                    )
                    .field("path_backing_size_bytes", &self.state.path_backing.len())
                    .field("is_sparse", &self.state.is_sparse)
                    .field("end_of_index_at_decode_time", &self.state.end_of_index_at_decode_time)
                    .field("offset_table_at_decode_time", &self.state.offset_table_at_decode_time)
                    .field("tree", &self.state.tree)
                    .field("has_link", &self.state.link.is_some())
                    .field("has_resolve_undo", &self.state.resolve_undo.is_some())
                    .field("untracked", &self.state.untracked)
                    .field("has_fs_monitor", &self.state.fs_monitor.is_some())
                    .finish();
            }
            f.debug_struct("File")
                .field("path", &self.path.display())
                .field("checksum", &self.checksum())
                .finish_non_exhaustive()
        }
    }

    struct EntriesDebug<'a> {
        entries: &'a [Entry],
        path_backing: &'a PathStorageRef,
    }

    impl std::fmt::Debug for EntriesDebug<'_> {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            if !f.alternate() {
                return f.debug_list().entries(self.entries).finish();
            }

            writeln!(f, "[")?;
            for entry in self.entries {
                write!(f, "    ")?;
                entry.fmt_debug(f, Some(self.path_backing))?;
                writeln!(f, ",")?;
            }
            write!(f, "]")
        }
    }

    impl From<File> for State {
        fn from(f: File) -> Self {
            f.state
        }
    }
}

mod access {
    use crate::File;

    /// Consumption
    impl File {
        /// Take all non-copy parts of the index.
        pub fn into_parts(self) -> (crate::State, std::path::PathBuf) {
            (self.state, self.path)
        }
    }

    /// Access
    impl File {
        /// The path from which the index was read or to which it is supposed to be written when used with [`File::from_state()`].
        pub fn path(&self) -> &std::path::Path {
            &self.path
        }

        /// The checksum over the file that was read or written to disk.
        /// Returns `None` if the state in memory was never serialized or the checksum was omitted
        /// via [`crate::write::Options::skip_hash`].
        ///
        /// Note that even if `Some`, it will only represent the state in memory right after reading or [writing][File::write()].
        pub fn checksum(&self) -> Option<gix_hash::ObjectId> {
            match self.snapshot {
                Some(super::Snapshot::Checksum(id)) => Some(id),
                _ => None,
            }
        }
    }
}

mod mutation {
    use std::path::PathBuf;

    use crate::File;

    /// Mutating access
    impl File {
        /// Set the path at which we think we are located to the given `path`.
        ///
        /// This is useful to change the location of the index *once* it is written via [`write()`][File::write()].
        pub fn set_path(&mut self, path: impl Into<PathBuf>) {
            self.path = path.into();
        }
    }
}

///
pub mod init;
///
pub mod verify;
///
pub mod write;
