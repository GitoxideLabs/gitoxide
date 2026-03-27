use std::{rc::Rc, sync::Arc};

use gix_error::{ErrorExt, Exn};

/// Read capabilities of a reference store.
pub trait StoreRead {
    /// Try to find a reference by `partial` name.
    ///
    /// Returns `Ok(None)` if no matching reference exists.
    fn try_find(&self, partial: &crate::PartialNameRef) -> Result<Option<crate::Reference>, Exn>;

    /// Return a platform to iterate references as loose-and-packed overlay.
    fn iter(&self) -> Result<crate::file::iter::Platform<'_>, Exn>;

    /// Return `true` if a reflog exists for `name`.
    fn reflog_exists(&self, name: &crate::FullNameRef) -> Result<bool, Exn>;

    /// Return a forward reflog iterator for `name`, or `None` if there is no reflog.
    fn reflog_iter<'a>(
        &self,
        name: &crate::FullNameRef,
        buf: &'a mut Vec<u8>,
    ) -> Result<Option<crate::file::log::iter::Forward<'a>>, Exn>;

    /// Return a reverse reflog iterator for `name`, or `None` if there is no reflog.
    fn reflog_iter_rev<'a>(
        &self,
        name: &crate::FullNameRef,
        buf: &'a mut [u8],
    ) -> Result<Option<crate::file::log::iter::Reverse<'a, std::fs::File>>, Exn>;
}

/// Convenience methods built on top of [`StoreRead`].
pub trait StoreReadExt: StoreRead {
    /// Like [`StoreRead::try_find()`], but a missing reference is treated as error.
    fn find(&self, partial: &crate::PartialNameRef) -> Result<crate::Reference, Exn> {
        self.try_find(partial)?.ok_or_else(|| {
            crate::file::find::existing::Error::NotFound {
                name: partial.to_partial_path().to_owned(),
            }
            .raise_erased()
        })
    }
}

impl<T: StoreRead + ?Sized> StoreReadExt for T {}

/// Mutation capabilities of a reference store.
pub trait StoreMutate {
    /// Return a transaction platform for mutating references.
    fn transaction(&self) -> Result<crate::file::Transaction<'_, '_>, Exn>;
}

impl StoreRead for crate::file::Store {
    fn try_find(&self, partial: &crate::PartialNameRef) -> Result<Option<crate::Reference>, Exn> {
        crate::file::Store::try_find(self, partial).map_err(|err| err.raise_erased())
    }

    fn iter(&self) -> Result<crate::file::iter::Platform<'_>, Exn> {
        crate::file::Store::iter(self).map_err(|err| err.raise_erased())
    }

    fn reflog_exists(&self, name: &crate::FullNameRef) -> Result<bool, Exn> {
        Ok(crate::file::Store::reflog_exists(self, name).expect("a FullNameRef is always valid"))
    }

    fn reflog_iter<'a>(
        &self,
        name: &crate::FullNameRef,
        buf: &'a mut Vec<u8>,
    ) -> Result<Option<crate::file::log::iter::Forward<'a>>, Exn> {
        crate::file::Store::reflog_iter(self, name, buf).map_err(|err| err.raise_erased())
    }

    fn reflog_iter_rev<'a>(
        &self,
        name: &crate::FullNameRef,
        buf: &'a mut [u8],
    ) -> Result<Option<crate::file::log::iter::Reverse<'a, std::fs::File>>, Exn> {
        crate::file::Store::reflog_iter_rev(self, name, buf).map_err(|err| err.raise_erased())
    }
}

impl StoreMutate for crate::file::Store {
    fn transaction(&self) -> Result<crate::file::Transaction<'_, '_>, Exn> {
        Ok(crate::file::Store::transaction(self))
    }
}

impl<T> StoreRead for &T
where
    T: StoreRead + ?Sized,
{
    fn try_find(&self, partial: &crate::PartialNameRef) -> Result<Option<crate::Reference>, Exn> {
        (*self).try_find(partial)
    }

    fn iter(&self) -> Result<crate::file::iter::Platform<'_>, Exn> {
        (*self).iter()
    }

    fn reflog_exists(&self, name: &crate::FullNameRef) -> Result<bool, Exn> {
        (*self).reflog_exists(name)
    }

    fn reflog_iter<'a>(
        &self,
        name: &crate::FullNameRef,
        buf: &'a mut Vec<u8>,
    ) -> Result<Option<crate::file::log::iter::Forward<'a>>, Exn> {
        (*self).reflog_iter(name, buf)
    }

    fn reflog_iter_rev<'a>(
        &self,
        name: &crate::FullNameRef,
        buf: &'a mut [u8],
    ) -> Result<Option<crate::file::log::iter::Reverse<'a, std::fs::File>>, Exn> {
        (*self).reflog_iter_rev(name, buf)
    }
}

