//!
#![allow(clippy::empty_docs)]

use gix_path::RelativePath;
use gix_ref::store::ReferenceExt;

/// A platform to create iterators over references.
#[must_use = "Iterators should be obtained from this iterator platform"]
pub struct Platform<'r> {
    pub(crate) platform: gix_ref::store::iter::Platform<'r>,
    /// The owning repository.
    pub repo: &'r crate::Repository,
}

/// An iterator over references, with or without filter.
pub struct Iter<'platform, 'repo> {
    inner: gix_ref::store::iter::Iter<'platform>,
    peel: bool,
    repo: &'repo crate::Repository,
}

impl<'platform, 'repo> Iter<'platform, 'repo> {
    fn new(repo: &'repo crate::Repository, platform: gix_ref::store::iter::Iter<'platform>) -> Self {
        Iter {
            inner: platform,
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
        Ok(Iter::new(self.repo, self.platform.all()?))
    }

    /// Return an iterator over all references that match the given `prefix`.
    ///
    /// These are of the form `refs/heads/` or `refs/remotes/origin`, and must not contain relative paths components like `.` or `..`.
    pub fn prefixed<'a>(
        &self,
        prefix: impl TryInto<&'a RelativePath, Error = gix_path::relative_path::Error>,
    ) -> Result<Iter<'_, 'repo>, init::Error> {
        Ok(Iter::new(self.repo, self.platform.prefixed(prefix.try_into()?)?))
    }

    /// Return an iterator over all references that are tags.
    ///
    /// They are all prefixed with `refs/tags`.
    pub fn tags(&self) -> Result<Iter<'_, 'repo>, init::Error> {
        Ok(Iter::new(self.repo, self.platform.prefixed(b"refs/tags/".try_into()?)?))
    }

    // TODO: tests
    /// Return an iterator over all local branches.
    ///
    /// They are all prefixed with `refs/heads`.
    pub fn local_branches(&self) -> Result<Iter<'_, 'repo>, init::Error> {
        Ok(Iter::new(
            self.repo,
            self.platform.prefixed(b"refs/heads/".try_into()?)?,
        ))
    }

    // TODO: tests
    /// Return an iterator over all local pseudo references.
    pub fn pseudo(&self) -> Result<Iter<'_, 'repo>, init::Error> {
        Ok(Iter::new(self.repo, self.platform.pseudo()?))
    }

    // TODO: tests
    /// Return an iterator over all remote branches.
    ///
    /// They are all prefixed with `refs/remotes`.
    pub fn remote_branches(&self) -> Result<Iter<'_, 'repo>, init::Error> {
        Ok(Iter::new(
            self.repo,
            self.platform.prefixed(b"refs/remotes/".try_into()?)?,
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
    /// Peeling through the iterator reuses the same reference-store snapshot as
    /// iteration, so related lookups observe one adapter-defined view.
    pub fn peeled(mut self) -> Result<Self, gix_ref::store::snapshot::Error> {
        self.peel = true;
        Ok(self)
    }
}

impl<'r> Iterator for Iter<'_, 'r> {
    type Item = Result<crate::Reference<'r>, Box<dyn std::error::Error + Send + Sync + 'static>>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut reference = match self.inner.next()? {
            Ok(reference) => reference,
            Err(err) => return Some(Err(Box::new(err))),
        };
        if self.peel
            && let Err(err) = reference.peel_to_id_with_snapshot(self.inner.snapshot(), &self.repo.objects)
        {
            return Some(Err(Box::new(err)));
        }
        Some(Ok(crate::Reference::from_ref(reference, self.repo)))
    }
}

///
pub mod init {
    /// The error returned by [`Platform::all()`](super::Platform::all()) or [`Platform::prefixed()`](super::Platform::prefixed()).
    #[derive(Debug, thiserror::Error)]
    #[expect(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        Io(#[from] std::io::Error),
        #[error(transparent)]
        RelativePath(#[from] gix_path::relative_path::Error),
    }
}

/// The error returned by [references()][crate::Repository::references()].
pub type Error = gix_ref::store::snapshot::Error;
