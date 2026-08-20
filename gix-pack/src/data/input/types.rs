/// Returned by [`BytesToEntriesIter::new_from_header()`][crate::data::input::BytesToEntriesIter::new_from_header()] and as part
/// of `Item` of [`BytesToEntriesIter`][crate::data::input::BytesToEntriesIter].
#[derive(thiserror::Error, Debug)]
#[expect(missing_docs)]
pub enum Error {
    #[error("An IO operation failed while streaming an entry")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    PackParse(#[from] crate::data::header::decode::Error),
    #[error("Failed to verify pack checksum in trailer")]
    Verify(#[from] gix_hash::verify::Error),
    #[error("pack is incomplete: it was decompressed into {actual} bytes but {expected} bytes where expected.")]
    IncompletePack { actual: u64, expected: u64 },
    #[error("The object {object_id} could not be decoded or wasn't found")]
    NotFound { object_id: gix_hash::ObjectId },
}

impl From<gix_hash::io::Error> for Error {
    fn from(err: gix_hash::io::Error) -> Self {
        Error::Io(std::io::Error::other(err.into_error()))
    }
}

/// Iteration Mode
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Mode {
    /// Provide the trailer as read from the pack
    AsIs,
    /// Generate an own hash and trigger an error on the last iterated object
    /// if it does not match the hash provided with the pack.
    ///
    /// This way the one iterating the data cannot miss corruption as long as
    /// the iteration is continued through to the end.
    Verify,
    /// Generate an own hash and if there was an error or the objects are depleted early
    /// due to partial packs, return the last valid entry and with our own hash thus far.
    /// Note that the existing pack hash, if present, will be ignored.
    /// As we won't know which objects fails, every object will have the hash obtained thus far.
    /// This also means that algorithms must know about this possibility, or else might wrongfully
    /// assume the pack is finished.
    Restore,
}

/// Define what to do with the compressed bytes portion of a pack [`Entry`][super::Entry]
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EntryDataMode {
    /// Do nothing with the compressed bytes we read
    Ignore,
    /// Only create a CRC32 of the entry, otherwise similar to `Ignore`
    Crc32,
    /// Keep them and pass them along in a newly allocated buffer
    Keep,
    /// As above, but also compute a CRC32
    KeepAndCrc32,
}

impl EntryDataMode {
    /// Returns true if a crc32 should be computed
    pub fn crc32(&self) -> bool {
        match self {
            EntryDataMode::KeepAndCrc32 | EntryDataMode::Crc32 => true,
            EntryDataMode::Keep | EntryDataMode::Ignore => false,
        }
    }
    /// Returns true if compressed bytes should be kept
    pub fn keep(&self) -> bool {
        match self {
            EntryDataMode::Keep | EntryDataMode::KeepAndCrc32 => true,
            EntryDataMode::Ignore | EntryDataMode::Crc32 => false,
        }
    }
}
