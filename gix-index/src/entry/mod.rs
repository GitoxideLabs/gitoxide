/// The stage of an entry.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum Stage {
    /// This is the default, and most entries are in this stage.
    #[default]
    Unconflicted = 0,
    /// The entry is the common base between 'our' change and 'their' change, for comparison.
    Base = 1,
    /// The entry represents our change.
    Ours = 2,
    /// The entry represents their change.
    Theirs = 3,
}

// The stage of an entry, one of…
/// * 0 = no conflict,
/// * 1 = base,
/// * 2 = ours,
/// * 3 = theirs
pub type StageRaw = u32;

///
pub mod mode;

mod flags;
pub use flags::Flags;
pub(crate) use flags::at_rest;

///
pub mod stat;
mod write;

use bitflags::bitflags;

bitflags! {
    /// The kind of file of an entry.
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
    pub struct Mode: u32 {
        /// directory (only used for sparse checkouts), equivalent to a tree, which is _excluded_ from the index via
        /// cone-mode.
        const DIR = 0o040000;
        /// regular file
        const FILE = 0o100644;
        /// regular file, executable
        const FILE_EXECUTABLE = 0o100755;
        /// Symbolic link
        const SYMLINK = 0o120000;
        /// A git commit for submodules
        const COMMIT = 0o160000;
    }
}

/// An entry's filesystem stat information.
#[derive(Debug, Default, PartialEq, Eq, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Stat {
    /// Modification time
    pub mtime: stat::Time,
    /// Creation time
    pub ctime: stat::Time,
    /// Device number
    pub dev: u32,
    /// Inode number
    pub ino: u32,
    /// User id of the owner
    pub uid: u32,
    /// Group id of the owning group
    pub gid: u32,
    /// The size of bytes on disk. Capped to u32 so files bigger than that will need thorough additional checking
    pub size: u32,
}

mod access {
    use bstr::{BStr, ByteSlice};

    use crate::{Entry, State, entry};

    impl Entry {
        /// Return an entry's path, relative to the repository, which is extracted from its owning `state`.
        pub fn path<'a>(&self, state: &'a State) -> &'a BStr {
            state.path_backing[self.path.clone()].as_bstr()
        }

        /// Return an entry's path using the given `backing`.
        pub fn path_in<'backing>(&self, backing: &'backing crate::PathStorageRef) -> &'backing BStr {
            backing[self.path.clone()].as_bstr()
        }

        /// Return an entry's stage. See [entry::Stage] for possible values.
        pub fn stage(&self) -> entry::Stage {
            self.flags.stage()
        }

        /// Return an entry's stage as raw number between 0 and 4.
        /// Possible values are:
        ///
        /// * 0 = no conflict,
        /// * 1 = base,
        /// * 2 = ours,
        /// * 3 = theirs
        pub fn stage_raw(&self) -> u32 {
            self.flags.stage_raw()
        }
    }
}

mod _impls {
    use std::cmp::Ordering;

    use bstr::BStr;
    use gix_object::tree::EntryKind;

    use crate::{Entry, State, entry};

    impl From<EntryKind> for entry::Mode {
        fn from(value: EntryKind) -> Self {
            match value {
                EntryKind::Tree => entry::Mode::DIR,
                EntryKind::Blob => entry::Mode::FILE,
                EntryKind::BlobExecutable => entry::Mode::FILE_EXECUTABLE,
                EntryKind::Link => entry::Mode::SYMLINK,
                EntryKind::Commit => entry::Mode::COMMIT,
            }
        }
    }

    impl Entry {
        /// Compare one entry to another by their path, by comparing only their common path portion byte by byte, then resorting to
        /// entry length and stage.
        pub fn cmp(&self, other: &Self, state: &State) -> Ordering {
            let lhs = self.path(state);
            let rhs = other.path(state);
            Entry::cmp_filepaths(lhs, rhs).then_with(|| self.stage().cmp(&other.stage()))
        }

        /// Compare one entry to another by their path, by comparing only their common path portion byte by byte, then resorting to
        /// entry length.
        pub fn cmp_filepaths(a: &BStr, b: &BStr) -> Ordering {
            let common_len = a.len().min(b.len());
            a[..common_len]
                .cmp(&b[..common_len])
                .then_with(|| a.len().cmp(&b.len()))
        }
    }
}
