use std::{borrow::Cow, collections::BTreeMap, fmt, path::PathBuf};

use bstr::BString;

use crate::{Class, Error, ErrorExt, ResourceExhaustionKind};

/// An ordered dictionary of named diagnostic values belonging to a single error context.
///
/// Functions returning metadata document the keys and their meaning. [`crate::Exn::metadata()`] and
/// [`crate::Error::metadata()`] yield non-empty dictionaries separately; dictionaries from independent causes
/// are never merged.
///
/// # Common schemas
///
/// These schemas define conventional keys, not required fields. Store only information already available at the
/// failure site; do not collect additional information just to populate a schema. Omitted keys mean the information
/// was not recorded, not that it was empty or absent. Functions returning metadata document which keys they provide.
/// Metadata alone does not establish a semantic classification or whether recovery is safe.
///
/// ## Input validation
///
/// | Key | Value | Meaning |
/// | --- | --- | --- |
/// | `input` | Any [`MetadataValue`] | The offending input, preserving its original representation where possible. |
///
/// Use [`Message::with_input()`] to record already-known input. This convention also applies to malformed data
/// classified as [`Class::Corruption`]; attaching `input` does not imply [`Class::Validation`]. Do not record secrets
/// or other sensitive input that should not appear in diagnostics.
///
/// ## External program runtime failure
///
/// | Key | Value | Meaning |
/// | --- | --- | --- |
/// | `program` | [`MetadataValue::Path`] | The invoked program name or path, without resolving it. |
/// | `exit_status` | [`MetadataValue::String`] | The display representation of [`std::process::ExitStatus`], including termination without an exit code. |
/// | `exit_code` | [`MetadataValue::I64`] | The exit code, when [`std::process::ExitStatus::code()`] returns one. |
/// | `stdout` | [`MetadataValue::Bytes`] | Captured standard output, without text decoding. |
/// | `stderr` | [`MetadataValue::Bytes`] | Captured standard error, without text decoding. |
///
/// Use [`Message::with_command_status()`] to record `program`, `exit_status`, and an available `exit_code` together,
/// or [`Message::with_command_output()`] to include already-captured output. [`Message::with_program()`] and
/// [`Message::with_exit_status()`] record the fields available at spawn failures or after program identity was lost.
///
/// A failed exit status does not by itself establish a recovery class. Preserve native errors from spawning or
/// communicating with a program as causes; their contexts can also use `program`, without an exit status or output.
pub type Metadata = BTreeMap<Cow<'static, str>, MetadataValue>;

/// A diagnostic message with an optional semantic class and named diagnostic values.
///
/// Use this instead of chaining message, classification, and scalar-context errors when they describe a single
/// failure. [`Self::new()`] starts without a class or values; [`Self::with_class()`] and [`Self::with()`] add them.
/// Class builders such as [`Self::corrupted()`], [`Self::validation()`], [`Self::not_found()`], [`Self::retryable()`],
/// and [`Self::resource_exhaustion()`] are useful with formatted [`crate::message!`]s.
/// [`Self::allocation_limit()`] and [`Self::allocation_failure()`] select common resource exhaustion kinds.
/// Builders ending in `_error`, such as [`Self::corrupted_error()`] and [`Self::validation_error()`],
/// also raise the classified message as an [`Error`].
/// Class-based constructors such as [`crate::not_found()`] combine the message and class in one step.
///
/// Unlike [`ClassificationMarker`](crate::ClassificationMarker), this is a visible diagnostic: it participates in
/// error iteration, downcasting, reports, and cause selection. A marker only adds a classification to an existing
/// error without a diagnostic of its own, preserving that error's concrete type. Both are inspected by [`crate::classify()`].
/// The class itself isn't displayed, and [`crate::types::Classification::error()`] refers to this error, not a synthetic source.
///
/// Preserve real callee errors with [`ResultExt::or_raise()`](crate::ResultExt::or_raise) or
/// [`Exn::raise()`](crate::Exn::raise). Keep concrete error types when recovery requires a specific condition or payload;
/// use classification predicates to recognize categories, and document diagnostic keys on the function returning them.
/// [`Exn::metadata()`](crate::Exn::metadata) and [`crate::Error::metadata()`] yield each message's non-empty value dictionary.
/// Use [`Exn::metadata_merged()`](crate::Exn::metadata_merged) or [`crate::Error::metadata_merged()`] to combine them,
/// letting more specific causes override their enclosing contexts. To identify a specific failure, downcast to its
/// operation's error enum and match a variant; see [matching a specific failure](crate#matching-a-specific-failure).
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

