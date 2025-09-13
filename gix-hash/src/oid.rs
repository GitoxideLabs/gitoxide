use std::hash;

use crate::{Kind, ObjectId, SIZE_OF_SHA1_DIGEST};

/// A borrowed reference to a hash identifying objects.
///
/// # Future Proofing
///
/// In case we wish to support multiple hashes with the same length we cannot discriminate
/// using the slice length anymore. To make that work, we will use the high bits of the
/// internal `bytes` slice length (a fat pointer, pointing to data and its length in bytes)
/// to encode additional information. Before accessing or returning the bytes, a new adjusted
/// slice will be constructed, while the high bits will be used to help resolving the
/// hash [`kind()`][oid::kind()].
/// We expect to have quite a few bits available for such 'conflict resolution' as most hashes aren't longer
/// than 64 bytes.
#[derive(PartialEq, Eq, Ord, PartialOrd)]
#[repr(transparent)]
#[allow(non_camel_case_types)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct oid {
    bytes: [u8],
}

// False positive:
// Using an automatic implementation of `Hash` for `oid` would lead to
// it attempting to hash the length of the slice first. On 32 bit systems
// this can lead to issues with the custom `gix_hashtable` `Hasher` implementation,
// and it currently ends up being discarded there anyway.
#[allow(clippy::derived_hash_with_manual_eq)]
impl hash::Hash for oid {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        state.write(self.as_bytes());
    }
}

/// A utility able to format itself with the given number of characters in hex.
#[derive(PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct HexDisplay<'a> {
    inner: &'a oid,
    hex_len: usize,
}

impl std::fmt::Display for HexDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut hex = Kind::hex_buf();
        let hex = self.inner.hex_to_buf(hex.as_mut());
        let max_len = hex.len();
        f.write_str(&hex[..self.hex_len.min(max_len)])
    }
}

impl std::fmt::Debug for oid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}({})",
            match self.kind() {
                Kind::Sha1 => "Sha1",
            },
            self.to_hex(),
        )
    }
}

/// The error returned when trying to convert a byte slice to an [`oid`] or [`ObjectId`]
#[allow(missing_docs)]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Cannot instantiate git hash from a digest of length {0}")]
    InvalidByteSliceLength(usize),
}

/// Conversion
impl oid {
    /// Try to create a shared object id from a slice of bytes representing a hash `digest`
    #[inline]
    pub fn try_from_bytes(digest: &[u8]) -> Result<&Self, Error> {
        match digest.len() {
            20 => Ok(
                #[allow(unsafe_code)]
                unsafe {
                    &*(digest as *const [u8] as *const oid)
                },
            ),
            len => Err(Error::InvalidByteSliceLength(len)),
        }
    }

    /// Create an OID from the input `value` slice without performing any safety check.
    /// Use only once sure that `value` is a hash of valid length.
    pub fn from_bytes_unchecked(value: &[u8]) -> &Self {
        Self::from_bytes(value)
    }

    /// Only from code that statically assures correct sizes using array conversions.
    pub(crate) fn from_bytes(value: &[u8]) -> &Self {
        #[allow(unsafe_code)]
        unsafe {
            &*(value as *const [u8] as *const oid)
        }
    }
}

/// Access
impl oid {
    /// The kind of hash used for this instance.
    #[inline]
    pub fn kind(&self) -> Kind {
        Kind::from_len_in_bytes(self.bytes.len())
    }

    /// The first byte of the hash, commonly used to partition a set of object ids.
    #[inline]
    pub fn first_byte(&self) -> u8 {
        self.bytes[0]
    }

    /// Interpret this object id as raw byte slice.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Return a type which can display itself in hexadecimal form with the `len` amount of characters.
    #[inline]
    pub fn to_hex_with_len(&self, len: usize) -> HexDisplay<'_> {
        HexDisplay {
            inner: self,
            hex_len: len,
        }
    }

    /// Return a type which displays this oid as hex in full.
    #[inline]
    pub fn to_hex(&self) -> HexDisplay<'_> {
        HexDisplay {
            inner: self,
            hex_len: self.bytes.len() * 2,
        }
    }

    /// Returns `true` if this hash consists of all null bytes.
    #[inline]
    #[doc(alias = "is_zero", alias = "git2")]
    pub fn is_null(&self) -> bool {
        match self.kind() {
            Kind::Sha1 => &self.bytes == oid::null_sha1().as_bytes(),
        }
    }

    /// Returns `true` if this hash is equal to an empty blob.
    #[inline]
    pub fn is_empty_blob(&self) -> bool {
        match self.kind() {
            Kind::Sha1 => &self.bytes == oid::empty_blob_sha1().as_bytes(),
        }
    }

    /// Returns `true` if this hash is equal to an empty tree.
    #[inline]
    pub fn is_empty_tree(&self) -> bool {
        match self.kind() {
            Kind::Sha1 => &self.bytes == oid::empty_tree_sha1().as_bytes(),
        }
    }
}

