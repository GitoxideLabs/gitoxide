use gix_error::{Error, Result};
use std::{fs, io};

use gix_error::{ErrorExt, ResultExt, bail, message};
use gix_hash::{Hasher, ObjectId};
use gix_zlib::stream::inflate::Measure;

use crate::data::input;

/// An iterator over [`Entries`][input::Entry] in a byte stream.
///
/// The iterator used as part of [`Bundle::write_to_directory(…)`][crate::Bundle::write_to_directory()].
pub struct BytesToEntriesIter<BR> {
    read: BR,
    measure: Measure,
    offset: u64,
    had_error: bool,
    version: crate::data::Version,
    objects_left: u32,
    hash: Option<Hasher>,
    mode: input::Mode,
    compressed: input::EntryDataMode,
    compressed_buf: Option<Vec<u8>>,
    hash_len: usize,
    object_hash: gix_hash::Kind,
}

/// Access
impl<BR> BytesToEntriesIter<BR> {
    /// The pack version currently being iterated
    pub fn version(&self) -> crate::data::Version {
        self.version
    }

    /// The kind of iteration
    pub fn mode(&self) -> input::Mode {
        self.mode
    }
}

/// Initialization
impl<BR> BytesToEntriesIter<BR>
where
    BR: io::BufRead,
{
    /// Obtain an iterator from a `read` stream to a pack data file and configure it using `mode` and `compressed`.
    /// `object_hash` specifies which hash is used for objects in ref-delta entries.
    ///
    /// Note that `read` is expected at the beginning of a valid pack data file with a header, entries and a trailer.
    pub fn new_from_header(
        mut read: BR,
        mode: input::Mode,
        compressed: input::EntryDataMode,
        object_hash: gix_hash::Kind,
    ) -> Result<BytesToEntriesIter<BR>> {
        let mut header_data = [0u8; crate::data::header::SIZE];
        read.read_exact(&mut header_data).map_err(io_error)?;

        let (version, num_objects) = crate::data::header::decode(&header_data)?;
        Ok(BytesToEntriesIter {
            read,
            measure: Measure::default(),
            compressed,
            offset: crate::data::header::SIZE as u64,
            had_error: false,
            version,
            objects_left: num_objects,
            hash: (mode != input::Mode::AsIs).then(|| {
                let mut hash = gix_hash::hasher(object_hash);
                hash.update(&header_data);
                hash
            }),
            mode,
            compressed_buf: None,
            hash_len: object_hash.len_in_bytes(),
            object_hash,
        })
    }

    fn next_inner(&mut self) -> Result<input::Entry> {
        self.objects_left -= 1; // even an error counts as objects

        // Read header
        let entry = match self.hash.as_mut() {
            Some(hash) => {
                let mut read = read_and_pass_to(
                    &mut self.read,
                    HashWrite {
                        inner: io::sink(),
                        hash,
                    },
                );
                crate::data::Entry::from_read(&mut read, self.offset, self.hash_len)
            }
            None => crate::data::Entry::from_read(&mut self.read, self.offset, self.hash_len),
        }
        .map_err(io_error)?;

        // Decompress object to learn its compressed bytes
        let compressed_buf = self.compressed_buf.take().unwrap_or_else(|| Vec::with_capacity(4096));
        let mut compressed_reader = read_and_pass_to(
            &mut self.read,
            if self.compressed.keep() {
                // This buffer stores compressed bytes. The decompressed size is unrelated to the compressed byte
                // count and is declared by pack data, so using it as capacity could cause excessive allocation.
                // It's notable that `decompressed_size` is also untrusted.
                Vec::new()
            } else {
                compressed_buf
            },
        );

        let (bytes_copied, compressed_size) = self.measure.read(&mut compressed_reader).map_err(io_error)?;
        if bytes_copied != entry.decompressed_size {
            bail!(gix_error::corruption(format!(
                "pack is incomplete: it was decompressed into {bytes_copied} bytes but {} bytes where expected.",
                entry.decompressed_size
            )));
        }

        let pack_offset = self.offset;
        self.offset += entry.header_size() as u64 + compressed_size;

        let mut compressed = compressed_reader.write;
        debug_assert_eq!(
            compressed_size,
            compressed.len() as u64,
            "we must track exactly the same amount of bytes as read by the decompressor"
        );
        if let Some(hash) = self.hash.as_mut() {
            hash.update(&compressed);
        }

        let crc32 = if self.compressed.crc32() {
            let mut header_buf = [0u8; 12 + gix_hash::Kind::longest().len_in_bytes()];
            let header_len = entry
                .header
                .write_to(bytes_copied, &mut header_buf.as_mut())
                .map_err(io_error)?;
            let mut crc32 = crc32fast::Hasher::new();
            crc32.update(&header_buf[..header_len]);
            crc32.update(&compressed);
            Some(crc32.finalize())
        } else {
            None
        };

        let compressed = if self.compressed.keep() {
            Some(compressed)
        } else {
            compressed.clear();
            self.compressed_buf = Some(compressed);
            None
        };

        // Last objects gets trailer (which is potentially verified)
        let trailer = self.try_read_trailer()?;
        Ok(input::Entry {
            header: entry.header,
            header_size: entry.header_size() as u16,
            compressed,
            compressed_size,
            crc32,
            pack_offset,
            decompressed_size: bytes_copied,
            trailer,
        })
    }

    fn try_read_trailer(&mut self) -> Result<Option<ObjectId>> {
        Ok(if self.objects_left == 0 {
            let mut id = gix_hash::ObjectId::null(self.object_hash);
            if let Err(err) = self.read.read_exact(id.as_mut_slice())
                && self.mode != input::Mode::Restore
            {
                return Err(io_error(err));
            }

            if let Some(hash) = self.hash.take() {
                let actual_id = hash.try_finalize().map_err(hash_io_error)?;
                if self.mode == input::Mode::Restore {
                    id = actual_id;
                } else {
                    actual_id
                        .verify(&id)
                        .or_raise(|| message("Failed to verify pack checksum in trailer"))?;
                }
            }
            Some(id)
        } else if self.mode == input::Mode::Restore {
            let hash = self.hash.clone().expect("in restore mode a hash is set");
            Some(hash.try_finalize().map_err(hash_io_error)?)
        } else {
            None
        })
    }
}

