use std::path::PathBuf;

use bstr::{BStr, BString, ByteSlice};
use gix_hash::{Kind, ObjectId, oid};

/// A reftable format version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    /// Version 1, which always uses SHA-1 object identifiers.
    V1,
    /// Version 2, which records its object hash in the header.
    V2,
}

impl Version {
    pub(crate) const fn byte(self) -> u8 {
        match self {
            Version::V1 => 1,
            Version::V2 => 2,
        }
    }

    pub(crate) const fn header_len(self) -> usize {
        match self {
            Version::V1 => 24,
            Version::V2 => 28,
        }
    }

    pub(crate) const fn footer_len(self) -> usize {
        self.header_len() + 5 * 8 + 4
    }
}

/// Information duplicated in a reftable's header and footer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// The on-disk format version.
    pub version: Version,
    /// The configured aligned block size, or zero for unaligned blocks.
    pub block_size: u32,
    /// The smallest stack update index covered by the table and the base for reference-record deltas.
    ///
    /// Historical log entries copied or tombstoned by a newer transaction may
    /// retain keys older than this bound, matching Git's writer behavior.
    pub min_update_index: u64,
    /// The largest stack update index covered by the table.
    pub max_update_index: u64,
    /// The object hash used by reference and log records.
    pub object_hash: Kind,
}

/// Resource limits applied before allocating or walking untrusted input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Maximum accepted table size in bytes.
    pub max_file_size: usize,
    /// Maximum accepted inflated block size in bytes.
    pub max_block_size: usize,
    /// Maximum key, symbolic target, identity, or message size.
    pub max_value_size: usize,
    /// Maximum total number of decoded records.
    pub max_records: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_file_size: 512 * 1024 * 1024,
            max_block_size: 0x00ff_ffff,
            max_value_size: 16 * 1024 * 1024,
            max_records: 16 * 1024 * 1024,
        }
    }
}

/// The value carried by a reference record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefValue {
    /// A tombstone hiding older values in the stack.
    Deletion,
    /// A direct object-id value.
    Direct(ObjectId),
    /// A direct value accompanied by the peeled object id.
    Peeled {
        /// The object id named by the reference.
        target: ObjectId,
        /// The fully peeled object id.
        peeled: ObjectId,
    },
    /// A symbolic reference target.
    Symbolic(BString),
}

impl RefValue {
    /// Borrow this value without cloning object IDs or symbolic targets.
    pub fn to_ref(&self) -> RefValueRef<'_> {
        match self {
            RefValue::Deletion => RefValueRef::Deletion,
            RefValue::Direct(object_id) => RefValueRef::Direct(object_id.as_ref()),
            RefValue::Peeled { target, peeled } => RefValueRef::Peeled {
                target: target.as_ref(),
                peeled: peeled.as_ref(),
            },
            RefValue::Symbolic(target) => RefValueRef::Symbolic(target.as_bstr()),
        }
    }
}

/// A borrowed reftable reference value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefValueRef<'a> {
    /// A tombstone hiding older values in the stack.
    Deletion,
    /// A direct object-id value.
    Direct(&'a oid),
    /// A direct value accompanied by the peeled object id.
    Peeled {
        /// The object id named by the reference.
        target: &'a oid,
        /// The fully peeled object id.
        peeled: &'a oid,
    },
    /// A symbolic reference target.
    Symbolic(&'a BStr),
}

/// One owned reference record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefRecord {
    /// The complete reference name.
    pub name: BString,
    /// The logical update index.
    pub update_index: u64,
    /// The reference value.
    pub value: RefValue,
}

impl RefRecord {
    /// Borrow this record without cloning its name or value.
    pub fn to_ref(&self) -> RefRecordRef<'_> {
        RefRecordRef {
            name: self.name.as_bstr(),
            update_index: self.update_index,
            value: self.value.to_ref(),
        }
    }
}

/// One borrowed reference record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefRecordRef<'a> {
    /// The complete reference name.
    pub name: &'a BStr,
    /// The logical update index.
    pub update_index: u64,
    /// The reference value.
    pub value: RefValueRef<'a>,
}

