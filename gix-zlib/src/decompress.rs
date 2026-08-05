//! Implementations for [`Decompress`](crate::Decompress).

use zlib_rs::InflateError;

use crate::{Decompress, FlushDecompress, Status};
///
/// The error produced by [`Decompress::decompress()`].
#[derive(Debug)]
#[expect(missing_docs)]
pub enum DecompressError {
    StreamError,
    InsufficientMemory,
    DataError,
    NeedDict,
}

impl std::fmt::Display for DecompressError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            DecompressError::StreamError => "stream error",
            DecompressError::InsufficientMemory => "Not enough memory",
            DecompressError::DataError => "Invalid input data",
            DecompressError::NeedDict => "Decompressing this input requires a dictionary",
        })
    }
}

impl std::error::Error for DecompressError {}

impl Default for Decompress {
    fn default() -> Self {
        Self::new()
    }
}

impl Decompress {
    /// The amount of bytes consumed from the input so far.
    pub fn total_in(&self) -> u64 {
        self.0.total_in()
    }

    /// The amount of decompressed bytes that have been written to the output thus far.
    pub fn total_out(&self) -> u64 {
        self.0.total_out()
    }

    /// Create a new instance. Note that it allocates in various ways and thus should be re-used.
    pub fn new() -> Self {
        let config = zlib_rs::InflateConfig::default();
        let header = true;
        let inner = zlib_rs::Inflate::new(header, config.window_bits as u8);
        Self(inner)
    }

    /// Reset the state to allow handling a new stream.
    pub fn reset(&mut self) {
        self.0.reset(true);
    }

    /// The message describing the last error that occurred in [`Self::decompress()`], if available.
    pub fn error_message(&self) -> Option<&'static str> {
        self.0.error_message()
    }

    /// Decompress `input` and write all decompressed bytes into `output`, with `flush` defining some details about this.
    pub fn decompress(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        flush: FlushDecompress,
    ) -> Result<Status, DecompressError> {
        let inflate_flush = match flush {
            FlushDecompress::None => zlib_rs::InflateFlush::NoFlush,
            FlushDecompress::Sync => zlib_rs::InflateFlush::SyncFlush,
            FlushDecompress::Finish => zlib_rs::InflateFlush::Finish,
        };

        let status = self.0.decompress(input, output, inflate_flush)?;
        match status {
            zlib_rs::Status::Ok => Ok(Status::Ok),
            zlib_rs::Status::BufError => Ok(Status::BufError),
            zlib_rs::Status::StreamEnd => Ok(Status::StreamEnd),
        }
    }
}

impl From<InflateError> for DecompressError {
    fn from(value: InflateError) -> Self {
        match value {
            InflateError::NeedDict { .. } => DecompressError::NeedDict,
            InflateError::StreamError => DecompressError::StreamError,
            InflateError::DataError => DecompressError::DataError,
            InflateError::MemError => DecompressError::InsufficientMemory,
        }
    }
}
