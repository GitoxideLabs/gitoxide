use std::io::Write;

use crate::data::{entry::Header, input};

impl input::Entry {
    /// Create a new input entry from a given data `obj` set to be placed at the given `pack_offset`.
    ///
    /// This method is useful when arbitrary base entries are created
    pub fn from_data_obj(obj: &gix_object::Data<'_>, pack_offset: u64) -> Result<Self, input::Error> {
        let header = to_header(obj.kind);
        let compressed = compress_data(obj)?;
        let compressed_size = compressed.len() as u64;
        let mut entry = input::Entry {
            header,
            header_size: header.size(obj.data.len() as u64) as u16,
            pack_offset,
            compressed: Some(compressed),
            compressed_size,
            crc32: None,
            decompressed_size: obj.data.len() as u64,
            trailer: None,
        };
        entry.crc32 = Some(entry.compute_crc32());
        Ok(entry)
    }
    /// The amount of bytes this entry may consume in a pack data file
    pub fn bytes_in_pack(&self) -> u64 {
        u64::from(self.header_size) + self.compressed_size
    }

    /// Update our CRC value by recalculating it from our header and compressed data.
    pub fn compute_crc32(&self) -> u32 {
        let mut header_buf = [0u8; 12 + gix_hash::Kind::longest().len_in_bytes()];
        let header_len = self
            .header
            .write_to(self.decompressed_size, &mut header_buf.as_mut())
            .expect("write to memory will not fail");
        let state = gix_features::hash::crc32_update(0, &header_buf[..header_len]);
        gix_features::hash::crc32_update(state, self.compressed.as_ref().expect("we always set it"))
    }
}

fn to_header(kind: gix_object::Kind) -> Header {
    use gix_object::Kind::*;
    match kind {
        Tree => Header::Tree,
        Blob => Header::Blob,
        Commit => Header::Commit,
        Tag => Header::Tag,
    }
}

fn compress_data(obj: &gix_object::Data<'_>) -> Result<Vec<u8>, input::Error> {
    let mut out = gix_features::zlib::stream::deflate::Write::new(Vec::new());
    if let Err(err) = std::io::copy(&mut &*obj.data, &mut out) {
        match err.kind() {
            std::io::ErrorKind::Other => return Err(input::Error::Io(err)),
            err => {
                unreachable!("Should never see other errors than zlib, but got {:?}", err,)
            }
        }
    }
    out.flush().expect("zlib flush should never fail");
    Ok(out.into_inner())
}
