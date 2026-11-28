use gix_error::{Result, ResultExt, message};

use gix_object::bstr::ByteSlice;
use gix_path::RelativePath;
use std::{
    borrow::Cow,
    cmp::Ordering,
    io::Read,
    iter::Peekable,
    path::{Path, PathBuf},
};

use crate::{
    BStr, FullNameRef, Namespace, Reference,
    file::loose::{self, iter::SortedLoosePaths},
    store_impl::{file, packed},
};

/// An iterator stepping through sorted input of loose references and packed references, preferring loose refs over otherwise
/// equivalent packed references.
///
/// All errors will be returned verbatim, while packed errors are depleted first if loose refs also error.
pub struct LooseThenPacked<'p, 's> {
    git_dir: &'s Path,
    common_dir: Option<&'s Path>,
    object_hash: gix_hash::Kind,
    namespace: Option<&'s Namespace>,
    iter_packed: Option<Peekable<packed::Iter<'p>>>,
    iter_git_dir: SortedLoosePaths,
    iter_common_dir: Option<SortedLoosePaths>,
    buf: Vec<u8>,
    /// The store whose `packed-refs` file is read afresh when a loose reference disappears after its directory was listed.
    vanished_loose_refs_store: Option<&'s file::Store>,
}

enum IterKind {
    Git,
    GitAndConsumeCommon,
    Common,
}

/// An intermediate structure to hold shared state alive long enough for iteration to happen.
#[must_use = "Iterators should be obtained from this platform"]
pub struct Platform<'s> {
    store: &'s file::Store,
    packed: Option<file::packed::SharedBufferSnapshot>,
}

impl<'p, 's> LooseThenPacked<'p, 's> {
    fn strip_namespace(&self, mut r: Reference) -> Reference {
        if let Some(namespace) = &self.namespace {
            r.strip_namespace(namespace);
        }
        r
    }

    fn convert_packed(&self, packed: Result<packed::Reference<'p>>, name_buf: &mut Vec<u8>) -> Result<Reference> {
        packed.map(|r| self.strip_namespace(r.into_reference(name_buf)))
    }

    /// Read failures include [metadata](gix_error::Error::metadata()) `path` (native path), the loose reference being
    /// visited.
    ///
    /// Return `None` if the loose reference was deleted after its directory was listed.
    fn next_loose(
        &mut self,
        kind: IterKind,
        name_buf: &mut Vec<u8>,
        symbolic_target_buf: impl FnOnce() -> Vec<u8>,
    ) -> Result<Option<Reference>> {
        let iter = match kind {
            IterKind::GitAndConsumeCommon => {
                drop(self.iter_common_dir.as_mut().and_then(SortedLoosePaths::take_peeked));
                &mut self.iter_git_dir
            }
            IterKind::Git => &mut self.iter_git_dir,
            IterKind::Common => self.iter_common_dir.as_mut().expect("peeked a common ref"),
        };
        let entry = iter
            .take_peeked()
            .expect("the loose ref was peeked")
            .or_raise(|| message("Could not traverse reference directory"))?;
        let buf = &mut self.buf;
        let git_dir = self.git_dir;
        let common_dir = self.common_dir;
        let read = std::fs::File::open(entry.path).and_then(|mut f| {
            buf.clear();
            f.read_to_end(buf)
        });
        match (read, self.vanished_loose_refs_store) {
            (Err(err), Some(store)) if err.kind() == std::io::ErrorKind::NotFound => {
                // `git pack-refs --prune` writes `packed-refs` before it removes loose files,
                // so a fresh `packed-refs` has the reference unless it was deleted.
                let Some(packed) = store.open_packed_buffer()? else {
                    return Ok(None);
                };
                let reference = packed
                    .try_find_full_name(entry.name)?
                    .map(|packed_ref| packed_ref.into_reference(name_buf));
                return Ok(reference.map(|reference| self.strip_namespace(reference)));
            }
            (read, _) => read.or_raise(|| file::find::read_reference_error(entry.path))?,
        };
        let target = loose::Reference::decode_target(buf, self.object_hash, symbolic_target_buf).or_raise(|| {
            let relative_path = entry
                .path
                .strip_prefix(git_dir)
                .ok()
                .or_else(|| common_dir.and_then(|common_dir| entry.path.strip_prefix(common_dir).ok()))
                .expect("one of our bases contains the path");
            file::find::ReferenceDecode {
                relative_path: relative_path.into(),
            }
        })?;
        let reference = Reference {
            name: entry.name.to_owned_with_buffer(name_buf),
            target,
            peeled: None,
        };
        Ok(Some(self.strip_namespace(reference)))
    }