/// The value carried by a reference-log record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogValue {
    /// A log tombstone.
    Deletion,
    /// Git's all-zero sentinel indicating that a reflog exists but has no entries.
    Placeholder,
    /// A complete reflog entry.
    Update {
        /// The value before the update.
        old_id: ObjectId,
        /// The value after the update.
        new_id: ObjectId,
        /// The committer name without angle brackets.
        name: BString,
        /// The committer email without angle brackets.
        email: BString,
        /// Seconds since the Unix epoch.
        time: u64,
        /// Signed timezone offset in minutes east of UTC.
        tz_offset: i16,
        /// The reflog message, without its on-disk terminating newline.
        ///
        /// Writers reject values ending in a newline because they cannot round-trip
        /// through the format's mandatory terminator without ambiguity.
        message: BString,
    },
}

impl LogValue {
    /// Borrow this value without cloning object IDs, identities, or messages.
    pub fn to_ref(&self) -> LogValueRef<'_> {
        match self {
            LogValue::Deletion => LogValueRef::Deletion,
            LogValue::Placeholder => LogValueRef::Placeholder,
            LogValue::Update {
                old_id,
                new_id,
                name,
                email,
                time,
                tz_offset,
                message,
            } => LogValueRef::Update {
                old_id: old_id.as_ref(),
                new_id: new_id.as_ref(),
                name: name.as_bstr(),
                email: email.as_bstr(),
                time: *time,
                tz_offset: *tz_offset,
                message: message.as_bstr(),
            },
        }
    }
}

/// A borrowed reference-log value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogValueRef<'a> {
    /// A log tombstone.
    Deletion,
    /// Git's all-zero sentinel indicating that a reflog exists but has no entries.
    Placeholder,
    /// A complete reflog entry.
    Update {
        /// The value before the update.
        old_id: &'a oid,
        /// The value after the update.
        new_id: &'a oid,
        /// The committer name without angle brackets.
        name: &'a BStr,
        /// The committer email without angle brackets.
        email: &'a BStr,
        /// Seconds since the Unix epoch.
        time: u64,
        /// Signed timezone offset in minutes east of UTC.
        tz_offset: i16,
        /// The reflog message, without its on-disk terminating newline.
        message: &'a BStr,
    },
}

/// One owned reference-log record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRecord {
    /// The complete reference name.
    pub ref_name: BString,
    /// The logical update index, newest first for a given reference.
    pub update_index: u64,
    /// The log value.
    pub value: LogValue,
}

impl LogRecord {
    /// Borrow this record without cloning its reference name or value.
    pub fn to_ref(&self) -> LogRecordRef<'_> {
        LogRecordRef {
            ref_name: self.ref_name.as_bstr(),
            update_index: self.update_index,
            value: self.value.to_ref(),
        }
    }
}

/// One borrowed reference-log record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogRecordRef<'a> {
    /// The complete reference name.
    pub ref_name: &'a BStr,
    /// The logical update index, newest first for a given reference.
    pub update_index: u64,
    /// The log value.
    pub value: LogValueRef<'a>,
}

/// A contextual error produced while reading or writing a reftable.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reading or writing a table failed.
    #[error("I/O failed for {path}")]
    Io {
        /// The affected path.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
    /// A table read from disk could not be parsed or exceeded a configured limit.
    #[error("failed to parse reftable at {path}")]
    Parse {
        /// The affected table path.
        path: PathBuf,
        /// The underlying format or limit error.
        #[source]
        source: Box<Error>,
    },
    /// Compressing a log block failed.
    #[error("reftable log compression failed")]
    Compression {
        /// The underlying encoder error.
        #[source]
        source: std::io::Error,
    },
    /// The table is malformed at a byte offset.
    #[error("malformed reftable at byte {offset}: {message}")]
    Malformed {
        /// Absolute byte offset in the table.
        offset: usize,
        /// What invariant was violated.
        message: &'static str,
    },
    /// The table uses a format capability not compiled into this crate.
    #[error("unsupported reftable capability: {0}")]
    Unsupported(&'static str),
    /// A configured resource limit was exceeded.
    #[error("reftable resource limit exceeded: {0}")]
    Limit(&'static str),
    /// Input records cannot be represented in a valid table.
    #[error("invalid reftable writer input: {0}")]
    InvalidInput(&'static str),
}

pub(crate) fn malformed(offset: usize, message: &'static str) -> Error {
    Error::Malformed { offset, message }
}
