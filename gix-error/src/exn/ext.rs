// Copyright 2025 FastLabs Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{Error, Exn, ExnResult, Result};

use super::impls::into_frame;

/// Raise native errors with caller locations and optional context.
pub trait ErrorExt: std::error::Error + Send + Sync + 'static {
    /// Raise this error at the caller's location, returning a public [`Error`].
    /// An existing [`Error`] is returned unchanged, preserving its representation and allocation.
    #[track_caller]
    fn raise(self) -> Error
    where
        Self: Sized,
    {
        let mut error = Some(self);
        if let Some(error) = (&mut error as &mut dyn std::any::Any).downcast_mut::<Option<Error>>() {
            return error.take().expect("the error has not been consumed");
        }
        Exn::new(error.expect("a different error type was not taken")).into_error()
    }

    /// Raise this error as a typed exception, retaining `Self` even when it is already an [`Error`].
    #[track_caller]
    fn raise_typed(self) -> Exn<Self>
    where
        Self: Sized,
    {
        Exn::new(self)
    }

    /// Raise this error as a cause of `context`, returning a public [`Error`].
    ///
    /// Tree-backed [`crate::Error`] values reuse their existing frame, retaining its original caller location.
    #[track_caller]
    fn and_raise<T: std::error::Error + Send + Sync + 'static>(self, context: T) -> Error
    where
        Self: Sized,
    {
        self.and_raise_typed(context).into_error()
    }

    /// Like [`Self::and_raise()`], retaining the context's type in an exception.
    #[track_caller]
    fn and_raise_typed<T: std::error::Error + Send + Sync + 'static>(self, context: T) -> Exn<T>
    where
        Self: Sized,
    {
        Exn::with_cause(self, context)
    }

    /// Raise this error as a new exception, with type erasure.
    /// Tree-backed [`crate::Error`] values reuse their existing frame and caller location.
    #[track_caller]
    fn raise_erased(self) -> Exn
    where
        Self: Sized,
    {
        Exn::from_boxed_frame(into_frame(self))
    }

    /// Raise this error as a new exception, with `sources` as causes.
    #[track_caller]
    fn raise_all<T, I>(self, sources: I) -> Exn<Self>
    where
        Self: Sized,
        T: std::error::Error + Send + Sync + 'static,
        I: IntoIterator,
        I::Item: Into<Exn<T>>,
    {
        Exn::raise_all(sources, self)
    }
}

impl<T> ErrorExt for T where T: std::error::Error + Send + Sync + 'static {}

/// Raise errors lazily on [`Option::None`].
pub trait OptionExt {
    /// The `Some` type.
    type Some;

    /// Raise `err()` on `None`, returning a public [`Result`]. Existing [`Error`] values are retained unchanged.
    fn ok_or_raise<A, F>(self, err: F) -> Result<Self::Some>
    where
        A: std::error::Error + Send + Sync + 'static,
        F: FnOnce() -> A;

    /// Construct a typed [`Exn`] on the `None` variant.
    fn ok_or_raise_typed<A, F>(self, err: F) -> ExnResult<Self::Some, A>
    where
        A: std::error::Error + Send + Sync + 'static,
        F: FnOnce() -> A;

    /// Construct a new [`Exn`] on the `None` variant, with type erasure.
    /// The generated error is raised with [`ErrorExt::raise_erased`].
    fn ok_or_raise_erased<A, F>(self, err: F) -> ExnResult<Self::Some>
    where
        A: std::error::Error + Send + Sync + 'static,
        F: FnOnce() -> A;
}

impl<T> OptionExt for Option<T> {
    type Some = T;