/// Lifecycle
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
}

/// Builders
impl Message {
    /// Classify malformed or internally inconsistent stored or streamed data as [`Class::Corruption`].
    ///
    /// Like [`Self::with_class()`], this replaces any previous class without changing the message or values or adding a cause.
    pub fn corrupted(self) -> Self {
        self.with_class(Class::Corruption)
    }

    /// Classify malformed or internally inconsistent stored or streamed data and raise it as an [`Error`].
    ///
    /// Like [`Self::corrupted()`], this preserves the message and values and replaces any previous class without adding a cause.
    /// The error records the caller's location, just like [`ErrorExt::raise()`].
    #[track_caller]
    pub fn corrupted_error(self) -> Error {
        self.corrupted().raise()
    }

    /// Classify invalid function or method input as [`Class::Validation`].
    ///
    /// Like [`Self::with_class()`], this replaces any previous class without changing the message or values or adding a cause.
    pub fn validation(self) -> Self {
        self.with_class(Class::Validation)
    }

    /// Classify invalid function or method input and raise it as an [`Error`].
    ///
    /// Like [`Self::validation()`], this preserves the message and values and replaces any previous class without adding a cause.
    /// The error records the caller's location, just like [`ErrorExt::raise()`].
    #[track_caller]
    pub fn validation_error(self) -> Error {
        self.validation().raise()
    }

    /// Classify a missing resource as [`Class::NotFound`].
    ///
    /// Like [`Self::with_class()`], this replaces any previous class without changing the message or values or adding a cause.
    pub fn not_found(self) -> Self {
        self.with_class(Class::NotFound)
    }

    /// Classify a missing resource and raise it as an [`Error`].
    ///
    /// Like [`Self::not_found()`], this preserves the message and values and replaces any previous class without adding a cause.
    /// The error records the caller's location, just like [`ErrorExt::raise()`].
    #[track_caller]
    pub fn not_found_error(self) -> Error {
        self.not_found().raise()
    }

    /// Classify an operation that may succeed when retried as [`Class::Retryable`].
    ///
    /// Like [`Self::with_class()`], this replaces any previous class without changing the message or values or adding a cause.
    pub fn retryable(self) -> Self {
        self.with_class(Class::Retryable)
    }

    /// Classify an operation that may succeed when retried and raise it as an [`Error`].
    ///
    /// Like [`Self::retryable()`], this preserves the message and values and replaces any previous class without adding a cause.
    /// The error records the caller's location, just like [`ErrorExt::raise()`].
    #[track_caller]
    pub fn retryable_error(self) -> Error {
        self.retryable().raise()
    }

    /// The caller requested cancellation; stop rather than retry.
    ///
    /// Replaces any previous class without changing the message or values or adding a cause.
    pub fn cancelled(self) -> Self {
        self.with_class(Class::Cancelled)
    }

    /// Apply [`Self::cancelled()`] and raise the message, recording the caller location.
    #[track_caller]
    pub fn cancelled_error(self) -> Error {
        self.cancelled().raise()
    }

    /// Authorization or permissions are insufficient; obtain authorization or change permissions.
    ///
    /// Replaces any previous class without changing the message or values or adding a cause.
    pub fn permission_denied(self) -> Self {
        self.with_class(Class::PermissionDenied)
    }

    /// Apply [`Self::permission_denied()`] and raise the message, recording the caller location.
    #[track_caller]
    pub fn permission_denied_error(self) -> Error {
        self.permission_denied().raise()
    }

    /// Credentials are missing or rejected; obtain or refresh credentials.
    ///
    /// Replaces any previous class without changing the message or values or adding a cause.
    pub fn unauthenticated(self) -> Self {
        self.with_class(Class::Unauthenticated)
    }

    /// Apply [`Self::unauthenticated()`] and raise the message, recording the caller location.
    #[track_caller]
    pub fn unauthenticated_error(self) -> Error {
        self.unauthenticated().raise()
    }

