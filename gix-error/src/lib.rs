//! Common error types and utilities for error handling.
//!
//! # Usage
//!
//! * When there is **no callee error** to track, use *simple* `std::error::Error` implementations directly,
//!   e.g. `Result<_, Simple>`.
//! * When there **is callee error to track** *in a `gix-plumbing`*, use e.g. `Result<_, Exn<Simple>>`.
//!      - Remember that `Exn<T>` does not implement `std::error::Error` so it's not easy to use outside `gix-` crates.
//!      - Use the type-erased version in callbacks like [`Exn`] (without type arguments), i.e. `Result<T, Exn>`.
//! * When there **is callee error to track** *in the `gix` crate*, convert both `std::error::Error` and `Exn<E>` into [`Error`]
//!
//! # Standard Error Types
//!
//! These should always be used if they match the meaning of the error well enough instead of creating an own
//! [`Error`](std::error::Error)-implementing type, and used with
//! [`ResultExt::or_raise(<StandardErrorType>)`](ResultExt::or_raise) or
//! [`OptionExt::ok_or_raise(<StandardErrorType>)`](OptionExt::ok_or_raise), or sibling methods.
//!
//! All these types implement [`Error`](std::error::Error).
//!
//! ## [`Message`]
//!
//! The baseline that provides a formatted message.
//! Formatting can more easily be done with the [`message!`] macro as convenience, roughly equivalent to
//! [`Message::new(format!("…"))`](Message::new) or `format!("…").into()`.
//!
//! ## Specialised types
//!
//! - [`ValidationError`]
//!    - like [`Message`], but can optionally store the input that caused the failure.
//!
//! # [`Exn<ErrorType>`](Exn) and [`Exn`]
//!
//! The [`Exn`] type does not implement [`Error`](std::error::Error) itself, but is able to store causing errors
//! via [`ResultExt::or_raise()`] (and sibling methods) as well as location information of the creation site.
//!
//! While plumbing functions that need to track causes should always return a distinct type like [`Exn<Message>`](Exn),
//! if that's not possible, use [`Exn::erased`] to let it return `Result<T, Exn>` instead, allowing any return type.
//!
//! A side effect of this is that any callee that causes errors needs to be annotated with
//! `.or_raise(|| message!("context information"))` or `.or_raise_erased(|| message!("context information"))`.
//!
//! # Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
//! # Why not `anyhow`?
//!
//! `anyhow` is a proven and optimized library, and it would certainly suffice for an error-chain based approach
//! where users are expected to downcast to concrete types.
//!
//! What's missing though is `track-caller` which will always capture the location of error instantiation, along with
//! compatibility for error trees, which are happening when multiple calls are in flight during concurrency.
//!
//! Both libraries share the shortcoming of not being able to implement `std::error::Error` on their error type,
//! and both provide workarounds.
//!
//! `exn` is much less optimized, but also costs only a `Box` on the stack,
//! which in any case is a step up from `thiserror` which exposed a lot of heft to the stack.
#![deny(missing_docs, unsafe_code)]
/// A result type to hide the [Exn] error wrapper.
mod exn;

pub use bstr;
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
///
/// # The `auto-chain-error` feature
///
/// If it's enabled, this type is merely a wrapper around [`ChainedError`]. This happens automatically
/// so applications that require this don't have to go through an extra conversion.
///
/// When both the `tree-error` and `auto-chain-error` features are enabled, the `tree-error`
/// behavior takes precedence and this type uses the tree-based representation.
pub struct Error {
    #[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
    inner: error::Inner,
    #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
    inner: ChainedError,
}

mod error;

/// Various kinds of concrete errors that implement [`std::error::Error`].
mod concrete;
pub use concrete::chain::ChainedError;
pub use concrete::message::{message, Message};
pub use concrete::validate::ValidationError;

pub(crate) fn write_location(f: &mut std::fmt::Formatter<'_>, location: &std::panic::Location) -> std::fmt::Result {
    write!(f, ", at {}:{}", location.file(), location.line())
}