fn read_and_pass_to<R: io::Read, W: io::Write>(read: &mut R, to: W) -> PassThrough<&mut R, W> {
    PassThrough { read, write: to }
}

impl<R> Iterator for BytesToEntriesIter<R>
where
    R: io::BufRead,
{
    type Item = Result<input::Entry>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.had_error || self.objects_left == 0 {
            return None;
        }
        let result = self.next_inner();
        self.had_error = result.is_err();
        if self.had_error {
            self.objects_left = 0;
        }
        if self.mode == input::Mode::Restore && self.had_error {
            None
        } else {
            Some(result)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.objects_left as usize, Some(self.objects_left as usize))
    }
}

impl<R> std::iter::ExactSizeIterator for BytesToEntriesIter<R> where R: io::BufRead {}

struct PassThrough<R, W> {
    read: R,
    write: W,
}

impl<R, W> io::BufRead for PassThrough<R, W>
where
    Self: io::Read,
    R: io::BufRead,
    W: io::Write,
{
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.read.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        let buf = self
            .read
            .fill_buf()
            .expect("never fail as we called fill-buf before and this does nothing");
        self.write
            .write_all(&buf[..amt])
            .expect("a write to never fail - should be a memory buffer");
        self.read.consume(amt);
    }
}

impl<R, W> io::Read for PassThrough<R, W>
where
    W: io::Write,
    R: io::Read,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let bytes_read = self.read.read(buf)?;
        self.write.write_all(&buf[..bytes_read])?;
        Ok(bytes_read)
    }
}

impl<T> crate::data::File<T>
where
    T: crate::FileData,
{
    /// Returns an iterator over [`Entries`][crate::data::input::Entry], without making use of the memory mapping.
    pub fn streaming_iter(&self) -> Result<BytesToEntriesIter<impl io::BufRead>> {
        let reader = io::BufReader::with_capacity(4096 * 8, fs::File::open(&self.path).map_err(io_error)?);
        BytesToEntriesIter::new_from_header(
            reader,
            input::Mode::Verify,
            input::EntryDataMode::KeepAndCrc32,
            self.object_hash,
        )
    }
}

fn io_error(err: io::Error) -> Error {
    err.and_raise(message("An IO operation failed while streaming an entry"))
}

fn hash_io_error(err: Error) -> Error {
    err.and_raise(message("An IO operation failed while streaming an entry"))
}

/// A utility to automatically generate a hash while writing into an inner writer.
pub struct HashWrite<'a, T> {
    /// The hash implementation.
    pub hash: &'a mut Hasher,
    /// The inner writer.
    pub inner: T,
}

impl<T> std::io::Write for HashWrite<'_, T>
where
    T: std::io::Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let written = self.inner.write(buf)?;
        self.hash.update(&buf[..written]);
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