    /// Current state conflicts with the operation; refresh or reconcile state before retrying.
    ///
    /// Replaces any previous class without changing the message or values or adding a cause.
    pub fn conflict(self) -> Self {
        self.with_class(Class::Conflict)
    }

    /// Apply [`Self::conflict()`] and raise the message, recording the caller location.
    #[track_caller]
    pub fn conflict_error(self) -> Error {
        self.conflict().raise()
    }

    /// A required capability is unsupported; switch implementation, format, protocol, or strategy.
    ///
    /// Replaces any previous class without changing the message or values or adding a cause.
    pub fn unsupported(self) -> Self {
        self.with_class(Class::Unsupported)
    }

    /// Apply [`Self::unsupported()`] and raise the message, recording the caller location.
    #[track_caller]
    pub fn unsupported_error(self) -> Error {
        self.unsupported().raise()
    }

    /// Classify an exhausted resource as [`Class::ResourceExhaustion`] of `kind`.
    ///
    /// Like [`Self::with_class()`], this replaces any previous class without changing the message or values or adding a cause.
    pub fn resource_exhaustion(self, kind: ResourceExhaustionKind) -> Self {
        self.with_class(Class::ResourceExhaustion(kind))
    }

    /// Classify an exhausted resource of `kind` and raise it as an [`Error`].
    ///
    /// Like [`Self::resource_exhaustion()`], this preserves the message and values and replaces any previous class without adding a cause.
    /// The error records the caller's location, just like [`ErrorExt::raise()`].
    #[track_caller]
    pub fn resource_exhaustion_error(self, kind: ResourceExhaustionKind) -> Error {
        self.resource_exhaustion(kind).raise()
    }

    /// Classify an exceeded application-configured allocation limit.
    ///
    /// Like [`Self::resource_exhaustion()`], this preserves the message and values and replaces any previous class without adding a cause.
    pub fn allocation_limit(self) -> Self {
        self.resource_exhaustion(ResourceExhaustionKind::AllocationLimit)
    }

    /// Classify an exceeded application-configured allocation limit and raise it as an [`Error`].
    ///
    /// Like [`Self::allocation_limit()`], this preserves the message and values and replaces any previous class without adding a cause.
    /// The error records the caller's location, just like [`ErrorExt::raise()`].
    #[track_caller]
    pub fn allocation_limit_error(self) -> Error {
        self.allocation_limit().raise()
    }

    /// Classify an unrepresentable allocation size or memory that could not be reserved.
    ///
    /// Like [`Self::resource_exhaustion()`], this preserves the message and values and replaces any previous class without adding a cause.
    pub fn allocation_failure(self) -> Self {
        self.resource_exhaustion(ResourceExhaustionKind::AllocationFailure)
    }

    /// Classify an unrepresentable allocation size or memory that could not be reserved and raise it as an [`Error`].
    ///
    /// Like [`Self::allocation_failure()`], this preserves the message and values and replaces any previous class without adding a cause.
    /// The error records the caller's location, just like [`ErrorExt::raise()`].
    #[track_caller]
    pub fn allocation_failure_error(self) -> Error {
        self.allocation_failure().raise()
    }
}

/// Metadata builders
impl Message {
    /// Record the offending input using the [input validation schema](Metadata#input-validation).
    ///
    /// Replaces `input` in this context, preserving its representation through [`MetadataValue`]. Other values,
    /// the message, and the classification are unchanged; input does not imply [`Class::Validation`].
    /// Only record already-known input that is appropriate for diagnostics, never secrets or other sensitive data.
    pub fn with_input(self, input: impl Into<MetadataValue>) -> Self {
        self.with("input", input)
    }

    /// Record a command's program and exit status using the [external program runtime failure schema](Metadata#external-program-runtime-failure).
    ///
    /// Replaces `program` with the native program name or path from [`std::process::Command::get_program()`], without
    /// resolving it, and `exit_status` with the status's display representation. Sets `exit_code` when available,
    /// otherwise removes any previous `exit_code`. Other values, the message, and the classification are unchanged.
    /// This does not require a failed status or imply a recovery class, and does not capture output.
    pub fn with_command_status(self, command: &std::process::Command, status: std::process::ExitStatus) -> Self {
        self.with_program(command.get_program()).with_exit_status(status)
    }

