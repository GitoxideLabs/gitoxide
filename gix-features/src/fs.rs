//! Filesystem utilities
//!
//! These are will be parallel if the `parallel` feature is enabled, at the expense of compiling additional dependencies
//! along with runtime costs for maintaining a global [`rayon`](https://docs.rs/rayon) thread pool.
//!
//! For information on how to use the [`WalkDir`] type, have a look at
//! * [`jwalk::WalkDir`](https://docs.rs/jwalk/0.5.1/jwalk/type.WalkDir.html) if `parallel` feature is enabled
//! * [walkdir::WalkDir](https://docs.rs/walkdir/2.3.1/walkdir/struct.WalkDir.html) otherwise

#[cfg(any(feature = "walkdir", feature = "fs-walkdir-parallel"))]
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

#[cfg(any(feature = "walkdir", feature = "fs-walkdir-parallel", feature = "fs-read-dir"))]
mod walkdir_precompose {
    use std::borrow::Cow;
    use std::ffi::OsStr;
    use std::path::Path;

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
    #[cfg(any(feature = "walkdir", feature = "fs-walkdir-parallel"))]
    pub struct WalkDir<T> {
        pub(crate) inner: Option<T>,
        pub(crate) precompose_unicode: bool,
    }

    #[cfg(any(feature = "walkdir", feature = "fs-walkdir-parallel"))]
    pub struct WalkDirIter<T, I, E>
    where
        T: Iterator<Item = Result<I, E>>,
        I: DirEntryApi,
    {
        pub(crate) inner: T,
        pub(crate) precompose_unicode: bool,
    }

    #[cfg(any(feature = "walkdir", feature = "fs-walkdir-parallel"))]
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
    use std::borrow::Cow;
    use std::ffi::OsStr;
    use std::fs::FileType;
    use std::path::Path;

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
#[cfg(feature = "fs-walkdir-parallel")]
pub mod walkdir {
    use std::borrow::Cow;
    use std::ffi::OsStr;
    use std::fs::FileType;
    use std::path::Path;

    use jwalk::WalkDir as WalkDirImpl;
    pub use jwalk::{DirEntry as DirEntryGeneric, DirEntryIter as DirEntryIterGeneric, Error};

    pub use super::shared::Parallelism;

    type DirEntryImpl = DirEntryGeneric<((), ())>;

    /// A directory entry returned by [DirEntryIter].
    pub type DirEntry = super::walkdir_precompose::DirEntry<DirEntryImpl>;
    /// A platform to create a [DirEntryIter] from.
    pub type WalkDir = super::walkdir_precompose::WalkDir<WalkDirImpl>;

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
        type Item = Result<DirEntry, jwalk::Error>;
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

    impl From<Parallelism> for jwalk::Parallelism {
        fn from(v: Parallelism) -> Self {
            match v {
                Parallelism::Serial => jwalk::Parallelism::Serial,
                Parallelism::ThreadPoolPerTraversal { thread_name } => std::thread::available_parallelism()
                    .map_or_else(
                        |_| Parallelism::Serial.into(),
                        |threads| {
                            let pool = jwalk::rayon::ThreadPoolBuilder::new()
                                .num_threads(threads.get().min(16))
                                .stack_size(128 * 1024)
                                .thread_name(move |idx| format!("{thread_name} {idx}"))
                                .build()
                                .expect("we only set options that can't cause a build failure");
                            jwalk::Parallelism::RayonExistingPool {
                                pool: pool.into(),
                                busy_timeout: None,
                            }
                        },
                    ),
            }
        }
    }

    /// Instantiate a new directory iterator which will not skip hidden files, with the given level of `parallelism`.
    ///
    /// Use `precompose_unicode` to represent the `core.precomposeUnicode` configuration option.
    pub fn walkdir_new(root: &Path, parallelism: Parallelism, precompose_unicode: bool) -> WalkDir {
        WalkDir {
            inner: WalkDirImpl::new(root)
                .skip_hidden(false)
                .parallelism(parallelism.into())
                .into(),
            precompose_unicode,
        }
    }

    /// Instantiate a new directory iterator which will not skip hidden files and is sorted
    ///
    /// Use `precompose_unicode` to represent the `core.precomposeUnicode` configuration option.
    pub fn walkdir_sorted_new(root: &Path, parallelism: Parallelism, precompose_unicode: bool) -> WalkDir {
        WalkDir {
            inner: WalkDirImpl::new(root)
                .skip_hidden(false)
                .sort(true)
                .parallelism(parallelism.into())
                .into(),
            precompose_unicode,
        }
    }

    type DirEntryIterImpl = DirEntryIterGeneric<((), ())>;

    /// The Iterator yielding directory items
    pub type DirEntryIter = super::walkdir_precompose::WalkDirIter<DirEntryIterImpl, DirEntryImpl, jwalk::Error>;
}

///
#[cfg(all(feature = "walkdir", not(feature = "fs-walkdir-parallel")))]
pub mod walkdir {
    use std::borrow::Cow;
    use std::ffi::OsStr;
    use std::fs::FileType;
    use std::path::Path;

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
    pub fn walkdir_sorted_new(root: &Path, _: Parallelism, precompose_unicode: bool) -> WalkDir {
        fn ft_to_number(ft: std::fs::FileType) -> usize {
            if ft.is_file() {
                1
            } else {
                2
            }
        }
        WalkDir {
            inner: WalkDirImpl::new(root)
                .sort_by(|a, b| {
                    ft_to_number(a.file_type())
                        .cmp(&ft_to_number(b.file_type()))
                        .then_with(|| a.file_name().cmp(b.file_name()))
                })
                .into(),
            precompose_unicode,
        }
    }

    /// The Iterator yielding directory items
    pub type DirEntryIter = super::walkdir_precompose::WalkDirIter<walkdir::IntoIter, DirEntryImpl, walkdir::Error>;
}

#[cfg(any(feature = "walkdir", feature = "fs-walkdir-parallel"))]
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
