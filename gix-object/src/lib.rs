//! This crate provides types for [read-only git objects][crate::ObjectRef] backed by bytes provided in git's serialization format
//! as well as [mutable versions][Object] of these. Both types of objects can be encoded.
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs, rust_2018_idioms)]
#![forbid(unsafe_code)]

use std::borrow::Cow;

/// For convenience to allow using `bstr` without adding it to own cargo manifest.
pub use bstr;
use bstr::{BStr, BString, ByteSlice};
/// For convenience to allow using `gix-date` without adding it to own cargo manifest.
pub use gix_date as date;
use smallvec::SmallVec;

///
pub mod commit;
mod object;
///
pub mod tag;
///
pub mod tree;

mod blob;
///
pub mod data;

///
pub mod find;

///
pub mod write {
    /// The error type returned by the [`Write`](crate::Write) trait.
    pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
}

mod traits;
pub use traits::{Exists, Find, FindExt, FindObjectOrHeader, Header as FindHeader, HeaderExt, Write, WriteTo};

pub mod encode;
pub(crate) mod parse;

///
pub mod kind;

/// The four types of objects that git differentiates.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[allow(missing_docs)]
pub enum Kind {
    Tree,
    Blob,
    Commit,
    Tag,
}
/// A chunk of any [`data`](BlobRef::data).
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlobRef<'a> {
    /// The bytes themselves.
    pub data: &'a [u8],
}

/// A mutable chunk of any [`data`](Blob::data).
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Blob {
    /// The data itself.
    pub data: Vec<u8>,
}

/// A git commit parsed using [`from_bytes()`](CommitRef::from_bytes()).
///
/// A commit encapsulates information about a point in time at which the state of the repository is recorded, usually after a
/// change which is documented in the commit `message`.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CommitRef<'a> {
    /// HEX hash of tree object we point to. Usually 40 bytes long.
    ///
    /// Use [`tree()`](CommitRef::tree()) to obtain a decoded version of it.
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub tree: &'a BStr,
    /// HEX hash of each parent commit. Empty for first commit in repository.
    pub parents: SmallVec<[&'a BStr; 1]>,
    /// The raw author header value as encountered during parsing.
    ///
    /// Use the [`author()`](CommitRef::author()) method to obtain a parsed version of it.
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub author: &'a BStr,
    /// The raw committer header value as encountered during parsing.
    ///
    /// Use the [`committer()`](CommitRef::committer()) method to obtain a parsed version of it.
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub committer: &'a BStr,
    /// The name of the message encoding, otherwise [UTF-8 should be assumed](https://github.com/git/git/blob/e67fbf927dfdf13d0b21dc6ea15dc3c7ef448ea0/commit.c#L1493:L1493).
    pub encoding: Option<&'a BStr>,
    /// The commit message documenting the change.
    pub message: &'a BStr,
    /// Extra header fields, in order of them being encountered, made accessible with the iterator returned by [`extra_headers()`](CommitRef::extra_headers()).
    pub extra_headers: Vec<(&'a BStr, Cow<'a, BStr>)>,
}

/// Like [`CommitRef`], but as `Iterator` to support (up to) entirely allocation free parsing.
/// It's particularly useful to traverse the commit graph without ever allocating arrays for parents.
#[derive(Copy, Clone)]
pub struct CommitRefIter<'a> {
    data: &'a [u8],
    state: commit::ref_iter::State,
}

/// A mutable git commit, representing an annotated state of a working tree along with a reference to its historical commits.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Commit {
    /// The hash of recorded working tree state.
    pub tree: gix_hash::ObjectId,
    /// Hash of each parent commit. Empty for the first commit in repository.
    pub parents: SmallVec<[gix_hash::ObjectId; 1]>,
    /// Who wrote this commit.
    pub author: gix_actor::Signature,
    /// Who committed this commit.
    ///
    /// This may be different from the `author` in case the author couldn't write to the repository themselves and
    /// is commonly encountered with contributed commits.
    pub committer: gix_actor::Signature,
    /// The name of the message encoding, otherwise [UTF-8 should be assumed](https://github.com/git/git/blob/e67fbf927dfdf13d0b21dc6ea15dc3c7ef448ea0/commit.c#L1493:L1493).
    pub encoding: Option<BString>,
    /// The commit message documenting the change.
    pub message: BString,
    /// Extra header fields, in order of them being encountered, made accessible with the iterator returned
    /// by [`extra_headers()`](Commit::extra_headers()).
    pub extra_headers: Vec<(BString, BString)>,
}

/// Represents a git tag, commonly indicating a software release.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TagRef<'a> {
    /// The hash in hexadecimal being the object this tag points to. Use [`target()`](TagRef::target()) to obtain a byte representation.
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub target: &'a BStr,
    /// The kind of object that `target` points to.
    pub target_kind: Kind,
    /// The name of the tag, e.g. "v1.0".
    pub name: &'a BStr,
    /// The raw tagger header value as encountered during parsing.
    ///
    /// Use the [`tagger()`](TagRef::tagger()) method to obtain a parsed version of it.
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub tagger: Option<&'a BStr>,
    /// The message describing this release.
    pub message: &'a BStr,
    /// A cryptographic signature over the entire content of the serialized tag object thus far.
    pub pgp_signature: Option<&'a BStr>,
}

