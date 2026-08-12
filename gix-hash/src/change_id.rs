use std::{borrow::Borrow, ops::Deref, str::FromStr};

use crate::{ChangeId, Kind, ObjectId, oid};

const REVERSE_HEX: &[u8; 16] = b"zyxwvutsrqponmlk";

impl ChangeId {
    /// Parse a complete SHA-1 or SHA-256 hash written with reverse-hex alphabet.
    pub fn from_reverse_hex(buffer: &[u8]) -> Result<Self, crate::decode::Error> {
        let len = buffer.len();
        if crate::Kind::from_hex_len(len).is_none_or(|kind| kind.len_in_hex() != len) {
            return Err(crate::decode::Error::InvalidHexEncodingLength(len));
        }

        let mut hex = Kind::hex_buf();
        reverse_hex_to_hex(buffer, &mut hex[..len]).map_err(|()| crate::decode::Error::Invalid)?;
        ObjectId::from_hex(&hex[..len]).map(ChangeId)
    }

    /// Return a type which displays this ID in reverse-hex notation.
    pub fn to_reverse_hex(&self) -> ReverseHexDisplay<'_> {
        ReverseHexDisplay::new(self, self.kind().len_in_hex())
    }

    /// Return a type which displays at most `len` characters of this ID in reverse-hex notation.
    pub fn to_reverse_hex_with_len(&self, len: usize) -> ReverseHexDisplay<'_> {
        ReverseHexDisplay::new(self, len)
    }
}

/// A utility able to format a [`ChangeId`] or [`crate::Prefix`] with a given number of reverse-hex characters.
#[derive(PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct ReverseHexDisplay<'a> {
    inner: &'a oid,
    hex_len: usize,
}

impl FromStr for ChangeId {
    type Err = crate::decode::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::from_reverse_hex(value.as_bytes())
    }
}

impl From<ObjectId> for ChangeId {
    fn from(value: ObjectId) -> Self {
        ChangeId(value)
    }
}

impl From<&oid> for ChangeId {
    fn from(value: &oid) -> Self {
        ChangeId(value.into())
    }
}

impl From<ChangeId> for ObjectId {
    fn from(value: ChangeId) -> Self {
        value.0
    }
}

impl Deref for ChangeId {
    type Target = oid;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl AsRef<oid> for ChangeId {
    fn as_ref(&self) -> &oid {
        self
    }
}

impl Borrow<oid> for ChangeId {
    fn borrow(&self) -> &oid {
        self
    }
}

impl std::fmt::Display for ChangeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_reverse_hex().fmt(f)
    }
}

impl ReverseHexDisplay<'_> {
    pub(crate) fn new(inner: &oid, hex_len: usize) -> ReverseHexDisplay<'_> {
        ReverseHexDisplay { inner, hex_len }
    }
}

impl std::fmt::Display for ReverseHexDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buf = Kind::hex_buf();
        let reverse_hex = encode_reverse_hex(self.inner, &mut buf);
        f.write_str(&reverse_hex[..self.hex_len.min(reverse_hex.len())])
    }
}

pub(crate) fn reverse_hex_to_hex(reverse_hex: &[u8], hex: &mut [u8]) -> Result<(), ()> {
    for (src, dst) in reverse_hex.iter().zip(hex) {
        let nibble = match src {
            b'k'..=b'z' => b'z' - src,
            b'K'..=b'Z' => b'Z' - src,
            _ => return Err(()),
        };
        *dst = b"0123456789abcdef"[usize::from(nibble)];
    }
    Ok(())
}

fn encode_reverse_hex<'a>(id: &oid, buf: &'a mut [u8]) -> &'a str {
    for (byte, pair) in id.as_bytes().iter().zip(buf.chunks_exact_mut(2)) {
        pair[0] = REVERSE_HEX[usize::from(byte >> 4)];
        pair[1] = REVERSE_HEX[usize::from(byte & 0x0f)];
    }
    let len = id.kind().len_in_hex();
    std::str::from_utf8(&buf[..len]).expect("reverse hex is always ASCII")
}
