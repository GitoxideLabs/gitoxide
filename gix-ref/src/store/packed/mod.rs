use std::path::PathBuf;

use gix_hash::ObjectId;
use gix_object::bstr::{BStr, BString};
use memmap2::Mmap;

use crate::{FullNameRef, Namespace, file, transaction::RefEdit};

#[derive(Debug)]
enum Backing {
    /// The buffer is loaded entirely in memory, along with the `offset` to the first record past the header.
    InMemory(Vec<u8>),
    /// The buffer is mapping the file on disk, along with the offset to the first record past the header
    Mapped(Mmap),
}

/// A buffer containing a packed-ref file that is either memory mapped or fully in-memory depending on a cutoff.
///
/// The buffer is guaranteed to be sorted as per the packed-ref rules which allows some operations to be more efficient.
#[derive(Debug)]
pub struct Buffer {
    data: Backing,
    /// The hash kind to expect when parsing packed references.
    object_hash: gix_hash::Kind,
    /// The offset to the first record, how many bytes to skip past the header
    offset: usize,
    /// The path from which we were loaded
    path: PathBuf,
}

struct Edit {
    inner: RefEdit,
    peeled: Option<ObjectId>,
}

/// A transaction for editing packed references
pub(crate) struct Transaction {
    buffer: Option<file::packed::SharedBufferSnapshot>,
    edits: Option<Vec<Edit>>,
    lock: Option<gix_lock::File>,
    #[allow(dead_code)] // It just has to be kept alive, hence no reads
    closed_lock: Option<gix_lock::Marker>,
    precompose_unicode: bool,
    /// The namespace to use when preparing or writing refs
    namespace: Option<Namespace>,
}

/// A reference as parsed from the `packed-refs` file
#[derive(Debug, PartialEq, Eq)]
pub struct Reference<'a> {
    /// The validated full name of the reference.
    pub name: &'a FullNameRef,
    /// The target object id of the reference, hex encoded.
    pub target: &'a BStr,
    /// The fully peeled object id, hex encoded, that the ref is ultimately pointing to
    /// i.e. when all indirections are removed.
    pub object: Option<&'a BStr>,
}

impl Reference<'_> {
    /// Decode the target as object
    pub fn target(&self) -> ObjectId {
        gix_hash::ObjectId::from_hex(self.target).expect("parser validation")
    }

    /// Decode the object this reference is ultimately pointing to. Note that this is
    /// the [`target()`][Reference::target()] if this is not a fully peeled reference like a tag.
    pub fn object(&self) -> ObjectId {
        self.object.map_or_else(
            || self.target(),
            |id| ObjectId::from_hex(id).expect("parser validation"),
        )
    }
}

/// An iterator over references in a packed refs file
pub struct Iter<'a> {
    /// The position at which to parse the next reference
    cursor: &'a [u8],
    /// The hash kind to expect when parsing packed references.
    object_hash: gix_hash::Kind,
    /// The next line, starting at 1
    current_line: usize,
    /// If set, references returned will match the prefix, the first failed match will stop all iteration.
    prefix: Option<BString>,
}

mod decode;

///
pub mod iter;

///
pub mod buffer;

///
pub mod find;

///
pub mod transaction;