/// Sha1 specific methods
impl oid {
    /// Write ourselves to the `out` in hexadecimal notation, returning the hex-string ready for display.
    ///
    /// **Panics** if the buffer isn't big enough to hold twice as many bytes as the current binary size.
    #[inline]
    #[must_use]
    pub fn hex_to_buf<'a>(&self, buf: &'a mut [u8]) -> &'a mut str {
        let num_hex_bytes = self.bytes.len() * 2;
        faster_hex::hex_encode(&self.bytes, &mut buf[..num_hex_bytes]).expect("to count correctly")
    }

    /// Write ourselves to `out` in hexadecimal notation.
    #[inline]
    pub fn write_hex_to(&self, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        let mut hex = Kind::hex_buf();
        let hex_len = self.hex_to_buf(&mut hex).len();
        out.write_all(&hex[..hex_len])
    }

    /// Returns a Sha1 digest with all bytes being initialized to zero.
    #[inline]
    pub(crate) fn null_sha1() -> &'static Self {
        oid::from_bytes([0u8; SIZE_OF_SHA1_DIGEST].as_ref())
    }

    /// Returns an oid representing the hash of an empty blob.
    #[inline]
    pub(crate) fn empty_blob_sha1() -> &'static Self {
        oid::from_bytes(b"\xe6\x9d\xe2\x9b\xb2\xd1\xd6\x43\x4b\x8b\x29\xae\x77\x5a\xd8\xc2\xe4\x8c\x53\x91")
    }

    /// Returns an oid representing the hash of an empty tree.
    #[inline]
    pub(crate) fn empty_tree_sha1() -> &'static Self {
        oid::from_bytes(b"\x4b\x82\x5d\xc6\x42\xcb\x6e\xb9\xa0\x60\xe5\x4b\xf8\xd6\x92\x88\xfb\xee\x49\x04")
    }
}

impl AsRef<oid> for &oid {
    fn as_ref(&self) -> &oid {
        self
    }
}

impl<'a> TryFrom<&'a [u8]> for &'a oid {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        oid::try_from_bytes(value)
    }
}

impl ToOwned for oid {
    type Owned = ObjectId;

    fn to_owned(&self) -> Self::Owned {
        match self.kind() {
            Kind::Sha1 => ObjectId::Sha1(self.bytes.try_into().expect("no bug in hash detection")),
        }
    }
}

impl<'a> From<&'a [u8; SIZE_OF_SHA1_DIGEST]> for &'a oid {
    fn from(v: &'a [u8; SIZE_OF_SHA1_DIGEST]) -> Self {
        oid::from_bytes(v.as_ref())
    }
}

impl std::fmt::Display for &oid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buf = Kind::hex_buf();
        f.write_str(self.hex_to_buf(&mut buf))
    }
}

impl PartialEq<ObjectId> for &oid {
    fn eq(&self, other: &ObjectId) -> bool {
        *self == other.as_ref()
    }
}

/// Manually created from a version that uses a slice, and we forcefully try to convert it into a borrowed array of the desired size
/// Could be improved by fitting this into serde.
/// Unfortunately the `serde::Deserialize` derive wouldn't work for borrowed arrays.
#[cfg(feature = "serde")]
impl<'de: 'a, 'a> serde::Deserialize<'de> for &'a oid {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as serde::Deserializer<'de>>::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct __Visitor<'de: 'a, 'a> {
            marker: std::marker::PhantomData<&'a oid>,
            lifetime: std::marker::PhantomData<&'de ()>,
        }
        impl<'de: 'a, 'a> serde::de::Visitor<'de> for __Visitor<'de, 'a> {
            type Value = &'a oid;
            fn expecting(&self, __formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Formatter::write_str(__formatter, "tuple struct Digest")
            }
            #[inline]
            fn visit_newtype_struct<__E>(self, __e: __E) -> std::result::Result<Self::Value, __E::Error>
            where
                __E: serde::Deserializer<'de>,
            {
                let __field0: &'a [u8] = match <&'a [u8] as serde::Deserialize>::deserialize(__e) {
                    Ok(__val) => __val,
                    Err(__err) => {
                        return Err(__err);
                    }
                };
                Ok(oid::try_from_bytes(__field0).expect("hash of known length"))
            }
            #[inline]
            fn visit_seq<__A>(self, mut __seq: __A) -> std::result::Result<Self::Value, __A::Error>
            where
                __A: serde::de::SeqAccess<'de>,
            {
                let __field0 = match match serde::de::SeqAccess::next_element::<&'a [u8]>(&mut __seq) {
                    Ok(__val) => __val,
                    Err(__err) => {
                        return Err(__err);
                    }
                } {
                    Some(__value) => __value,
                    None => {
                        return Err(serde::de::Error::invalid_length(
                            0usize,
                            &"tuple struct Digest with 1 element",
                        ));
                    }
                };
                Ok(oid::try_from_bytes(__field0).expect("hash of known length"))
            }
        }
        serde::Deserializer::deserialize_newtype_struct(
            deserializer,
            "Digest",
            __Visitor {
                marker: std::marker::PhantomData::<&'a oid>,
                lifetime: std::marker::PhantomData,
            },
        )
    }
}
