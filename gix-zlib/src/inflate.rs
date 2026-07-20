use crate::{FlushDecompress, Inflate, Status};

/// The error returned by various [Inflate methods][super::Inflate]
#[derive(Debug)]
#[expect(missing_docs)]
pub enum Error {
    WriteInflated(std::io::Error),
    Inflate(super::DecompressError),
    Status(super::Status),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::WriteInflated(_) => f.write_str("Could not write all bytes when decompressing content"),
            Error::Inflate(status) => write!(f, "Could not decode zip stream, status was '{status}'"),
            Error::Status(status) => write!(f, "The zlib status indicated an error, status was '{status:?}'"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::WriteInflated(err) => Some(err),
            Error::Inflate(err) => Some(err),
            Error::Status(_) => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::WriteInflated(err)
    }
}

impl From<super::DecompressError> for Error {
    fn from(err: super::DecompressError) -> Self {
        Error::Inflate(err)
    }
}

impl Inflate {
    /// Run the decompressor exactly once. Cannot be run multiple times
    pub fn once(&mut self, input: &[u8], out: &mut [u8]) -> Result<(Status, usize, usize), Error> {
        let before_in = self.state.total_in();
        let before_out = self.state.total_out();
        let status = self.state.decompress(input, out, FlushDecompress::None)?;
        Ok((
            status,
            (self.state.total_in() - before_in) as usize,
            (self.state.total_out() - before_out) as usize,
        ))
    }

    /// Ready this instance for decoding another data stream.
    pub fn reset(&mut self) {
        self.state.reset();
    }
}
