/// Returned by [`BytesToEntriesIter::new_from_header()`][crate::data::input::BytesToEntriesIter::new_from_header()] and as part
/// of `Item` of [`BytesToEntriesIter`][crate::data::input::BytesToEntriesIter].
#[derive(Debug)]
#[allow(missing_docs)]
pub enum Error {
    Io(gix_hash::io::Error),
    PackParse(crate::data::header::decode::Error),
    Verify(gix_hash::verify::Error),
    IncompletePack { actual: u64, expected: u64 },
    NotFound { object_id: gix_hash::ObjectId },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(_) => f.write_str("An IO operation failed while streaming an entry"),
            Error::PackParse(err) => std::fmt::Display::fmt(err, f),
            Error::Verify(_) => f.write_str("Failed to verify pack checksum in trailer"),
            Error::IncompletePack { actual, expected } => write!(
                f,
                "pack is incomplete: it was decompressed into {actual} bytes but {expected} bytes where expected."
            ),
            Error::NotFound { object_id } => {
                write!(f, "The object {object_id} could not be decoded or wasn't found")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            Error::PackParse(err) => err.source(),
            Error::Verify(err) => Some(err),
            Error::IncompletePack { .. } | Error::NotFound { .. } => None,
        }
    }
}

impl From<gix_hash::io::Error> for Error {
    fn from(err: gix_hash::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<crate::data::header::decode::Error> for Error {
    fn from(err: crate::data::header::decode::Error) -> Self {
        Error::PackParse(err)
    }
}

impl From<gix_hash::verify::Error> for Error {
    fn from(err: gix_hash::verify::Error) -> Self {
        Error::Verify(err)
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
