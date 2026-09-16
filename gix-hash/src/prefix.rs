use std::cmp::Ordering;

use crate::{ChangeId, ObjectId, Prefix, change_id::ReverseHexDisplay, oid};

/// The error returned by [`Prefix::new()`].
pub type Error = gix_error::ValidationError;

///
pub mod from_hex {
    /// The error returned by [`Prefix::from_hex`][super::Prefix::from_hex()].
    pub type Error = gix_error::ValidationError;
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
            Err(Error::new(format!(
                "An object of kind {} cannot be larger than {} in hex, but {hex_len} was requested",
                id.kind(),
                id.kind().len_in_hex()
            )))
        } else if hex_len < Self::MIN_HEX_LEN {
            Err(Error::new(format!(
                "The minimum hex length of a short object id is {}, got {hex_len}",
                Self::MIN_HEX_LEN
            )))
        } else {
            let mut prefix = ObjectId::null(id.kind());
            let b = prefix.as_mut_slice();
            let copy_len = hex_len.div_ceil(2);
            b[..copy_len].copy_from_slice(&id.as_bytes()[..copy_len]);
            if hex_len % 2 == 1 {
                b[hex_len / 2] &= 0xf0;
            }

            Ok(Prefix { bytes: prefix, hex_len })
        }
    }

    /// Write this prefix into `buf` as lowercase hexadecimal characters and return the initialized portion.
    ///
    /// # Panics
    ///
    /// If `buf` is shorter than [`Self::hex_len()`].
    #[inline]
    #[must_use]
    pub fn hex_to_buf<'a>(&self, buf: &'a mut [u8]) -> &'a mut str {
        let complete_bytes = self.hex_len / 2;
        let complete_hex_len = complete_bytes * 2;
        if complete_bytes != 0 {
            faster_hex::hex_encode(&self.bytes.as_bytes()[..complete_bytes], &mut buf[..complete_hex_len])
                .expect("buffer size was checked before encoding");
        }
        if self.hex_len % 2 == 1 {
            const HEX: &[u8; 16] = b"0123456789abcdef";
            buf[complete_hex_len] = HEX[usize::from(self.bytes.as_bytes()[complete_bytes] >> 4)];
        }
        std::str::from_utf8_mut(&mut buf[..self.hex_len]).expect("hexadecimal object IDs are valid UTF-8")
    }

    /// Write this prefix to `out` as lowercase hexadecimal characters.
    #[inline]
    pub fn write_hex_to(&self, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        let mut buf = crate::Kind::hex_buf();
        out.write_all(self.hex_to_buf(&mut buf).as_bytes())
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
    /// Note that the minimum hex length is `4` - use [`Self::from_hex_nonempty()`].
    pub fn from_hex(value: &str) -> Result<Self, from_hex::Error> {
        let hex_len = value.len();
        if hex_len < Self::MIN_HEX_LEN {
            return Err(from_hex::Error::new(format!(
                "The minimum hex length of a short object id is {}, got {hex_len}",
                Self::MIN_HEX_LEN
            )));
        }
        Self::from_hex_nonempty(value)
    }

    /// Create an instance from the given hexadecimal prefix `value`, e.g. `35e` would yield a `Prefix` with `hex_len()` = 3.
    /// Note that this function supports all non-empty hex input - for a more typical implementation, use [`Self::from_hex()`].
    pub fn from_hex_nonempty(value: &str) -> Result<Self, from_hex::Error> {
        let hex_len = value.len();

        if hex_len > crate::Kind::longest().len_in_hex() {
            return Err(from_hex::Error::new(format!(
                "An id cannot be larger than {} chars in hex, but {hex_len} was requested",
                crate::Kind::longest().len_in_hex()
            )));
        } else if hex_len == 0 {
            return Err(from_hex::Error::new(format!(
                "The minimum hex length of a short object id is {}, got {hex_len}",
                Self::MIN_HEX_LEN
            )));
        }

        let kind = crate::Kind::from_hex_len(hex_len).expect("hex-len is already checked");
        let mut bytes = ObjectId::null(kind);
        let dst = &mut bytes.as_mut_slice()[..hex_len.div_ceil(2)];
        let decode_result = if hex_len.is_multiple_of(2) {
            faster_hex::hex_decode(value.as_bytes(), dst)
        } else {
            let mut hex = crate::Kind::hex_buf();
            hex[..hex_len].copy_from_slice(value.as_bytes());
            hex[hex_len] = b'0';
            faster_hex::hex_decode(&hex[..=hex_len], dst)
        };
        decode_result.map_err(|e| match e {
            faster_hex::Error::InvalidChar | faster_hex::Error::Overflow => {
                from_hex::Error::new("Invalid hex character")
            }
            faster_hex::Error::InvalidLength(_) => panic!("This is already checked"),
        })?;

        Ok(Prefix { bytes, hex_len })
    }

    /// Create an instance from a reverse-hex prefix, requiring at least [`Self::MIN_HEX_LEN`] characters.
    pub fn from_reverse_hex(value: &str) -> Result<Self, from_hex::Error> {
        let hex_len = value.len();
        if hex_len < Self::MIN_HEX_LEN {
            return Err(from_hex::Error::new(format!(
                "The minimum hex length of a short object id is {}, got {hex_len}",
                Self::MIN_HEX_LEN
            )));
        }
        Self::from_reverse_hex_nonempty(value)
    }

    /// Create an instance from a non-empty prefix written with JJ's reverse-hex alphabet.
    pub fn from_reverse_hex_nonempty(value: &str) -> Result<Self, from_hex::Error> {
        let hex_len = value.len();
        if hex_len > crate::Kind::longest().len_in_hex() {
            return Err(from_hex::Error::new(format!(
                "An id cannot be larger than {} chars in hex, but {hex_len} was requested",
                crate::Kind::longest().len_in_hex()
            )));
        } else if hex_len == 0 {
            return Err(from_hex::Error::new(format!(
                "The minimum hex length of a short object id is {}, got {hex_len}",
                Self::MIN_HEX_LEN
            )));
        }

        let mut hex = crate::Kind::hex_buf();
        crate::change_id::reverse_hex_to_hex(value.as_bytes(), &mut hex[..hex_len])
            .map_err(|()| from_hex::Error::new("Invalid hex character"))?;
        let hex = std::str::from_utf8(&hex[..hex_len]).expect("translated reverse hex is always ASCII");
        Self::from_hex_nonempty(hex)
    }

    /// Return a type which displays this prefix in JJ-compatible reverse-hex notation.
    pub fn to_reverse_hex(&self) -> ReverseHexDisplay<'_> {
        ReverseHexDisplay::new(&self.bytes, self.hex_len)
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

impl Prefix {
    fn eq_str(&self, other: &str) -> bool {
        self.bytes.to_hex_with_len(self.hex_len).eq_str(other)
    }
}

// Keep this directional as the hash kind and unused suffix aren't uniquely identified by the displayed prefix.
impl_partial_eq_str_one_way!(Prefix);

impl From<ObjectId> for Prefix {
    fn from(oid: ObjectId) -> Self {
        Prefix {
            bytes: oid,
            hex_len: oid.kind().len_in_hex(),
        }
    }
}

impl From<ChangeId> for Prefix {
    fn from(change_id: ChangeId) -> Self {
        ObjectId::from(change_id).into()
    }
}
