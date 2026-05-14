use std::iter::Peekable;

use crate::data::input;

/// An implementation of [`Iterator`] to write [encoded entries][input::Entry] to an inner implementation each time
/// `next()` is called.
///
/// It is able to deal with an unknown amount of objects as it will rewrite the pack header once the entries iterator
/// is depleted and compute the hash in one go by re-reading the whole file.
pub struct EntriesToBytesIter<I: Iterator, W> {
    /// An iterator for input [`input::Entry`] instances
    pub input: Peekable<I>,
    /// A way of writing encoded bytes.
    output: W,
    /// Our trailing hash when done writing all input entries
    trailer: Option<gix_hash::ObjectId>,
    /// The amount of objects in the iteration and the version of the packfile to be written.
    /// Will be `None` to signal the header was written already.
    data_version: crate::data::Version,
    /// The amount of entries seen so far
    num_entries: u32,
    /// If we are done, no additional writes will occur
    is_done: bool,
    /// The kind of hash to use for the digest
    object_hash: gix_hash::Kind,
}

impl<I, W> EntriesToBytesIter<I, W>
where
    I: Iterator<Item = Result<input::Entry, input::Error>>,
    W: std::io::Read + std::io::Write + std::io::Seek,
{
    /// Create a new instance reading [entries][input::Entry] from an `input` iterator and write pack data bytes to
    /// `output` writer, resembling a pack of `version`. The amount of entries will be dynamically determined and
    /// the pack is completed once the last entry was written.
    /// `object_hash` is the kind of hash to use for the pack checksum and maybe other places, depending on the version.
    ///
    /// # Panics
    ///
    /// Only [Version::V2](crate::data::Version::V2) is allowed for `version.
    pub fn new(input: I, output: W, version: crate::data::Version, object_hash: gix_hash::Kind) -> Self {
        assert!(
            matches!(version, crate::data::Version::V2),
            "currently only pack version 2 can be written",
        );
        EntriesToBytesIter {
            input: input.peekable(),
            output,
            object_hash,
            num_entries: 0,
            trailer: None,
            data_version: version,
            is_done: false,
        }
    }

    /// Returns the trailing hash over all ~ entries once done.
    /// It's `None` if we are not yet done writing.
    pub fn digest(&self) -> Option<gix_hash::ObjectId> {
        self.trailer
    }

    fn next_inner(&mut self, entry: input::Entry) -> Result<input::Entry, gix_hash::io::Error> {
        if self.num_entries == 0 {
            let header_bytes = crate::data::header::encode(self.data_version, 0);
            self.output.write_all(&header_bytes[..])?;
        }
        self.num_entries += 1;
        entry.header.write_to(entry.decompressed_size, &mut self.output)?;
        self.output.write_all(
            entry
                .compressed
                .as_deref()
                .expect("caller must configure generator to keep compressed bytes"),
        )?;
        Ok(entry)
    }

    fn write_header_and_digest(&mut self, last_entry: Option<&mut input::Entry>) -> Result<(), gix_hash::io::Error> {
        let header_bytes = crate::data::header::encode(self.data_version, self.num_entries);
        let num_bytes_written = if last_entry.is_some() {
            self.output.stream_position()?
        } else {
            header_bytes.len() as u64
        };
        self.output.rewind()?;
        self.output.write_all(&header_bytes[..])?;
        self.output.flush()?;

        self.output.rewind()?;
        let interrupt_never = std::sync::atomic::AtomicBool::new(false);
        let digest = gix_hash::bytes(
            &mut self.output,
            num_bytes_written,
            self.object_hash,
            &mut gix_features::progress::Discard,
            &interrupt_never,
        )?;
        self.output.write_all(digest.as_slice())?;
        self.output.flush()?;

        self.is_done = true;
        if let Some(last_entry) = last_entry {
            last_entry.trailer = Some(digest);
        }
        self.trailer = Some(digest);
        Ok(())
    }
}

impl<I, W> Iterator for EntriesToBytesIter<I, W>
where
    I: Iterator<Item = Result<input::Entry, input::Error>>,
    W: std::io::Read + std::io::Write + std::io::Seek,
{
    /// The amount of bytes written to `out` if `Ok` or the error `E` received from the input.
    type Item = Result<input::Entry, input::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_done {
            return None;
        }

        match self.input.next() {
            Some(res) => Some(match res {
                Ok(entry) => self
                    .next_inner(entry)
                    .and_then(|mut entry| {
                        if self.input.peek().is_none() {
                            self.write_header_and_digest(Some(&mut entry)).map(|_| entry)
                        } else {
                            Ok(entry)
                        }
                    })
                    .map_err(input::Error::from),
                Err(err) => {
                    self.is_done = true;
                    Err(err)
                }
            }),
            None => match self.write_header_and_digest(None) {
                Ok(_) => None,
                Err(err) => Some(Err(err.into())),
            },
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.input.size_hint()
    }
}
