#![deny(missing_docs)]
//! Streaming compression and decompression utilities used by gitoxide.

/// The compression level to use for zlib-based streams, in the range from 0 (no compression)
/// to 9 (best compression, slowest).
///
/// Note that `git` maps its configured level of `-1` to the zlib default, which is level 6
/// and available as [`Compression::DEFAULT`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Compression(i32);

impl Compression {
    /// Do not compress at all, while still producing a valid zlib stream.
    pub const NONE: Compression = Compression(0);
    /// The fastest compression, with the lowest compression ratio, also known as level 1.
    ///
    /// This is what `git` uses for loose objects unless configured otherwise with `core.looseCompression`.
    pub const BEST_SPEED: Compression = Compression(1);
    /// The default compromise between speed and compression ratio, also known as level 6.
    ///
    /// This is what `git` uses when writing packs unless configured otherwise with `pack.compression`.
    pub const DEFAULT: Compression = Compression(6);
    /// The best compression ratio at the expense of speed, also known as level 9.
    pub const BEST: Compression = Compression(9);

    /// Create a new instance from `level` if it is within the valid range from 0 to 9, inclusive.
    pub fn new(level: i32) -> Option<Self> {
        (0..=9).contains(&level).then_some(Compression(level))
    }

    /// Return the compression level as integer in the range from 0 to 9, inclusive.
    pub fn level(&self) -> i32 {
        self.0
    }
}

impl Default for Compression {
    fn default() -> Self {
        Compression::DEFAULT
    }
}

/// A type to hold all state needed for decompressing a ZLIB encoded stream.
pub struct Decompress(zlib_rs::Inflate);

/// The status returned by [`Decompress::decompress()`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The decompress operation went well. Not to be confused with `StreamEnd`, so one can continue
    /// the decompression.
    Ok,
    /// An error occurred when decompression.
    BufError,
    /// The stream was fully decompressed.
    StreamEnd,
}

/// Values which indicate the form of flushing to be used when
/// decompressing in-memory data.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum FlushDecompress {
    /// A typical parameter for passing to compression/decompression functions,
    /// this indicates that the underlying stream to decide how much data to
    /// accumulate before producing output in order to maximize compression.
    None = 0,

    /// All pending output is flushed to the output buffer and the output is
    /// aligned on a byte boundary so that the decompressor can get all input
    /// data available so far.
    ///
    /// Flushing may degrade compression for some compression algorithms and so
    /// it should only be used when necessary. This will complete the current
    /// deflate block and follow it with an empty stored block.
    Sync = 2,

    /// Pending input is processed and pending output is flushed.
    ///
    /// The return value may indicate that the stream is not yet done and more
    /// data has yet to be processed.
    Finish = 4,
}

/// Decompress a few bytes of a zlib stream without allocation
#[derive(Default)]
pub struct Inflate {
    /// The actual decompressor doing all the work.
    pub state: Decompress,
}

/// Streaming compression and decompression utilities built on [`std::io`] traits.
pub mod stream;

/// Types supporting single-step, allocation-free decompression.
pub mod inflate;

mod decompress;
