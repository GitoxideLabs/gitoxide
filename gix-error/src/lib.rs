//! Common error types and utilities for error handling.
//!
//! # Usage
//!
//! * When there is **no callee error** to track, use *simple* `std::error::Error` implementations directly,
//!   e.g. `Result<_, Simple>`.
//!      - If call-site tracking is important, prefer `ExnResult<_, Simple>` instead:
//!        [`Exn`] stores the location where the error was raised, which plain error values do not.
//! * When there **is callee error to track** *in a `gix-plumbing`*, use e.g. `ExnResult<_, Simple>`.
//!      - Remember that `Exn<T>` does not implement `std::error::Error` so it's not easy to use outside `gix-` crates.
//!      - Use the type-erased version in callbacks like [`Exn`] (without type arguments), i.e. `ExnResult<T>`.
//! * When there **is callee error to track** *in the `gix` crate*, convert both `std::error::Error` and `Exn<E>` into [`Error`]
//!
//! [`ExnResult<T, E>`](ExnResult) abbreviates a result with an [`Exn<E>`](Exn) error. Its defaults are
//! `T = ()` and `E = exn::Untyped`, matching bare [`Exn`]. Use `ExnMessageResult<T>` for message contexts.
//!
//! # Standard Error Types
//!
//! Use these types for diagnostic context when recovery does not depend on a specific condition or structured payload.
//! Otherwise retain a concrete [`Error`](std::error::Error)-implementing type, and use it with
//! [`ResultExt::or_raise(<StandardErrorType>)`](ResultExt::or_raise) or
//! [`OptionExt::ok_or_raise(<StandardErrorType>)`](OptionExt::ok_or_raise), or sibling methods.
//!
//! All these types implement [`Error`](std::error::Error).
//!
//! ## [`Message`] and [`ClassificationMarker`]
//!
//! [`Message`] combines a diagnostic message, an optional [`Class`], and named scalar values. Use it
//! for diagnostic context, or instead of a chain of type-bearing errors when those layers only describe a single
//! failure. Keep concrete errors when callers need to match a particular condition, even without a payload.
//! [`not_found()`], [`validation()`], [`corruption()`], [`retryable()`], [`resource_exhaustion()`],
//! [`allocation_limit()`], [`allocation_failure()`], and [`io()`] construct classified messages.
//! [`message()`] and [`Message::new()`] start without a class or values. [`Message::with_class()`] and
//! [`Message::with()`] add them to the same diagnostic. Use [`message!`] for formatting, equivalent to
//! [`Message::new(format!("…"))`](Message::new) or `format!("…").into()`.
//!
//! Classification does not determine which diagnostic values can be attached. For example,
//! `corruption("Malformed reference").with("input", bytes)` preserves offending bytes in the same
//! error that describes their corruption. No extra validation error is needed just to store input.
//! Use explicit classified constructors: converting a string to [`Message`] does not infer a class
//! from the function's return type.
//!
//! | Type | Diagnostic | Classification | Purpose |
//! |------|------------|----------------|---------|
//! | [`Message`] | Visible message and optional values | Optional | Describe a failure without a custom error type |
//! | [`ClassificationMarker`] | Transparent, no diagnostic of its own | Required | Classify an existing error while preserving its concrete type |
//!
//! ```
//! use gix_error::{ErrorExt, Message, MetadataValue};
//!
//! let error = gix_error::not_found("Reference does not exist")
//!     .with("path", std::path::Path::new("HEAD"))
//!     .raise();
//! assert!(error.is_not_found());
//! assert!(error.probable_cause().is::<Message>());
//! assert_eq!(error.metadata().next().expect("lookup details")["path"], MetadataValue::Path("HEAD".into()));
//! ```
//!
//! Callers should add context using information they already possess and document its keys on the function
//! that returns it. Preserve real callee errors, especially concrete recovery signals and complex results
//! discovered by the callee, such as partial outcomes:
//!
//! ```
//! use gix_error::{message, ResultExt, MetadataValue};
//!
//! let error = Err::<(), _>(std::io::Error::from(std::io::ErrorKind::NotFound))
//!     .or_raise(|| message("Could not read reference").with("path", std::path::Path::new("HEAD")))
//!     .expect_err("the lookup failed");
//! assert!(error.is_not_found());
//! assert!(error.probable_cause().is::<std::io::Error>());
//! assert_eq!(error.error().class, None, "the callee, not the context, supplies the classification");
//! let values = error.metadata().next().expect("lookup context");
//! assert_eq!(values["path"], MetadataValue::Path("HEAD".into()));
//! ```
//!
//! [`Exn::metadata()`] and [`Error::metadata()`] yield each message's non-empty [`Metadata`] dictionary in error traversal order.
//! Each dictionary maps names to [`MetadataValue`]s. Keys are local to their context; dictionaries from independent causes
//! are never combined. To identify a specific failure without inspecting its values, see
//! [matching a specific failure](#matching-a-specific-failure).
//!
//! # [`Exn<ErrorType>`](Exn) and [`Exn`]
//!
//! The [`Exn`] type does not implement [`Error`](std::error::Error) itself, but is able to store causing errors
//! via [`ResultExt::or_raise()`] (and sibling methods) as well as location information of the creation site.
//!
//! While plumbing functions that need to track causes should always return a distinct type like [`Exn<Message>`](Exn),
//! if that's not possible, use [`Exn::erased`] to let it return `ExnResult<T>` instead, allowing any return type.
//!
//! A side effect of this is that any callee that causes errors needs to be annotated with
//! `.or_raise(|| message!("context information"))` or `.or_raise_erased(|| message!("context information"))`.
//!
//! # Using [`ExnResult`] in closure *bounds*
//!
//! Callback and closure **bounds** should use `ExnResult<T>` (without an explicit error type)
//! rather than `ExnMessageResult<T>` or any other specific type. This allows callers to
//! return any error type from their callbacks without being forced into `Message`.
//!
//! Functions should still return the most specific type possible (usually `ExnMessageResult<T>`);
//! only the *bound* on the callback parameter should use the default, erased error type.
//!
//! ```rust,ignore
//! use gix_error::{ExnMessageResult, ExnResult};
//!
//! // GOOD — callback bound is flexible, function return is specific:
//! fn process(cb: impl FnMut() -> ExnResult) -> ExnMessageResult { ... }
//!
//! // BAD — forces caller to construct Message errors in their callback:
//! fn process(cb: impl FnMut() -> ExnMessageResult) -> ExnMessageResult { ... }
//! ```
//!
//! Inside the function, use [`.or_raise()`](ResultExt::or_raise) to convert the bare `Exn` from the
//! callback into the function's typed error, adding context:
//! ```rust,ignore
//! let entry = callback().or_raise(|| message("context about the callback call"))?;
//! ```
//!
//! Inside a closure that must return `ExnResult<T>`, use [`.or_erased()`](ResultExt::or_erased) to
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
//! # Tests with [`TestResult`]
//!
//! Return [`TestResult`] from `#[test]` functions to propagate ordinary errors, [`Exn<E>`](Exn), and [`Error`]
//! directly with `?`. It defaults to `Result<(), TestError>`; helpers returning a value can use `TestResult<T>`.
//! Accepted errors must convert into `Box<dyn std::error::Error + Send + Sync + 'static>`.
//!
//! When a test returns an error, Rust's test harness prints [`TestError`]'s [`Debug`](std::fmt::Debug) output,
//! including the complete diagnostic tree or chain and captured caller locations.
//!
//! ```rust,test_harness
//! use gix_error::{message, ResultExt, TestResult};
//!
//! #[test]
//! fn parses_count() -> TestResult {
//!     let expected: usize = "42".parse()?;
//!     let actual = "42".parse::<usize>().or_raise(|| message("could not parse count"))?;
//!     assert_eq!(actual, expected, "context preserves the parsed count");
//!     Ok(())
//! }
//! ```
//!
//! # Migrating from `thiserror`
//!
//! This section describes the mechanical translation from `thiserror` error enums to `gix-error`.
//! In `Cargo.toml`, replace `thiserror = "<version>"` with `gix-error = { version = "^0.1.0", path = "../gix-error" }`.
//!
//! ## Choosing the replacement type
//!
//! Use [`ExnMessageResult`] for diagnostic messages, including validation failures without callee errors.
//! [`Message`] carries an optional class and named scalar values; [`Exn`] retains the diagnostic context and causes.
//! Keep a concrete error type in [`ExnResult`] when recovery requires a specific condition or structured payload.
//! Use [`Result`] at porcelain boundaries that return [`Error`].
//! Define at most one operation-specific error type, normally a public, `#[non_exhaustive]` enum named `Error`.
//! Related methods should share it. Broad [`Class`] values categorize errors; variants
//! define specific recovery decisions. Preserve genuine callee errors as causes instead of formatting them into text.
//!
//! Use the chosen type directly in signatures, importing it under its canonical name where helpful.
//! Crate-specific and operation-specific forwarding aliases or renamed error exports are unnecessary.
//! Facades may re-export the canonical types, as `gix` does with `Error`, `Exn`, `Result`, `ExnResult`, and `ExnMessageResult`.
//! Always import the result aliases directly and use their bare names in signatures.
//!
//! ## Translating variants
//!
//! Translate variants to messages only when they provide diagnostics without a specific recovery contract.
//! Use [`.raise()`](ErrorExt::raise) to wrap standalone errors into an [`Exn`], and
//! [`ResultExt::or_raise()`] to preserve callee errors with additional context.
//!
//! **Static message variant:**
//! ```rust,ignore
//! // BEFORE:
//! #[error("something went wrong")]
//! SomethingFailed,
//! // → Err(Error::SomethingFailed)
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
//! // AFTER (returning Exn<Message>):
//! // → Err(message!("unsupported format '{format:?}'").raise())
//! ```
//!
//! **`#[from]` / `#[error(transparent)]` variant** without a recovery contract — delete the forwarding variant;
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
//! **`#[source]` variant with diagnostic context only** — use [`ResultExt::or_raise()`]:
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
//! // AFTER (returning Exn<Message>, with a validation class):
//! ensure!(condition, gix_error::validation("something went wrong"));
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
//! // AFTER:
//! use gix_error::{message, ErrorExt, ExnMessageResult, ResultExt};
//! fn parse(input: &str) -> ExnMessageResult<Value> { ... }
//! ```
//!
//! ## Updating tests
//!
//! Tests of diagnostic wording can use string assertions:
//! ```rust,ignore
//! assert_eq!(result.expect_err("the operation fails").to_string(), "something went wrong");
//! ```
//! Keep variant assertions for recovery contracts, and test structured payloads directly.
//!
//! For semantic checks, both [`Exn`] and [`Error`] provide [`is_retryable()`](Exn::is_retryable),
//! [`is_not_found()`](Exn::is_not_found), [`is_validation()`](Exn::is_validation),
//! [`is_corrupted()`](Exn::is_corrupted), and [`is_resource_exhausted()`](Exn::is_resource_exhausted).
//! These inspect causes as well as the outermost error. `is_retryable()` requires an explicit retry classification;
//! [`Exn::can_retry()`] and [`Error::can_retry()`] additionally recognize certain I/O error kinds.
//!
//! For application-level interruption or cancellation that permits retrying, use [`retryable()`] instead
//! of an unclassified message or a synthetic [`io()`] diagnostic with `std::io::ErrorKind::Interrupted`.
//! This records [`Class::Retryable`], so both `is_retryable()` and `can_retry()` return `true`.
//! Preserve genuine I/O errors as causes. Classification itself neither clears interruption state nor retries.
//! ```
//! use gix_error::{ErrorExt, retryable};
//!
//! let err = retryable("Cancelled by user").raise();
//! assert!(err.is_retryable());
//! assert!(err.can_retry());
//! ```
//!
//! Use [`Exn::probable_cause()`] to inspect the likely root cause. It follows a single causal path, stopping at the
//! first branch rather than choosing an arbitrary sibling. Classification markers are transparent to this selection.
//! [`Exn::classify()`] and [`Error::classify()`] expose each known classification together with its original error.
//! Custom payloads of [`std::io::Error`] are inspected too, including any nested [`Error`] trees.
//!
//! [`Message`] supplies its own diagnostic and optional classification. In contrast, [`ClassificationMarker`]
//! only supplies classification metadata. Prefer defining intrinsic classifications on error types you control:
//!
//! * For a leaf error or variant whose classification is part of its meaning, return a constant marker from
//!   [`std::error::Error::source()`]. This makes every construction site carry the classification without repeated
//!   [`tag()`] calls.
//! * Use [`tag()`] when a classification depends on the calling context, or when you cannot modify the error type.
//!   It preserves the concrete error and its diagnostic.
//!
//! For example, a caller may know that an `AlreadyExists` I/O error is retryable in its operation:
//! ```
//! use gix_error::{Class, ClassificationMarker, ErrorExt, tag};
//!
//! let err = tag(
//!     std::io::Error::from(std::io::ErrorKind::AlreadyExists),
//!     Class::Retryable,
//! ).raise();
//! assert!(err.is_retryable());
//! assert!(err.probable_cause().is::<std::io::Error>());
//! assert!(err.downcast_any_ref::<ClassificationMarker>().is_none());
//! let classification = err.classify().next().expect("the tag is first");
//! assert!(classification.error().is::<std::io::Error>());
//! assert_eq!(classification.io_kind(), Some(std::io::ErrorKind::AlreadyExists));
//! ```
//!
//! Custom error types preserve classifications by exposing their immediate cause as `Some(inner)` from
//! [`std::error::Error::source()`]. Forwarding to `inner.source()` instead can hide a classification carried by
//! `inner` itself. A custom leaf error can borrow a constant such as [`ClassificationMarker::NOT_FOUND`]
//! as its source to preserve its classification without defining a static or adding a generic category to its diagnostic:
//! ```
//! use gix_error::{ClassificationMarker, ErrorExt};
//!
//! #[derive(Debug)]
//! struct MissingObject;
//!
//! impl std::fmt::Display for MissingObject {
//!     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//!         f.write_str("the requested object is missing from the object database")
//!     }
//! }
//!
//! impl std::error::Error for MissingObject {
//!     fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
//!         Some(const { &ClassificationMarker::NOT_FOUND })
//!     }
//! }
//!
//! let err = MissingObject.raise();
//! assert!(err.is_not_found());
//! assert!(err.probable_cause().is::<MissingObject>());
//! assert!(err.classify().next().expect("a classified owner").error().is::<MissingObject>());
//! ```
//! Use classification predicates rather than downcasting to [`Message`] just to recognize
//! a category: diagnostic iterators and downcasts skip all classification markers. Exception and test reports
//! omit their wrappers too, while raw [`std::error::Error::source()`] chains retain them. Genuine classified errors
//! remain causal and can still be downcast to inspect their payloads. When storing an [`Exn`] in a custom error, convert it with
//! [`Exn::into_error()`] so the source can expose its complete tree.
//!
//! To access scalar diagnostics such as offending input, inspect the documented [metadata](Exn::metadata()) key:
//! ```
//! use gix_error::{ErrorExt, MetadataValue};
//!
//! let err = gix_error::validation("invalid input").with("input", b"bad".as_slice()).raise();
//! let values = err.metadata().find(|values| values.contains_key("input")).expect("input context");
//! assert_eq!(values["input"], MetadataValue::Bytes("bad".into()));
//! ```
//!
//! ## Matching a specific failure
//!
//! Downcast to the operation's error enum and match a variant when a broad category such as [`Class::NotFound`]
//! isn't specific enough for recovery. The enum retains structured data and identifies the condition independently
//! of diagnostic wording. For intrinsic classifications on leaf variants of an enum you define, prefer an exhaustive
//! match on `self` in [`std::error::Error::source()`], returning a constant marker as below. Each new variant then
//! requires an explicit classification choice. Wrapping, erasure, and conversion to [`Error`] preserve the
//! classification, so callers do not need to repeat it with [`tag()`].
//!
//! ```
//! use gix_error::{ErrorExt, message};
//!
//! mod merge {
//!     use gix_error::ClassificationMarker;
//!
//!     #[derive(Debug)]
//!     #[non_exhaustive]
//!     pub enum Error {
//!         MissingBinaryMergeResult,
//!     }
//!
//!     impl std::fmt::Display for Error {
//!         fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//!             match self {
//!                 Self::MissingBinaryMergeResult => f.write_str("The binary merge result could not be selected"),
//!             }
//!         }
//!     }
//!     impl std::error::Error for Error {
//!         fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
//!             match self {
//!                 Self::MissingBinaryMergeResult => Some(const { &ClassificationMarker::NOT_FOUND }),
//!             }
//!         }
//!     }
//! }
//!
//! fn recover(err: gix_error::Error) -> gix_error::Result<()> {
//!     match err.downcast_any_ref::<merge::Error>() {
//!         Some(merge::Error::MissingBinaryMergeResult) => Ok(()), // Apply the caller's fallback.
//!         _ => Err(err), // Preserve unfamiliar variants and all other errors.
//!     }
//! }
//!
//! let err = merge::Error::MissingBinaryMergeResult
//!     .raise()
//!     .raise(message("Tree merge failed"))
//!     .into_error();
//!
//! assert!(err.is_not_found(), "the variant supplies its intrinsic classification");
//! assert!(
//!     matches!(err.downcast_any_ref::<merge::Error>(), Some(merge::Error::MissingBinaryMergeResult)),
//!     "the classified variant remains available for recovery"
//! );
//! recover(err)?;
//! # Ok::<(), gix_error::Error>(())
//! ```
//!
//! Downcasting and [`types::Classification::error()`] retain concrete subjects through contexts and [`Error`] conversion.
//! Matching one cause does not make other failures in an aggregate ignorable.
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
//! e.g. to return different error types from the same function via `ExnResult<T>`.
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
//! Porcelain crates (like `gix`) should **not** expose [`Exn<Message>`](Exn) in their public API
//! because it does not itself implement [`std::error::Error`].
//!
//! Instead, convert to [`Error`] (which does implement `std::error::Error`) at the boundary.
//! [`Exn`] also converts directly into `Box<dyn std::error::Error + Send + Sync>`, so `?` works
//! without an explicit conversion when that is the receiving result's error type:
//! ```rust,ignore
//! fn porcelain_operation() -> Result<(), gix_error::Error> {
//!     // From<Exn<E>> for Error converts the plumbing error at this boundary.
//!     plumbing_operation()?;
//!     Ok(())
//! }
//! ```
//!
//! # Supporting types
//!
//! Frequently used error types, extension traits, result aliases, and constructors are available at the crate root.
//! Utility types for flattened chains, classification, and diagnostic display live in [`types`]. Exception frames
//! and the default type-erasure marker live in [`exn`]; [`Exn`] and its extension traits are only exported at the root.
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
pub mod exn;
pub mod types;

