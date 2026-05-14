//! Parsing for data types used in `git-config` files to allow their use from environment variables and other sources.
//!
//! ## Examples
//!
//! ```
//! use std::borrow::Cow;
//!
//! use bstr::ByteSlice;
//! use gix_config_value::{Boolean, Integer, Path};
//!
//! let auto_crlf: bool = Boolean::try_from("true".as_bytes().as_bstr()).unwrap().into();
//! assert!(auto_crlf);
//!
//! let packed_limit = Integer::try_from("10m".as_bytes().as_bstr()).unwrap();
//! assert_eq!(packed_limit.to_decimal(), Some(10 * 1024 * 1024));
//!
//! let ignore_revs = Path::from(Cow::Borrowed(b":(optional)~/.git-blame-ignore-revs".as_bstr()));
//! assert!(ignore_revs.is_optional);
//! assert_eq!(ignore_revs.value.as_ref(), b"~/.git-blame-ignore-revs".as_bstr());
//! ```
//!
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs, unsafe_code)]

/// The error returned when any config value couldn't be instantiated due to malformed input.
#[derive(Debug, thiserror::Error, Eq, PartialEq)]
#[allow(missing_docs)]
#[error("Could not decode '{input}': {message}")]
pub struct Error {
    pub message: &'static str,
    pub input: bstr::BString,
    #[source]
    pub utf8_err: Option<std::str::Utf8Error>,
}

impl Error {
    /// Create a new value error from `message`, with `input` being what's causing the error.
    pub fn new(message: &'static str, input: impl Into<bstr::BString>) -> Self {
        Error {
            message,
            input: input.into(),
            utf8_err: None,
        }
    }

    pub(crate) fn with_err(mut self, err: std::str::Utf8Error) -> Self {
        self.utf8_err = Some(err);
        self
    }
}

mod boolean;
///
pub mod color;
///
pub mod integer;
///
pub mod path;

mod types;
pub use types::{Boolean, Color, Integer, Path};
