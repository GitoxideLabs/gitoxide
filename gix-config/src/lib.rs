//! # `gix_config`
//!
//! This crate is a high performance `git-config` file reader and writer. It
//! exposes a high level API to parse, read, and write [`git-config` files].
//!
//! This crate has a few primary offerings and various accessory functions. The
//! table below gives a brief explanation of all offerings, loosely in order
//! from the highest to lowest abstraction.
//!
//! | Offering         | Description                                      |
//! | ---------------- | ------------------------------------------------ |
//! | [`File`]         | Accelerated wrapper for reading and writing values. |
//! | [`parse::Events`] | Syntactic events for `git-config` files.        |
//! | value wrappers   | Wrappers for `git-config` value types.           |
//!
//! This crate also exposes value normalization which unescapes characters and
//! removes quotes through [`value::normalize()`].
//!
//! # Examples
//!
//! ## Read And Update Values
//!
//! ```
//! use bstr::ByteSlice;
//! use std::str::FromStr;
//!
//! const SAMPLE: &str = "[core]\neditor = vim\nbare = false\n[remote \"origin\"]\nurl = https://example.com/gitoxide.git\n";
//! let mut config = gix_config::File::from_str(SAMPLE).unwrap();
//! assert_eq!(config.string_by("core", None, "editor").unwrap(), "vim");
//! assert_eq!(config.boolean_by("core", None, "bare").unwrap().unwrap(), false);
//!
//! let previous = config.set_raw_value(&"core.editor", "nvim").unwrap().unwrap();
//! assert_eq!(previous, "vim");
//! assert_eq!(config.raw_value("core.editor").unwrap(), "nvim");
//! assert!(config.to_bstring().find(b"nvim").is_some());
//! ```
//!
//! # Known differences to the `git config` specification
//!
//! - Legacy headers like `[section.subsection]` are supposed to be turned into lower case. We keep their case,
//!   but like git, look them up by their subsection name in lower case and compare it case-sensitively.
//!
//! [`git-config` files]: https://git-scm.com/docs/git-config#_configuration_file
//! [`File`]: crate::File
//!
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs, unsafe_code)]
#![allow(rustdoc::redundant_explicit_links)] // Rust 1.99 flags links in re-exported documentation.

pub mod file;

/// Reformat a git-config file with normalized, sanitized whitespace.
pub mod format;
///
pub mod lookup;
pub mod parse;
///
pub mod value;
pub use gix_config_value::{Boolean, Color, Integer, Path, color, integer, path};

pub use gix_utils::{AsBStr, AsBStrOpt};
mod key;
pub use key::{AsKey, KeyRef};
mod types;
pub use types::{File, Source};
///
pub mod source;
