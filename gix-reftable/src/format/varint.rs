//! Git offset-varints, as used throughout the reftable format.

/// An error returned while decoding an offset-varint.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The input ended before the integer did.
    #[error("the offset-varint is truncated")]
    Truncated,
    /// The encoded value cannot be represented by a `u64`.
    #[error("the offset-varint exceeds 64 bits")]
    Overflow,
}

/// Append `value` in Git's offset-varint representation to `out`.
pub fn encode(mut value: u64, out: &mut Vec<u8>) {
    let mut bytes = [0u8; 10];
    let mut cursor = bytes.len();
    cursor -= 1;
    bytes[cursor] = (value & 0x7f) as u8;
    while value > 0x7f {
        value = (value >> 7) - 1;
        cursor -= 1;
        bytes[cursor] = ((value & 0x7f) as u8) | 0x80;
    }
    out.extend_from_slice(&bytes[cursor..]);
}

/// Decode one Git offset-varint and return its value and byte length.
pub fn decode(input: &[u8]) -> Result<(u64, usize), Error> {
    let first = *input.first().ok_or(Error::Truncated)?;
    let mut value = u64::from(first & 0x7f);
    let mut consumed = 1;
    let mut byte = first;
    while byte & 0x80 != 0 {
        byte = *input.get(consumed).ok_or(Error::Truncated)?;
        consumed += 1;
        value = value
            .checked_add(1)
            .and_then(|value| value.checked_mul(128))
            .and_then(|value| value.checked_add(u64::from(byte & 0x7f)))
            .ok_or(Error::Overflow)?;
    }
    Ok((value, consumed))
}
