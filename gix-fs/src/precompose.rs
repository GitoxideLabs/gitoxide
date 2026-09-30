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

    /// Obtain the file name of this entry, possibly with precomposed Unicode if enabled.
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