    #[track_caller]
    fn ok_or_raise<A, F>(self, err: F) -> Result<T>
    where
        A: std::error::Error + Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        match self {
            Some(v) => Ok(v),
            None => Err(err().raise()),
        }
    }

    #[track_caller]
    fn ok_or_raise_typed<A, F>(self, err: F) -> ExnResult<T, A>
    where
        A: std::error::Error + Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        match self {
            Some(v) => Ok(v),
            None => Err(Exn::new(err())),
        }
    }

    #[track_caller]
    fn ok_or_raise_erased<A, F>(self, err: F) -> ExnResult<T>
    where
        A: std::error::Error + Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        match self {
            Some(v) => Ok(v),
            None => Err(err().raise_erased()),
        }
    }
}

/// Convert native errors and exceptions into public [`Result`]s, or add context lazily on failure.
pub trait ResultExt {
    /// The `Ok` type.
    type Success;

    /// The `Err` type that would be wrapped in an [`Exn`].
    type Error: std::error::Error + Send + Sync + 'static;

    /// Add `err()` as context on failure, returning a public [`Result`].
    ///
    /// Reuses existing exception trees and preserves native sources and their caller locations.
    #[track_caller]
    fn or_raise<A, F>(self, err: F) -> Result<Self::Success>
    where
        Self: Sized,
        A: std::error::Error + Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        self.or_raise_typed(err).map_err(Exn::into_error)
    }

    /// Convert to a public [`Result`] without adding context.
    /// Native errors are raised at the caller; existing [`Error`] values and exceptions retain their locations.
    #[track_caller]
    fn or_error(self) -> Result<Self::Success>;

    /// Like [`Self::or_raise()`], retaining the context's type in an exception.
    fn or_raise_typed<A, F>(self, err: F) -> ExnResult<Self::Success, A>
    where
        A: std::error::Error + Send + Sync + 'static,
        F: FnOnce() -> A;

    /// Raise a new exception on the [`Exn`] inside the [`Result`], but erase its type.
    /// Errors implementing [`std::error::Error`] use [`ErrorExt::raise_erased`].
    ///
    /// Apply [`Exn::erased`] on the `Err` variant, refer to it for more information.
    fn or_erased(self) -> ExnResult<Self::Success>;

    /// Raise a new exception on the [`Exn`] inside the [`Result`], and type-erase the result.
    ///
    /// Apply [`Exn::raise`] and [`Exn::erased`] on the `Err` variant, refer to it for more information.
    #[track_caller]
    fn or_raise_erased<A, F>(self, err: F) -> ExnResult<Self::Success>
    where
        Self: Sized,
        A: std::error::Error + Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        self.or_raise_typed(err).map_err(Exn::erased)
    }
}

impl<T, E> ResultExt for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    type Success = T;
    type Error = E;

    #[track_caller]
    fn or_raise_typed<A, F>(self, err: F) -> ExnResult<Self::Success, A>
    where
        A: std::error::Error + Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.and_raise_typed(err())),
        }
    }

    #[track_caller]
    fn or_error(self) -> Result<T> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.raise()),
        }
    }

    #[track_caller]
    fn or_erased(self) -> ExnResult<Self::Success> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.raise_erased()),
        }
    }
}

/// Extension methods for results containing an already boxed error.
///
/// This complements [`ResultExt`], whose blanket implementation cannot accept boxed trait objects.
pub trait BoxedResultExt {
    /// The `Ok` type.
    type Success;

    /// Type-erase the boxed error inside the [`Result`].
    fn or_erased(self) -> ExnResult<Self::Success>;
}

impl<T> BoxedResultExt for std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>> {
    type Success = T;

    #[track_caller]
    fn or_erased(self) -> ExnResult<Self::Success> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(Exn::new(crate::exn::Untyped::from_boxed(e))),
        }
    }
}

impl<T, E> ResultExt for ExnResult<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    type Success = T;
    type Error = E;

    #[track_caller]
    fn or_raise_typed<A, F>(self, err: F) -> ExnResult<Self::Success, A>
    where
        A: std::error::Error + Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.raise(err())),
        }
    }

    fn or_error(self) -> Result<T> {
        self.map_err(Exn::into_error)
    }

    #[track_caller]
    fn or_erased(self) -> ExnResult<Self::Success> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.erased()),
        }
    }
}