impl<T> StoreMutate for &T
where
    T: StoreMutate + ?Sized,
{
    fn transaction(&self) -> Result<crate::file::Transaction<'_, '_>, Exn> {
        (*self).transaction()
    }
}

impl<T> StoreRead for Rc<T>
where
    T: StoreRead + ?Sized,
{
    fn try_find(&self, partial: &crate::PartialNameRef) -> Result<Option<crate::Reference>, Exn> {
        self.as_ref().try_find(partial)
    }

    fn iter(&self) -> Result<crate::file::iter::Platform<'_>, Exn> {
        self.as_ref().iter()
    }

    fn reflog_exists(&self, name: &crate::FullNameRef) -> Result<bool, Exn> {
        self.as_ref().reflog_exists(name)
    }

    fn reflog_iter<'a>(
        &self,
        name: &crate::FullNameRef,
        buf: &'a mut Vec<u8>,
    ) -> Result<Option<crate::file::log::iter::Forward<'a>>, Exn> {
        self.as_ref().reflog_iter(name, buf)
    }

    fn reflog_iter_rev<'a>(
        &self,
        name: &crate::FullNameRef,
        buf: &'a mut [u8],
    ) -> Result<Option<crate::file::log::iter::Reverse<'a, std::fs::File>>, Exn> {
        self.as_ref().reflog_iter_rev(name, buf)
    }
}

impl<T> StoreMutate for Rc<T>
where
    T: StoreMutate + ?Sized,
{
    fn transaction(&self) -> Result<crate::file::Transaction<'_, '_>, Exn> {
        self.as_ref().transaction()
    }
}

impl<T> StoreRead for Arc<T>
where
    T: StoreRead + ?Sized,
{
    fn try_find(&self, partial: &crate::PartialNameRef) -> Result<Option<crate::Reference>, Exn> {
        self.as_ref().try_find(partial)
    }

    fn iter(&self) -> Result<crate::file::iter::Platform<'_>, Exn> {
        self.as_ref().iter()
    }

    fn reflog_exists(&self, name: &crate::FullNameRef) -> Result<bool, Exn> {
        self.as_ref().reflog_exists(name)
    }

    fn reflog_iter<'a>(
        &self,
        name: &crate::FullNameRef,
        buf: &'a mut Vec<u8>,
    ) -> Result<Option<crate::file::log::iter::Forward<'a>>, Exn> {
        self.as_ref().reflog_iter(name, buf)
    }

    fn reflog_iter_rev<'a>(
        &self,
        name: &crate::FullNameRef,
        buf: &'a mut [u8],
    ) -> Result<Option<crate::file::log::iter::Reverse<'a, std::fs::File>>, Exn> {
        self.as_ref().reflog_iter_rev(name, buf)
    }
}

impl<T> StoreMutate for Arc<T>
where
    T: StoreMutate + ?Sized,
{
    fn transaction(&self) -> Result<crate::file::Transaction<'_, '_>, Exn> {
        self.as_ref().transaction()
    }
}

impl<T> StoreRead for Box<T>
where
    T: StoreRead + ?Sized,
{
    fn try_find(&self, partial: &crate::PartialNameRef) -> Result<Option<crate::Reference>, Exn> {
        self.as_ref().try_find(partial)
    }

    fn iter(&self) -> Result<crate::file::iter::Platform<'_>, Exn> {
        self.as_ref().iter()
    }

    fn reflog_exists(&self, name: &crate::FullNameRef) -> Result<bool, Exn> {
        self.as_ref().reflog_exists(name)
    }

    fn reflog_iter<'a>(
        &self,
        name: &crate::FullNameRef,
        buf: &'a mut Vec<u8>,
    ) -> Result<Option<crate::file::log::iter::Forward<'a>>, Exn> {
        self.as_ref().reflog_iter(name, buf)
    }

    fn reflog_iter_rev<'a>(
        &self,
        name: &crate::FullNameRef,
        buf: &'a mut [u8],
    ) -> Result<Option<crate::file::log::iter::Reverse<'a, std::fs::File>>, Exn> {
        self.as_ref().reflog_iter_rev(name, buf)
    }
}

impl<T> StoreMutate for Box<T>
where
    T: StoreMutate + ?Sized,
{
    fn transaction(&self) -> Result<crate::file::Transaction<'_, '_>, Exn> {
        self.as_ref().transaction()
    }
}
