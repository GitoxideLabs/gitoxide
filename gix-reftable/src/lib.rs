//! A specification-driven implementation of Git's immutable reftable files.
//!
//! The format contract is Git's published
//! [`reftable` specification](https://github.com/git/git/blob/f78ce2f7b6df702f93d40b85d6bda92a3f65da79/Documentation/technical/reftable.adoc)
//! at commit `f78ce2f7b6df702f93d40b85d6bda92a3f65da79`. This crate is an
//! independent Rust implementation of that document. Cross-implementation
//! tests compare its behavior with Git.
//!
//! The immutable codec implements the specification's [Details], [File format]
//! (including its header, ref, object, log, index, and footer subsections), and
//! [Binary search] sections. The stack engine implements the storage mechanics
//! from [Repository format], [Update transactions], [Reference deletions], and
//! [Compaction], including the deferred deletion needed by [Windows] readers.
//!
//! Stack generations are published with atomic rename. On Unix, the containing
//! directory is also synchronized at each durability boundary. Platforms
//! without directory synchronization retain atomic publication, while
//! power-loss durability remains subject to their filesystem and operating
//! system. Generated staged artifacts can be cleaned up after an irregular
//! exit; stale lock files require operator verification before removal.
//! Repository reference policy lives in higher layers.
//!
//! [Details]: https://github.com/git/git/blob/f78ce2f7b6df702f93d40b85d6bda92a3f65da79/Documentation/technical/reftable.adoc#details
//! [File format]: https://github.com/git/git/blob/f78ce2f7b6df702f93d40b85d6bda92a3f65da79/Documentation/technical/reftable.adoc#file-format
//! [Binary search]: https://github.com/git/git/blob/f78ce2f7b6df702f93d40b85d6bda92a3f65da79/Documentation/technical/reftable.adoc#binary-search
//! [Repository format]: https://github.com/git/git/blob/f78ce2f7b6df702f93d40b85d6bda92a3f65da79/Documentation/technical/reftable.adoc#repository-format
//! [Update transactions]: https://github.com/git/git/blob/f78ce2f7b6df702f93d40b85d6bda92a3f65da79/Documentation/technical/reftable.adoc#update-transactions
//! [Reference deletions]: https://github.com/git/git/blob/f78ce2f7b6df702f93d40b85d6bda92a3f65da79/Documentation/technical/reftable.adoc#reference-deletions
//! [Compaction]: https://github.com/git/git/blob/f78ce2f7b6df702f93d40b85d6bda92a3f65da79/Documentation/technical/reftable.adoc#compaction
//! [Windows]: https://github.com/git/git/blob/f78ce2f7b6df702f93d40b85d6bda92a3f65da79/Documentation/technical/reftable.adoc#windows
#![deny(missing_docs, unsafe_code)]

/// Checked low-level format primitives.
pub mod format;
mod read;
mod stack;
mod types;
mod write;

pub use read::Table;
pub use stack::{
    Cleanup, CleanupFailure, CompactOptions, CompactOutcome, Error as StackError, LockOptions, LockedAddition,
    LockedSnapshot, MemberInfo, Snapshot, SnapshotOptions, Stack, Verification,
};
pub use types::{
    Error, Header, Limits, LogRecord, LogRecordRef, LogValue, LogValueRef, RefRecord, RefRecordRef, RefValue,
    RefValueRef, Version,
};
pub use write::{WriteOptions, Writer};
