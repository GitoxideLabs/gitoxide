//! Common error types and reporting policies for gitoxide.
//!
//! [`Exn`], its construction traits, and [`bail!`]/[`ensure!`] are re-exported directly from `exn`.
//! Use upstream exceptions to carry typed context within plumbing crates, and bare `Exn` for callbacks
//! whose implementations return different error types. Standard `From` conversions and `?` erase the root marker.
//!
//! Gitoxide adds concrete errors such as [`ValidationError`] and [`CorruptionError`], plus two reporting boundaries:
//! [`Error`] implements [`std::error::Error`] with typed inspection, classification and gitoxide diagnostics;
//! [`ChainedError`] exposes every branch as a standard source chain for integrations such as `anyhow`.
//! Both accept upstream exceptions through `From`, without introducing another exception type.
//!
//! ```
//! use gix_error::{Error, ErrorExt, Exn, Message, ResultExt, message};
//!
//! fn read_config() -> std::result::Result<(), Exn<Message>> {
//!     std::fs::read("config").or_raise(|| message("could not read config"))?;
//!     Ok(())
//! }
//!
//! fn callback() -> std::result::Result<(), Exn> {
//!     read_config()?;
//!     Ok(())
//! }
//!
//! let native: exn::Exn<_> = message("operation failed").raise();
//! let error = Error::from(native);
//! assert_eq!(error.to_string(), "operation failed");
//! ```
//!
//! Raw exceptions use upstream formatting and boxed conversion. Convert to [`Error`] for gitoxide's tree display,
//! or use `ChainedError::from(exception)` when a standard source-chain walker must visit every branch.
//!
//! # Feature flags
#![cfg_attr(all(doc, feature = "document-features"), doc = ::document_features::document_features!())]
#![deny(missing_docs, unsafe_code)]

pub use ::exn::{ErrorExt, Exn, Frame, IteratorExt, OptionExt, ResultExt, bail, ensure};
pub use bstr;

/// An error type that wraps an inner type-erased boxed `std::error::Error` or an `Exn` frame.
///
/// In that, it's similar to `anyhow`, but with support for tracking the call site and trees of errors.
///
/// # Native error sources
///
/// Upstream construction snapshots native source messages. This boundary traverses the original sources instead,
/// preserving their concrete types and avoiding duplicate snapshot nodes. Nested `Error` values retain their graphs.
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

mod test;
pub use test::{TestError, TestResult};

mod error;
pub use error::{Class, Classification, DisplaySource, can_retry, can_retry_lenient};

/// Various kinds of concrete errors that implement [`std::error::Error`].
mod concrete;
pub use concrete::chain::ChainedError;
pub use concrete::classify::{
    CorruptionError, NotFoundError, ResourceExhaustionError, ResourceExhaustionKind, RetryableError,
};
pub use concrete::message::{Message, message};
pub use concrete::validate::ValidationError;

pub(crate) fn write_location(f: &mut std::fmt::Formatter<'_>, location: &std::panic::Location) -> std::fmt::Result {
    write!(f, ", at {}:{}", location.file(), location.line())
}
