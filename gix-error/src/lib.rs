//! Common error types and utilities for error handling.
//!
//! # Usage
//!
//! * When there is **no callee error** to track, use *simple* `std::error::Error` implementations directly,
//!   e.g. `Result<_, Simple>`.
//!      - If call-site tracking is important, prefer `Result<_, Exn<Simple>>` instead:
//!        [`Exn`] stores the location where the error was raised, which plain error values do not.
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
//!    - For message-only validation failures, use `Into<ValidationError>` conversions instead of spelling out
//!      [`ValidationError::new()`]:
//! ```rust,ignore
//! // All of these produce ValidationError:
//! return Err(message("eof reading amount of bits").into());
//! let value = maybe_value.ok_or(message("missing value").into())?;
//! let header = maybe_header.ok_or("missing header".into())?;
//! ```
//!    - Use [`ValidationError::new_with_input()`] when you need to preserve the offending input.
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
//! # Using `Exn` (bare) in closure *bounds*
//!
//! Callback and closure **bounds** should use `Result<T, Exn>` (bare, without a type parameter)
//! rather than `Result<T, Exn<Message>>` or any other specific type. This allows callers to
//! return any error type from their callbacks without being forced into `Message`.
//!
//! Note that functions should still return the most specific type possible (usually `Exn<Message>`);
//! only the *bound* on the callback parameter should use the bare `Exn`.
//!
//! ```rust,ignore
//! // GOOD — callback bound is flexible, function return is specific:
//! fn process(cb: impl FnMut() -> Result<(), Exn>) -> Result<(), Exn<Message>> { ... }
//!
//! // BAD — forces caller to construct Message errors in their callback:
//! fn process(cb: impl FnMut() -> Result<(), Exn<Message>>) -> Result<(), Exn<Message>> { ... }
//! ```
//!
//! Inside the function, use [`.or_raise()`](ResultExt::or_raise) to convert the bare `Exn` from the
//! callback into the function's typed error, adding context:
//! ```rust,ignore
//! let entry = callback().or_raise(|| message("context about the callback call"))?;
//! ```
//!
//! Inside a closure that must return bare `Exn`, use [`.or_erased()`](ResultExt::or_erased) to
//! convert a typed `Exn<E>` to `Exn`, or [`raise_erased()`](ErrorExt::raise_erased) for standalone errors:
//! ```rust,ignore
//! |stream| {
//!     stream.next_entry().or_erased()   // Exn<Message> → Exn
//! }
//! ```
//!
//! # [`Error`] — `Exn` with `std::error::Error`
//!
//! Since [`Exn`] does not implement [`std::error::Error`], it cannot be used where that trait is required
//! (e.g. `std::io::Error::other()`, or as a `#[source]` in another error type).
//! The [`Error`] type bridges this gap: it implements [`std::error::Error`] and converts from any
//! [`Exn<E>`](Exn) via [`From`], preserving the full error tree and location information.
//!
//! ```rust,ignore
//! // Convert an Exn to something usable as std::error::Error:
//! let exn: Exn<Message> = message("something failed").raise();
//! let err: gix_error::Error = exn.into();
//! let err: gix_error::Error = exn.into_error();
//!
//! // Useful where std::error::Error is required:
//! std::io::Error::other(exn.into_error())
//! ```
//!
//! It can also be created directly from any `std::error::Error` via [`Error::from_error()`].
//!
//! # Migrating from `thiserror`
//!
//! This section describes the mechanical translation from `thiserror` error enums to `gix-error`.
//! In `Cargo.toml`, replace `thiserror = "<version>"` with `gix-error = { version = "^0.1.0", path = "../gix-error" }`.
//!
//! ## Choosing the replacement type
//!
//! There are two decisions: whether to wrap in [`Exn`], and which error type to use.
//!
//! **With or without [`Exn`]:**
//!
//! | `thiserror` enum shape                                      | Wrap in `Exn`? |
//! |--------------------------------------------------------------|----------------|
//! | All variants are simple messages (no `#[from]`/`#[source]`)  | No             |
//! | Has `#[from]` or `#[source]` (wraps callee errors)           | Yes            |
//!
//! **Which error type** (used directly or as the `E` in `Exn<E>`):
//!
//! | Semantics                                                    | Error type            |
//! |--------------------------------------------------------------|-----------------------|
//! | General-purpose error messages                                | [`Message`]           |
//! | Validation/parsing, optionally storing the offending input   | [`ValidationError`]   |
//!
//! For example, a validation function with no callee errors returns `Result<_, ValidationError>`,
//! while a function that wraps I/O errors during parsing could return `Result<_, Exn<ValidationError>>`.
//! When in doubt, [`Message`] is the default choice.
//!
//! ## Translating variants
//!
//! The translation depends on the chosen return type. When the function returns a plain error
//! type like `Result<_, Message>`, return the error directly. When it returns `Result<_, Exn<_>>`,
//! use [`.raise()`](ErrorExt::raise) to wrap the error into an [`Exn`].
//!
//! **Static message variant:**
//! ```rust,ignore
//! // BEFORE:
//! #[error("something went wrong")]
//! SomethingFailed,
//! // → Err(Error::SomethingFailed)
//!
//! // AFTER (returning Message):
//! // → Err(message("something went wrong"))
//!
//! // AFTER (returning Exn<Message>):
//! // → Err(message("something went wrong").raise())
//! ```
//!
//! **Formatted message variant:**
//! ```rust,ignore
//! // BEFORE:
//! #[error("unsupported format '{format:?}'")]
//! Unsupported { format: Format },
//! // → Err(Error::Unsupported { format })
//!
//! // AFTER (returning Message):
//! // → Err(message!("unsupported format '{format:?}'"))
//!
//! // AFTER (returning Exn<Message>):
//! // → Err(message!("unsupported format '{format:?}'").raise())
//! ```
//!
//! **`#[from]` / `#[error(transparent)]` variant** — delete the variant;
//! at each call site, use [`ResultExt::or_raise()`] to add context:
//! ```rust,ignore
//! // BEFORE:
//! #[error(transparent)]
//! Io(#[from] std::io::Error),
//! // → something_that_returns_io_error()?  // auto-converted via From
//!
//! // AFTER (the variant is deleted):
//! // → something_that_returns_io_error()
//! //       .or_raise(|| message("context about what failed"))?
//! ```
//!
//! **`#[source]` variant with message** — use [`ResultExt::or_raise()`]:
//! ```rust,ignore
//! // BEFORE:
//! #[error("failed to parse config")]
//! Config(#[source] config::Error),
//! // → Err(Error::Config(err))
//!
//! // AFTER:
//! // → config_call().or_raise(|| message("failed to parse config"))?
//! ```
//!
//! **Guard / assertion** — use [`ensure!`]:
//! ```rust,ignore
//! // BEFORE:
//! if !condition {
//!     return Err(Error::SomethingFailed);
//! }
//!
//! // AFTER (returning ValidationError):
//! ensure!(condition, ValidationError::new("something went wrong"));
//!
//! // AFTER (returning Exn<Message>):
//! ensure!(condition, message("something went wrong"));
//! ```
//!
//! ## Updating the function signature
//!
//! Change the return type, and add the necessary imports:
//! ```rust,ignore
//! // BEFORE:
//! fn parse(input: &str) -> Result<Value, Error> { ... }
//!
//! // AFTER (no callee errors wrapped):
//! fn parse(input: &str) -> Result<Value, Message> { ... }
//!
//! // AFTER (callee errors wrapped):
//! use gix_error::{message, ErrorExt, Exn, Message, ResultExt};
//! fn parse(input: &str) -> Result<Value, Exn<Message>> { ... }
//! ```
//!
//! ## Updating tests
//!
//! Pattern-matching on enum variants can be replaced with string assertions:
//! ```rust,ignore
//! // BEFORE:
//! assert!(matches!(result.unwrap_err(), Error::SomethingFailed));
//!
//! // AFTER:
//! assert_eq!(result.unwrap_err().to_string(), "something went wrong");
//! ```
//!
//! To access error-specific metadata (e.g. the `input` field on [`ValidationError`]),
//! use [`Exn::downcast_any_ref()`] to find a specific error type within the error tree:
//! ```rust,ignore
//! // BEFORE:
//! match result.unwrap_err() {
//!     Error::InvalidInput { input } => assert_eq!(input, "bad"),
//!     other => panic!("unexpected: {other}"),
//! }
//!
//! // AFTER:
//! let err = result.unwrap_err();
//! let ve = err.downcast_any_ref::<ValidationError>().expect("is a ValidationError");
//! assert_eq!(ve.input.as_deref(), Some("bad".into()));
//! ```
//!
//! # Common Pitfalls
//!
//! ## Don't use `.erased()` to change the `Exn` type parameter
//!
//! [`Exn::raise()`] already nests the current `Exn<E>` as a child of a new `Exn<T>`,
//! so there is no need to erase the type first. Use [`ErrorExt::and_raise()`] as shorthand:
//! ```rust,ignore
//! // WRONG — double-boxes and discards type information:
//! io_err.raise().erased().raise(message("context"))
//!
//! // OK — raise() nests the Exn<io::Error> as a child of Exn<Message> directly:
//! io_err.raise().raise(message("context"))
//!
//! // BEST — and_raise() is a shorthand for .raise().raise():
//! io_err.and_raise(message("context"))
//! ```
//!
//! Only use [`.erased()`](Exn::erased) when you genuinely need a type-erased `Exn` (no type parameter),
//! e.g. to return different error types from the same function via `Result<T, Exn>`.
//!
//! ## Don't use `.raise_all()` with a single error
//!
//! [`Exn::raise_all()`] is meant for creating error trees with *multiple* causes.
//! If you only have a single causing error, use [`.or_raise()`](ResultExt::or_raise) instead:
//! ```rust,ignore
//! // WRONG — raise_all() is for multiple causes, not a single one:
//! result.map_err(|e| message("context").raise_all(Some(e.raise())))?;
//!
//! // RIGHT — or_raise() wraps the error with context directly:
//! result.or_raise(|| message("context"))?;
//! ```
//!
//! ## Convert `Exn` to [`Error`] at public API boundaries
//!
//! Porcelain crates (like `gix`) should not expose [`Exn<Message>`](Exn) in their public API
//! because it does not implement [`std::error::Error`], which makes it incompatible
//! with `anyhow`, `Box<dyn Error>`, and the `?` operator in those contexts.
//!
//! Instead, convert to [`Error`] (which does implement `std::error::Error`) at the boundary:
//! ```rust,ignore
//! // In the porcelain crate's error module:
//! pub type Error = gix_error::Error;  // not gix_archive::Error (which is Exn<Message>)
//!
//! // The conversion happens automatically via From<Exn<E>> for Error,
//! // so `?` works without explicit .into_error() calls.
//! ```
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

/// A Result type that uses the [`Error`] type.
pub type Result<T = ()> = std::result::Result<T, Error>;

mod error;

/// Various kinds of concrete errors that implement [`std::error::Error`].
mod concrete;
pub use concrete::chain::ChainedError;
pub use concrete::message::{message, Message};
pub use concrete::validate::ValidationError;

pub(crate) fn write_location(f: &mut std::fmt::Formatter<'_>, location: &std::panic::Location) -> std::fmt::Result {
    write!(f, ", at {}:{}", location.file(), location.line())
}
