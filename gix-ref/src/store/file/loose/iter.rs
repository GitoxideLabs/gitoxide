use std::{
    borrow::Cow,
    collections::VecDeque,
    io,
    ops::Range,
    path::{Path, PathBuf},
};

use gix_object::bstr::ByteSlice;
use gix_path::RelativePath;

use crate::{BString, FullNameRef, file::iter::LooseThenPacked, store_impl::file};

/// A loose reference discovered during traversal, before its contents are read.
///
/// Both fields borrow from the walk's reusable buffers.
pub(in crate::store_impl::file) struct LoosePath<'a> {
    /// The full native filesystem path used to open the reference file.
    pub path: &'a Path,
    /// The validated, base-relative Git ref name with `/` separators.
    pub name: &'a FullNameRef,
}

/// A sorted walk over loose refs that lends paths until the next advance.
pub(in crate::store_impl::file) struct SortedLoosePaths {
    path: PathBuf,
    name: BString,
    // Frames remain available at each depth when walking sibling directories.
    directories: Vec<Directory>,
    depth: usize,

    open_root: bool,
    path_has_ref: bool,
    peeked: Option<io::Result<()>>,
    prefix: Option<BString>,
    suffix: Option<BString>,
    precompose_unicode: bool,
}

#[derive(Default)]
struct Directory {
    names: Vec<u8>,
    entries: Vec<Entry>,
    errors: VecDeque<io::Error>,
    prefix_len: usize,
}

struct Entry {
    file_name: Range<usize>,
    sort_name: Range<usize>,
    is_dir: bool,
}

impl Directory {
    fn read(&mut self, path: &Path, prefix_len: usize, precompose_unicode: bool) -> io::Result<()> {
        self.names.clear();
        self.entries.clear();
        self.errors.clear();
        self.prefix_len = prefix_len;
        for entry in std::fs::read_dir(path)? {
            let (kind, entry) = match entry.and_then(|entry| Ok((entry.file_type()?, entry))) {
                Ok(entry) => entry,
                Err(err) => {
                    self.errors.push_back(err);
                    continue;
                }
            };
            let is_dir = kind.is_dir();
            if !is_dir && !kind.is_file() {
                continue;
            }
            let native_name = entry.file_name();
            let Ok(name) = gix_path::os_str_into_bstr(&native_name) else {
                continue;
            };
            let file_name = self.push_name(name);
            let mut sort_name = file_name.clone();
            if precompose_unicode
                && let Cow::Owned(name) = gix_utils::str::precompose_os_string(native_name.as_os_str().into())
            {
                sort_name = self.push_name(name.as_encoded_bytes());
            }
            // A directory's children sort at its trailing slash, as in `foo.`, `foo/bar`, `foo0`.
            if is_dir {
                self.names.push(b'/');
                sort_name.end += 1;
            }
            self.entries.push(Entry {
                file_name,
                sort_name,
                is_dir,
            });
        }
        self.entries.sort_unstable_by(|a, b| {
            self.names[b.sort_name.clone()]
                .cmp(&self.names[a.sort_name.clone()])
                .then_with(|| self.names[b.file_name.clone()].cmp(&self.names[a.file_name.clone()]))
        });
        Ok(())
    }

    fn push_name(&mut self, name: &[u8]) -> Range<usize> {
        let start = self.names.len();
        self.names.extend_from_slice(name);
        start..self.names.len()
    }
}

impl SortedLoosePaths {
    pub fn at(
        path: PathBuf,
        base: &Path,
        prefix: Option<BString>,
        suffix: Option<BString>,
        precompose_unicode: bool,
    ) -> Self {
        let relative = path.strip_prefix(base).expect("the walk starts inside its base");
        let relative = if precompose_unicode {
            gix_utils::str::precompose_path(relative.into())
        } else {
            Cow::Borrowed(relative)
        };
        let name = gix_path::into_bstr(relative)
            .map(gix_path::to_unix_separators_on_windows)
            .ok();
        let open_root = name.is_some() && path.is_dir();
        let mut name = name.map(Cow::into_owned).unwrap_or_default();
        if !name.is_empty() && !name.ends_with(b"/") {
            name.push(b'/');
        }
        Self {
            path,
            name,
            directories: Vec::new(),
            depth: 0,

            open_root,
            path_has_ref: false,
            peeked: None,
            prefix,
            suffix,
            precompose_unicode,
        }
    }

    pub fn peek_name(&mut self, skip_worktree_private: bool) -> Option<Result<&FullNameRef, ()>> {
        if self.peeked.is_none() {
            self.peeked = self.advance(skip_worktree_private);
        }
        self.peeked.as_ref().map(|entry| {
            entry
                .as_ref()
                .map(|()| FullNameRef::new_unchecked(self.name.as_bstr()))
                .map_err(|_| ())
        })
    }

    pub fn take_peeked(&mut self) -> Option<io::Result<LoosePath<'_>>> {
        self.peeked.take().map(|entry| {
            entry.map(|()| LoosePath {
                path: &self.path,
                name: FullNameRef::new_unchecked(self.name.as_bstr()),
            })
        })
    }

    fn enter_directory(&mut self) -> io::Result<()> {
        if self.depth == self.directories.len() {
            self.directories.push(Directory::default());
        }
        self.directories[self.depth].read(&self.path, self.name.len(), self.precompose_unicode)?;
        self.depth += 1;
        Ok(())
    }

    fn advance(&mut self, skip_worktree_private: bool) -> Option<io::Result<()>> {
        if std::mem::take(&mut self.path_has_ref) {
            self.path.pop();
        }
        if std::mem::take(&mut self.open_root)
            && let Err(err) = self.enter_directory()
        {
            return Some(Err(err));
        }
        while self.depth > 0 {
            let dir = &mut self.directories[self.depth - 1];
            self.name.truncate(dir.prefix_len);
            if let Some(err) = dir.errors.pop_front() {
                return Some(Err(err));
            }
            let Some(entry) = dir.entries.pop() else {
                self.depth -= 1;
                if self.depth != 0 {
                    self.path.pop();
                }
                continue;
            };
            if entry.is_dir && self.suffix.is_some() {
                continue;
            }
            self.name.extend_from_slice(&dir.names[entry.sort_name]);
            if !entry.is_dir {
                let name = self.name.as_bstr();
                if self.prefix.as_ref().is_some_and(|prefix| !name.starts_with(prefix))
                    || self.suffix.as_ref().is_some_and(|suffix| !name.ends_with(suffix))
                    || gix_validate::reference::name_partial(name).is_err()
                    || (skip_worktree_private
                        && FullNameRef::new_unchecked(name)
                            .category()
                            .is_some_and(|cat| cat.is_worktree_private()))
                {
                    continue;
                }
            }
            let name = gix_path::from_byte_slice(&dir.names[entry.file_name]).expect("entries store native path bytes");
            self.path.push(name);
            if entry.is_dir {
                if let Err(err) = self.enter_directory() {
                    self.path.pop();
                    return Some(Err(err));
                }
            } else {
                self.path_has_ref = true;
                return Some(Ok(()));
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
