//! Parsing for data types used in `git-config` files to allow their use from environment variables and other sources.
//!
//! ## Examples
//!
//! ```
//! use bstr::ByteSlice;
//! use gix_config_value::{Boolean, Integer, Path};
//!
//! let auto_crlf: bool = Boolean::try_from("true").unwrap().into();
//! assert!(auto_crlf);
//!
//! let packed_limit = Integer::try_from("10m".as_bytes().as_bstr()).unwrap();
//! assert_eq!(packed_limit.to_decimal(), Some(10 * 1024 * 1024));
//!
//! let ignore_revs = Path::from(":(optional)~/.git-blame-ignore-revs");
//! assert!(ignore_revs.is_optional);
//! assert_eq!(ignore_revs.value.as_bstr(), "~/.git-blame-ignore-revs");
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
#[derive(Debug, Eq, PartialEq)]
#[expect(missing_docs)]
pub struct Error {
    pub message: &'static str,
    pub input: bstr::BString,
    pub utf8_err: Option<std::str::Utf8Error>,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Could not decode '{}': {}", self.input, self.message)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.utf8_err
            .as_ref()
            .map(|err| err as &(dyn std::error::Error + 'static))
    }
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
/// Color value parsing and the supported color names and attributes.
pub mod color;
/// Integer suffix parsing and conversion support.
pub mod integer;
/// Path interpolation support.
pub mod path;

mod types;
pub use types::{Boolean, Color, Integer, Path};
