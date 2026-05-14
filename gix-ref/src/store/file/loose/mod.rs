use crate::{FullName, Kind, Target};

/// A git _ref_ which is stored in a file.
#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Hash, Clone)]
pub struct Reference {
    /// The path to uniquely identify this ref within its store.
    pub name: FullName,
    /// The target of the reference, either a symbolic reference by full name or an object by its id.
    pub target: Target,
}

impl Reference {
    /// Return the kind of ref.
    pub fn kind(&self) -> Kind {
        self.target.kind()
    }
}

///
pub(crate) mod reflog;

///
pub(crate) mod iter;
///
pub mod reference;

mod init {
    use std::path::PathBuf;

    use crate::store_impl::file;

    impl file::Store {
        /// Create a new instance at the given `git_dir`, which commonly is a standard git repository with a
        /// `refs/` subdirectory.
        /// Use [`Options`](crate::store::init::Options) to adjust settings.
        ///
        /// Note that if [`precompose_unicode`](crate::store::init::Options::precompose_unicode) is set in the options,
        /// the `git_dir` is also expected to use precomposed unicode, or else some operations that strip prefixes will fail.
        pub fn at(
            git_dir: PathBuf,
            crate::store::init::Options {
                write_reflog,
                object_hash,
                precompose_unicode,
                prohibit_windows_device_names,
            }: crate::store::init::Options,
        ) -> Self {
            file::Store {
                git_dir,
                packed_buffer_mmap_threshold: packed_refs_mmap_threshold(),
                common_dir: None,
                write_reflog,
                namespace: None,
                prohibit_windows_device_names,
                packed: gix_fs::SharedFileSnapshotMut::new().into(),
                object_hash,
                precompose_unicode,
            }
        }

        /// Like [`at()`][file::Store::at()], but for _linked_ work-trees which use `git_dir` as private ref store and `common_dir` for
        /// shared references.
        ///
        /// Note that if [`precompose_unicode`](crate::store::init::Options::precompose_unicode) is set, the `git_dir` and
        /// `common_dir` are also expected to use precomposed unicode, or else some operations that strip prefixes will fail.
        pub fn for_linked_worktree(
            git_dir: PathBuf,
            common_dir: PathBuf,
            crate::store::init::Options {
                write_reflog,
                object_hash,
                precompose_unicode,
                prohibit_windows_device_names,
            }: crate::store::init::Options,
        ) -> Self {
            file::Store {
                git_dir,
                packed_buffer_mmap_threshold: packed_refs_mmap_threshold(),
                common_dir: Some(common_dir),
                write_reflog,
                namespace: None,
                prohibit_windows_device_names,
                packed: gix_fs::SharedFileSnapshotMut::new().into(),
                object_hash,
                precompose_unicode,
            }
        }
    }

    fn packed_refs_mmap_threshold() -> u64 {
        if cfg!(windows) { u64::MAX } else { 32 * 1024 }
    }
}
