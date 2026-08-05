/// Errors produced by reftable parsing and encoding.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// I/O failure while accessing block data.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Input ended unexpectedly.
    #[error("input ended unexpectedly")]
    Truncated,
    /// Data has an invalid checksum.
    #[error("checksum mismatch")]
    ChecksumMismatch,
    /// API misuse by caller.
    #[error("api error: {0}")]
    Api(&'static str),
    /// A compressed log block could not be decoded.
    #[error("invalid compressed log block")]
    Zlib,
    /// A varint could not be represented in `u64`.
    #[error("varint overflow")]
    VarintOverflow,
    /// Input data is malformed.
    #[error("malformed data: {0}")]
    Malformed(&'static str),
}
