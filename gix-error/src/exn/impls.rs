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

use std::fmt;
use std::fmt::Formatter;
use std::marker::PhantomData;
use std::ops::Deref;
use std::panic::Location;

/// An error that merely says that something is wrong.
/// It's the default type for [Exn], indicating that this is about the error chain,
/// not about this specific error.
#[derive(Debug)]
pub struct Something;

impl fmt::Display for Something {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Something went wrong")
    }
}

impl std::error::Error for Something {}

/// An exception type that can hold an [error tree](Exn::from_iter) and the call site.
///
/// While an error chain, a list, is automatically created when [raise](Exn::raise)
/// and friends are invoked, one can also use [`Exn::from_iter`] to create an error
/// that has multiple causes.
///
/// # `Exn` == `Exn<Something>`
///
/// `Exn` act's like `Box<dyn std::error::Error + Send + Sync + 'static>`, but with the capability
/// to store a tree of errors along with their *call sites*.
pub struct Exn<E: std::error::Error + Send + Sync + 'static = Something> {
    // trade one more indirection for less stack size
    frame: Box<Frame>,
    phantom: PhantomData<E>,
}

impl<E: std::error::Error + Send + Sync + 'static> From<E> for Exn<E> {
    #[track_caller]
    fn from(error: E) -> Self {
        Exn::new(error)
    }
}

impl<E: std::error::Error + Send + Sync + 'static> Exn<E> {
    /// Create a new exception with the given error.
    ///
    /// This will automatically walk the [source chain of the error] and add them as children
    /// frames.
    ///
    /// See also [`ErrorExt::raise`](crate::ErrorExt) for a fluent way to convert an error into an `Exn` instance.
    ///
    /// Note that **sources of `error` are degenerated to their string representation** and all type information is erased.
    ///
    /// [source chain of the error]: std::error::Error::source
    #[track_caller]
    pub fn new(error: E) -> Self {
        /// A way to keep all information of errors returned by `source()` chains.
        struct SourceError {
            display: String,
            debug: String,
            alt_debug: String,
        }

        impl fmt::Debug for SourceError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let dbg = if f.alternate() { &self.alt_debug } else { &self.debug };
                f.write_str(dbg)
            }
        }

        impl fmt::Display for SourceError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.display)
            }
        }

        impl std::error::Error for SourceError {}

        fn walk_sources(error: &dyn std::error::Error, location: &'static Location<'static>) -> Vec<Frame> {
            if let Some(source) = error.source() {
                let children = vec![Frame {
                    error: Box::new(SourceError {
                        display: source.to_string(),
                        debug: format!("{:?}", source),
                        alt_debug: format!("{:#?}", source),
                    }),
                    location,
                    children: walk_sources(source, location),
                }];
                children
            } else {
                vec![]
            }
        }

        let location = Location::caller();
        let children = walk_sources(&error, location);
        let frame = Frame {
            error: Box::new(error),
            location,
            children,
        };

        Self {
            frame: Box::new(frame),
            phantom: PhantomData,
        }
    }

    /// Create a new exception with the given error and children.
    ///
    /// It's no error if `children` is empty.
    #[track_caller]
    pub fn from_iter<T, I>(children: I, err: E) -> Self
    where
        T: std::error::Error + Send + Sync + 'static,
        I: IntoIterator,
        I::Item: Into<Exn<T>>,
    {
        let mut new_exn = Exn::new(err);
        for exn in children {
            let exn = exn.into();
            new_exn.frame.children.push(*exn.frame);
        }
        new_exn
    }

    /// Raise a new exception; this will make the current exception a child of the new one.
    #[track_caller]
    pub fn raise<T: std::error::Error + Send + Sync + 'static>(self, err: T) -> Exn<T> {
        let mut new_exn = Exn::new(err);
        new_exn.frame.children.push(*self.frame);
        new_exn
    }

    /// Use the current exception the head of a chain, adding `err` to its children.
    #[track_caller]
    pub fn chain<T: std::error::Error + Send + Sync + 'static>(mut self, err: impl Into<Exn<T>>) -> Exn<E> {
        let err = err.into();
        self.frame.children.push(*err.frame);
        self
    }

    /// Raise a new [`Something`] exception; this will make the current exception a child [`Something`].
    pub fn something(self) -> Exn<Something> {
        self.raise(Something)
    }

    /// Return the current exception.
    pub fn as_error(&self) -> &E {
        self.frame
            .error
            .downcast_ref()
            .expect("the owned frame always matches the compile-time error type")
    }

    /// Discard all error context and return the underlying error.
    ///
    /// This may be needed to obtain something that once again implements `std::error::Error`.
    pub fn into_box(self) -> Box<E> {
        match self.frame.error.downcast() {
            Ok(err) => err,
            Err(_) => unreachable!("The type in the frame is always the type of this instance"),
        }
    }

    /// Turn ourselves into a type that implements [`std::error::Error`].
    pub fn into_error(self) -> crate::Error {
        self.into()
    }

    /// Return the underlying exception frame.
    pub fn as_frame(&self) -> &Frame {
        &self.frame
    }
}

impl<E> Deref for Exn<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    type Target = E;

    fn deref(&self) -> &Self::Target {
        self.as_error()
    }
}

/// A frame in the exception tree.
pub struct Frame {
    /// The error that occurred at this frame.
    error: Box<dyn std::error::Error + Send + Sync + 'static>,
    /// The source code location where this exception frame was created.
    location: &'static Location<'static>,
    /// Child exception frames that provide additional context or source errors.
    children: Vec<Frame>,
}

impl Frame {
    /// Return the error as a reference to [`std::error::Error`].
    pub fn as_error(&self) -> &(dyn std::error::Error + 'static) {
        &*self.error
    }

    /// Return the source code location where this exception frame was created.
    pub fn location(&self) -> &'static Location<'static> {
        self.location
    }

    /// Return a slice of the children of the exception.
    pub fn children(&self) -> &[Frame] {
        &self.children
    }
}

impl<E> From<Exn<E>> for Box<Frame>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(err: Exn<E>) -> Self {
        err.frame
    }
}

impl<E> From<Exn<E>> for Frame
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(err: Exn<E>) -> Self {
        *err.frame
    }
}
