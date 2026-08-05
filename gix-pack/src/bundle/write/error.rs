use std::io;

use gix_tempfile::handle::Writable;

/// The error returned by [`Bundle::write_to_directory()`][crate::Bundle::write_to_directory()]
#[derive(Debug)]
#[allow(missing_docs)]
pub enum Error {
    Io(io::Error),
    PackIter(crate::data::input::Error),
    Persist(gix_tempfile::handle::persist::Error<Writable>),
    IndexWrite(crate::index::write::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(_) => f.write_str("An IO error occurred when reading the pack or creating a temporary file"),
            Error::PackIter(err) => std::fmt::Display::fmt(err, f),
            Error::Persist(_) => f.write_str("Could not move a temporary file into its desired place"),
            Error::IndexWrite(err) => std::fmt::Display::fmt(err, f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            Error::PackIter(err) => err.source(),
            Error::Persist(err) => Some(err),
            Error::IndexWrite(err) => err.source(),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<crate::data::input::Error> for Error {
    fn from(err: crate::data::input::Error) -> Self {
        Error::PackIter(err)
    }
}

impl From<gix_tempfile::handle::persist::Error<Writable>> for Error {
    fn from(err: gix_tempfile::handle::persist::Error<Writable>) -> Self {
        Error::Persist(err)
    }
}

impl From<crate::index::write::Error> for Error {
    fn from(err: crate::index::write::Error) -> Self {
        Error::IndexWrite(err)
    }
}