/// Like [`TagRef`], but as `Iterator` to support entirely allocation free parsing.
/// It's particularly useful to dereference only the target chain.
#[derive(Copy, Clone)]
pub struct TagRefIter<'a> {
    data: &'a [u8],
    state: tag::ref_iter::State,
}

/// A mutable git tag.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Tag {
    /// The hash this tag is pointing to.
    pub target: gix_hash::ObjectId,
    /// The kind of object this tag is pointing to.
    pub target_kind: Kind,
    /// The name of the tag, e.g. "v1.0".
    pub name: BString,
    /// The tags author.
    pub tagger: Option<gix_actor::Signature>,
    /// The message describing the tag.
    pub message: BString,
    /// A pgp signature over all bytes of the encoded tag, excluding the pgp signature itself.
    pub pgp_signature: Option<BString>,
}

/// Immutable objects are read-only structures referencing most data from [a byte slice](ObjectRef::from_bytes()).
///
/// Immutable objects are expected to be deserialized from bytes that acts as backing store, and they
/// cannot be mutated or serialized. Instead, one will [convert](ObjectRef::into_owned()) them into their [`mutable`](Object) counterparts
/// which support mutation and serialization.
///
/// An `ObjectRef` is representing [`Trees`](TreeRef), [`Blobs`](BlobRef), [`Commits`](CommitRef), or [`Tags`](TagRef).
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum ObjectRef<'a> {
    #[cfg_attr(feature = "serde", serde(borrow))]
    Tree(TreeRef<'a>),
    Blob(BlobRef<'a>),
    Commit(CommitRef<'a>),
    Tag(TagRef<'a>),
}

/// Mutable objects with each field being separately allocated and changeable.
///
/// Mutable objects are Commits, Trees, Blobs and Tags that can be changed and serialized.
///
/// They either created using object [construction](Object) or by [deserializing existing objects](ObjectRef::from_bytes())
/// and converting these [into mutable copies](ObjectRef::into_owned()) for adjustments.
///
/// An `Object` is representing [`Trees`](Tree), [`Blobs`](Blob), [`Commits`](Commit), or [`Tags`](Tag).
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::large_enum_variant, missing_docs)]
pub enum Object {
    Tree(Tree),
    Blob(Blob),
    Commit(Commit),
    Tag(Tag),
}
/// A directory snapshot containing files (blobs), directories (trees) and submodules (commits).
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TreeRef<'a> {
    /// The directories and files contained in this tree.
    ///
    /// Beware that the sort order isn't *quite* by name, so one may bisect only with a [`tree::EntryRef`] to handle ordering correctly.
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub entries: Vec<tree::EntryRef<'a>>,
}

/// A directory snapshot containing files (blobs), directories (trees) and submodules (commits), lazily evaluated.
#[derive(Default, PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
pub struct TreeRefIter<'a> {
    /// The directories and files contained in this tree.
    data: &'a [u8],
}

/// A mutable Tree, containing other trees, blobs or commits.
#[derive(Default, PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Tree {
    /// The directories and files contained in this tree. They must be and remain sorted by [`filename`][tree::Entry::filename].
    ///
    /// Beware that the sort order isn't *quite* by name, so one may bisect only with a [`tree::Entry`] to handle ordering correctly.
    pub entries: Vec<tree::Entry>,
}

impl Tree {
    /// Return an empty tree which serializes to a well-known hash
    pub fn empty() -> Self {
        Tree { entries: Vec::new() }
    }
}

/// A borrowed object using a slice as backing buffer, or in other words a bytes buffer that knows the kind of object it represents.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
pub struct Data<'a> {
    /// kind of object
    pub kind: Kind,
    /// decoded, decompressed data, owned by a backing store.
    pub data: &'a [u8],
}

/// Information about an object, which includes its kind and the amount of bytes it would have when obtained.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
pub struct Header {
    /// The kind of object.
    pub kind: Kind,
    /// The object's size in bytes, or the size of the buffer when it's retrieved in full.
    pub size: u64,
}

///
pub mod decode {
    #[cfg(feature = "verbose-object-parsing-errors")]
    mod _decode {
        /// The type to be used for parse errors.
        pub type ParseError = winnow::error::ContextError<winnow::error::StrContext>;

        pub(crate) fn empty_error() -> Error {
            Error {
                inner: winnow::error::ContextError::new(),
                remaining: Default::default(),
            }
        }

        /// A type to indicate errors during parsing and to abstract away details related to `nom`.
        #[derive(Debug, Clone)]
        pub struct Error {
            /// The actual error
            pub inner: ParseError,
            /// Where the error occurred
            pub remaining: Vec<u8>,
        }

