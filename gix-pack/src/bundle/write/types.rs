use std::{hash::Hash, io, io::SeekFrom, path::PathBuf, sync::Arc};

use gix_tempfile::handle::Writable;

/// Configuration for [`write_to_directory`][crate::Bundle::write_to_directory()] or
/// [`write_to_directory_eagerly`][crate::Bundle::write_to_directory_eagerly()]
#[derive(Debug, Clone)]
pub struct Options {
    /// The amount of threads to use at most when resolving the pack. If `None`, all logical cores are used.
    pub thread_limit: Option<usize>,
    /// Determine how much processing to spend on protecting against corruption or recovering from errors.
    pub iteration_mode: crate::data::input::Mode,
    /// The version of pack index to write, should be [`crate::index::Version::default()`]
    pub index_version: crate::index::Version,
    /// If `Some`, rejects individual allocations above the given number of bytes while building the delta tree or resolving
    /// streamed pack entries into decoded object and delta result buffers to write the index.
    /// `Some(0)` rejects all non-empty allocations.
    pub alloc_limit_bytes: Option<usize>,
    /// The compression level to use when deflating base objects that are added to the pack to complete a thin pack.
    ///
    /// Defaults to [`Compression::BEST_SPEED`](gix_zlib::Compression::BEST_SPEED) - `git` compresses
    /// these with the same level it uses for loose objects, which is `core.looseCompression` or
    /// `core.compression`, or level 1 if neither is set.
    pub compression: gix_zlib::Compression,
}

impl Default for Options {
    /// Options which favor speed and correctness and write the most commonly supported index file.
    fn default() -> Self {
        Options {
            thread_limit: None,
            iteration_mode: crate::data::input::Mode::Verify,
            index_version: Default::default(),
            alloc_limit_bytes: None,
            compression: gix_zlib::Compression::BEST_SPEED,
        }
    }
}

/// Returned by [`write_to_directory`][crate::Bundle::write_to_directory()] or
/// [`write_to_directory_eagerly`][crate::Bundle::write_to_directory_eagerly()]
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Outcome {
    /// The successful result of the index write operation.
    pub index: crate::index::write::Outcome,
    /// The version of the pack.
    pub pack_version: crate::data::Version,
    /// The kind of hash stored within the pack and indices.
    pub object_hash: gix_hash::Kind,

    /// The path to the pack index file.
    pub index_path: Option<PathBuf>,
    /// The path to the pack data file.
    pub data_path: Option<PathBuf>,
    /// The path to the `.keep` file to prevent collection of the newly written pack until refs are pointing to it.
    /// It might be `None` if the file at `data_path` already existed, indicating that we have received a pack that
    /// was already present locally.
    ///
    /// The file is created right before moving the pack data and index data into place (i.e. `data_path` and `index_path`)
    /// and is expected to be removed by the caller when ready.
    pub keep_path: Option<PathBuf>,
}

impl Outcome {
    /// Instantiate a bundle from the newly written index and data file that are represented by this `Outcome`
    pub fn to_bundle(&self) -> Option<Result<crate::Bundle, crate::bundle::init::Error>> {
        self.index_path
            .as_ref()
            .map(|path| crate::Bundle::at(path, self.object_hash))
    }
}

pub(crate) type SharedTempFile = Arc<parking_lot::Mutex<std::io::BufWriter<gix_tempfile::Handle<Writable>>>>;

pub(crate) struct PassThrough<R> {
    pub reader: R,
    pub writer: Option<SharedTempFile>,
}

impl<R> io::Read for PassThrough<R>
where
    R: io::Read,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let bytes_read = self.reader.read(buf)?;
        if let Some(writer) = self.writer.as_mut() {
            use std::io::Write;
            writer.lock().write_all(&buf[..bytes_read])?;
        }
        Ok(bytes_read)
    }
}
impl<R> io::BufRead for PassThrough<R>
where
    R: io::BufRead,
{
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.reader.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.reader.consume(amt);
    }
}

pub(crate) struct LockWriter {
    pub writer: SharedTempFile,
}

impl io::Write for LockWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writer.lock().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.lock().flush()
    }
}

impl io::Read for LockWriter {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.writer.lock().get_mut().read(buf)
    }
}

impl io::Seek for LockWriter {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.writer.lock().seek(pos)
    }
}
