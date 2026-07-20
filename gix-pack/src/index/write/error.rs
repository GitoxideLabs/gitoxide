/// Returned by [`crate::index::write_data_iter_to_stream()`]
#[derive(Debug)]
#[allow(missing_docs)]
pub enum Error {
    Io(gix_hash::io::Error),
    PackEntryDecode(crate::data::input::Error),
    Unsupported(crate::index::Version),
    IteratorInvariantNoRefDelta,
    IteratorInvariantTrailer,
    IteratorInvariantTooManyObjects(usize),
    IteratorInvariantBaseOffset { pack_offset: u64, distance: u64 },
    Tree(crate::cache::delta::Error),
    TreeTraversal(crate::cache::delta::traverse::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(_) => f.write_str("An error occurred when writing the pack index file"),
            Error::PackEntryDecode(_) => f.write_str("A pack entry could not be extracted"),
            Error::Unsupported(version) => write!(
                f,
                "Indices of type {} cannot be written, only {} are supported",
                *version as usize,
                crate::index::Version::default() as usize
            ),
            Error::IteratorInvariantNoRefDelta => f.write_str(
                "Ref delta objects are not supported as there is no way to look them up. Resolve them beforehand.",
            ),
            Error::IteratorInvariantTrailer => f.write_str(
                "The iterator failed to set a trailing hash over all prior pack entries in the last provided entry",
            ),
            Error::IteratorInvariantTooManyObjects(count) => {
                write!(f, "Only u32::MAX objects can be stored in a pack, found {count}")
            }
            Error::IteratorInvariantBaseOffset { pack_offset, distance } => {
                write!(f, "{pack_offset} is not a valid offset for pack offset {distance}")
            }
            Error::Tree(err) => std::fmt::Display::fmt(err, f),
            Error::TreeTraversal(err) => std::fmt::Display::fmt(err, f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            Error::PackEntryDecode(err) => Some(err),
            Error::Tree(err) => err.source(),
            Error::TreeTraversal(err) => err.source(),
            Error::Unsupported(_)
            | Error::IteratorInvariantNoRefDelta
            | Error::IteratorInvariantTrailer
            | Error::IteratorInvariantTooManyObjects(_)
            | Error::IteratorInvariantBaseOffset { .. } => None,
        }
    }
}

impl From<gix_hash::io::Error> for Error {
    fn from(err: gix_hash::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<crate::data::input::Error> for Error {
    fn from(err: crate::data::input::Error) -> Self {
        Error::PackEntryDecode(err)
    }
}

impl From<crate::cache::delta::Error> for Error {
    fn from(err: crate::cache::delta::Error) -> Self {
        Error::Tree(err)
    }
}

impl From<crate::cache::delta::traverse::Error> for Error {
    fn from(err: crate::cache::delta::traverse::Error) -> Self {
        Error::TreeTraversal(err)
    }
}
