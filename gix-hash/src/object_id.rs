use std::{
    borrow::Borrow,
    hash::{Hash, Hasher},
    ops::Deref,
};

use crate::{borrowed::oid, Kind, EMPTY_BLOB_SHA1, EMPTY_TREE_SHA1, SIZE_OF_SHA1_DIGEST};

#[cfg(feature = "sha256")]
use crate::{EMPTY_BLOB_SHA256, EMPTY_TREE_SHA256, SIZE_OF_SHA256_DIGEST};

/// An owned hash identifying objects, most commonly `Sha1`
#[derive(PartialEq, Eq, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum ObjectId {
    /// A SHA1 hash digest
    Sha1([u8; SIZE_OF_SHA1_DIGEST]),
    /// A SHA256 hash digest
    #[cfg(feature = "sha256")]
    Sha256([u8; SIZE_OF_SHA256_DIGEST]),
}

// False positive: https://github.com/rust-lang/rust-clippy/issues/2627
// ignoring some fields while hashing is perfectly valid and just leads to
// increased HashCollisions. One SHA1 being a prefix of another SHA256 is
// extremely unlikely to begin with so it doesn't matter.
// This implementation matches the `Hash` implementation for `oid`
// and allows the usage of custom Hashers that only copy a truncated ShaHash
#[allow(clippy::derived_hash_with_manual_eq)]
impl Hash for ObjectId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(self.as_slice());
    }
}

#[allow(missing_docs)]
pub mod decode {
    use std::str::FromStr;

    use crate::{object_id::ObjectId, SIZE_OF_SHA1_DIGEST, SIZE_OF_SHA1_HEX_DIGEST};

    #[cfg(feature = "sha256")]
    use crate::{SIZE_OF_SHA256_DIGEST, SIZE_OF_SHA256_HEX_DIGEST};

    /// An error returned by [`ObjectId::from_hex()`][crate::ObjectId::from_hex()]
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("A hash sized {0} hexadecimal characters is invalid")]
        InvalidHexEncodingLength(usize),
        #[error("Invalid character encountered")]
        Invalid,
    }

    /// Hash decoding
    impl ObjectId {
        /// Create an instance from a `buffer` of 40 bytes or 64 bytes encoded with hexadecimal
        /// notation. The former will be interpreted as SHA1 while the latter will be interpreted
        /// as SHA256 when it is enabled.
        ///
        /// Such a buffer can be obtained using [`oid::write_hex_to(buffer)`][super::oid::write_hex_to()]
        pub fn from_hex(buffer: &[u8]) -> Result<ObjectId, Error> {
            match buffer.len() {
                SIZE_OF_SHA1_HEX_DIGEST => Ok({
                    ObjectId::Sha1({
                        let mut buf = [0; SIZE_OF_SHA1_DIGEST];
                        faster_hex::hex_decode(buffer, &mut buf).map_err(|err| match err {
                            faster_hex::Error::InvalidChar | faster_hex::Error::Overflow => Error::Invalid,
                            faster_hex::Error::InvalidLength(_) => {
                                unreachable!("BUG: This is already checked")
                            }
                        })?;
                        buf
                    })
                }),
                #[cfg(feature = "sha256")]
                SIZE_OF_SHA256_HEX_DIGEST => Ok({
                    ObjectId::Sha256({
                        let mut buf = [0; SIZE_OF_SHA256_DIGEST];
                        faster_hex::hex_decode(buffer, &mut buf).map_err(|err| match err {
                            faster_hex::Error::InvalidChar | faster_hex::Error::Overflow => Error::Invalid,
                            faster_hex::Error::InvalidLength(_) => {
                                unreachable!("BUG: This is already checked")
                            }
                        })?;
                        buf
                    })
                }),
                len => Err(Error::InvalidHexEncodingLength(len)),
            }
        }
    }

    impl FromStr for ObjectId {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Self::from_hex(s.as_bytes())
        }
    }
}

/// Access and conversion
impl ObjectId {
    /// Returns the kind of hash used in this instance.
    #[inline]
    pub fn kind(&self) -> Kind {
        match self {
            ObjectId::Sha1(_) => Kind::Sha1,
            #[cfg(feature = "sha256")]
            ObjectId::Sha256(_) => Kind::Sha256,
        }
    }
    /// Return the raw byte slice representing this hash.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        match self {
            Self::Sha1(b) => b.as_ref(),
            #[cfg(feature = "sha256")]
            Self::Sha256(b) => b.as_ref(),
        }
    }
    /// Return the raw mutable byte slice representing this hash.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        match self {
            Self::Sha1(b) => b.as_mut(),
            #[cfg(feature = "sha256")]
            Self::Sha256(b) => b.as_mut(),
        }
    }

    /// The hash of an empty blob.
    #[inline]
    pub const fn empty_blob(hash: Kind) -> ObjectId {
        match hash {
            Kind::Sha1 => ObjectId::Sha1(*EMPTY_BLOB_SHA1),
            #[cfg(feature = "sha256")]
            Kind::Sha256 => ObjectId::Sha256(*EMPTY_BLOB_SHA256),
        }
    }

    /// The hash of an empty tree.
    #[inline]
    pub const fn empty_tree(hash: Kind) -> ObjectId {
        match hash {
            Kind::Sha1 => ObjectId::Sha1(*EMPTY_TREE_SHA1),
            #[cfg(feature = "sha256")]
            Kind::Sha256 => ObjectId::Sha256(*EMPTY_TREE_SHA256),
        }
    }

    /// Returns an instances whose bytes are all zero.
    #[inline]
    #[doc(alias = "zero", alias = "git2")]
    pub const fn null(kind: Kind) -> ObjectId {
        match kind {
            Kind::Sha1 => Self::null_sha1(),
            #[cfg(feature = "sha256")]
            Kind::Sha256 => Self::null_sha256(),
        }
    }

    /// Returns `true` if this hash consists of all null bytes.
    #[inline]
    #[doc(alias = "is_zero", alias = "git2")]
    pub fn is_null(&self) -> bool {
        match self {
            ObjectId::Sha1(digest) => &digest[..] == oid::null_sha1().as_bytes(),
            #[cfg(feature = "sha256")]
            ObjectId::Sha256(digest) => &digest[..] == oid::null_sha256().as_bytes(),
        }
    }

    /// Returns `true` if this hash is equal to an empty blob.
    #[inline]
    pub fn is_empty_blob(&self) -> bool {
        self == &Self::empty_blob(self.kind())
    }

    /// Returns `true` if this hash is equal to an empty tree.
    #[inline]
    pub fn is_empty_tree(&self) -> bool {
        self == &Self::empty_tree(self.kind())
    }
}