    fn look_up_vanished_loose_refs_in(mut self, store: &'s file::Store) -> Self {
        self.vanished_loose_refs_store = Some(store);
        self
    }
}

impl LooseThenPacked<'_, '_> {
    /// Like [`Iterator::next()`], but reuse storage for the returned reference's name and symbolic target name.
    ///
    /// On success, the iterator clears `name_buf` and moves its allocation into the reference name.
    /// It calls `symbolic_target_buf` only after validating a symbolic target name and moves the returned
    /// buffer's allocation into that name. Recover either allocation with
    /// [`FullName::into_inner()`](crate::FullName::into_inner()).
    pub fn next_with_buffers(
        &mut self,
        name_buf: &mut Vec<u8>,
        symbolic_target_buf: impl FnOnce() -> Vec<u8>,
    ) -> Option<Result<Reference>> {
        fn peek_loose<'a>(
            git_dir: &'a mut SortedLoosePaths,
            common_dir: Option<&'a mut SortedLoosePaths>,
        ) -> Option<(std::result::Result<&'a FullNameRef, ()>, IterKind)> {
            let git = git_dir.peek_name(false);
            match common_dir {
                Some(common_dir) => match (git, common_dir.peek_name(true)) {
                    (None, None) => None,
                    (None, Some(res)) | (Some(_), Some(res @ Err(_))) => Some((res, IterKind::Common)),
                    (Some(res), None) | (Some(res @ Err(_)), Some(_)) => Some((res, IterKind::Git)),
                    (Some(r_gitdir @ Ok(git_name)), Some(r_cd @ Ok(common_name))) => match git_name.cmp(common_name) {
                        Ordering::Less => Some((r_gitdir, IterKind::Git)),
                        Ordering::Equal => Some((r_gitdir, IterKind::GitAndConsumeCommon)),
                        Ordering::Greater => Some((r_cd, IterKind::Common)),
                    },
                },
                None => git.map(|r| (r, IterKind::Git)),
            }
        }
        let mut symbolic_target_buf = Some(symbolic_target_buf);
        loop {
            let kind = match self.iter_packed.as_mut() {
                Some(packed_iter) => match (
                    peek_loose(&mut self.iter_git_dir, self.iter_common_dir.as_mut()),
                    packed_iter.peek(),
                ) {
                    (None, None) => return None,
                    (None, Some(_)) | (Some(_), Some(Err(_))) => {
                        let res = packed_iter.next().expect("peeked value exists");
                        return Some(self.convert_packed(res, name_buf));
                    }
                    (Some((_, kind)), None) | (Some((Err(_), kind)), Some(_)) => kind,
                    (Some((Ok(name), kind)), Some(Ok(packed))) => match name.cmp(packed.name) {
                        Ordering::Less => kind,
                        Ordering::Equal => {
                            drop(packed_iter.next());
                            kind
                        }
                        Ordering::Greater => {
                            let res = packed_iter.next().expect("name retrieval configured");
                            return Some(self.convert_packed(res, name_buf));
                        }
                    },
                },
                None => peek_loose(&mut self.iter_git_dir, self.iter_common_dir.as_mut())?.1,
            };
            let symbolic_target_buf = || symbolic_target_buf.take().expect("taken only for a returned reference")();
            if let Some(res) = self.next_loose(kind, name_buf, symbolic_target_buf).transpose() {
                return Some(res);
            }
        }
    }
}

impl Iterator for LooseThenPacked<'_, '_> {
    type Item = Result<Reference>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_with_buffers(&mut Vec::new(), Vec::new)
    }
}

impl<'repo> Platform<'repo> {
    /// Return an iterator over all references, loose or packed, sorted by their name.
    ///
    /// Errors are returned similarly to what would happen when loose and packed refs were iterated by themselves.
    ///
    /// A loose reference whose file disappears after its directory was listed, as `git pack-refs --prune` or a
    /// deletion does, is read from `packed-refs` afresh, and skipped if it isn't there either.
    pub fn all<'p>(&'p self) -> std::io::Result<LooseThenPacked<'p, 'repo>> {
        self.store
            .iter_packed(self.packed.as_ref().map(|b| &***b))
            .map(|iter| iter.look_up_vanished_loose_refs_in(self.store))
    }

    /// As [`iter(…)`](file::Store::iter()), but filters by `prefix`, i.e. "refs/heads/" or
    /// "refs/heads/feature-".
    ///
    /// Note that if a prefix isn't using a trailing `/`, like in `refs/heads/foo`, it will effectively
    /// start the traversal in the parent directory, e.g. `refs/heads/` and list everything inside that
    /// starts with `foo`, like `refs/heads/foo` and `refs/heads/foobar`.
    ///
    /// Prefixes are relative paths with slash-separated components.
    pub fn prefixed<'p>(&'p self, prefix: &RelativePath) -> std::io::Result<LooseThenPacked<'p, 'repo>> {
        self.store
            .iter_prefixed_packed(prefix, self.packed.as_ref().map(|b| &***b))
            .map(|iter| iter.look_up_vanished_loose_refs_in(self.store))
    }

    /// Return an iterator over the pseudo references, like `HEAD` or `FETCH_HEAD`, or anything else suffixed with `HEAD`
    /// in the root of the `.git` directory, sorted by name.
    pub fn pseudo<'p>(&'p self) -> std::io::Result<LooseThenPacked<'p, 'repo>> {
        self.store.iter_pseudo()
    }
}

