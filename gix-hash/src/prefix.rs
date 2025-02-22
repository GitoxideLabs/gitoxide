use std::cmp::Ordering;

use crate::{oid, ObjectId, Prefix};

/// The error returned by [`Prefix::new()`].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(
        "The minimum hex length of a short object id is {}, got {hex_len}",
        Prefix::MIN_HEX_LEN
    )]
    TooShort { hex_len: usize },
    #[error("An object of kind {object_kind} cannot be larger than {} in hex, but {hex_len} was requested", object_kind.len_in_hex())]
    TooLong { object_kind: crate::Kind, hex_len: usize },
}

///
pub mod from_hex {
    /// The error returned by [`Prefix::from_hex`][super::Prefix::from_hex()].
    #[derive(Debug, Eq, PartialEq, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(
            "The minimum hex length of a short object id is {}, got {hex_len}",
            super::Prefix::MIN_HEX_LEN
        )]
        TooShort { hex_len: usize },
        #[error("An id cannot be larger than {} chars in hex, but {hex_len} was requested", crate::Kind::longest().len_in_hex())]
        TooLong { hex_len: usize },
        #[error("Invalid hex character")]
        Invalid,
    }
}

impl Prefix {
    /// The smallest allowed prefix length below which chances for collisions are too high even in small repositories.
    pub const MIN_HEX_LEN: usize = 4;

    /// Create a new instance by taking a full `id` as input and truncating it to `hex_len`.
    ///
    /// For instance, with `hex_len` of 7 the resulting prefix is 3.5 bytes, or 3 bytes and 4 bits
    /// wide, with all other bytes and bits set to zero.
    pub fn new(id: &oid, hex_len: usize) -> Result<Self, Error> {
        if hex_len > id.kind().len_in_hex() {
            Err(Error::TooLong {
                object_kind: id.kind(),
                hex_len,
            })
        } else if hex_len < Self::MIN_HEX_LEN {
            Err(Error::TooShort { hex_len })
        } else {
            let mut prefix = ObjectId::null(id.kind());
            let b = prefix.as_mut_slice();
            let copy_len = (hex_len + 1) / 2;
            b[..copy_len].copy_from_slice(&id.as_bytes()[..copy_len]);
            if hex_len % 2 == 1 {
                b[hex_len / 2] &= 0xf0;
            }

            Ok(Prefix { bytes: prefix, hex_len })
        }
    }

    /// Returns the prefix as object id.
    ///
    /// Note that it may be deceptive to use given that it looks like a full
    /// object id, even though its post-prefix bytes/bits are set to zero.
    pub fn as_oid(&self) -> &oid {
        &self.bytes
    }

    /// Return the amount of hexadecimal characters that are set in the prefix.
    ///
    /// This gives the prefix a granularity of 4 bits.
    pub fn hex_len(&self) -> usize {
        self.hex_len
    }

    /// Provided with candidate id which is a full hash, determine how this prefix compares to it,
    /// only looking at the prefix bytes, ignoring everything behind that.
    pub fn cmp_oid(&self, candidate: &oid) -> Ordering {
        let common_len = self.hex_len / 2;

        self.bytes.as_bytes()[..common_len]
            .cmp(&candidate.as_bytes()[..common_len])
            .then(if self.hex_len % 2 == 1 {
                let half_byte_idx = self.hex_len / 2;
                self.bytes.as_bytes()[half_byte_idx].cmp(&(candidate.as_bytes()[half_byte_idx] & 0xf0))
            } else {
                Ordering::Equal
            })
    }

    /// Create an instance from the given hexadecimal prefix `value`, e.g. `35e77c16` would yield a `Prefix` with `hex_len()` = 8.
    pub fn from_hex(value: &str) -> Result<Self, from_hex::Error> {
        let hex_len = value.len();

        if hex_len > crate::Kind::longest().len_in_hex() {
            return Err(from_hex::Error::TooLong { hex_len });
        } else if hex_len < Self::MIN_HEX_LEN {
            return Err(from_hex::Error::TooShort { hex_len });
        }

        let src = if value.len() % 2 == 0 {
            let mut out = Vec::from_iter(std::iter::repeat(0).take(value.len() / 2));
            faster_hex::hex_decode(value.as_bytes(), &mut out).map(move |_| out)
        } else {
            // TODO(perf): do without heap allocation here.
            let mut buf = [0u8; crate::Kind::longest().len_in_hex()];
            buf[..value.len()].copy_from_slice(value.as_bytes());
            buf[value.len()] = b'0';
            let src = &buf[..=value.len()];
            let mut out = Vec::from_iter(std::iter::repeat(0).take(src.len() / 2));
            faster_hex::hex_decode(src, &mut out).map(move |_| out)
        }
        .map_err(|e| match e {
            faster_hex::Error::InvalidChar | faster_hex::Error::Overflow => from_hex::Error::Invalid,
            faster_hex::Error::InvalidLength(_) => panic!("This is already checked"),
        })?;

        let mut bytes = ObjectId::null(crate::Kind::from_hex_len(value.len()).expect("hex-len is already checked"));
        let dst = bytes.as_mut_slice();
        let copy_len = src.len();
        dst[..copy_len].copy_from_slice(&src);

        Ok(Prefix { bytes, hex_len })
    }
}

/// Create an instance from the given hexadecimal prefix, e.g. `35e77c16` would yield a `Prefix`
/// with `hex_len()` = 8.
impl TryFrom<&str> for Prefix {
    type Error = from_hex::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Prefix::from_hex(value)
    }
}

impl std::fmt::Display for Prefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.bytes.to_hex_with_len(self.hex_len).fmt(f)
    }
}

impl From<ObjectId> for Prefix {
    fn from(oid: ObjectId) -> Self {
        Prefix {
            bytes: oid,
            hex_len: oid.kind().len_in_hex(),
        }
    }
}
