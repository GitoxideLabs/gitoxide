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
                    .field("checksum", &self.checksum)
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
                .field("checksum", &self.checksum)
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

        /// The checksum over the file that was read or written to disk, or `None` if the state in memory was never serialized.
        ///
        /// Note that even if `Some`, it will only represent the state in memory right after reading or [writing][File::write()].
        pub fn checksum(&self) -> Option<gix_hash::ObjectId> {
            self.checksum
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
