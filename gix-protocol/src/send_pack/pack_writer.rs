//! Stream a `.pack` onto a `Write` sink for send-pack.
//!
//! The pack format is: `PACK` (4) + version u32 BE (4) + object count u32 BE (4)
//! + per-entry (variable-length type/size header + zlib-compressed body) + hash trailer.
//!
//! This module provides [`write_pack`], a thin wrapper over
//! [`gix_pack::data::output::bytes::FromEntriesIter`] which already handles the full
//! serialisation, including the variable-length entry headers and the trailing hash.
//! We collect entries first (to know the count), then drive the iterator to completion.

use std::io::Write;

use gix_pack::data::output::Entry;

/// Write `entries` as a complete pack stream onto `out`.
///
/// Entries are collected into a `Vec` so that the object count can be written into
/// the 12-byte pack header before any entry data.  The function drives
/// [`gix_pack::data::output::bytes::FromEntriesIter`] which handles entry-header
/// serialisation (variable-length type+size MSB-continuation encoding) and the
/// trailing `hash_kind` checksum of all bytes written.
///
/// Returns the trailing [`gix_hash::ObjectId`] (the pack checksum) so callers can
/// include it in any verification step.
pub fn write_pack<I>(entries: I, hash_kind: gix_hash::Kind, out: &mut impl Write) -> std::io::Result<gix_hash::ObjectId>
where
    I: IntoIterator<Item = Entry>,
{
    let entries: Vec<Entry> = entries.into_iter().collect();
    let num_entries = u32::try_from(entries.len())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "pack entry count exceeds u32::MAX"))?;

    // FromEntriesIter expects chunks: Iterator<Item = Result<Vec<Entry>, E>>.
    // We hand it a single-chunk Ok(entries).
    let chunk_iter = std::iter::once(Ok::<Vec<Entry>, std::convert::Infallible>(entries));

    let mut iter = gix_pack::data::output::bytes::FromEntriesIter::new(
        chunk_iter,
        &mut *out,
        num_entries,
        gix_pack::data::Version::V2,
        hash_kind,
    );

    // Drive the iterator. Each call to next() writes bytes; the final None-arm
    // writes the trailer hash and sets is_done.
    for result in &mut iter {
        result.map_err(|e| std::io::Error::other(e.to_string()))?;
    }

    iter.digest()
        .ok_or_else(|| std::io::Error::other("pack iterator did not produce a digest"))
}
