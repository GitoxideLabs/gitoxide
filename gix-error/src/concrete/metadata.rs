use std::{borrow::Cow, collections::BTreeMap, fmt, path::PathBuf};

use bstr::BString;

/// A context message with named diagnostic values.
///
/// Attach it with [`ResultExt::or_raise()`](crate::ResultExt::or_raise) or [`Exn::raise()`](crate::Exn::raise),
/// keeping the original error as its cause. Use concrete error types and classifications for recovery;
/// metadata describes a particular context and dictionaries from separate contexts aren't merged.
#[derive(Debug)]
pub struct Metadata {
    /// The operation or situation described by these values.
    pub message: Cow<'static, str>,
    /// Diagnostic values, ordered by key. Functions returning metadata document their keys.
    pub values: BTreeMap<Cow<'static, str>, Value>,
}

impl Metadata {
    /// Create a context with `message` and no values.
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            message: message.into(),
            values: BTreeMap::new(),
        }
    }

    /// Add `value` under `key`, replacing any previous value in this context.
    pub fn with(mut self, key: impl Into<Cow<'static, str>>, value: impl Into<Value>) -> Self {
        self.values.insert(key.into(), value.into());
        self
    }
}

impl fmt::Display for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)?;
        for (key, value) in &self.values {
            write!(f, ", {key:?}={value}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Metadata {}

/// An owned scalar diagnostic value. Bytes and native paths retain their original representation.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Value {
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

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Bool(value) => fmt::Display::fmt(value, f),
            Value::I64(value) => fmt::Display::fmt(value, f),
            Value::U64(value) => fmt::Display::fmt(value, f),
            Value::F64(value) => fmt::Display::fmt(value, f),
            Value::String(value) => fmt::Debug::fmt(value, f),
            Value::Bytes(value) => fmt::Debug::fmt(value, f),
            Value::Path(value) => fmt::Debug::fmt(value, f),
        }
    }
}

macro_rules! from {
    ($variant:ident: $($ty:ty),+ $(,)?) => {
        $(impl From<$ty> for Value {
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

impl From<usize> for Value {
    fn from(value: usize) -> Self {
        Self::U64(value as u64)
    }
}

impl From<isize> for Value {
    fn from(value: isize) -> Self {
        Self::I64(value as i64)
    }
}
