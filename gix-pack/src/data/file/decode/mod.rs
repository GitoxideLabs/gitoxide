///
pub mod entry;
///
pub mod header;

/// A ref-delta base that could not be resolved.
///
/// Its source is a classification-only [`gix_error::ClassificationMarker`].
/// Use [`gix_error::classify()`] or `is_not_found()` on [`gix_error::Exn`] and [`gix_error::Error`] to check the
/// classification, without depending on the concrete diagnostic type. Downcast to this type for the base object ID.
#[derive(Debug)]
pub struct DeltaBaseUnresolved(
    /// The object ID named by the ref-delta.
    pub gix_hash::ObjectId,
);

impl std::fmt::Display for DeltaBaseUnresolved {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A delta chain could not be followed as the ref base with id {} could not be found",
            self.0
        )
    }
}

impl std::error::Error for DeltaBaseUnresolved {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(const { &gix_error::ClassificationMarker::NOT_FOUND })
    }
}

#[cold]
pub(super) fn allocation_error(kind: gix_error::ResourceExhaustionKind) -> gix_error::Exn {
    use gix_error::ErrorExt;
    gix_error::resource_exhaustion(kind, "Entry too large to fit in memory").raise_erased()
}
