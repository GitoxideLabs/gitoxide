//! Common error types and utilities for error handling.
//!
//! # Usage
//!
//! * When there is **no callee error** to track, use *simple* `std::error::Error` implementations directly,
//!   via `Result<_, Simple>`.
//! * When there **is callee error to track** *in a `gix-plumbing`*, use `Result<_, Exn<Simple>>`.
//!      - Remember that `Exn<Simple>` does not implement `std::error::Error` so it's not easy to use outside `gix-` crates.
//! * When there **is callee error to track** *in a `gix`*, convert both `std::error::Error` and `Exn<E>` into [`Error`]
//!
#![deny(missing_docs, unsafe_code)]
/// A result type to hide the [Exn] error wrapper.
mod exn;

pub use exn::{ErrorExt, Exn, OptionExt, ResultExt};

/// An error type that wraps an inner type-erased boxed `std::error::Error` or an `Exn` frame.
pub struct Error {
    #[expect(dead_code)]
    inner: Inner,
}

#[expect(dead_code)]
enum Inner {
    Boxed(Box<dyn std::error::Error + Send + Sync>),
    Exn(Box<exn::Frame>),
}

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
