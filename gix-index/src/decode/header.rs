pub(crate) const SIZE: usize = 4 /*signature*/ + 4 /*version*/ + 4 /* num entries */;

use crate::{Version, util::from_be_u32};

pub(crate) const SIGNATURE: &[u8] = b"DIRC";

mod error {

    /// The error produced when failing to decode an index header.
    #[derive(Debug)]
    #[allow(missing_docs)]
    pub enum Error {
        Corrupt(&'static str),
        UnsupportedVersion(u32),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Corrupt(msg) => f.write_str(msg),
                Error::UnsupportedVersion(version) => write!(f, "Index version {version} is not supported"),
            }
        }
    }

    impl std::error::Error for Error {}
}
pub use error::Error;

pub(crate) fn decode(data: &[u8], object_hash: gix_hash::Kind) -> Result<(Version, u32, &[u8]), Error> {
    if data.len() < (3 * 4) + object_hash.len_in_bytes() {
        return Err(Error::Corrupt(
            "File is too small even for header with zero entries and smallest hash",
        ));
    }

    let (signature, data) = data.split_at(4);
    if signature != SIGNATURE {
        return Err(Error::Corrupt(
            "Signature mismatch - this doesn't claim to be a header file",
        ));
    }

    let (version, data) = data.split_at(4);
    let version = match from_be_u32(version) {
        2 => Version::V2,
        3 => Version::V3,
        4 => Version::V4,
        unknown => return Err(Error::UnsupportedVersion(unknown)),
    };
    let (entries, data) = data.split_at(4);
    let entries = from_be_u32(entries);

    Ok((version, entries, data))
}