/// Lifecycle
impl ObjectId {
    /// Convert `bytes` into an owned object Id or panic if the slice length doesn't indicate a supported hash.
    ///
    /// Use `Self::try_from(bytes)` for a fallible version.
    pub fn from_bytes_or_panic(bytes: &[u8]) -> Self {
        match bytes.len() {
            SIZE_OF_SHA1_DIGEST => Self::Sha1(bytes.try_into().expect("prior length validation")),
            other => panic!("BUG: unsupported hash len: {other}"),
        }
    }
}

/// Methods related to SHA1 and SHA256
impl ObjectId {
    /// Instantiate an `ObjectId` from a 20 bytes SHA1 digest.
    #[inline]
    fn new_sha1(id: [u8; SIZE_OF_SHA1_DIGEST]) -> Self {
        ObjectId::Sha1(id)
    }

    /// Instantiate an `ObjectId` from a 32 bytes SHA256 digest.
    #[inline]
    #[cfg(feature = "sha256")]
    fn new_sha256(id: [u8; SIZE_OF_SHA256_DIGEST]) -> Self {
        ObjectId::Sha256(id)
    }

    /// Instantiate an `ObjectId` from a borrowed 20 bytes SHA1 digest.
    ///
    /// Panics if the slice doesn't have a length of 20.
    #[inline]
    pub(crate) fn from_20_bytes(b: &[u8]) -> ObjectId {
        let mut id = [0; SIZE_OF_SHA1_DIGEST];
        id.copy_from_slice(b);
        ObjectId::Sha1(id)
    }

    /// Instantiate an `ObjectId` from a borrowed 32 bytes SHA256 digest.
    ///
    /// Panics if the slice doesn't have a length of 32.
    #[inline]
    #[cfg(feature = "sha256")]
    pub(crate) fn from_32_bytes(b: &[u8]) -> ObjectId {
        let mut id = [0; SIZE_OF_SHA256_DIGEST];
        id.copy_from_slice(b);
        ObjectId::Sha256(id)
    }

    /// Returns an `ObjectId` representing a SHA1 whose memory is zeroed.
    #[inline]
    pub(crate) const fn null_sha1() -> ObjectId {
        ObjectId::Sha1([0u8; SIZE_OF_SHA1_DIGEST])
    }

    /// Returns an `ObjectId` representing a SHA256 whose memory is zeroed.
    #[inline]
    #[cfg(feature = "sha256")]
    pub(crate) const fn null_sha256() -> ObjectId {
        ObjectId::Sha256([0u8; SIZE_OF_SHA256_DIGEST])
    }
}

impl std::fmt::Debug for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectId::Sha1(_hash) => f.write_str("Sha1(")?,
            #[cfg(feature = "sha256")]
            ObjectId::Sha256(_) => f.write_str("Sha256(")?,
        }
        for b in self.as_bytes() {
            write!(f, "{b:02x}")?;
        }
        f.write_str(")")
    }
}

impl From<[u8; SIZE_OF_SHA1_DIGEST]> for ObjectId {
    fn from(v: [u8; SIZE_OF_SHA1_DIGEST]) -> Self {
        Self::new_sha1(v)
    }
}

#[cfg(feature = "sha256")]
impl From<[u8; SIZE_OF_SHA256_DIGEST]> for ObjectId {
    fn from(v: [u8; SIZE_OF_SHA256_DIGEST]) -> Self {
        Self::new_sha256(v)
    }
}

impl From<&oid> for ObjectId {
    fn from(v: &oid) -> Self {
        match v.kind() {
            Kind::Sha1 => ObjectId::from_20_bytes(v.as_bytes()),
            #[cfg(feature = "sha256")]
            Kind::Sha256 => ObjectId::from_32_bytes(v.as_bytes()),
        }
    }
}

impl TryFrom<&[u8]> for ObjectId {
    type Error = crate::Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Ok(oid::try_from_bytes(bytes)?.into())
    }
}

impl Deref for ObjectId {
    type Target = oid;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl AsRef<oid> for ObjectId {
    fn as_ref(&self) -> &oid {
        oid::from_bytes_unchecked(self.as_slice())
    }
}

impl Borrow<oid> for ObjectId {
    fn borrow(&self) -> &oid {
        self.as_ref()
    }
}

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl PartialEq<&oid> for ObjectId {
    fn eq(&self, other: &&oid) -> bool {
        self.as_ref() == *other
    }
}
