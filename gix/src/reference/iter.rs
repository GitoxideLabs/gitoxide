//!
#![allow(clippy::empty_docs)]

use gix_error::ResultExt;

use gix_path::RelativePath;
use gix_ref::file::ReferenceExt;

use crate::{Result, error::Error, repository::freelist::Buffer};

/// A platform to create iterators over references.
#[must_use = "Iterators should be obtained from this iterator platform"]
pub struct Platform<'repo> {
    pub(crate) platform: gix_ref::file::iter::Platform<'repo>,
    pub(crate) repo: &'repo crate::Repository,
}

/// An iterator over references, with or without filter.
pub struct Iter<'packed, 'repo> {
    inner: gix_ref::file::iter::LooseThenPacked<'packed, 'repo>,
    peel: Option<Peel<'repo>>,
    repo: &'repo crate::Repository,
}

struct Peel<'repo> {
    packed: Option<gix_ref::file::packed::SharedBufferSnapshot>,
    buf: Buffer<'repo>,
}

impl<'packed, 'repo> Iter<'packed, 'repo> {
    fn new(repo: &'repo crate::Repository, platform: gix_ref::file::iter::LooseThenPacked<'packed, 'repo>) -> Self {
        Iter {
            inner: platform,
            peel: None,
            repo,
        }
    }
}

impl<'repo> Platform<'repo> {
    /// Return the owning repository.
    pub fn repo(&self) -> &'repo crate::Repository {
        self.repo
    }

    /// Return an iterator over all references in the repository, excluding
    /// pseudo references.
    ///
    /// Even broken or otherwise unparsable or inaccessible references are returned and have to be handled by the caller on a
    /// case by case basis.
    pub fn all(&self) -> Result<Iter<'_, 'repo>> {
        Ok(Iter::new(self.repo, self.platform.all().or_error()?))
    }

    /// Return an iterator over all references that match the given `prefix`.
    ///
    /// These are of the form `refs/heads/` or `refs/remotes/origin`, and must not contain relative paths components like `.` or `..`.
    pub fn prefixed<'a>(&self, prefix: impl TryInto<&'a RelativePath, Error = Error>) -> Result<Iter<'_, 'repo>> {
        let prefix = prefix.try_into()?;
        Ok(Iter::new(self.repo, self.platform.prefixed(prefix).or_error()?))
    }

    /// Return an iterator over all references that are tags.
    ///
    /// They are all prefixed with `refs/tags`.
    pub fn tags(&self) -> Result<Iter<'_, 'repo>> {
        let prefix = b"refs/tags/".try_into()?;
        Ok(Iter::new(self.repo, self.platform.prefixed(prefix).or_error()?))
    }

    // TODO: tests
    /// Return an iterator over all local branches.
    ///
    /// They are all prefixed with `refs/heads`.
    pub fn local_branches(&self) -> Result<Iter<'_, 'repo>> {
        let prefix = b"refs/heads/".try_into()?;
        Ok(Iter::new(self.repo, self.platform.prefixed(prefix).or_error()?))
    }

    // TODO: tests
    /// Return an iterator over all local pseudo references.
    pub fn pseudo(&self) -> Result<Iter<'_, 'repo>> {
        Ok(Iter::new(self.repo, self.platform.pseudo().or_error()?))
    }

    // TODO: tests
    /// Return an iterator over all remote branches.
    ///
    /// They are all prefixed with `refs/remotes`.
    pub fn remote_branches(&self) -> Result<Iter<'_, 'repo>> {
        let prefix = b"refs/remotes/".try_into()?;
        Ok(Iter::new(self.repo, self.platform.prefixed(prefix).or_error()?))
    }
}

impl Iter<'_, '_> {
    /// Automatically peel references before yielding them during iteration.
    ///
    /// This reuses a single object buffer and packed-refs snapshot across the iterator.
    pub fn peeled(mut self) -> Result<Self> {
        self.peel = Some(Peel {
            packed: self.repo.refs.cached_packed_buffer()?,
            buf: self.repo.empty_reusable_buffer(),
        });
        Ok(self)
    }
}

impl<'repo> Iterator for Iter<'_, 'repo> {
    type Item = Result<crate::Reference<'repo>>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut name_buf = self.repo.empty_reusable_buffer();
        let name_from_freelist = name_buf.capacity() != 0;
        let mut symbolic_target_from_freelist = false;
        let next = self.inner.next_with_buffers(&mut name_buf, || {
            let buf = self.repo.free_buf();
            symbolic_target_from_freelist = buf.capacity() != 0;
            buf
        });
        next.map(|res| {
            let mut reference = crate::Reference::from_ref(res?, self.repo);
            reference.name_from_freelist = name_from_freelist;
            reference.symbolic_target_from_freelist = symbolic_target_from_freelist;
            if let Some(peel) = &mut self.peel {
                reference.inner_mut().peel_to_id_packed_with_buffer(
                    &self.repo.refs,
                    &self.repo.objects,
                    peel.packed.as_ref().map(|p| &***p),
                    &mut peel.buf,
                )?;
            }
            Ok(reference)
        })
    }
}
