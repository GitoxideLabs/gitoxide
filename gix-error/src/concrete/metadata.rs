use std::{borrow::Cow, collections::BTreeMap, fmt, path::PathBuf};

use bstr::BString;

use crate::{Class, ResourceExhaustionKind};

/// An ordered dictionary of named diagnostic values belonging to a single error context.
///
/// Functions returning metadata document the keys and their meaning. [`crate::Exn::metadata()`] and
/// [`crate::Error::metadata()`] yield non-empty dictionaries separately; dictionaries from independent causes
/// are never merged.
pub type Metadata = BTreeMap<Cow<'static, str>, MetadataValue>;

/// A diagnostic message with an optional semantic class and named diagnostic values.
///
/// Use this instead of chaining message, classification, and scalar-context errors when they describe a single
/// failure. [`Self::new()`] starts without a class or values; [`Self::with_class()`] and [`Self::with()`] add them.
/// Class-based constructors such as [`crate::not_found()`] combine the message and class in one step.
///
/// Unlike [`ClassificationMarker`](crate::ClassificationMarker), this is a visible diagnostic: it participates in
/// error iteration, downcasting, reports, and cause selection. A marker only adds a classification to an existing
/// error without a diagnostic of its own, preserving that error's concrete type. Both are inspected by [`crate::classify()`].
/// The class itself isn't displayed, and [`crate::types::Classification::error()`] refers to this error, not a synthetic source.
///
/// Preserve real callee errors with [`ResultExt::or_raise()`](crate::ResultExt::or_raise) or
/// [`Exn::raise()`](crate::Exn::raise). Keep concrete error types when recovery requires their specific payloads;
/// use classification predicates to recognize categories, and document diagnostic keys on the function returning them.
/// [`Exn::metadata()`](crate::Exn::metadata) and [`crate::Error::metadata()`] yield each message's non-empty value dictionary.
/// Dictionaries from separate contexts aren't merged. To identify a specific failure without inspecting its values,
/// use [`Class::Tagged`]; see [matching a specific failure](crate#matching-a-specific-failure).
///
/// Debug formatting omits absent classes and empty values. Present classes omit their `Some` wrapper,
/// and the class and values stay on single lines, even in pretty output.
pub struct Message {
    /// The operation or situation described by these values.
    pub message: Cow<'static, str>,
    /// The semantic class of this diagnostic, if known.
    pub class: Option<Class>,
    /// Diagnostic values, ordered by key. Functions returning metadata document their keys.
    pub values: Metadata,
}

