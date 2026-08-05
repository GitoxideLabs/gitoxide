use std::collections::TryReserveError;

///
pub mod entry;
///
pub mod header;

/// Returned by [`File::decode_header()`][crate::data::File::decode_header()],
/// [`File::decode_entry()`][crate::data::File::decode_entry()] and .
/// [`File::decompress_entry()`][crate::data::File::decompress_entry()]
#[derive(Debug)]
#[allow(missing_docs)]
pub enum Error {
    ZlibInflate(gix_zlib::inflate::Error),
    DeltaBaseUnresolved(gix_hash::ObjectId),
    EntryType(crate::data::entry::decode::Error),
    OutOfMemory,
    Delta(crate::data::delta::apply::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ZlibInflate(_) => f.write_str("Failed to decompress pack entry"),
            Error::DeltaBaseUnresolved(id) => write!(
                f,
                "A delta chain could not be followed as the ref base with id {id} could not be found"
            ),
            Error::EntryType(err) => std::fmt::Display::fmt(err, f),
            Error::OutOfMemory => f.write_str("Entry too large to fit in memory"),
            Error::Delta(err) => std::fmt::Display::fmt(err, f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::ZlibInflate(err) => Some(err),
            Error::EntryType(err) => err.source(),
            Error::Delta(err) => err.source(),
            Error::DeltaBaseUnresolved(_) | Error::OutOfMemory => None,
        }
    }
}

impl From<gix_zlib::inflate::Error> for Error {
    fn from(err: gix_zlib::inflate::Error) -> Self {
        Error::ZlibInflate(err)
    }
}

impl From<crate::data::entry::decode::Error> for Error {
    fn from(err: crate::data::entry::decode::Error) -> Self {
        Error::EntryType(err)
    }
}

impl From<crate::data::delta::apply::Error> for Error {
    fn from(err: crate::data::delta::apply::Error) -> Self {
        Error::Delta(err)
    }
}

impl From<TryReserveError> for Error {
    #[cold]
    fn from(_: TryReserveError) -> Self {
        Self::OutOfMemory
    }
}
