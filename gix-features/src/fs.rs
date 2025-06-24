//! Filesystem utilities
//!
//! These are will be parallel if the `parallel` feature is enabled, at the expense of compiling additional dependencies
//! along with runtime costs for maintaining a global [`rayon`](https://docs.rs/rayon) thread pool.
//!
//! For information on how to use the [`WalkDir`] type, have a look at
// TODO: Move all this to `gix-fs` in a breaking change.

#[cfg(feature = "walkdir")]
mod shared {
    /// The desired level of parallelism.
    pub enum Parallelism {
        /// Do not parallelize at all by making a serial traversal on the current thread.
        Serial,
        /// Create a new thread pool for each traversal with up to 16 threads or the amount of logical cores of the machine.
        ThreadPoolPerTraversal {
            /// The base name of the threads we create as part of the thread-pool.
            thread_name: &'static str,
        },
    }
}

#[cfg(any(feature = "walkdir", feature = "fs-read-dir"))]
mod walkdir_precompose {
    use std::{borrow::Cow, ffi::OsStr, path::Path};

    #[derive(Debug)]
    pub struct DirEntry<T: std::fmt::Debug> {
        inner: T,
        precompose_unicode: bool,
    }

    impl<T: std::fmt::Debug> DirEntry<T> {
        /// Create a new instance.
        pub fn new(inner: T, precompose_unicode: bool) -> Self {
            Self {
                inner,
                precompose_unicode,
            }
        }
    }

    pub trait DirEntryApi {
        fn path(&self) -> Cow<'_, Path>;
        fn file_name(&self) -> Cow<'_, OsStr>;
        fn file_type(&self) -> std::io::Result<std::fs::FileType>;
    }

    impl<T: DirEntryApi + std::fmt::Debug> DirEntry<T> {
        /// Obtain the full path of this entry, possibly with precomposed unicode if enabled.
        ///
        /// Note that decomposing filesystem like those made by Apple accept both precomposed and
        /// decomposed names, and consider them equal.
        pub fn path(&self) -> Cow<'_, Path> {
            let path = self.inner.path();
            if self.precompose_unicode {
                gix_utils::str::precompose_path(path)
            } else {
                path
            }
        }

        /// Obtain filen name of this entry, possibly with precomposed unicode if enabled.
        pub fn file_name(&self) -> Cow<'_, OsStr> {
            let name = self.inner.file_name();
            if self.precompose_unicode {
                gix_utils::str::precompose_os_string(name)
            } else {
                name
            }
        }

        /// Return the file type for the file that this entry points to.
        ///
        /// If `follow_links` was `true`, this is the file type of the item the link points to.
        pub fn file_type(&self) -> std::io::Result<std::fs::FileType> {
            self.inner.file_type()
        }
    }

    /// A platform over entries in a directory, which may or may not precompose unicode after retrieving
    /// paths from the file system.
    #[cfg(feature = "walkdir")]
    pub struct WalkDir<T> {
        pub(crate) inner: Option<T>,
        pub(crate) precompose_unicode: bool,
    }

    #[cfg(feature = "walkdir")]
    pub struct WalkDirIter<T, I, E>
    where
        T: Iterator<Item = Result<I, E>>,
        I: DirEntryApi,
    {
        pub(crate) inner: T,
        pub(crate) precompose_unicode: bool,
    }

    #[cfg(feature = "walkdir")]
    impl<T, I, E> Iterator for WalkDirIter<T, I, E>
    where
        T: Iterator<Item = Result<I, E>>,
        I: DirEntryApi + std::fmt::Debug,
    {
        type Item = Result<DirEntry<I>, E>;

        fn next(&mut self) -> Option<Self::Item> {
            self.inner
                .next()
                .map(|res| res.map(|entry| DirEntry::new(entry, self.precompose_unicode)))
        }
    }
}

///
#[cfg(feature = "fs-read-dir")]
pub mod read_dir {
    use std::{borrow::Cow, ffi::OsStr, fs::FileType, path::Path};

    /// A directory entry adding precompose-unicode support to [`std::fs::DirEntry`].
    pub type DirEntry = super::walkdir_precompose::DirEntry<std::fs::DirEntry>;

    impl super::walkdir_precompose::DirEntryApi for std::fs::DirEntry {
        fn path(&self) -> Cow<'_, Path> {
            self.path().into()
        }

        fn file_name(&self) -> Cow<'_, OsStr> {
            self.file_name().into()
        }

        fn file_type(&self) -> std::io::Result<FileType> {
            self.file_type()
        }
    }
}

///
#[cfg(feature = "walkdir")]
pub mod walkdir {
    use std::{borrow::Cow, ffi::OsStr, fs::FileType, path::Path};

