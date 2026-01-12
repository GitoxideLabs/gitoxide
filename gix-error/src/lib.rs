//! Common error types and utilities for error handling.
//!
//! # Usage
//!
//! * When there is **no callee error** to track, use *simple* `std::error::Error` implementations directly,
//!   via `Result<_, Simple>`.
//! * When there **is callee error to track** *in a `gix-plumbing`*, use `Result<_, Exn<Simple>>`.
//!      - Remember that `Exn<Simple>` does not implement `std::error::Error` so it's not easy to use outside `gix-` crates.
//!      - Use the type-erased version in callbacks like [`Exn`] (without type arguments).
//! * When there **is callee error to track** *in a `gix`*, convert both `std::error::Error` and `Exn<E>` into [`Error`]
//!
#![deny(missing_docs, unsafe_code)]
/// A result type to hide the [Exn] error wrapper.
mod exn;

#[cfg(feature = "anyhow")]
mod anyhow;

#[cfg(feature = "anyhow")]
pub use self::anyhow::IntoAnyhow;

pub use exn::{ErrorExt, Exn, Frame, OptionExt, ResultExt, Something, Untyped};

/// An error type that wraps an inner type-erased boxed `std::error::Error` or an `Exn` frame.
///
/// In that, it's similar to `anyhow`, but with support for tracking the call site and trees of errors.
///
/// # Warning: `source()` information is stringified and type-erased
///
/// All `source()` values when created with [`Error::from_error()`] are turned into frames,
/// but lose their type information completely.
/// This is because they are only seen as reference and thus can't be stored.
pub struct Error {
    inner: error::Inner,
}

mod error;

mod message {
    use std::borrow::Cow;
    use std::fmt::{Debug, Display, Formatter};

    /// An error that is further described in a message.
    #[derive(Debug)]
    pub struct Message(
        /// The error message.
        pub Cow<'static, str>,
    );

    /// Lifecycle
    impl Message {
        /// Create a new instance that displays the given `message`.
        pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
            Message(message.into())
        }
    }

    impl Display for Message {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.write_str(self.0.as_ref())
        }
    }

    impl std::error::Error for Message {}
}
pub use message::Message;

mod parse {
    use bstr::BString;
    use std::borrow::Cow;
    use std::fmt::{Debug, Display, Formatter};

    /// An error occurred when parsing input
    #[derive(Debug)]
    pub struct ParseError {
        /// The error message.
        pub message: Cow<'static, str>,
        /// The input or portion of the input that failed to parse.
        pub input: Option<BString>,
    }

    /// Lifecycle
    impl ParseError {
        /// Create a new error with `message` and `input`. Note that `input` isn't printed.
        pub fn new_with_input(message: impl Into<Cow<'static, str>>, input: impl Into<BString>) -> Self {
            ParseError {
                message: message.into(),
                input: Some(input.into()),
            }
        }

        /// Create a new instance that displays the given `message`.
        pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
            ParseError {
                message: message.into(),
                input: None,
            }
        }
    }

    impl Display for ParseError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match &self.input {
                None => f.write_str(self.message.as_ref()),
                Some(input) => {
                    write!(f, "{}: {input}", self.message)
                }
            }
        }
    }

    impl std::error::Error for ParseError {}
}
pub use parse::ParseError;
