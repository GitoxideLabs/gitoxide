use std::{borrow::Cow, ffi::OsStr, fs::FileType, path::Path};

pub use walkdir::Error;
use walkdir::{DirEntry as DirEntryImpl, WalkDir as WalkDirImpl};

/// A directory entry returned by [DirEntryIter].
pub type DirEntry = crate::precompose::DirEntry<DirEntryImpl>;
/// A platform to create a [DirEntryIter] from.
pub type WalkDir = crate::precompose::WalkDir<WalkDirImpl>;

impl crate::precompose::DirEntryApi for DirEntryImpl {
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

/// Instantiate a new directory iterator which will not skip hidden files.
///
/// Use `precompose_unicode` to represent the `core.precomposeUnicode` configuration option.
pub fn walkdir_new(root: &Path, precompose_unicode: bool) -> WalkDir {
    WalkDir {
        inner: WalkDirImpl::new(root).into(),
        precompose_unicode,
    }
}

/// Instantiate  new directory iterator which will not skip hidden files and uses Git's directory-aware sorting.
///
/// Use `precompose_unicode` to represent the `core.precomposeUnicode` configuration option.
/// Use `max_depth` to limit the depth of the recursive walk.
///   * `0`
///       - Returns only the root path with no children
///   * `1`
///       - Root directory and children.
///   * `1..n`
///       - Root directory, children and {n}-grandchildren
pub fn walkdir_sorted_new(root: &Path, max_depth: usize, precompose_unicode: bool) -> WalkDir {
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
pub type DirEntryIter = crate::precompose::WalkDirIter<walkdir::IntoIter, DirEntryImpl, walkdir::Error>;
