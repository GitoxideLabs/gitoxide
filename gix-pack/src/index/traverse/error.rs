use crate::index;

/// Returned by [`index::File::traverse_with_index()`] and [`index::File::traverse_with_lookup`]
#[derive(Debug)]
#[allow(missing_docs)]
pub enum Error<E: std::error::Error + Send + Sync + 'static> {
    Processor(E),
    IndexVerify(index::verify::checksum::Error),
    Tree(crate::cache::delta::from_offsets::Error),
    TreeTraversal(crate::cache::delta::traverse::Error),
    EntryType(crate::data::entry::decode::Error),
    PackDecode {
        id: gix_hash::ObjectId,
        offset: u64,
        source: crate::data::decode::Error,
    },
    PackMismatch(gix_hash::verify::Error),
    PackVerify(crate::verify::checksum::Error),
    PackObjectVerify {
        offset: u64,
        source: gix_object::data::verify::Error,
    },
    Crc32Mismatch {
        expected: u32,
        actual: u32,
        offset: u64,
        kind: gix_object::Kind,
    },
    Interrupted,
}

impl<E: std::error::Error + Send + Sync + 'static> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Processor(_) => f.write_str("One of the traversal processors failed"),
            Error::IndexVerify(_) => f.write_str("Failed to verify index file checksum"),
            Error::Tree(_) => f.write_str("The pack delta tree index could not be built"),
            Error::TreeTraversal(_) => f.write_str("The tree traversal failed"),
            Error::EntryType(err) => std::fmt::Display::fmt(err, f),
            Error::PackDecode { id, offset, .. } => {
                write!(f, "Object {id} at offset {offset} could not be decoded")
            }
            Error::PackMismatch(_) => f.write_str("The packfiles checksum didn't match the index file checksum"),
            Error::PackVerify(_) => f.write_str("Failed to verify pack file checksum"),
            Error::PackObjectVerify { offset, .. } => write!(
                f,
                "Error verifying object at offset {offset} against checksum in the index file"
            ),
            Error::Crc32Mismatch {
                expected,
                actual,
                offset,
                kind,
            } => write!(
                f,
                "The CRC32 of {kind} object at offset {offset} didn't match the checksum in the index file: expected {expected}, got {actual}"
            ),
            Error::Interrupted => f.write_str("Interrupted"),
        }
    }
}

impl<E: std::error::Error + Send + Sync + 'static> std::error::Error for Error<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Processor(err) => Some(err),
            Error::IndexVerify(err) => Some(err),
            Error::Tree(err) => Some(err),
            Error::TreeTraversal(err) => Some(err),
            Error::EntryType(err) => err.source(),
            Error::PackDecode { source, .. } => Some(source),
            Error::PackMismatch(err) => Some(err),
            Error::PackVerify(err) => Some(err),
            Error::PackObjectVerify { source, .. } => Some(source),
            Error::Crc32Mismatch { .. } | Error::Interrupted => None,
        }
    }
}

impl<E: std::error::Error + Send + Sync + 'static> From<crate::cache::delta::from_offsets::Error> for Error<E> {
    fn from(err: crate::cache::delta::from_offsets::Error) -> Self {
        Error::Tree(err)
    }
}

impl<E: std::error::Error + Send + Sync + 'static> From<crate::cache::delta::traverse::Error> for Error<E> {
    fn from(err: crate::cache::delta::traverse::Error) -> Self {
        Error::TreeTraversal(err)
    }
}

impl<E: std::error::Error + Send + Sync + 'static> From<crate::data::entry::decode::Error> for Error<E> {
    fn from(err: crate::data::entry::decode::Error) -> Self {
        Error::EntryType(err)
    }
}