impl file::Store {
    /// Return a platform to obtain iterator over all references, or prefixed ones, loose or packed, sorted by their name.
    ///
    /// Errors are returned similarly to what would happen when loose and packed refs were iterated by themselves.
    /// Loose iteration reuses path and sorting storage across directory entries.
    ///
    /// Note that since packed-refs are storing refs as precomposed unicode if [`Self::precompose_unicode`] is true, for consistency
    /// we also return loose references as precomposed unicode.
    pub fn iter(&self) -> Result<Platform<'_>> {
        Ok(Platform {
            store: self,
            packed: self.assure_packed_refs_uptodate()?,
        })
    }
}

#[derive(Debug)]
pub(crate) enum IterInfo<'a> {
    Base {
        base: &'a Path,
        precompose_unicode: bool,
    },
    BaseAndIterRoot {
        base: &'a Path,
        iter_root: PathBuf,
        prefix: PathBuf,
        precompose_unicode: bool,
    },
    PrefixAndBase {
        base: &'a Path,
        prefix: &'a Path,
        precompose_unicode: bool,
    },
    ComputedIterationRoot {
        /// The root to iterate over
        iter_root: PathBuf,
        /// The top-level directory as boundary of all references, used to create their short-names after iteration.
        base: &'a Path,
        /// The original prefix.
        prefix: Cow<'a, BStr>,
        /// If `true`, we will convert decomposed into precomposed unicode.
        precompose_unicode: bool,
    },
    Pseudo {
        base: &'a Path,
        precompose_unicode: bool,
    },
}

impl<'a> IterInfo<'a> {
    fn prefix(&self) -> Result<Option<Cow<'_, BStr>>> {
        match self {
            IterInfo::Base { .. } => None,
            IterInfo::PrefixAndBase { prefix, .. } => Some(gix_path::into_bstr(*prefix)),
            IterInfo::BaseAndIterRoot { prefix, .. } => Some(gix_path::into_bstr(prefix.clone())),
            IterInfo::ComputedIterationRoot { prefix, .. } => Some(Ok(prefix.clone())),
            IterInfo::Pseudo { .. } => None,
        }
        .transpose()
    }

    fn into_iter(self) -> SortedLoosePaths {
        match self {
            IterInfo::Base {
                base,
                precompose_unicode,
            } => SortedLoosePaths::at(base.join("refs"), base, None, None, precompose_unicode),
            IterInfo::BaseAndIterRoot {
                base,
                iter_root,
                prefix: _,
                precompose_unicode,
            } => SortedLoosePaths::at(iter_root, base, None, None, precompose_unicode),
            IterInfo::PrefixAndBase {
                base,
                prefix,
                precompose_unicode,
            } => SortedLoosePaths::at(base.join(prefix), base, None, None, precompose_unicode),
            IterInfo::ComputedIterationRoot {
                iter_root,
                base,
                prefix,
                precompose_unicode,
            } => SortedLoosePaths::at(iter_root, base, Some(prefix.into_owned()), None, precompose_unicode),
            IterInfo::Pseudo {
                base,
                precompose_unicode,
            } => SortedLoosePaths::at(base.into(), base, None, Some("HEAD".into()), precompose_unicode),
        }
    }

    fn from_prefix(base: &'a Path, prefix: &'a RelativePath, precompose_unicode: bool) -> std::io::Result<Self> {
        let prefix_path = gix_path::from_bstr(prefix.as_ref().as_bstr())
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
        let iter_root = base.join(&prefix_path);
        if prefix.as_ref().ends_with(b"/") {
            Ok(IterInfo::BaseAndIterRoot {
                base,
                iter_root,
                prefix: prefix_path.into_owned(),
                precompose_unicode,
            })
        } else {
            let iter_root = iter_root
                .parent()
                .expect("a parent is always there unless empty")
                .to_owned();
            Ok(IterInfo::ComputedIterationRoot {
                base,
                prefix: prefix.as_ref().as_bstr().into(),
                iter_root,
                precompose_unicode,
            })
        }
    }
}

