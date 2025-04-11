use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use gix_features::fs::walkdir::DirEntryIter;
use gix_object::bstr::ByteSlice;

use crate::{file::iter::LooseThenPacked, store_impl::file, BStr, BString, FullName};

/// An iterator over all valid loose reference paths as seen from a particular base directory.
pub(in crate::store_impl::file) struct SortedLoosePaths {
    pub(crate) base: PathBuf,
    /// A optional prefix to match against if the prefix is not the same as the `file_walk` path.
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
                    let full_name = match gix_path::try_into_bstr(full_name) {
                        Ok(name) => {
                            let name = gix_path::to_unix_separators_on_windows(name);
                            name.into_owned()
                        }
                        Err(_) => continue, // TODO: silently skipping ill-formed UTF-8 on windows here, maybe there are better ways?
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
    /// Otherwise it's similar to [`loose_iter()`][file::Store::loose_iter()].
    pub fn loose_iter_prefixed<'a>(
        &self,
        prefix: impl Into<Cow<'a, BStr>>,
    ) -> std::io::Result<LooseThenPacked<'_, '_>> {
        self.iter_prefixed_packed(prefix.into(), None)
    }
}
