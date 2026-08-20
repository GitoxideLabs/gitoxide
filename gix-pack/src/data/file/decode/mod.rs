///
pub mod entry;
///
pub mod header;

/// A ref-delta base that could not be resolved.
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

impl std::error::Error for DeltaBaseUnresolved {}

/// Returned by [`File::decode_header()`][crate::data::File::decode_header()],
/// [`File::decode_entry()`][crate::data::File::decode_entry()] and .
/// [`File::decompress_entry()`][crate::data::File::decompress_entry()]
pub type Error = gix_error::Exn;

#[cold]
pub(super) fn out_of_memory() -> Error {
    use gix_error::ErrorExt;
    gix_error::message("Entry too large to fit in memory").raise_erased()
}