pub use bstr;
pub use exn::{
    ext::{BoxedResultExt, ErrorExt, OptionExt, ResultExt},
    impls::Exn,
};

/// An error type that wraps an inner type-erased boxed `std::error::Error` or an `Exn` frame.
///
/// In that, it's similar to `anyhow`, but with support for tracking the call site and trees of errors.
///
/// # Native error sources
///
/// [`Error::from_error()`] retains the concrete error and its native [`source()`](std::error::Error::source) chain.
/// Use [`Error::downcast_any_ref()`] or [`Error::iter_errors()`] to inspect the original types, including sources
/// within nested [`Error`] values. This also applies when the `auto-chain-error` feature is enabled.
///
/// # The `auto-chain-error` feature
///
/// If it's enabled, this type is merely a wrapper around [`ChainedError`](types::ChainedError). This happens automatically
/// so applications that require this don't have to go through an extra conversion.
///
/// When both the `tree-error` and `auto-chain-error` features are enabled, the `tree-error`
/// behavior takes precedence and this type uses the tree-based representation.
pub struct Error {
    #[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
    inner: error::Inner,
    #[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
    inner: types::ChainedError,
}

fn root_error_eq(mut error: &(dyn std::error::Error + 'static), other: &str) -> bool {
    while let Some(nested) = error.downcast_ref::<Error>() {
        error = nested.error();
    }
    error.to_string() == other
}

impl PartialEq<str> for Error {
    fn eq(&self, other: &str) -> bool {
        root_error_eq(self.error(), other)
    }
}

impl PartialEq<&str> for Error {
    fn eq(&self, other: &&str) -> bool {
        <Self as PartialEq<str>>::eq(self, other)
    }
}

impl PartialEq<String> for Error {
    fn eq(&self, other: &String) -> bool {
        <Self as PartialEq<str>>::eq(self, other)
    }
}

/// A Result type that uses the [`Error`] type.
pub type Result<T = ()> = std::result::Result<T, Error>;

/// A result with an [`Exn<E>`](Exn) error, defaulting to unit success and an erased error type.
///
/// `ExnResult<T>` uses the same [`exn::Untyped`] marker as bare [`Exn`]. Specify `E` to retain a
/// concrete error type; [`ExnMessageResult`] is the shorthand for message contexts. All standard result operations and
/// [`ResultExt`] methods remain available. Use [`ResultExt::or_erased()`] for callbacks accepting
/// different error types, and `?` to propagate exceptions into [`Error`] at API boundaries.
///
/// ```
/// use gix_error::{message, ErrorExt, ExnMessageResult, ExnResult, ResultExt};
///
/// fn parse_count(input: &str) -> ExnMessageResult<u64> {
///     input.parse::<u64>().or_raise(|| message("could not parse count"))
/// }
///
/// fn process(callback: impl FnOnce() -> ExnResult<u64>) -> ExnMessageResult {
///     let count = callback().or_raise(|| message("callback failed"))?;
///     assert_eq!(count, 42, "the callback supplies the parsed count");
///     Ok(())
/// }
///
/// let done: ExnResult = process(|| parse_count("42").or_erased()).or_erased();
/// done?;
///
/// let io: ExnResult<(), std::io::Error> =
///     Err(std::io::Error::from(std::io::ErrorKind::NotFound).raise());
/// assert_eq!(io.expect_err("the I/O operation failed").error().kind(), std::io::ErrorKind::NotFound);
/// # Ok::<(), gix_error::Error>(())
/// ```
pub type ExnResult<T = (), E = exn::Untyped> = std::result::Result<T, Exn<E>>;

/// A result with a [`Message`] exception, defaulting to unit success.
///
/// This is [`ExnResult<T, Message>`](ExnResult). Use it for operations that attach message contexts
/// with [`ResultExt::or_raise()`], or return standalone messages with [`ErrorExt::raise()`].
/// Use [`ExnResult<T>`](ExnResult) with its erased error type for callback bounds.
///
/// ```
/// use gix_error::{message, ErrorExt, ExnMessageResult};
///
/// fn validate(ready: bool) -> ExnMessageResult {
///     if !ready {
///         return Err(message("not ready").raise());
///     }
///     Ok(())
/// }
///
/// validate(true)?;
/// assert_eq!(validate(false).expect_err("not ready").error().message, "not ready");
/// # Ok::<(), gix_error::Error>(())
/// ```
pub type ExnMessageResult<T = ()> = ExnResult<T, Message>;

mod test;
pub use test::{TestError, TestResult};

mod error;
pub use error::{Class, classify};

/// Various kinds of concrete errors that implement [`std::error::Error`].
mod concrete;

pub use concrete::classify::{ClassificationMarker, ResourceExhaustionKind, tag};
pub use concrete::message::message;
pub use concrete::metadata::{
    Message, Metadata, MetadataValue, allocation_failure, allocation_limit, corruption, io, not_found,
    resource_exhaustion, retryable, validation,
};

pub(crate) fn write_location(f: &mut std::fmt::Formatter<'_>, location: &std::panic::Location) -> std::fmt::Result {
    write!(f, ", at {}:{}", location.file(), location.line())
}
