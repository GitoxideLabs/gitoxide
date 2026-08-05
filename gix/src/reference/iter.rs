//!
#![allow(clippy::empty_docs)]

use gix_path::RelativePath;
use gix_ref::file::ReferenceExt;

/// A platform to create iterators over references.
#[must_use = "Iterators should be obtained from this iterator platform"]
pub struct Platform<'r> {
    pub(crate) platform: gix_ref::file::iter::Platform<'r>,
    /// The owning repository.
    pub repo: &'r crate::Repository,
}

/// An iterator over references, with or without filter.
pub struct Iter<'packed, 'repo> {
    inner: gix_ref::file::iter::LooseThenPacked<'packed, 'repo>,
    peel_with_packed: Option<gix_ref::file::packed::SharedBufferSnapshot>,
    peel: bool,
    repo: &'repo crate::Repository,
}

impl<'packed, 'repo> Iter<'packed, 'repo> {
    fn new(repo: &'repo crate::Repository, platform: gix_ref::file::iter::LooseThenPacked<'packed, 'repo>) -> Self {
        Iter {
            inner: platform,
            peel_with_packed: None,
            peel: false,
            repo,
        }
    }
}

impl<'repo> Platform<'repo> {
    /// Return an iterator over all references in the repository, excluding
    /// pseudo references.
    ///
    /// Even broken or otherwise unparsable or inaccessible references are returned and have to be handled by the caller on a
    /// case by case basis.
    pub fn all(&self) -> Result<Iter<'_, 'repo>, init::Error> {
        Ok(Iter::new(
            self.repo,
            self.platform.all().map_err(gix_error::Error::from_error)?,
        ))
    }

    /// Return an iterator over all references that match the given `prefix`.
    ///
    /// These are of the form `refs/heads/` or `refs/remotes/origin`, and must not contain relative paths components like `.` or `..`.
    pub fn prefixed<'a>(
        &self,
        prefix: impl TryInto<&'a RelativePath, Error = gix_path::relative_path::Error>,
    ) -> Result<Iter<'_, 'repo>, init::Error> {
        let prefix = prefix.try_into().map_err(gix_error::Error::from_error)?;
        Ok(Iter::new(
            self.repo,
            self.platform.prefixed(prefix).map_err(gix_error::Error::from_error)?,
        ))
    }

    /// Return an iterator over all references that are tags.
    ///
    /// They are all prefixed with `refs/tags`.
    pub fn tags(&self) -> Result<Iter<'_, 'repo>, init::Error> {
        let prefix = b"refs/tags/".try_into().map_err(gix_error::Error::from_error)?;
        Ok(Iter::new(
            self.repo,
            self.platform.prefixed(prefix).map_err(gix_error::Error::from_error)?,
        ))
    }

    // TODO: tests
    /// Return an iterator over all local branches.
    ///
    /// They are all prefixed with `refs/heads`.
    pub fn local_branches(&self) -> Result<Iter<'_, 'repo>, init::Error> {
        let prefix = b"refs/heads/".try_into().map_err(gix_error::Error::from_error)?;
        Ok(Iter::new(
            self.repo,
            self.platform.prefixed(prefix).map_err(gix_error::Error::from_error)?,
        ))
    }

    // TODO: tests
    /// Return an iterator over all local pseudo references.
    pub fn pseudo(&self) -> Result<Iter<'_, 'repo>, init::Error> {
        Ok(Iter::new(
            self.repo,
            self.platform.pseudo().map_err(gix_error::Error::from_error)?,
        ))
    }

    // TODO: tests
    /// Return an iterator over all remote branches.
    ///
    /// They are all prefixed with `refs/remotes`.
    pub fn remote_branches(&self) -> Result<Iter<'_, 'repo>, init::Error> {
        let prefix = b"refs/remotes/".try_into().map_err(gix_error::Error::from_error)?;
        Ok(Iter::new(
            self.repo,
            self.platform.prefixed(prefix).map_err(gix_error::Error::from_error)?,
        ))
    }
}

impl Iter<'_, '_> {
    /// Automatically peel references before yielding them during iteration.
    ///
    /// This has the same effect as using `iter.map(|r| {r.peel_to_id(); r})`.
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

impl<'r> Iterator for Iter<'_, 'r> {
    type Item = Result<crate::Reference<'r>, Box<dyn std::error::Error + Send + Sync + 'static>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|res| {
            res.map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync + 'static>)
                .and_then(|mut r| {
                    if self.peel {
                        let repo = &self.repo;
                        r.peel_to_id_packed(&repo.refs, &repo.objects, self.peel_with_packed.as_ref().map(|p| &***p))
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
    /// The error returned by [`Platform::all()`](super::Platform::all()) or [`Platform::prefixed()`](super::Platform::prefixed()).
    pub type Error = gix_error::Error;
}

/// The error returned by [references()][crate::Repository::references()].
pub type Error = gix_ref::packed::buffer::open::Error;
