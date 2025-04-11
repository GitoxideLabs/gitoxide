//!
#![allow(clippy::empty_docs)]
use std::borrow::Cow;

use gix_ref::{
    bstr::{BStr, ByteSlice},
    file::ReferenceExt,
};

/// A platform to create iterators over references.
#[must_use = "Iterators should be obtained from this iterator platform"]
pub struct Platform<'r> {
    pub(crate) platform: gix_ref::file::iter::Platform<'r>,
    /// The owning repository.
    pub repo: &'r crate::Repository,
}

/// An iterator over references, with or without filter.
pub struct Iter<'r> {
    inner: gix_ref::file::iter::LooseThenPacked<'r, 'r>,
    peel_with_packed: Option<gix_ref::file::packed::SharedBufferSnapshot>,
    peel: bool,
    repo: &'r crate::Repository,
}

impl<'r> Iter<'r> {
    fn new(repo: &'r crate::Repository, platform: gix_ref::file::iter::LooseThenPacked<'r, 'r>) -> Self {
        Iter {
            inner: platform,
            peel_with_packed: None,
            peel: false,
            repo,
        }
    }
}

impl Platform<'_> {
    /// Return an iterator over all references in the repository.
    ///
    /// Even broken or otherwise unparsable or inaccessible references are returned and have to be handled by the caller on a
    /// case by case basis.
    pub fn all(&self) -> Result<Iter<'_>, init::Error> {
        Ok(Iter::new(self.repo, self.platform.all()?))
    }

    /// Return an iterator over all references that match the given `prefix`.
    ///
    /// These are of the form `refs/heads/` or `refs/remotes/origin`, and must not contain relative paths components like `.` or `..`.
    // TODO: Create a custom `Path` type that enforces the requirements of git naturally, this type is surprising possibly on windows
    //       and when not using a trailing '/' to signal directories.
    pub fn prefixed<'a>(&self, prefix: impl Into<Cow<'a, BStr>>) -> Result<Iter<'_>, init::Error> {
        Ok(Iter::new(self.repo, self.platform.prefixed(prefix.into())?))
    }

    // TODO: tests
    /// Return an iterator over all references that are tags.
    ///
    /// They are all prefixed with `refs/tags`.
    pub fn tags(&self) -> Result<Iter<'_>, init::Error> {
        Ok(Iter::new(self.repo, self.platform.prefixed(b"refs/tags/".as_bstr())?))
    }

    // TODO: tests
    /// Return an iterator over all local branches.
    ///
    /// They are all prefixed with `refs/heads`.
    pub fn local_branches(&self) -> Result<Iter<'_>, init::Error> {
        Ok(Iter::new(self.repo, self.platform.prefixed(b"refs/heads/".as_bstr())?))
    }

    // TODO: tests
    /// Return an iterator over all remote branches.
    ///
    /// They are all prefixed with `refs/remotes`.
    pub fn remote_branches(&self) -> Result<Iter<'_>, init::Error> {
        Ok(Iter::new(
            self.repo,
            self.platform.prefixed(b"refs/remotes/".as_bstr())?,
        ))
    }
}

impl Iter<'_> {
    /// Automatically peel references before yielding them during iteration.
    ///
    /// This has the same effect as using `iter.map(|r| {r.peel_to_id_in_place(); r})`.
    ///
    /// # Note
    ///
    /// Doing this is necessary as the packed-refs buffer is already held by the iterator, disallowing the consumer of the iterator
    /// to peel the returned references themselves.
    pub fn peeled(mut self) -> Result<Self, gix_ref::packed::buffer::open::Error> {
        self.peel_with_packed = self.repo.refs.cached_packed_buffer()?;
        self.peel = true;
        Ok(self)
    }
}

impl<'r> Iterator for Iter<'r> {
    type Item = Result<crate::Reference<'r>, Box<dyn std::error::Error + Send + Sync + 'static>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|res| {
            res.map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync + 'static>)
                .and_then(|mut r| {
                    if self.peel {
                        let repo = &self.repo;
                        r.peel_to_id_in_place_packed(
                            &repo.refs,
                            &repo.objects,
                            self.peel_with_packed.as_ref().map(|p| &***p),
                        )
                        .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync + 'static>)
                        .map(|_| r)
                    } else {
                        Ok(r)
                    }
                })
                .map(|r| crate::Reference::from_ref(r, self.repo))
        })
    }
}

///
pub mod init {
    /// The error returned by [`Platform::all()`][super::Platform::all()] or [`Platform::prefixed()`][super::Platform::prefixed()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        Io(#[from] std::io::Error),
    }
}

/// The error returned by [references()][crate::Repository::references()].
pub type Error = gix_ref::packed::buffer::open::Error;