    /// Record an already-known program name or path as `program`, without resolving it or changing the classification.
    ///
    /// Use [`std::process::Command::get_program()`] when a prepared command is available.
    pub fn with_program(self, program: impl AsRef<std::ffi::OsStr>) -> Self {
        self.with("program", std::path::Path::new(program.as_ref()))
    }

    /// Record `exit_status` and an available `exit_code` using the
    /// [external program runtime failure schema](Metadata#external-program-runtime-failure).
    ///
    /// Replaces any previous status and removes a previous `exit_code` if this status has none.
    /// Does not require a failed status, change the classification, or collect program identity or output.
    pub fn with_exit_status(mut self, status: std::process::ExitStatus) -> Self {
        self = self.with("exit_status", status.to_string());
        if let Some(code) = status.code() {
            self = self.with("exit_code", code);
        } else {
            self.values.remove("exit_code");
        }
        self
    }

    /// Record a command's program, exit status, and already-captured output using the
    /// [external program runtime failure schema](Metadata#external-program-runtime-failure).
    ///
    /// Like [`Self::with_command_status()`], replaces the program and status fields, then replaces `stdout` and
    /// `stderr` with the captured bytes, including empty buffers. Does not execute a command or capture more output.
    /// Only use this when both streams are appropriate for diagnostics; in particular, do not record credential output.
    pub fn with_command_output(self, command: &std::process::Command, output: std::process::Output) -> Self {
        self.with_command_status(command, output.status)
            .with("stdout", output.stdout)
            .with("stderr", output.stderr)
    }

    /// Add `value` under `key`, replacing any previous value in this context.
    /// Inspect values through [`crate::Error::metadata()`], or [`crate::Exn::metadata()`] on typed exceptions.
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
///
/// If the offending input is recorded, use the `input` key from the [input validation schema](Metadata#input-validation).
pub fn validation(message: impl Into<Cow<'static, str>>) -> Message {
    Message::new(message).validation()
}

/// Create a diagnostic for malformed or internally inconsistent data, classified as [`Class::Corruption`].
pub fn corruption(message: impl Into<Cow<'static, str>>) -> Message {
    Message::new(message).corrupted()
}

/// Create a diagnostic for a missing resource, classified as [`Class::NotFound`].
pub fn not_found(message: impl Into<Cow<'static, str>>) -> Message {
    Message::new(message).not_found()
}

/// Create a diagnostic for an operation that may succeed when retried, classified as [`Class::Retryable`].
pub fn retryable(message: impl Into<Cow<'static, str>>) -> Message {
    Message::new(message).retryable()
}

/// The caller requested cancellation; stop rather than retry.
pub fn cancelled(message: impl Into<Cow<'static, str>>) -> Message {
    Message::new(message).cancelled()
}

/// Authorization or permissions are insufficient; obtain authorization or change permissions.
pub fn permission_denied(message: impl Into<Cow<'static, str>>) -> Message {
    Message::new(message).permission_denied()
}

/// Credentials are missing or rejected; obtain or refresh credentials.
pub fn unauthenticated(message: impl Into<Cow<'static, str>>) -> Message {
    Message::new(message).unauthenticated()
}

/// Current state conflicts with the operation; refresh or reconcile state before retrying.
pub fn conflict(message: impl Into<Cow<'static, str>>) -> Message {
    Message::new(message).conflict()
}

/// A required capability is unsupported; switch implementation, format, protocol, or strategy.
pub fn unsupported(message: impl Into<Cow<'static, str>>) -> Message {
    Message::new(message).unsupported()
}

/// Create a diagnostic for an exhausted resource, classified as [`Class::ResourceExhaustion`] of `kind`.
pub fn resource_exhaustion(kind: ResourceExhaustionKind, message: impl Into<Cow<'static, str>>) -> Message {
    Message::new(message).resource_exhaustion(kind)
}

/// Create a diagnostic for an exceeded application-configured allocation limit.
pub fn allocation_limit(message: impl Into<Cow<'static, str>>) -> Message {
    Message::new(message).allocation_limit()
}

/// Create a diagnostic for an unrepresentable allocation size or memory that could not be reserved.
pub fn allocation_failure(message: impl Into<Cow<'static, str>>) -> Message {
    Message::new(message).allocation_failure()
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
