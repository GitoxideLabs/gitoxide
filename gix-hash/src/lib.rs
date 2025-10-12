//! This crate provides types for identifying git objects using a hash digest.
//!
//! These are provided in [borrowed versions][oid] as well as an [owned one][ObjectId].
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg, doc_auto_cfg))]
#![deny(missing_docs, rust_2018_idioms, unsafe_code)]

// Remove this once other hashes (e.g., SHA-256, and potentially others)
// are supported, and this crate can build without [`ObjectId::Sha1`].
#[cfg(not(feature = "sha1"))]
compile_error!("The feature `sha1` feature is required.");

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

/// Denotes the kind of function to produce a [`ObjectId`].
#[derive(Default, PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Kind {
    /// The Sha1 hash with 160 bits.
    #[default]
    Sha1 = 1,
}

mod kind;
