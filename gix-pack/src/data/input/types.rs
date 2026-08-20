/// Returned by [`BytesToEntriesIter::new_from_header()`][crate::data::input::BytesToEntriesIter::new_from_header()] and as part
/// of `Item` of [`BytesToEntriesIter`][crate::data::input::BytesToEntriesIter].
pub type Error = gix_error::Exn;

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