    pub use walkdir::Error;
    use walkdir::{DirEntry as DirEntryImpl, WalkDir as WalkDirImpl};

    /// A directory entry returned by [DirEntryIter].
    pub type DirEntry = super::walkdir_precompose::DirEntry<DirEntryImpl>;
    /// A platform to create a [DirEntryIter] from.
    pub type WalkDir = super::walkdir_precompose::WalkDir<WalkDirImpl>;

    pub use super::shared::Parallelism;

    impl super::walkdir_precompose::DirEntryApi for DirEntryImpl {
        fn path(&self) -> Cow<'_, Path> {
            self.path().into()
        }

        fn file_name(&self) -> Cow<'_, OsStr> {
            self.file_name().into()
        }

        fn file_type(&self) -> std::io::Result<FileType> {
            Ok(self.file_type())
        }
    }

    impl IntoIterator for WalkDir {
        type Item = Result<DirEntry, walkdir::Error>;
        type IntoIter = DirEntryIter;

        fn into_iter(self) -> Self::IntoIter {
            DirEntryIter {
                inner: self.inner.expect("always set (builder fix)").into_iter(),
                precompose_unicode: self.precompose_unicode,
            }
        }
    }

    impl WalkDir {
        /// Set the minimum component depth of paths of entries.
        pub fn min_depth(mut self, min: usize) -> Self {
            self.inner = Some(self.inner.take().expect("always set").min_depth(min));
            self
        }
        /// Set the maximum component depth of paths of entries.
        pub fn max_depth(mut self, max: usize) -> Self {
            self.inner = Some(self.inner.take().expect("always set").max_depth(max));
            self
        }
        /// Follow symbolic links.
        pub fn follow_links(mut self, toggle: bool) -> Self {
            self.inner = Some(self.inner.take().expect("always set").follow_links(toggle));
            self
        }
    }

    /// Instantiate a new directory iterator which will not skip hidden files, with the given level of `parallelism`.
    ///
    /// Use `precompose_unicode` to represent the `core.precomposeUnicode` configuration option.
    pub fn walkdir_new(root: &Path, _: Parallelism, precompose_unicode: bool) -> WalkDir {
        WalkDir {
            inner: WalkDirImpl::new(root).into(),
            precompose_unicode,
        }
    }

    /// Instantiate a new directory iterator which will not skip hidden files and is sorted, with the given level of `parallelism`.
    ///
    /// Use `precompose_unicode` to represent the `core.precomposeUnicode` configuration option.
    /// Use `max_depth` to limit the depth of the recursive walk.
    ///   * 0 -> Returns only the root path with no children
    ///   * 1 -> Root directory and children.
    ///   * 2..n -> Root directory, children and {n}-grandchildren
    pub fn walkdir_sorted_new(root: &Path, _: Parallelism, max_depth: usize, precompose_unicode: bool) -> WalkDir {
        WalkDir {
            inner: WalkDirImpl::new(root)
                .max_depth(max_depth)
                .sort_by(|a, b| {
                    let storage_a;
                    let storage_b;
                    let a_name = match gix_path::os_str_into_bstr(a.file_name()) {
                        Ok(f) => f,
                        Err(_) => {
                            storage_a = a.file_name().to_string_lossy();
                            storage_a.as_ref().into()
                        }
                    };
                    let b_name = match gix_path::os_str_into_bstr(b.file_name()) {
                        Ok(f) => f,
                        Err(_) => {
                            storage_b = b.file_name().to_string_lossy();
                            storage_b.as_ref().into()
                        }
                    };
                    // "common." < "common/" < "common0"
                    let common = a_name.len().min(b_name.len());
                    a_name[..common].cmp(&b_name[..common]).then_with(|| {
                        let a = a_name.get(common).or_else(|| a.file_type().is_dir().then_some(&b'/'));
                        let b = b_name.get(common).or_else(|| b.file_type().is_dir().then_some(&b'/'));
                        a.cmp(&b)
                    })
                })
                .into(),
            precompose_unicode,
        }
    }

    /// The Iterator yielding directory items
    pub type DirEntryIter = super::walkdir_precompose::WalkDirIter<walkdir::IntoIter, DirEntryImpl, walkdir::Error>;
}

#[cfg(feature = "walkdir")]
pub use self::walkdir::{walkdir_new, walkdir_sorted_new, WalkDir};

/// Prepare open options which won't follow symlinks when the file is opened.
///
/// Note: only effective on unix currently.
pub fn open_options_no_follow() -> std::fs::OpenOptions {
    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut options = std::fs::OpenOptions::new();
    #[cfg(unix)]
    {
        /// Make sure that it's impossible to follow through to the target of symlinks.
        /// Note that this will still follow symlinks in the path, which is what we assume
        /// has been checked separately.
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    options
}
