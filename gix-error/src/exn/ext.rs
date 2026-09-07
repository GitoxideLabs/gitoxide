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

use crate::Exn;

/// Convenience methods for raising standard errors using upstream exception storage.
pub trait ErrorExt: std::error::Error + Send + Sync + 'static {
    /// Raise this error and capture the caller location.
    #[track_caller]
    fn raise(self) -> Exn<Self>
    where
        Self: Sized,
    {
        Exn::from_upstream(::exn::ErrorExt::raise(self))
    }

    /// Raise this error under a new typed context.
    #[track_caller]
    fn and_raise<T: std::error::Error + Send + Sync + 'static>(self, context: T) -> Exn<T>
    where
        Self: Sized,
    {
        self.raise().raise(context)
    }

    /// Raise this error with an erased root marker.
    #[track_caller]
    fn raise_erased(self) -> Exn
    where
        Self: Sized,
    {
        self.into()
    }

    /// Raise this error over all the provided causes, in iteration order.
    #[track_caller]
    fn raise_all(self, sources: impl IntoIterator<Item: Into<::exn::Exn>>) -> Exn<Self>
    where
        Self: Sized,
    {
        Exn::raise_all(sources, self)
    }
}

impl<T: std::error::Error + Send + Sync + 'static> ErrorExt for T {}

/// Contextualize missing values with exceptions.
pub trait OptionExt {
    /// The present value type.
    type Some;

    /// Construct an exception when the value is absent.
    fn ok_or_raise<A: std::error::Error + Send + Sync + 'static>(
        self,
        error: impl FnOnce() -> A,
    ) -> Result<Self::Some, Exn<A>>;

    /// Construct a type-erased exception when the value is absent.
    fn ok_or_raise_erased<A: std::error::Error + Send + Sync + 'static>(
        self,
        error: impl FnOnce() -> A,
    ) -> Result<Self::Some, Exn>;
}

impl<T> OptionExt for Option<T> {
    type Some = T;

    #[track_caller]
    fn ok_or_raise<A: std::error::Error + Send + Sync + 'static>(self, error: impl FnOnce() -> A) -> Result<T, Exn<A>> {
        ::exn::OptionExt::ok_or_raise(self, error).map_err(Exn::from_upstream)
    }

    #[track_caller]
    fn ok_or_raise_erased<A: std::error::Error + Send + Sync + 'static>(
        self,
        error: impl FnOnce() -> A,
    ) -> Result<T, Exn> {
        self.ok_or_raise(error).map_err(Exn::erased)
    }
}

/// Add context to standard errors and typed or erased exceptions.
pub trait ResultExt {
    /// The successful value type.
    type Success;

    /// Add a new typed parent to an error.
    fn or_raise<A: std::error::Error + Send + Sync + 'static>(
        self,
        error: impl FnOnce() -> A,
    ) -> Result<Self::Success, Exn<A>>;

    /// Erase the root type, preserving its tree and runtime error types.
    fn or_erased(self) -> Result<Self::Success, Exn>;

    /// Add a parent and erase the resulting root marker.
    fn or_raise_erased<A: std::error::Error + Send + Sync + 'static>(
        self,
        error: impl FnOnce() -> A,
    ) -> Result<Self::Success, Exn>;
}

impl<T, E: Into<Exn>> ResultExt for Result<T, E> {
    type Success = T;

    #[track_caller]
    fn or_raise<A: std::error::Error + Send + Sync + 'static>(self, error: impl FnOnce() -> A) -> Result<T, Exn<A>> {
        ::exn::ResultExt::or_raise(self.or_erased().map_err(|error| error.inner), error).map_err(Exn::from_upstream)
    }

    #[track_caller]
    fn or_erased(self) -> Result<T, Exn> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into()),
        }
    }

    #[track_caller]
    fn or_raise_erased<A: std::error::Error + Send + Sync + 'static>(
        self,
        error: impl FnOnce() -> A,
    ) -> Result<T, Exn> {
        self.or_raise(error).map_err(Exn::erased)
    }
}

/// Conversion for boxed standard errors, which do not satisfy the standard error blanket bound.
pub trait BoxedResultExt {
    /// The successful value type.
    type Success;
    /// Raise the boxed error with an erased root marker.
    fn or_erased(self) -> Result<Self::Success, Exn>;
}

impl<T> BoxedResultExt for Result<T, Box<dyn std::error::Error + Send + Sync + 'static>> {
    type Success = T;

    #[track_caller]
    fn or_erased(self) -> Result<T, Exn> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(crate::Untyped::from_boxed(error).into()),
        }
    }
}
