///
pub mod entry;
///
pub mod header;

/// A ref-delta base that could not be resolved.
///
/// Its source preserves the missing-object classification when converted to [`gix_error::Error`].
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
        static NOT_FOUND: gix_error::NotFoundError = gix_error::NotFoundError {
            message: std::borrow::Cow::Borrowed("Delta base object not found"),
        };
        Some(&NOT_FOUND)
    }
}

#[cold]
pub(super) fn allocation_error(kind: gix_error::ResourceExhaustionKind) -> gix_error::Exn {
    use gix_error::ErrorExt;
    gix_error::ResourceExhaustionError::new(kind, "Entry too large to fit in memory").raise_erased()
}
