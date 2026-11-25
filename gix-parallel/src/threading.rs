//! Type definitions for putting shared ownership and synchronized mutation behind the `parallel` feature toggle.
//!
//! That way, single-threaded applications will not have to use thread-safe primitives, and simply do not specify the `parallel` feature.

#[cfg(feature = "parallel")]
mod _impl {
    use std::sync::Arc;

    /// A thread-safe cell which can be written to only once.
    ///
    /// Note: We use `once_cell` here because `std::sync::OnceLock::get_or_try_init()` is not yet stable.
    /// Once it's stabilized, we can switch to `std::sync::OnceLock`.
    #[cfg(feature = "once_cell")]
    pub type OnceCell<T> = once_cell::sync::OnceCell<T>;
    /// A reference counted pointer type for shared ownership.
    pub type OwnShared<T> = Arc<T>;
    /// A non-owning reference to an [`OwnShared`] value.
    pub type WeakShared<T> = std::sync::Weak<T>;
    /// A synchronization primitive which can start read-only and transition to support mutation.
    pub type MutableOnDemand<T> = parking_lot::RwLock<T>;
    /// A synchronization primitive which provides read-write access right away.
    pub type Mutable<T> = parking_lot::Mutex<T>;
    /// A guarded reference suitable for safekeeping in a struct.
    pub type RefGuard<'a, T> = parking_lot::RwLockReadGuard<'a, T>;
    /// A mapped reference created from a `RefGuard`
    pub type MappedRefGuard<'a, U> = parking_lot::MappedRwLockReadGuard<'a, U>;

    /// Get a shared reference through a [`MutableOnDemand`] for read-only access.
    pub fn get_ref<T>(v: &MutableOnDemand<T>) -> RefGuard<'_, T> {
        v.read()
    }

    /// Get a mutable reference through a [`MutableOnDemand`] for read-write access.
    pub fn get_mut<T>(v: &MutableOnDemand<T>) -> parking_lot::RwLockWriteGuard<'_, T> {
        v.write()
    }

    /// Get a mutable reference to the underlying data, with semantics similar to [Arc::make_mut()].
    pub fn make_mut<T: Clone>(this: &mut OwnShared<T>) -> &mut T {
        OwnShared::make_mut(this)
    }

    /// Get a mutable reference through a [`Mutable`] for read-write access.
    pub fn lock<T>(v: &Mutable<T>) -> parking_lot::MutexGuard<'_, T> {
        v.lock()
    }

    /// Downgrade a handle previously obtained with [`get_mut()`] to drop mutation support.
    pub fn downgrade_mut_to_ref<'a, T>(
        v: parking_lot::RwLockWriteGuard<'a, T>,
        _orig: &'a MutableOnDemand<T>,
    ) -> RefGuard<'a, T> {
        parking_lot::RwLockWriteGuard::downgrade(v)
    }

    /// Map a read guard into a sub-type it contains.
    pub fn map_ref<T, U: ?Sized>(v: RefGuard<'_, T>, f: impl FnOnce(&T) -> &U) -> MappedRefGuard<'_, U> {
        parking_lot::RwLockReadGuard::map(v, f)
    }
}

#[cfg(not(feature = "parallel"))]
mod _impl {
    use std::{
        cell::{Ref, RefCell, RefMut},
        rc::Rc,
    };

    /// A thread-safe cell which can be written to only once.
    ///
    /// Note: We use `once_cell` here because `std::cell::OnceCell::get_or_try_init()` is not yet stable.
    /// Once it's stabilized, we can switch to `std::cell::OnceCell`.
    #[cfg(feature = "once_cell")]
    pub type OnceCell<T> = once_cell::unsync::OnceCell<T>;
    /// A reference counted pointer type for shared ownership.
    pub type OwnShared<T> = Rc<T>;
    /// A non-owning reference to an [`OwnShared`] value.
    pub type WeakShared<T> = std::rc::Weak<T>;
    /// A synchronization primitive which can start read-only and transition to support mutation.
    pub type MutableOnDemand<T> = RefCell<T>;
    /// A synchronization primitive which provides read-write access right away.
    pub type Mutable<T> = RefCell<T>;
    /// A guarded reference suitable for safekeeping in a struct.
    pub type RefGuard<'a, T> = Ref<'a, T>;
    /// A mapped reference created from a RefGuard
    pub type MappedRefGuard<'a, U> = Ref<'a, U>;

    /// Get a shared reference through a [`MutableOnDemand`] for read-only access.
    pub fn get_mut<T>(v: &RefCell<T>) -> RefMut<'_, T> {
        v.borrow_mut()
    }

    /// Get a mutable reference to the underlying data, with semantics similar to [Rc::make_mut()].
    pub fn make_mut<T: Clone>(this: &mut OwnShared<T>) -> &mut T {
        OwnShared::make_mut(this)
    }

    /// Get a mutable reference through a [`Mutable`] for read-write access.
    pub fn lock<T>(v: &Mutable<T>) -> RefMut<'_, T> {
        v.borrow_mut()
    }

    /// Get a mutable reference through a [`MutableOnDemand`] for read-write access.
    pub fn get_ref<T>(v: &RefCell<T>) -> RefGuard<'_, T> {
        v.borrow()
    }

    /// Downgrade a handle previously obtained with [`get_mut()`] to drop mutation support.
    pub fn downgrade_mut_to_ref<'a, T>(v: RefMut<'a, T>, orig: &'a RefCell<T>) -> RefGuard<'a, T> {
        drop(v);
        orig.borrow()
    }

    /// Map a read guard into a sub-type it contains.
    pub fn map_ref<T, U: ?Sized>(v: RefGuard<'_, T>, f: impl FnOnce(&T) -> &U) -> MappedRefGuard<'_, U> {
        Ref::map(v, f)
    }
}

pub use _impl::*;
