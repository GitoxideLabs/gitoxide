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
use std::marker::PhantomData;
use std::panic::Location;

/// An exception type that can hold an error tree and additional context.
///
/// While an error chain, a list, is automatically created when [raise](Exn::raise)
/// and friends are invoked, one can also use [`Exn::from_iter`] to create an error
/// that has multiple causes.
pub struct Exn<E: std::error::Error + Send + Sync + 'static> {
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
    /// [source chain of the error]: std::error::Error::source
    #[track_caller]
    pub fn new(error: E) -> Self {
        struct SourceError(String);

        impl fmt::Debug for SourceError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Debug::fmt(&self.0, f)
            }
        }

        impl fmt::Display for SourceError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }

        impl std::error::Error for SourceError {}

        fn walk(error: &dyn std::error::Error, location: &'static Location<'static>) -> Vec<Frame> {
            if let Some(source) = error.source() {
                let children = vec![Frame {
                    error: Box::new(SourceError(source.to_string())),
                    location,
                    children: walk(source, location),
                }];
                children
            } else {
                vec![]
            }
        }

        let location = Location::caller();
        let children = walk(&error, location);
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

    /// Return the underlying exception frame.
    pub fn as_frame(&self) -> &Frame {
        &self.frame
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
    pub fn as_error(&self) -> &dyn std::error::Error {
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
