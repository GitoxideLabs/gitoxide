pub(crate) const SIZE: usize = 4 /*signature*/ + 4 /*version*/ + 4 /* num entries */;

use crate::{Version, util::from_be_u32};

pub(crate) const SIGNATURE: &[u8] = b"DIRC";

mod error {
    /// The error produced when failing to decode an index header.
    pub type Error = gix_error::Exn;
}
pub use error::Error;

pub(crate) fn decode(data: &[u8], object_hash: gix_hash::Kind) -> Result<(Version, u32, &[u8]), Error> {
    use gix_error::ErrorExt;

    if data.len() < (3 * 4) + object_hash.len_in_bytes() {
        return Err(gix_error::CorruptionError::new(
            "File is too small even for header with zero entries and smallest hash",
        )
        .raise_erased());
    }

    let (signature, data) = data.split_at(4);
    if signature != SIGNATURE {
        return Err(
            gix_error::CorruptionError::new("Signature mismatch - this doesn't claim to be a header file")
                .raise_erased(),
        );
    }

    let (version, data) = data.split_at(4);
    let version = match from_be_u32(version) {
        2 => Version::V2,
        3 => Version::V3,
        4 => Version::V4,
        unknown => {
            return Err(
                gix_error::ValidationError::new(format!("Index version {unknown} is not supported")).raise_erased(),
            );
        }
    };
    let (entries, data) = data.split_at(4);
    let entries = from_be_u32(entries);

    Ok((version, entries, data))
}
