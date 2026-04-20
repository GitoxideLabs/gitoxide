use std::collections::TryReserveError;

///
pub mod entry;
///
pub mod header;

/// Returned by [`File::decode_header()`][crate::data::File::decode_header()],
/// [`File::decode_entry()`][crate::data::File::decode_entry()] and .
/// [`File::decode_entry_to_write()`][crate::data::File::decode_entry_to_write()] and
/// [`File::decompress_entry()`][crate::data::File::decompress_entry()]
#[derive(thiserror::Error, Debug)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Failed to decompress pack entry")]
    ZlibInflate(#[from] gix_features::zlib::inflate::Error),
    #[error("Failed to write decoded object bytes")]
    Io(#[from] std::io::Error),
    #[error("A delta chain could not be followed as the ref base with id {0} could not be found")]
    DeltaBaseUnresolved(gix_hash::ObjectId),
    #[error(transparent)]
    EntryType(#[from] crate::data::entry::decode::Error),
    #[error("Entry too large to fit in memory")]
    OutOfMemory,
    #[error(transparent)]
    Delta(#[from] crate::data::delta::apply::Error),
}

impl From<TryReserveError> for Error {
    #[cold]
    fn from(_: TryReserveError) -> Self {
        Self::OutOfMemory
    }
}
