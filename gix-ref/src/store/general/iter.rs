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
    Reftable(std::vec::IntoIter<Result<Reference, crate::store::BackendError>>),
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
        match &self.snapshot.state {
            snapshot::State::Files { .. } => self.iter_with(|store, packed| store.iter_packed(packed)),
            snapshot::State::Reftable { snapshot } => Ok(self.reftable_iter(snapshot.all())),
        }
    }

    /// Iterate ordinary references matching `prefix`, sorted by name.
    pub fn prefixed(&self, prefix: &RelativePath) -> std::io::Result<Iter<'_>> {
        match &self.snapshot.state {
            snapshot::State::Files { .. } => self.iter_with(|store, packed| store.iter_prefixed_packed(prefix, packed)),
            snapshot::State::Reftable { snapshot } => Ok(self.reftable_iter(snapshot.prefixed(prefix))),
        }
    }

    /// Iterate pseudo references, sorted by name.
    pub fn pseudo(&self) -> std::io::Result<Iter<'_>> {
        match &self.snapshot.state {
            snapshot::State::Files { store, .. } => Ok(Iter {
                snapshot: &self.snapshot,
                state: State::Files(store.iter_pseudo()?),
            }),
            snapshot::State::Reftable { snapshot } => Ok(self.reftable_iter(snapshot.pseudo())),
        }
    }

    fn reftable_iter(&self, references: Vec<Result<Reference, crate::store_impl::reftable::Error>>) -> Iter<'_> {
        Iter {
            snapshot: &self.snapshot,
            state: State::Reftable(
                references
                    .into_iter()
                    .map(|reference| {
                        reference.map_err(|err| crate::store::BackendError::new("iterate reftable references", err))
                    })
                    .collect::<Vec<_>>()
                    .into_iter(),
            ),
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
            snapshot::State::Reftable { .. } => unreachable!("reftable iteration does not use files iterators"),
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
            State::Reftable(iter) => iter.next().map(|item| item.map_err(Error)),
        }
    }
}
