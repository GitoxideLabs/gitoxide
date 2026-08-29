use std::borrow::Cow;
use std::fmt::{Display, Formatter};

use crate::Message;

/// The kind of resource exhaustion which prevented an operation from completing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ResourceExhaustionKind {
    /// An application-configured allocation limit was exceeded.
    AllocationLimit,
    /// An allocation size could not be represented or memory could not be reserved.
    AllocationFailure,
}

/// An error caused by exhausting a finite resource.
#[derive(Debug)]
pub struct ResourceExhaustionError {
    /// The kind of resource exhaustion.
    pub kind: ResourceExhaustionKind,
    /// The error message.
    pub message: Cow<'static, str>,
}

impl ResourceExhaustionError {
    /// Create a new instance with `kind` that displays the given `message`.
    pub fn new(kind: ResourceExhaustionKind, message: impl Into<Cow<'static, str>>) -> Self {
        ResourceExhaustionError {
            kind,
            message: message.into(),
        }
    }

    /// Return the kind of resource exhaustion.
    pub fn kind(&self) -> ResourceExhaustionKind {
        self.kind
    }
}

impl Display for ResourceExhaustionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message.as_ref())
    }
}

impl std::error::Error for ResourceExhaustionError {}

/// An error caused by malformed or internally inconsistent data.
#[derive(Debug)]
pub struct CorruptionError {
    /// The error message.
    pub message: Cow<'static, str>,
}

impl CorruptionError {
    /// Create a new instance that displays the given `message`.
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        CorruptionError {
            message: message.into(),
        }
    }
}

impl Display for CorruptionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message.as_ref())
    }
}

impl std::error::Error for CorruptionError {}

impl From<Message> for CorruptionError {
    fn from(Message(msg): Message) -> Self {
        CorruptionError::new(msg)
    }
}

impl From<String> for CorruptionError {
    fn from(msg: String) -> Self {
        CorruptionError::new(msg)
    }
}

impl From<&'static str> for CorruptionError {
    fn from(msg: &'static str) -> Self {
        CorruptionError::new(msg)
    }
}

/// An error indicating that a requested resource does not exist.
#[derive(Debug)]
pub struct NotFoundError {
    /// The error message.
    pub message: Cow<'static, str>,
}

impl NotFoundError {
    /// Create a new instance that displays the given `message`.
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        NotFoundError {
            message: message.into(),
        }
    }
}

impl Display for NotFoundError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message.as_ref())
    }
}

impl std::error::Error for NotFoundError {}

impl From<Message> for NotFoundError {
    fn from(Message(msg): Message) -> Self {
        NotFoundError::new(msg)
    }
}

impl From<String> for NotFoundError {
    fn from(msg: String) -> Self {
        NotFoundError::new(msg)
    }
}

impl From<&'static str> for NotFoundError {
    fn from(msg: &'static str) -> Self {
        NotFoundError::new(msg)
    }
}

/// A transparent wrapper for dependency-specific errors known to be retryable.
#[derive(Debug)]
pub struct RetryableError(Box<dyn std::error::Error + Send + Sync + 'static>);

impl RetryableError {
    /// Mark `source` as retryable.
    pub fn new(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        RetryableError(Box::new(source))
    }
}

impl Display for RetryableError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl std::error::Error for RetryableError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}
