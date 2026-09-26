use std::fmt::{Debug, Display, Formatter};

use crate::Class;

/// A transparent classification marker with an optional owned source and no diagnostic of its own.
///
/// Unlike [`Message`](crate::Message), markers only supply classification metadata, never a visible
/// diagnostic. Use [`Message`](crate::Message) to combine a message, class, and scalar values in one causal error;
/// use a marker to classify an existing concrete error without changing its diagnostic or recovery payload.
///
/// For a leaf error or variant you define, prefer encoding its intrinsic classification in its
/// [`source()`](std::error::Error::source) implementation. The associated constants, such as [`Self::NOT_FOUND`],
/// are owned class-only markers: return `Some(const { &ClassificationMarker::NOT_FOUND })` without defining a static.
/// This classifies every construction site without repeated tagging.
/// [`Self::with_class()`] creates an owned class-only marker.
/// Use [`crate::tag()`] for classifications that depend on the calling context or for error types you cannot modify,
/// preserving the concrete type and diagnostic.
///
/// Diagnostic iterators, downcasts, cause selection, and exception/test reports skip all markers,
/// retaining their real descendants. [`crate::classify()`] still inspects markers. Raw standard-error sources can still
/// expose markers. A report with only class-only markers falls back to displaying the root classification.
/// If cause selection cannot choose a unique real descendant, it can likewise fall back to the stored marker root.
/// Preserve real [`std::io::Error`] sources: the marker itself has no I/O origin for [`crate::types::Classification::io_kind()`].
pub struct ClassificationMarker {
    class: Class,
    source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}

/// Add `class` to `err`, preserving its concrete type and diagnostic without a visible wrapper.
///
/// Use this when the classification depends on the calling context, or when you cannot modify the error type.
/// For intrinsic classifications on leaf errors or variants you define, prefer returning a constant marker such as
/// [`ClassificationMarker::NOT_FOUND`] from [`std::error::Error::source()`] instead of tagging every construction site.
/// Preserve genuine callee errors as sources rather than replacing them with class-only markers.
///
/// The returned marker identifies `err` through [`crate::types::Classification::error()`].
/// Use a concrete error's variants for specific recovery decisions, and [`Class`] for broad categorization.
pub fn tag(err: impl std::error::Error + Send + Sync + 'static, class: Class) -> ClassificationMarker {
    ClassificationMarker::with_source(class, err)
}

impl ClassificationMarker {
    /// A hidden marker for invalid input.
    pub const VALIDATION: Self = Self::with_class(Class::Validation);
    /// A hidden marker for malformed or internally inconsistent data.
    pub const CORRUPTION: Self = Self::with_class(Class::Corruption);
    /// A hidden marker for a requested resource that does not exist.
    pub const NOT_FOUND: Self = Self::with_class(Class::NotFound);
    /// A hidden marker for an operation which may succeed when retried.
    pub const RETRYABLE: Self = Self::with_class(Class::Retryable);
    /// A hidden marker for an application-configured allocation limit being exceeded.
    pub const ALLOCATION_LIMIT: Self =
        Self::with_class(Class::ResourceExhaustion(ResourceExhaustionKind::AllocationLimit));
    /// A hidden marker for an unrepresentable allocation size or memory that could not be reserved.
    pub const ALLOCATION_FAILURE: Self =
        Self::with_class(Class::ResourceExhaustion(ResourceExhaustionKind::AllocationFailure));

    /// Add `class` to `source`, preserving its concrete type and diagnostic without a visible wrapper.
    pub fn with_source(class: Class, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        ClassificationMarker {
            class,
            source: Some(Box::new(source)),
        }
    }

    /// Create a hidden metadata leaf, suitable for a custom error's static source.
    /// Prefer the associated constants, such as [`Self::NOT_FOUND`], for fixed classifications.
    pub const fn with_class(class: Class) -> Self {
        ClassificationMarker { class, source: None }
    }

    /// Return the classification supplied by this marker.
    pub fn class(&self) -> Class {
        self.class
    }
}

impl Display for ClassificationMarker {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.source {
            Some(source) => Display::fmt(source, f),
            None => Debug::fmt(&self.class, f),
        }
    }
}

impl Debug for ClassificationMarker {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.source {
            Some(source) => Debug::fmt(source, f),
            None => Display::fmt(self, f),
        }
    }
}

impl std::error::Error for ClassificationMarker {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_deref().map(|source| source as _)
    }
}

/// The kind of resource exhaustion which prevented an operation from completing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ResourceExhaustionKind {
    /// An application-configured allocation limit was exceeded.
    AllocationLimit,
    /// An allocation size could not be represented or memory could not be reserved.
    AllocationFailure,
}