impl file::Store {
    /// Return an iterator over all references, loose or `packed`, sorted by their name.
    ///
    /// Errors are returned similarly to what would happen when loose and packed refs were iterated by themselves.
    pub fn iter_packed<'s, 'p>(
        &'s self,
        packed: Option<&'p packed::Buffer>,
    ) -> std::io::Result<LooseThenPacked<'p, 's>> {
        match self.namespace.as_ref() {
            Some(namespace) => {
                let prefix = namespace
                    .to_path()
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                self.iter_from_info(
                    IterInfo::PrefixAndBase {
                        base: self.git_dir(),
                        prefix,
                        precompose_unicode: self.precompose_unicode,
                    },
                    self.common_dir().map(|base| IterInfo::PrefixAndBase {
                        base,
                        prefix,
                        precompose_unicode: self.precompose_unicode,
                    }),
                    packed,
                )
            }
            None => self.iter_from_info(
                IterInfo::Base {
                    base: self.git_dir(),
                    precompose_unicode: self.precompose_unicode,
                },
                self.common_dir().map(|base| IterInfo::Base {
                    base,
                    precompose_unicode: self.precompose_unicode,
                }),
                packed,
            ),
        }
    }

    /// Return an iterator over the pseudo references, like `HEAD` or `FETCH_HEAD`, or anything else suffixed with `HEAD`
    /// in the root of the `.git` directory, sorted by name.
    ///
    /// Errors are returned similarly to what would happen when loose refs were iterated by themselves.
    pub fn iter_pseudo<'p>(&'_ self) -> std::io::Result<LooseThenPacked<'p, '_>> {
        self.iter_from_info(
            IterInfo::Pseudo {
                base: self.git_dir(),
                precompose_unicode: self.precompose_unicode,
            },
            None,
            None,
        )
    }

    /// As [`iter(…)`](file::Store::iter()), but filters by `prefix`, i.e. `refs/heads/` or
    /// `refs/heads/feature-`.
    /// Note that if a prefix isn't using a trailing `/`, like in `refs/heads/foo`, it will effectively
    /// start the traversal in the parent directory, e.g. `refs/heads/` and list everything inside that
    /// starts with `foo`, like `refs/heads/foo` and `refs/heads/foobar`.
    ///
    /// Prefixes are relative paths with slash-separated components.
    pub fn iter_prefixed_packed<'s, 'p>(
        &'s self,
        prefix: &RelativePath,
        packed: Option<&'p packed::Buffer>,
    ) -> std::io::Result<LooseThenPacked<'p, 's>> {
        match self.namespace.as_ref() {
            None => {
                let git_dir_info = IterInfo::from_prefix(self.git_dir(), prefix, self.precompose_unicode)?;
                let common_dir_info = self
                    .common_dir()
                    .map(|base| IterInfo::from_prefix(base, prefix, self.precompose_unicode))
                    .transpose()?;
                self.iter_from_info(git_dir_info, common_dir_info, packed)
            }
            Some(namespace) => {
                let prefix = namespace.to_owned().into_namespaced_prefix(prefix);
                let prefix = prefix.as_bstr().try_into().map_err(std::io::Error::other)?;
                let git_dir_info = IterInfo::from_prefix(self.git_dir(), prefix, self.precompose_unicode)?;
                let common_dir_info = self
                    .common_dir()
                    .map(|base| IterInfo::from_prefix(base, prefix, self.precompose_unicode))
                    .transpose()?;
                self.iter_from_info(git_dir_info, common_dir_info, packed)
            }
        }
    }

    fn iter_from_info<'s, 'p>(
        &'s self,
        git_dir_info: IterInfo<'_>,
        common_dir_info: Option<IterInfo<'_>>,
        packed: Option<&'p packed::Buffer>,
    ) -> std::io::Result<LooseThenPacked<'p, 's>> {
        Ok(LooseThenPacked {
            git_dir: self.git_dir(),
            common_dir: self.common_dir(),
            object_hash: self.object_hash,
            iter_packed: match packed {
                Some(packed) => Some(
                    match git_dir_info
                        .prefix()
                        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?
                    {
                        Some(prefix) => packed.iter_prefixed(prefix.into_owned()),
                        None => packed.iter(),
                    }
                    .map_err(std::io::Error::other)?
                    .peekable(),
                ),
                None => None,
            },
            iter_git_dir: git_dir_info.into_iter(),
            iter_common_dir: common_dir_info.map(IterInfo::into_iter),
            buf: Vec::new(),
            namespace: self.namespace.as_ref(),
            vanished_loose_refs_store: None,
        })
    }
}
