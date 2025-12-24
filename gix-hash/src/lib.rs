//! This crate provides types for identifying git objects using a hash digest.
//!
//! These are provided in [borrowed versions][oid] as well as an [owned one][ObjectId].
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs, rust_2018_idioms, unsafe_code)]

// Remove this once other hashes (e.g., SHA256, and potentially others)
// are supported, and this crate can build without [`ObjectId::Sha1`].
#[cfg(not(feature = "sha1"))]
compile_error!("Please set the `sha1` feature flag");

#[path = "oid.rs"]
mod borrowed;
pub use borrowed::{oid, Error};

/// Hash functions and hash utilities
pub mod hasher;
pub use hasher::_impl::{hasher, Hasher};

/// Error types for utility hash functions
pub mod io;
pub use io::_impl::{bytes, bytes_of_file, bytes_with_hasher};

mod object_id;
pub use object_id::{decode, ObjectId};

///
pub mod prefix;

///
pub mod verify;

/// A partial, owned hash possibly identifying an object uniquely, whose non-prefix bytes are zeroed.
///
/// An example would `0000000000000000000000000000000032bd3242`, where `32bd3242` is the prefix,
/// which would be able to match all hashes that *start with* `32bd3242`.
#[derive(PartialEq, Eq, Hash, Ord, PartialOrd, Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Prefix {
    bytes: ObjectId,
    hex_len: usize,
}

/// The size of a SHA1 hash digest in bytes.
const SIZE_OF_SHA1_DIGEST: usize = 20;
/// The size of a SHA1 hash digest in hex.
const SIZE_OF_SHA1_HEX_DIGEST: usize = 2 * SIZE_OF_SHA1_DIGEST;

/// The size of a SHA256 hash digest in bytes.
#[cfg(feature = "sha256")]
const SIZE_OF_SHA256_DIGEST: usize = 32;
/// The size of a SHA256 hash digest in hex.
#[cfg(feature = "sha256")]
const SIZE_OF_SHA256_HEX_DIGEST: usize = 2 * SIZE_OF_SHA256_DIGEST;

const EMPTY_BLOB_SHA1: &[u8; SIZE_OF_SHA1_DIGEST] =
    b"\xe6\x9d\xe2\x9b\xb2\xd1\xd6\x43\x4b\x8b\x29\xae\x77\x5a\xd8\xc2\xe4\x8c\x53\x91";
const EMPTY_TREE_SHA1: &[u8; SIZE_OF_SHA1_DIGEST] =
    b"\x4b\x82\x5d\xc6\x42\xcb\x6e\xb9\xa0\x60\xe5\x4b\xf8\xd6\x92\x88\xfb\xee\x49\x04";

#[cfg(feature = "sha256")]
const EMPTY_BLOB_SHA256: &[u8; SIZE_OF_SHA256_DIGEST] = b"\x47\x3a\x0f\x4c\x3b\xe8\xa9\x36\x81\xa2\x67\xe3\xb1\xe9\xa7\xdc\xda\x11\x85\x43\x6f\xe1\x41\xf7\x74\x91\x20\xa3\x03\x72\x18\x13";
#[cfg(feature = "sha256")]
const EMPTY_TREE_SHA256: &[u8; SIZE_OF_SHA256_DIGEST] = b"\x6e\xf1\x9b\x41\x22\x5c\x53\x69\xf1\xc1\x04\xd4\x5d\x8d\x85\xef\xa9\xb0\x57\xb5\x3b\x14\xb4\xb9\xb9\x39\xdd\x74\xde\xcc\x53\x21";

/// Denotes the kind of function to produce a [`ObjectId`].
#[derive(Default, PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum Kind {
    /// The SHA1 hash with 160 bits.
    #[default]
    Sha1 = 1,
    /// The SHA256 hash with 256 bits.
    #[cfg(feature = "sha256")]
    Sha256 = 2,
}

mod kind;
