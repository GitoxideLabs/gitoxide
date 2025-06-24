use std::path::PathBuf;

use gix_features::fs::walkdir::DirEntryIter;
use gix_object::bstr::ByteSlice;

use crate::{file::overlay_iter, BString, FullName, Reference};

/// An iterator over all pseudo references in a given git directory
pub struct SortedPseudoRefPaths {
    pub(crate) git_dir: PathBuf,
    file_walk: Option<DirEntryIter>,
}

impl SortedPseudoRefPaths {
    /// Returns an iterator over the pseudo ref paths found at the given git_dir
    pub fn at(git_dir: PathBuf, precompose_unicode: bool) -> Self {
        SortedPseudoRefPaths {
            git_dir: git_dir.to_owned(),
            file_walk: git_dir.is_dir().then(|| {
                // serial iteration as we expect most refs in packed-refs anyway.
                gix_features::fs::walkdir_sorted_new(
                    &git_dir,
                    gix_features::fs::walkdir::Parallelism::Serial,
                    // In a given git directory pseudo refs are only at the root
                    1,
                    precompose_unicode,
                )
                .into_iter()
            }),
        }
    }
}

impl Iterator for SortedPseudoRefPaths {
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
                        .strip_prefix(&self.git_dir)
                        .expect("prefix-stripping cannot fail as base is within our root");
                    let Ok(full_name) = gix_path::try_into_bstr(full_name)
                        .map(|name| gix_path::to_unix_separators_on_windows(name).into_owned())
                    else {
                        continue;
                    };
                    // Pseudo refs must end with "HEAD"
                    if !full_name.ends_with(&BString::from("HEAD")) {
                        continue;
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

/// An iterator over all pseudo references in a given git directory
pub struct SortedPseudoRefIterator {
    pub(crate) git_dir: PathBuf,
    inner: SortedPseudoRefPaths,
    buf: Vec<u8>,
}

impl SortedPseudoRefIterator {
    /// Returns an iterator over the pseudo ref paths found at the given git_dir
    pub fn at(git_dir: &PathBuf, precompose_unicode: bool) -> Self {
        SortedPseudoRefIterator {
            inner: SortedPseudoRefPaths::at(git_dir.to_owned(), precompose_unicode),
            git_dir: git_dir.to_owned(),
            buf: vec![],
        }
    }
}

impl Iterator for SortedPseudoRefIterator {
    type Item = Result<Reference, overlay_iter::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|r| overlay_iter::convert_loose(&mut self.buf, &self.git_dir, None, None, r))
    }
}
