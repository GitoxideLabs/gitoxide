use crate::{FullName, store};

/// A platform for iterating one reference's log.
pub struct Platform<'store> {
    store: &'store crate::Store,
    name: FullName,
    buf: Vec<u8>,
}

/// An iterator over owned reflog entries from any built-in adapter.
pub struct Iter<'a> {
    inner: Box<dyn Iterator<Item = Result<crate::log::Line, iter::Error>> + 'a>,
}

impl Iterator for Iter<'_> {
    type Item = Result<crate::log::Line, iter::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

/// Errors returned when locating or opening a reflog.
#[derive(Debug, thiserror::Error)]
#[expect(missing_docs)]
pub enum Error {
    #[error("The reflog name is invalid")]
    RefnameValidation(#[from] crate::name::Error),
    #[error(transparent)]
    Backend(#[from] crate::store::BackendError),
}

impl crate::Store {
    /// Return whether a reflog exists for `name`.
    pub fn reflog_exists<Name, E>(&self, name: Name) -> Result<bool, Error>
    where
        Name: TryInto<FullName, Error = E>,
        crate::name::Error: From<E>,
    {
        let name = name
            .try_into()
            .map_err(|err| Error::RefnameValidation(crate::name::Error::from(err)))?;
        match &self.inner {
            store::State::Files { store } => Ok(store
                .reflog_exists::<_, std::convert::Infallible>(name.as_ref())
                .expect("a validated full reference name converts infallibly")),
            store::State::Reftable { store } => {
                let snapshot = store
                    .snapshot()
                    .map_err(|err| crate::store::BackendError::new("open a reftable reference-log snapshot", err))?;
                let route = store.route(name.as_ref());
                snapshot
                    .reflog_exists(&route)
                    .map_err(|err| crate::store::BackendError::new("find a reftable reference log", err).into())
            }
        }
    }

    /// Return a platform for iterating the reflog belonging to `name`.
    pub fn reflog_iter<Name, E>(&self, name: Name) -> Result<Platform<'_>, Error>
    where
        Name: TryInto<FullName, Error = E>,
        crate::name::Error: From<E>,
    {
        Ok(Platform {
            store: self,
            name: name
                .try_into()
                .map_err(|err| Error::RefnameValidation(crate::name::Error::from(err)))?,
            buf: Vec::new(),
        })
    }
}

impl Platform<'_> {
    /// Iterate log entries from oldest to newest.
    pub fn all(&mut self) -> Result<Option<Iter<'_>>, Error> {
        self.buf.clear();
        match &self.store.inner {
            store::State::Files { store } => Ok(store
                .reflog_iter(self.name.as_ref(), &mut self.buf)
                .map_err(|err| crate::store::BackendError::new("open a reference log", err))?
                .map(|forward| Iter {
                    inner: Box::new(forward.map(|line| {
                        line.map(|line| line.to_owned()).map_err(|err| {
                            iter::Error(crate::store::BackendError::new("decode a reference-log entry", err))
                        })
                    })),
                })),
            store::State::Reftable { store } => {
                let snapshot = store
                    .snapshot()
                    .map_err(|err| crate::store::BackendError::new("open a reftable reference-log snapshot", err))?;
                let route = store.route(self.name.as_ref());
                if !snapshot
                    .reflog_exists(&route)
                    .map_err(|err| crate::store::BackendError::new("find a reftable reference log", err))?
                {
                    return Ok(None);
                }
                let mut lines = snapshot
                    .reflog_lines(&route)
                    .map_err(|err| crate::store::BackendError::new("read a reftable reference log", err))?;
                lines.reverse();
                Ok(Some(Iter {
                    inner: Box::new(lines.into_iter().map(|line| {
                        line.map_err(|err| {
                            iter::Error(crate::store::BackendError::new(
                                "decode a reftable reference-log entry",
                                err,
                            ))
                        })
                    })),
                }))
            }
        }
    }

    /// Iterate log entries from newest to oldest.
    pub fn rev(&mut self) -> Result<Option<Iter<'_>>, Error> {
        self.buf.clear();
        self.buf.resize(4 * 1024, 0);
        match &self.store.inner {
            store::State::Files { store } => Ok(store
                .reflog_iter_rev(self.name.as_ref(), &mut self.buf)
                .map_err(|err| crate::store::BackendError::new("open a reference log", err))?
                .map(|reverse| Iter {
                    inner: Box::new(reverse.map(|line| {
                        line.map_err(|err| {
                            iter::Error(crate::store::BackendError::new("read a reference log in reverse", err))
                        })
                    })),
                })),
            store::State::Reftable { store } => {
                let snapshot = store
                    .snapshot()
                    .map_err(|err| crate::store::BackendError::new("open a reftable reference-log snapshot", err))?;
                let route = store.route(self.name.as_ref());
                if !snapshot
                    .reflog_exists(&route)
                    .map_err(|err| crate::store::BackendError::new("find a reftable reference log", err))?
                {
                    return Ok(None);
                }
                let lines = snapshot
                    .reflog_lines(&route)
                    .map_err(|err| crate::store::BackendError::new("read a reftable reference log", err))?;
                Ok(Some(Iter {
                    inner: Box::new(lines.into_iter().map(|line| {
                        line.map_err(|err| {
                            iter::Error(crate::store::BackendError::new(
                                "decode a reftable reference-log entry",
                                err,
                            ))
                        })
                    })),
                }))
            }
        }
    }
}

/// Errors returned while decoding reflog entries.
pub mod iter {
    /// An error produced by a reflog iterator.
    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    pub struct Error(pub(super) crate::store::BackendError);
}
