use std::io::Write;

use gix_features::hash;

use crate::data::output;
use crate::exact_vec;

/// The error returned by `next()` in the [`FromEntriesIter`] iterator.
#[allow(missing_docs)]
#[derive(Debug, thiserror::Error)]
pub enum Error<E>
where
    E: std::error::Error + 'static,
{
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Input(E),
}

/// An implementation of [`Iterator`] to write [encoded entries][output::Entry] to an inner implementation each time
/// `next()` is called.
pub struct FromEntriesIter<I, W> {
    /// An iterator for input [`output::Entry`] instances
    pub input: I,
    /// A way of writing encoded bytes.
    output: hash::Write<W>,
    /// Our trailing hash when done writing all input entries
    trailer: Option<gix_hash::ObjectId>,
    /// The amount of objects in the iteration and the version of the packfile to be written.
    /// Will be `None` to signal the header was written already.
    header_info: Option<(crate::data::Version, u32)>,
    /// The pack data version with which pack entries should be written.
    entry_version: crate::data::Version,
    /// The amount of written bytes thus far
    written: u64,
    /// Required to quickly find offsets by object IDs, as future objects may refer to those in the past to become a delta offset base.
    /// It stores the pack offsets at which objects begin.
    /// Additionally we store if an object was invalid, and if so we will not write it nor will we allow delta objects to it.
    pack_offsets_and_validity: Vec<(u64, bool)>,
    /// If we are done, no additional writes will occur
    is_done: bool,
}

impl<I, W, E> FromEntriesIter<I, W>
where
    I: Iterator<Item = Result<Vec<output::Entry>, E>>,
    W: std::io::Write,
    E: std::error::Error + 'static,
{
    /// Create a new instance reading [entries][output::Entry] from an `input` iterator and write pack data bytes to
    /// `output` writer, resembling a pack of `version` with exactly `num_entries` amount of objects contained in it.
    /// `object_hash` is the kind of hash to use for the pack checksum and maybe other places, depending on the version.
    ///
    /// The input chunks are expected to be sorted already. You can use the [`InOrderIter`][gix_features::parallel::InOrderIter] to assure
    /// this happens on the fly holding entire chunks in memory as long as needed for them to be dispensed in order.
    ///
    /// # Panics
    ///
    /// Not all combinations of `object_hash` and `version` are supported currently triggering assertion errors.
    pub fn new(
        input: I,
        output: W,
        num_entries: u32,
        version: crate::data::Version,
        object_hash: gix_hash::Kind,
    ) -> Self {
        assert!(
            matches!(version, crate::data::Version::V2),
            "currently only pack version 2 can be written",
        );
        FromEntriesIter {
            input,
            output: hash::Write::new(output, object_hash),
            trailer: None,
            entry_version: version,
            pack_offsets_and_validity: exact_vec(num_entries as usize),
            written: 0,
            header_info: Some((version, num_entries)),
            is_done: false,
        }
    }

    /// Consume this instance and return the `output` implementation.
    ///
    /// _Note_ that the `input` iterator can be moved out of this instance beforehand.
    pub fn into_write(self) -> W {
        self.output.inner
    }

    /// Returns the trailing hash over all written entries once done.
    /// It's `None` if we are not yet done writing.
    pub fn digest(&self) -> Option<gix_hash::ObjectId> {
        self.trailer
    }

    fn next_inner(&mut self) -> Result<u64, Error<E>> {
        let previous_written = self.written;
        if let Some((version, num_entries)) = self.header_info.take() {
            let header_bytes = crate::data::header::encode(version, num_entries);
            self.output.write_all(&header_bytes[..])?;
            self.written += header_bytes.len() as u64;
        }
        match self.input.next() {
            Some(entries) => {
                for entry in entries.map_err(Error::Input)? {
                    if entry.is_invalid() {
                        self.pack_offsets_and_validity.push((0, false));
                        continue;
                    }
                    self.pack_offsets_and_validity.push((self.written, true));
                    let header = entry.to_entry_header(self.entry_version, |index| {
                        let (base_offset, is_valid_object) = self.pack_offsets_and_validity[index];
                        if !is_valid_object {
                            unreachable!("if you see this the object database is correct as a delta refers to a non-existing object")
                        }
                        self.written - base_offset
                    });
                    self.written += header.write_to(entry.decompressed_size as u64, &mut self.output)? as u64;
                    self.written += std::io::copy(&mut &*entry.compressed_data, &mut self.output)?;
                }
            }
            None => {
                let digest = self.output.hash.clone().digest();
                self.output.inner.write_all(&digest[..])?;
                self.written += digest.len() as u64;
                self.output.inner.flush()?;
                self.is_done = true;
                self.trailer = Some(gix_hash::ObjectId::from(digest));
            }
        }
        Ok(self.written - previous_written)
    }
}

impl<I, W, E> Iterator for FromEntriesIter<I, W>
where
    I: Iterator<Item = Result<Vec<output::Entry>, E>>,
    W: std::io::Write,
    E: std::error::Error + 'static,
{
    /// The amount of bytes written to `out` if `Ok` or the error `E` received from the input.
    type Item = Result<u64, Error<E>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_done {
            return None;
        }
        Some(match self.next_inner() {
            Err(err) => {
                self.is_done = true;
                Err(err)
            }
            Ok(written) => Ok(written),
        })
    }
}
