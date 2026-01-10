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

use crate::exn::Exn;
use std::error::Error;

/// A trait bound of the supported error type of [`Exn`].
pub trait ErrorExt: std::error::Error + Send + Sync + 'static {
    /// Raise this error as a new exception.
    #[track_caller]
    fn raise(self) -> Exn<Self>
    where
        Self: Sized,
    {
        Exn::new(self)
    }

    /// Raise this error as a new exception, with type erasure.
    #[track_caller]
    fn erased(self) -> Exn
    where
        Self: Sized,
    {
        Exn::new(self).erased()
    }

    /// Raise this error as a new exception, with `sources` as causes.
    #[track_caller]
    fn raise_iter<T, I>(self, sources: I) -> Exn<Self>
    where
        Self: Sized,
        T: std::error::Error + Send + Sync + 'static,
        I: IntoIterator,
        I::Item: Into<Exn<T>>,
    {
        Exn::from_iter(sources, self)
    }
}

impl<T> ErrorExt for T where T: std::error::Error + Send + Sync + 'static {}

/// An extension trait for [`Option`] to provide raising new exceptions on `None`.
pub trait OptionExt {
    /// The `Some` type.
    type Some;

    /// Construct a new [`Exn`] on the `None` variant.
    fn ok_or_raise<A, F>(self, err: F) -> Result<Self::Some, Exn<A>>
    where
        A: std::error::Error + Send + Sync + 'static,
        F: FnOnce() -> A;

    /// Construct a new [`Exn`] on the `None` variant, with type erasure.
    fn ok_or_raise_erased<A, F>(self, err: F) -> Result<Self::Some, Exn>
    where
        A: std::error::Error + Send + Sync + 'static,
        F: FnOnce() -> A;
}

impl<T> OptionExt for Option<T> {
    type Some = T;

    #[track_caller]
    fn ok_or_raise<A, F>(self, err: F) -> Result<T, Exn<A>>
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
    fn ok_or_raise_erased<A, F>(self, err: F) -> Result<T, Exn>
    where
        A: std::error::Error + Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        self.ok_or_raise(err).map_err(Exn::erased)
    }
}

/// An extension trait for [`Result`] to provide context information on [`Exn`]s.
pub trait ResultExt {
    /// The `Ok` type.
    type Success;

    /// The `Err` type that would be wrapped in an [`Exn`].
    type Error: std::error::Error + Send + Sync + 'static;

    /// Raise a new exception on the [`Exn`] inside the [`Result`].
    ///
    /// Apply [`Exn::raise`] on the `Err` variant, refer to it for more information.
    fn or_raise<A, F>(self, err: F) -> Result<Self::Success, Exn<A>>
    where
        A: std::error::Error + Send + Sync + 'static,
        F: FnOnce() -> A;

    /// Raise a new exception on the [`Exn`] inside the [`Result`], but erase its type.
    ///
    /// Apply [`Exn::erased`] on the `Err` variant, refer to it for more information.
    fn or_erased(self) -> Result<Self::Success, Exn>;

    /// Raise a new exception on the [`Exn`] inside the [`Result`], and type-erase the result.
    ///
    /// Apply [`Exn::raise`] and [`Exn::erased`] on the `Err` variant, refer to it for more information.
    fn or_raise_erased<A, F>(self, err: F) -> Result<Self::Success, Exn>
    where
        A: std::error::Error + Send + Sync + 'static,
        F: FnOnce() -> A;
}

impl<T, E> ResultExt for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    type Success = T;
    type Error = E;

    #[track_caller]
    fn or_raise<A, F>(self, err: F) -> Result<Self::Success, Exn<A>>
    where
        A: std::error::Error + Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(Exn::new(e).raise(err())),
        }
    }

    #[track_caller]
    fn or_erased(self) -> Result<Self::Success, Exn> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(Exn::new(e).erased()),
        }
    }

    #[track_caller]
    fn or_raise_erased<A, F>(self, err: F) -> Result<Self::Success, Exn>
    where
        A: Error + Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        self.or_raise(err).map_err(Exn::erased)
    }
}

impl<T, E> ResultExt for std::result::Result<T, Exn<E>>
where
    E: std::error::Error + Send + Sync + 'static,
{
    type Success = T;
    type Error = E;

    #[track_caller]
    fn or_raise<A, F>(self, err: F) -> Result<Self::Success, Exn<A>>
    where
        A: std::error::Error + Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.raise(err())),
        }
    }

    #[track_caller]
    fn or_erased(self) -> Result<Self::Success, Exn> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.erased()),
        }
    }

    #[track_caller]
    fn or_raise_erased<A, F>(self, err: F) -> Result<Self::Success, Exn>
    where
        A: Error + Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        self.or_raise(err).map_err(Exn::erased)
    }
}
