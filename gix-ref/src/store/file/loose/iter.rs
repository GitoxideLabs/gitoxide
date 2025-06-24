use std::path::{Path, PathBuf};

use gix_features::fs::walkdir::DirEntryIter;
use gix_object::bstr::ByteSlice;
use gix_path::RelativePath;

use crate::{file::iter::LooseThenPacked, store_impl::file, BString, FullName};

/// An iterator over all valid loose reference paths as seen from a particular base directory.
pub(in crate::store_impl::file) struct SortedLoosePaths {
    pub(crate) base: PathBuf,
    /// An prefix like `refs/heads/foo/` or `refs/heads/prefix` that a returned reference must match against..
    prefix: Option<BString>,
    file_walk: Option<DirEntryIter>,
}

impl SortedLoosePaths {
    pub fn at(path: &Path, base: PathBuf, prefix: Option<BString>, precompose_unicode: bool) -> Self {
        SortedLoosePaths {
            base,
            prefix,
            file_walk: path.is_dir().then(|| {
                // serial iteration as we expect most refs in packed-refs anyway.
                gix_features::fs::walkdir_sorted_new(
                    path,
                    gix_features::fs::walkdir::Parallelism::Serial,
                    usize::MAX,
                    precompose_unicode,
                )
                .into_iter()
            }),
        }
    }
}

impl Iterator for SortedLoosePaths {
    type Item = std::io::Result<(PathBuf, FullName)>;

    fn next(&mut self) -> Option<Self::Item> {
        for entry in self.file_walk.as_mut()?.by_ref() {
            match entry {
                Ok(entry) => {
                    if !entry.file_type().is_ok_and(|ft| ft.is_file()) {
                        continue;
                    }
                    let full_path = entry.path().into_owned();
                    let full_name = full_path
                        .strip_prefix(&self.base)
                        .expect("prefix-stripping cannot fail as base is within our root");
                    let Ok(full_name) = gix_path::try_into_bstr(full_name)
                        .map(|name| gix_path::to_unix_separators_on_windows(name).into_owned())
                    else {
                        continue;
                    };
                    if let Some(prefix) = &self.prefix {
                        if !full_name.starts_with(prefix) {
                            continue;
                        }
                    }
                    if gix_validate::reference::name_partial(full_name.as_bstr()).is_ok() {
                        let name = FullName(full_name);
                        return Some(Ok((full_path, name)));
                    } else {
                        continue;
                    }
                }
                Err(err) => return Some(Err(err.into_io_error().expect("no symlink related errors"))),
            }
        }
        None
    }
}

impl file::Store {
    /// Return an iterator over all loose references, notably not including any packed ones, in lexical order.
    /// Each of the references may fail to parse and the iterator will not stop if parsing fails, allowing the caller
    /// to see all files that look like references whether valid or not.
    ///
    /// Reference files that do not constitute valid names will be silently ignored.
    pub fn loose_iter(&self) -> std::io::Result<LooseThenPacked<'_, '_>> {
        self.iter_packed(None)
    }

    /// Return an iterator over all loose references that start with the given `prefix`.
    ///
    /// Otherwise, it's similar to [`loose_iter()`](file::Store::loose_iter()).
    ///
    /// Note that if a prefix isn't using a trailing `/`, like in `refs/heads/foo`, it will effectively
    /// start the traversal in the parent directory, e.g. `refs/heads/` and list everything inside that
    /// starts with `foo`, like `refs/heads/foo` and `refs/heads/foobar`.
    ///
    /// Prefixes are relative paths with slash-separated components.
    pub fn loose_iter_prefixed(&self, prefix: &RelativePath) -> std::io::Result<LooseThenPacked<'_, '_>> {
        self.iter_prefixed_packed(prefix, None)
    }
}
