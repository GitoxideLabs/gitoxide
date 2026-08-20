use crate::index;

/// Returned by [`index::File::traverse_with_index()`] and [`index::File::traverse_with_lookup`]
#[derive(thiserror::Error, Debug)]
#[expect(missing_docs)]
pub enum Error<E: std::error::Error + Send + Sync + 'static> {
    #[error("One of the traversal processors failed")]
    Processor(#[source] E),
    #[error("Failed to verify index file checksum")]
    IndexVerify(#[source] index::verify::checksum::Error),
    #[error("The pack delta tree index could not be built")]
    Tree(#[from] crate::cache::delta::from_offsets::Error),
    #[error("The tree traversal failed")]
    TreeTraversal(#[from] crate::cache::delta::traverse::Error),
    #[error(transparent)]
    EntryType(#[from] crate::data::entry::decode::Error),
    #[error("Object {id} at offset {offset} could not be decoded")]
    PackDecode {
        id: gix_hash::ObjectId,
        offset: u64,
        source: crate::data::decode::Error,
    },
    #[error("The packfiles checksum didn't match the index file checksum")]
    PackMismatch(#[source] gix_hash::verify::Error),
    #[error("Failed to verify pack file checksum")]
    PackVerify(#[source] crate::verify::checksum::Error),
    #[error("Error verifying object at offset {offset} against checksum in the index file")]
    PackObjectVerify {
        offset: u64,
        #[source]
        source: gix_error::Error,
    },
    #[error(
        "The CRC32 of {kind} object at offset {offset} didn't match the checksum in the index file: expected {expected}, got {actual}"
    )]
    Crc32Mismatch {
        expected: u32,
        actual: u32,
        offset: u64,
        kind: gix_object::Kind,
    },
    #[error("Interrupted")]
    Interrupted,
}
