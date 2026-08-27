use crate::{PartialNameRef, Reference, store};

/// Errors returned when looking up a reference through [`crate::Store`].
#[derive(Debug, thiserror::Error)]
#[expect(missing_docs)]
pub enum Error {
    #[error("The reference name is invalid")]
    RefnameValidation(#[from] crate::name::Error),
    #[error(transparent)]
    Backend(#[from] crate::store::BackendError),
    #[error("A reference-store snapshot could not be obtained")]
    Snapshot(#[from] crate::store::snapshot::Error),
}

impl crate::store::snapshot::Snapshot<'_> {
    /// Find a reference through this snapshot using Git's partial-name lookup rules.
    pub fn try_find<'a, Name, E>(&self, partial: Name) -> Result<Option<Reference>, Error>
    where
        Name: TryInto<&'a PartialNameRef, Error = E>,
        crate::name::Error: From<E>,
    {
        let partial = partial
            .try_into()
            .map_err(|err| Error::RefnameValidation(crate::name::Error::from(err)))?;
        match &self.state {
            crate::store::snapshot::State::Files { store, packed } => store
                .try_find_packed(partial, packed.as_ref().map(|buffer| &***buffer))
                .map_err(|err| Error::Backend(crate::store::BackendError::new("look up a reference", err))),
            crate::store::snapshot::State::Reftable { snapshot } => snapshot
                .try_find(partial)
                .map_err(|err| Error::Backend(crate::store::BackendError::new("look up a reftable reference", err))),
        }
    }
}

impl From<std::convert::Infallible> for Error {
    fn from(_: std::convert::Infallible) -> Self {
        unreachable!("conversion from a validated reference name cannot fail")
    }
}

impl crate::Store {
    /// Find a reference using Git's partial-name lookup rules.
    ///
    /// Returns `Ok(None)` if no matching reference exists.
    pub fn try_find<'a, Name, E>(&self, partial: Name) -> Result<Option<Reference>, Error>
    where
        Name: TryInto<&'a PartialNameRef, Error = E>,
        crate::name::Error: From<E>,
    {
        let partial = partial
            .try_into()
            .map_err(|err| Error::RefnameValidation(crate::name::Error::from(err)))?;
        self.try_find_inner(partial)
    }

    /// Find a reference using Git's partial-name lookup rules, returning an error if it is absent.
    pub fn find<'a, Name, E>(&self, partial: Name) -> Result<Reference, existing::Error>
    where
        Name: TryInto<&'a PartialNameRef, Error = E>,
        crate::name::Error: From<E>,
    {
        let partial = partial
            .try_into()
            .map_err(|err| Error::RefnameValidation(crate::name::Error::from(err)))?;
        self.try_find_inner(partial)?.ok_or_else(|| existing::Error::NotFound {
            name: partial.to_owned(),
        })
    }

    fn try_find_inner(&self, partial: &PartialNameRef) -> Result<Option<Reference>, Error> {
        match &self.inner {
            store::State::Files { store } => store
                .try_find(partial)
                .map_err(|err| Error::Backend(crate::store::BackendError::new("look up a reference", err))),
            store::State::Reftable { store } => store
                .snapshot()
                .and_then(|snapshot| snapshot.try_find(partial))
                .map_err(|err| Error::Backend(crate::store::BackendError::new("look up a reftable reference", err))),
        }
    }
}

/// Errors returned when a reference is required to exist.
pub mod existing {
    use crate::PartialName;

    /// The error returned by [`crate::Store::find()`].
    #[derive(Debug, thiserror::Error)]
    #[expect(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        Find(#[from] super::Error),
        #[error("The ref partially named {name:?} could not be found", name = &name.0)]
        NotFound { name: PartialName },
    }
}