        impl Error {
            pub(crate) fn with_err(err: winnow::error::ErrMode<ParseError>, remaining: &[u8]) -> Self {
                Self {
                    inner: err.into_inner().expect("we don't have streaming parsers"),
                    remaining: remaining.to_owned(),
                }
            }
        }

        impl std::fmt::Display for Error {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "object parsing failed at `{}`", bstr::BStr::new(&self.remaining))?;
                if self.inner.context().next().is_some() {
                    writeln!(f)?;
                    self.inner.fmt(f)?;
                }
                Ok(())
            }
        }

        impl std::error::Error for Error {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                self.inner.cause().map(|v| v as &(dyn std::error::Error + 'static))
            }
        }
    }

    ///
    #[cfg(not(feature = "verbose-object-parsing-errors"))]
    mod _decode {
        /// The type to be used for parse errors, discards everything and is zero size
        pub type ParseError = ();

        pub(crate) fn empty_error() -> Error {
            Error { inner: () }
        }

        /// A type to indicate errors during parsing and to abstract away details related to `nom`.
        #[derive(Debug, Clone)]
        pub struct Error {
            /// The actual error
            pub inner: ParseError,
        }

        impl Error {
            pub(crate) fn with_err(err: winnow::error::ErrMode<ParseError>, _remaining: &[u8]) -> Self {
                Self {
                    inner: err.into_inner().expect("we don't have streaming parsers"),
                }
            }
        }

        impl std::fmt::Display for Error {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("object parsing failed")
            }
        }

        impl std::error::Error for Error {}
    }
    pub(crate) use _decode::empty_error;
    pub use _decode::{Error, ParseError};

    /// Returned by [`loose_header()`]
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum LooseHeaderDecodeError {
        #[error("{message}: {number:?}")]
        ParseIntegerError {
            source: gix_utils::btoi::ParseIntegerError,
            message: &'static str,
            number: bstr::BString,
        },
        #[error("{message}")]
        InvalidHeader { message: &'static str },
        #[error("The object header contained an unknown object kind.")]
        ObjectHeader(#[from] super::kind::Error),
    }

    use bstr::ByteSlice;
    /// Decode a loose object header, being `<kind> <size>\0`, returns
    /// ([`kind`](super::Kind), `size`, `consumed bytes`).
    ///
    /// `size` is the uncompressed size of the payload in bytes.
    pub fn loose_header(input: &[u8]) -> Result<(super::Kind, u64, usize), LooseHeaderDecodeError> {
        use LooseHeaderDecodeError::*;
        let kind_end = input.find_byte(0x20).ok_or(InvalidHeader {
            message: "Expected '<type> <size>'",
        })?;
        let kind = super::Kind::from_bytes(&input[..kind_end])?;
        let size_end = input.find_byte(0x0).ok_or(InvalidHeader {
            message: "Did not find 0 byte in header",
        })?;
        let size_bytes = &input[kind_end + 1..size_end];
        let size = gix_utils::btoi::to_signed(size_bytes).map_err(|source| ParseIntegerError {
            source,
            message: "Object size in header could not be parsed",
            number: size_bytes.into(),
        })?;
        Ok((kind, size, size_end + 1))
    }
}

fn object_hasher(hash_kind: gix_hash::Kind, object_kind: Kind, object_size: u64) -> gix_hash::Hasher {
    let mut hasher = gix_hash::hasher(hash_kind);
    hasher.update(&encode::loose_header(object_kind, object_size));
    hasher
}

/// A function to compute a hash of kind `hash_kind` for an object of `object_kind` and its `data`.
#[doc(alias = "hash_object", alias = "git2")]
pub fn compute_hash(
    hash_kind: gix_hash::Kind,
    object_kind: Kind,
    data: &[u8],
) -> Result<gix_hash::ObjectId, gix_hash::hasher::Error> {
    let mut hasher = object_hasher(hash_kind, object_kind, data.len() as u64);
    hasher.update(data);
    hasher.try_finalize()
}

/// A function to compute a hash of kind `hash_kind` for an object of `object_kind` and its data read from `stream`
/// which has to yield exactly `stream_len` bytes.
/// Use `progress` to learn about progress in bytes processed and `should_interrupt` to be able to abort the operation
/// if set to `true`.
#[doc(alias = "hash_file", alias = "git2")]
pub fn compute_stream_hash(
    hash_kind: gix_hash::Kind,
    object_kind: Kind,
    stream: &mut dyn std::io::Read,
    stream_len: u64,
    progress: &mut dyn gix_features::progress::Progress,
    should_interrupt: &std::sync::atomic::AtomicBool,
) -> Result<gix_hash::ObjectId, gix_hash::io::Error> {
    let hasher = object_hasher(hash_kind, object_kind, stream_len);
    gix_hash::bytes_with_hasher(stream, stream_len, hasher, progress, should_interrupt)
}
