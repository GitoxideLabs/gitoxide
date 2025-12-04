use std::ffi::c_int;

pub use zlib_rs::Inflate as Decompress;
pub use zlib_rs::InflateFlush as FlushDecompress;
pub use zlib_rs::Status;

/// The error produced by [`Decompress::decompress()`].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum DecompressError {
    #[error("stream error")]
    StreamError,
    #[error("Not enough memory")]
    InsufficientMemory,
    #[error("Invalid input data")]
    DataError,
    #[error("An unknown error occurred: {err}")]
    Unknown { err: c_int },
}

impl From<zlib_rs::InflateError> for DecompressError {
    fn from(value: zlib_rs::InflateError) -> Self {
        match value {
            zlib_rs::InflateError::NeedDict { .. } => Self::Unknown { err: 2 },
            zlib_rs::InflateError::StreamError => Self::StreamError,
            zlib_rs::InflateError::DataError => Self::DataError,
            zlib_rs::InflateError::MemError => Self::InsufficientMemory,
        }
    }
}

/// non-streaming interfaces for decompression
pub mod inflate {
    /// The error returned by various [Inflate methods][super::Inflate]
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Could not write all bytes when decompressing content")]
        WriteInflated(#[from] std::io::Error),
        #[error("Could not decode zip stream, status was '{0}'")]
        Inflate(#[from] super::DecompressError),
        #[error("The zlib status indicated an error, status was '{0:?}'")]
        Status(super::Status),
    }
}

/// Decompress a few bytes of a zlib stream without allocation
pub struct Inflate {
    /// The actual decompressor doing all the work.
    pub state: zlib_rs::Inflate,
}

impl Default for Inflate {
    fn default() -> Self {
        Self {
            state: zlib_rs::Inflate::new(true, 15),
        }
    }
}

impl Inflate {
    /// The amount of bytes consumed from the input so far.
    pub fn total_in(&self) -> u64 {
        self.state.total_in()
    }

    /// The amount of decompressed bytes that have been written to the output thus far.
    pub fn total_out(&self) -> u64 {
        self.state.total_out()
    }

    /// Decompress `input` and write all decompressed bytes into `output`, with `flush` defining some details about this.
    pub fn decompress(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        flush: zlib_rs::InflateFlush,
    ) -> Result<Status, zlib_rs::InflateError> {
        self.state.decompress(input, output, flush)
    }

    /// Run the decompressor exactly once. Cannot be run multiple times
    pub fn once(&mut self, input: &[u8], out: &mut [u8]) -> Result<(Status, usize, usize), inflate::Error> {
        let before_in = self.state.total_in();
        let before_out = self.state.total_out();
        match self.state.decompress(input, out, FlushDecompress::NoFlush) {
            Ok(status) => Ok((
                status,
                (self.state.total_in() - before_in) as usize,
                (self.state.total_out() - before_out) as usize,
            )),
            Err(e) => Err(inflate::Error::Inflate(e.into())),
        }
    }

    /// Ready this instance for decoding another data stream.
    pub fn reset(&mut self) {
        self.state.reset(true);
    }
}

///
pub mod stream;
