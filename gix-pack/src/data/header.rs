use crate::data;
use gix_error::ErrorExt;

pub(crate) const N32_SIZE: usize = std::mem::size_of::<u32>();

/// Parses the first 12 bytes of a pack file, returning the pack version as well as the number of objects contained in the pack.
pub fn decode(data: &[u8; 12]) -> Result<(data::Version, u32), decode::Error> {
    let mut ofs = 0;
    if &data[ofs..ofs + b"PACK".len()] != b"PACK" {
        return Err(gix_error::CorruptionError::new("Pack data type not recognized").raise_erased());
    }
    ofs += N32_SIZE;
    let kind = match crate::read_u32(&data[ofs..ofs + N32_SIZE]) {
        2 => data::Version::V2,
        3 => data::Version::V3,
        v => return Err(gix_error::ValidationError::new(format!("Unsupported pack version: {v}")).raise_erased()),
    };
    ofs += N32_SIZE;
    let num_objects = crate::read_u32(&data[ofs..ofs + N32_SIZE]);

    Ok((kind, num_objects))
}

/// Write a pack data header at `version` with `num_objects` and return a buffer.
pub fn encode(version: data::Version, num_objects: u32) -> [u8; 12] {
    use crate::data::Version::*;
    let mut buf = [0u8; 12];
    buf[..4].copy_from_slice(b"PACK");
    buf[4..8].copy_from_slice(
        &match version {
            V2 => 2u32,
            V3 => 3,
        }
        .to_be_bytes()[..],
    );
    buf[8..].copy_from_slice(&num_objects.to_be_bytes()[..]);
    buf
}

///
pub mod decode {
    /// Returned by [`decode()`][super::decode()].
    pub type Error = gix_error::Exn;
}
