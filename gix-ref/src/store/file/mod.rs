use std::path::PathBuf;

use crate::{store::WriteReflog, Namespace};

/// A store for reference which uses plain files.
///
/// Each ref is represented as a single file on disk in a folder structure that follows the relative path
/// used to identify [references][crate::Reference].
#[derive(Debug, Clone)]
pub struct Store {
    /// The location at which loose references can be found as per conventions of a typical git repository.
    ///
    /// Typical base paths are `.git` repository folders.
    git_dir: PathBuf,
    /// Possibly the common directory at which to find shared references. Only set if this `Store` is for a work tree.
    common_dir: Option<PathBuf>,
    /// The kind of hash to assume in a couple of situations. Note that currently we are able to read any valid hash from files
    /// which might want to change one day.
    object_hash: gix_hash::Kind,
    /// The amount of bytes needed for `mmap` to be used to open packed refs.
    packed_buffer_mmap_threshold: u64,

    /// The way to handle reflog edits
    pub write_reflog: WriteReflog,
    /// The namespace to use for edits and reads
    pub namespace: Option<Namespace>,
    /// This is only needed on Windows, where some device names are reserved at any level of a path, so that
    /// reading or writing `refs/heads/CON` for example would read from the console, or write to it.
    pub prohibit_windows_device_names: bool,
    /// If set, we will convert decomposed unicode like `a\u308` into precomposed unicode like `ä` when reading
    /// ref names from disk.
    /// Note that this is an internal operation that isn't observable on the outside, but it's needed for lookups
    /// to packed-refs or symlinks to work correctly.
    /// Iterated references will be returned verbatim, thus when sending them over the wire they have to be precomposed
    /// as needed.
    pub precompose_unicode: bool,
    /// A packed buffer which can be mapped in one version and shared as such.
    /// It's updated only in one spot, which is prior to reading it based on file stamps.
    /// Doing it like this has the benefit of being able to hand snapshots out to people without blocking others from updating it.
    packed: packed::modifiable::MutableSharedBuffer,
}

mod access {
    use std::path::Path;

    /// Mutation
    impl file::Store {
        /// Set the amount of `bytes` needed for the `.git/packed-refs` file to be memory mapped.
        /// Returns the previous value, which is always 32KB.
        pub fn set_packed_buffer_mmap_threshold(&mut self, mut bytes: u64) -> u64 {
            std::mem::swap(&mut self.packed_buffer_mmap_threshold, &mut bytes);
            bytes
        }
    }

    use crate::file;

    /// Access
    impl file::Store {
        /// Return the `.git` directory at which all references are loaded.
        ///
        /// For worktrees, this is the linked work-tree private ref location,
        /// then [`common_dir()`][file::Store::common_dir()] is `Some(parent_git_dir)`.
        pub fn git_dir(&self) -> &Path {
            &self.git_dir
        }

        /// If this is a linked work tree, there will be `Some(git_dir)` pointing to the parent repository,
        /// while [`git_dir()`][file::Store::git_dir()] points to the location holding linked work-tree private references.
        pub fn common_dir(&self) -> Option<&Path> {
            self.common_dir.as_deref()
        }

        /// Similar to [`common_dir()`][file::Store::common_dir()], but it will produce either the common-dir, or the git-dir if the former
        /// isn't present.
        ///
        /// This is also the directory in which the packed references file would be placed.
        pub fn common_dir_resolved(&self) -> &Path {
            self.common_dir.as_deref().unwrap_or(&self.git_dir)
        }
    }
}

/// A transaction on a file store
pub struct Transaction<'s, 'p> {
    store: &'s Store,
    packed_transaction: Option<crate::store_impl::packed::Transaction>,
    updates: Option<Vec<transaction::Edit>>,
    packed_refs: transaction::PackedRefs<'p>,
}

///
pub mod loose;
mod overlay_iter;

///
pub mod iter {
    pub use super::overlay_iter::{LooseThenPacked, Platform};

    ///
    pub mod loose_then_packed {
        pub use super::super::overlay_iter::Error;
    }
}

///
pub mod log;

///
pub mod find;

///
pub mod transaction;

///
pub mod packed;

mod raw_ext;
pub use raw_ext::ReferenceExt;
