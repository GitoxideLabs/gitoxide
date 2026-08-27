use gix_path::RelativePath;

use crate::{Reference, store::snapshot};

/// A platform for creating iterators over one stable store snapshot.
#[must_use = "iterators should be obtained from this platform"]
pub struct Platform<'store> {
    snapshot: snapshot::Snapshot<'store>,
}

/// An iterator over references from any built-in storage adapter.
pub struct Iter<'a> {
    snapshot: &'a snapshot::Snapshot<'a>,
    state: State<'a>,
}

enum State<'a> {
    Files(crate::file::iter::LooseThenPacked<'a, 'a>),
}

/// The error returned while iterating references.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error(crate::store::BackendError);

impl crate::Store {
    /// Return a platform for iterating references through a coordinated snapshot.
    pub fn iter(&self) -> Result<Platform<'_>, snapshot::Error> {
        Ok(Platform {
            snapshot: self.snapshot()?,
        })
    }
}

impl Platform<'_> {
    /// Iterate all ordinary references, sorted by name.
    pub fn all(&self) -> std::io::Result<Iter<'_>> {
        self.iter_with(|store, packed| store.iter_packed(packed))
    }

    /// Iterate ordinary references matching `prefix`, sorted by name.
    pub fn prefixed(&self, prefix: &RelativePath) -> std::io::Result<Iter<'_>> {
        self.iter_with(|store, packed| store.iter_prefixed_packed(prefix, packed))
    }

    /// Iterate pseudo references, sorted by name.
    pub fn pseudo(&self) -> std::io::Result<Iter<'_>> {
        match &self.snapshot.state {
            snapshot::State::Files { store, .. } => Ok(Iter {
                snapshot: &self.snapshot,
                state: State::Files(store.iter_pseudo()?),
            }),
        }
    }

    fn iter_with<'a>(
        &'a self,
        make_iter: impl FnOnce(
            &'a crate::file::Store,
            Option<&'a crate::packed::Buffer>,
        ) -> std::io::Result<crate::file::iter::LooseThenPacked<'a, 'a>>,
    ) -> std::io::Result<Iter<'a>> {
        match &self.snapshot.state {
            snapshot::State::Files { store, packed } => Ok(Iter {
                snapshot: &self.snapshot,
                state: State::Files(make_iter(store, packed.as_ref().map(|buffer| &***buffer))?),
            }),
        }
    }
}

impl<'a> Iter<'a> {
    /// Return the stable store view used to create this iterator.
    pub fn snapshot(&self) -> &snapshot::Snapshot<'a> {
        self.snapshot
    }
}

impl Iterator for Iter<'_> {
    type Item = Result<Reference, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.state {
            State::Files(iter) => iter
                .next()
                .map(|item| item.map_err(|err| Error(crate::store::BackendError::new("iterate references", err)))),
        }
    }
}