impl Message {
    /// Create a diagnostic with `message`, no classification, and no values.
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            message: message.into(),
            class: None,
            values: Metadata::new(),
        }
    }

    /// Set `class`, replacing any previous classification without adding a cause.
    pub fn with_class(mut self, class: Class) -> Self {
        self.class = Some(class);
        self
    }

    /// Add `value` under `key`, replacing any previous value in this context.
    /// Inspect values through [`crate::Exn::metadata()`] after raising, or [`crate::Error::metadata()`] after wrapping.
    pub fn with(mut self, key: impl Into<Cow<'static, str>>, value: impl Into<MetadataValue>) -> Self {
        self.values.insert(key.into(), value.into());
        self
    }
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("Message");
        debug.field("message", &self.message);
        if let Some(class) = self.class {
            debug.field("class", &format_args!("{class:?}"));
        }
        if !self.values.is_empty() {
            debug.field("values", &format_args!("{:?}", self.values));
        }
        debug.finish()
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)?;
        for (key, value) in &self.values {
            write!(f, ", {key:?}={value}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Message {}

impl From<Cow<'static, str>> for Message {
    fn from(message: Cow<'static, str>) -> Self {
        Self::new(message)
    }
}

impl From<String> for Message {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

impl From<&'static str> for Message {
    fn from(message: &'static str) -> Self {
        Self::new(message)
    }
}

/// Create a diagnostic for invalid function or method input, classified as [`Class::Validation`].
pub fn validation(message: impl Into<Cow<'static, str>>) -> Message {
    Message::new(message).with_class(Class::Validation)
}

/// Create a diagnostic for malformed or internally inconsistent data, classified as [`Class::Corruption`].
pub fn corruption(message: impl Into<Cow<'static, str>>) -> Message {
    Message::new(message).with_class(Class::Corruption)
}

/// Create a diagnostic for a missing resource, classified as [`Class::NotFound`].
pub fn not_found(message: impl Into<Cow<'static, str>>) -> Message {
    Message::new(message).with_class(Class::NotFound)
}

/// Create a diagnostic for an operation that may succeed when retried, classified as [`Class::Retryable`].
pub fn retryable(message: impl Into<Cow<'static, str>>) -> Message {
    Message::new(message).with_class(Class::Retryable)
}

/// Create a diagnostic for an exhausted resource, classified as [`Class::ResourceExhaustion`] of `kind`.
pub fn resource_exhaustion(kind: ResourceExhaustionKind, message: impl Into<Cow<'static, str>>) -> Message {
    Message::new(message).with_class(Class::ResourceExhaustion(kind))
}

/// Create a diagnostic for an exceeded application-configured allocation limit.
pub fn allocation_limit(message: impl Into<Cow<'static, str>>) -> Message {
    resource_exhaustion(ResourceExhaustionKind::AllocationLimit, message)
}

/// Create a diagnostic for an unrepresentable allocation size or memory that could not be reserved.
pub fn allocation_failure(message: impl Into<Cow<'static, str>>) -> Message {
    resource_exhaustion(ResourceExhaustionKind::AllocationFailure, message)
}

/// Create a diagnostic classified as [`Class::Io`] of `kind`, without an original [`std::io::Error`].
///
/// This does not supply an I/O origin for [`crate::types::Classification::io_kind()`]. When an actual I/O error is
/// available, preserve it as a cause with [`ResultExt::or_raise()`](crate::ResultExt::or_raise) instead.
pub fn io(kind: std::io::ErrorKind, message: impl Into<Cow<'static, str>>) -> Message {
    Message::new(message).with_class(Class::Io(kind))
}

/// An owned scalar value in a [`Metadata`] dictionary. Bytes and native paths retain their original representation.
///
/// Debug formatting keeps the variant and its value on a single line, even in pretty output.
#[derive(Clone, PartialEq)]
#[non_exhaustive]
pub enum MetadataValue {
    /// A boolean.
    Bool(bool),
    /// A signed integer.
    I64(i64),
    /// An unsigned integer.
    U64(u64),
    /// A floating-point number.
    F64(f64),
    /// UTF-8 text.
    String(String),
    /// An arbitrary byte string.
    Bytes(BString),
    /// A native filesystem path.
    Path(PathBuf),
}

impl fmt::Debug for MetadataValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetadataValue::Bool(value) => write!(f, "Bool({value:?})"),
            MetadataValue::I64(value) => write!(f, "I64({value:?})"),
            MetadataValue::U64(value) => write!(f, "U64({value:?})"),
            MetadataValue::F64(value) => write!(f, "F64({value:?})"),
            MetadataValue::String(value) => write!(f, "String({value:?})"),
            MetadataValue::Bytes(value) => write!(f, "Bytes({value:?})"),
            MetadataValue::Path(value) => write!(f, "Path({value:?})"),
        }
    }
}

impl fmt::Display for MetadataValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetadataValue::Bool(value) => fmt::Display::fmt(value, f),
            MetadataValue::I64(value) => fmt::Display::fmt(value, f),
            MetadataValue::U64(value) => fmt::Display::fmt(value, f),
            MetadataValue::F64(value) => fmt::Display::fmt(value, f),
            MetadataValue::String(value) => fmt::Debug::fmt(value, f),
            MetadataValue::Bytes(value) => fmt::Debug::fmt(value, f),
            MetadataValue::Path(value) => fmt::Debug::fmt(value, f),
        }
    }
}

macro_rules! from {
    ($variant:ident: $($ty:ty),+ $(,)?) => {
        $(impl From<$ty> for MetadataValue {
            fn from(value: $ty) -> Self {
                Self::$variant(value.into())
            }
        })+
    };
}

from!(Bool: bool);
from!(I64: i8, i16, i32, i64);
from!(U64: u8, u16, u32, u64);
from!(F64: f32, f64);
from!(String: String, &str);
from!(Bytes: BString, &bstr::BStr, Vec<u8>, &[u8]);
from!(Path: PathBuf, &std::path::Path);

impl From<usize> for MetadataValue {
    fn from(value: usize) -> Self {
        Self::U64(value as u64)
    }
}

impl From<isize> for MetadataValue {
    fn from(value: isize) -> Self {
        Self::I64(value as i64)
    }
}
