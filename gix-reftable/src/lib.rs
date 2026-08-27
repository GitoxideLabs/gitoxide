//! A specification-driven implementation of Git's immutable reftable files.
//!
//! The format contract is Git's published
//! [`reftable` specification](https://github.com/git/git/blob/f78ce2f7b6df702f93d40b85d6bda92a3f65da79/Documentation/technical/reftable.adoc)
//! at commit `f78ce2f7b6df702f93d40b85d6bda92a3f65da79`. This crate is an
//! independent Rust implementation of that document. Git itself is used only
//! as an executable compatibility oracle.
//!
//! The immutable codec implements the specification's [Details], [File format]
//! (including its header, ref, object, log, index, and footer subsections), and
//! [Binary search] sections. Stack behavior is mapped separately by the stack
//! module when that feature is present.
//!
//! This crate handles individual immutable tables. Stack publication,
//! locking, compaction, and repository reference policy live in higher layers.
//!
//! [Details]: https://github.com/git/git/blob/f78ce2f7b6df702f93d40b85d6bda92a3f65da79/Documentation/technical/reftable.adoc#details
//! [File format]: https://github.com/git/git/blob/f78ce2f7b6df702f93d40b85d6bda92a3f65da79/Documentation/technical/reftable.adoc#file-format
//! [Binary search]: https://github.com/git/git/blob/f78ce2f7b6df702f93d40b85d6bda92a3f65da79/Documentation/technical/reftable.adoc#binary-search
#![deny(missing_docs, unsafe_code)]

/// Checked low-level format primitives.
pub mod format;
mod read;
mod types;
mod write;

pub use read::Table;
pub use types::{
    Error, Header, Limits, LogRecord, LogRecordRef, LogValue, LogValueRef, RefRecord, RefRecordRef, RefValue,
    RefValueRef, Version,
};
pub use write::{WriteOptions, Writer};
