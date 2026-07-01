use crate::error::Error;

/// Hash identifiers used by reftable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum HashId {
    /// SHA-1 object IDs.
    Sha1,
    /// SHA-256 object IDs.
    Sha256,
}

impl HashId {
    /// Return the byte-size of object IDs for this hash.
    pub const fn size(self) -> usize {
        match self {
            HashId::Sha1 => 20,
            HashId::Sha256 => 32,
        }
    }

    /// Return the [gix_hash::Kind] if this hash ID is supported by `gix-hash`.
    pub const fn to_gix(self) -> gix_hash::Kind {
        match self {
            HashId::Sha1 => gix_hash::Kind::Sha1,
            HashId::Sha256 => gix_hash::Kind::Sha256,
        }
    }
}

/// Return the shared-prefix size between `a` and `b`.
pub fn common_prefix_size(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b.iter()).take_while(|(a, b)| a == b).count()
}

/// Put a big-endian 64-bit integer into `out`.
pub fn put_be64(out: &mut [u8; 8], value: u64) {
    *out = value.to_be_bytes();
}

/// Put a big-endian 32-bit integer into `out`.
pub fn put_be32(out: &mut [u8; 4], value: u32) {
    *out = value.to_be_bytes();
}

/// Put a big-endian 24-bit integer into `out`.
pub fn put_be24(out: &mut [u8; 3], value: u32) {
    out[0] = ((value >> 16) & 0xff) as u8;
    out[1] = ((value >> 8) & 0xff) as u8;
    out[2] = (value & 0xff) as u8;
}

/// Put a big-endian 16-bit integer into `out`.
pub fn put_be16(out: &mut [u8; 2], value: u16) {
    *out = value.to_be_bytes();
}

/// Read a big-endian 64-bit integer.
pub fn get_be64(input: &[u8; 8]) -> u64 {
    u64::from_be_bytes(*input)
}

/// Read a big-endian 32-bit integer.
pub fn get_be32(input: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*input)
}

/// Read a big-endian 24-bit integer.
pub fn get_be24(input: &[u8; 3]) -> u32 {
    ((input[0] as u32) << 16) | ((input[1] as u32) << 8) | (input[2] as u32)
}

/// Read a big-endian 16-bit integer.
pub fn get_be16(input: &[u8; 2]) -> u16 {
    u16::from_be_bytes(*input)
}

/// Encode a reftable varint.
///
/// The format is the same as reftable's/ofs-delta's encoding.
pub fn encode_varint(mut value: u64, out: &mut [u8; 10]) -> usize {
    let mut tmp = [0u8; 10];
    let mut n = 0usize;
    tmp[n] = (value & 0x7f) as u8;
    n += 1;
    while value >= 0x80 {
        value = (value >> 7) - 1;
        tmp[n] = 0x80 | (value & 0x7f) as u8;
        n += 1;
    }
    // reverse
    for (dst, src) in out.iter_mut().take(n).zip(tmp[..n].iter().rev()) {
        *dst = *src;
    }
    n
}

/// Decode a reftable varint from `input`.
///
/// Returns `(value, consumed_bytes)`.
pub fn decode_varint(input: &[u8]) -> Result<(u64, usize), Error> {
    if input.is_empty() {
        return Err(Error::Truncated);
    }
    let mut i = 0usize;
    let mut c = input[i];
    i += 1;
    let mut value = u64::from(c & 0x7f);
    while c & 0x80 != 0 {
        if i >= input.len() {
            return Err(Error::Truncated);
        }
        c = input[i];
        i += 1;
        value = value
            .checked_add(1)
            .ok_or(Error::VarintOverflow)?
            .checked_shl(7)
            .ok_or(Error::VarintOverflow)?
            .checked_add(u64::from(c & 0x7f))
            .ok_or(Error::VarintOverflow)?;
    }
    Ok((value, i))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_sizes() {
        assert_eq!(HashId::Sha1.size(), 20);
        assert_eq!(HashId::Sha256.size(), 32);
    }

    #[test]
    fn common_prefix() {
        assert_eq!(common_prefix_size(b"refs/heads/a", b"refs/heads/b"), 11);
        assert_eq!(common_prefix_size(b"x", b"y"), 0);
        assert_eq!(common_prefix_size(b"", b"abc"), 0);
    }

    #[test]
    fn be_roundtrip() {
        let mut be64 = [0u8; 8];
        put_be64(&mut be64, 0x0102_0304_0506_0708);
        assert_eq!(get_be64(&be64), 0x0102_0304_0506_0708);

        let mut be32 = [0u8; 4];
        put_be32(&mut be32, 0x0102_0304);
        assert_eq!(get_be32(&be32), 0x0102_0304);

        let mut be24 = [0u8; 3];
        put_be24(&mut be24, 0x01_02_03);
        assert_eq!(get_be24(&be24), 0x01_02_03);

        let mut be16 = [0u8; 2];
        put_be16(&mut be16, 0x0102);
        assert_eq!(get_be16(&be16), 0x0102);
    }

    #[test]
    fn varint_roundtrip() {
        let mut storage = [0u8; 10];
        for value in [0, 1, 2, 126, 127, 128, 129, 16_384, u32::MAX as u64, u64::MAX] {
            let n = encode_varint(value, &mut storage);
            let (decoded, consumed) = decode_varint(&storage[..n]).expect("valid");
            assert_eq!(consumed, n);
            assert_eq!(decoded, value);
        }
    }
}
