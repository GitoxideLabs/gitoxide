//! A crate with utility types for use by other crates that implement specifics.
//!
//! Unless specified differently, all operations need an index file (e.g. `.git/index`) as driver.
//!
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs, unsafe_code)]
use bstr::BString;
/// Provides types needed for using [`stack::Platform::matching_attributes()`].
#[cfg(feature = "attributes")]
pub use gix_attributes as attributes;
/// A way to access the [`Case`](glob::pattern::Case) enum which used throughout this API.
pub use gix_glob as glob;
/// Provides types needed for using [`stack::Platform::excluded_kind()`].
pub use gix_ignore as ignore;
/// Provides types needed for using [`Stack::at_path()`] and [`Stack::at_entry()`].
pub use gix_index as index;
/// Provides types needed for using [`Stack::at_path()`] and [`Stack::at_entry()`].
pub use gix_object as object;
/// Provides types needed for using [`stack::State::for_checkout()`].
#[cfg(feature = "attributes")]
pub use gix_validate as validate;

/// A cache for efficiently executing operations on directories and files which are encountered in sorted order.
/// That way, these operations can be re-used for subsequent invocations in the same directory.
///
/// This cache can be configured to create directories efficiently, read git-ignore files and git-attribute files,
/// in any combination.
///
/// A cache for directory creation to reduce the amount of stat calls when creating
/// directories safely, that is without following symlinks that might be on the way.
///
/// As a special case, it offers a 'prefix' which (by itself) is assumed to exist and may contain symlinks.
/// Everything past that prefix boundary must not contain a symlink. We do this by allowing any input path.
///
/// Another added benefit is its ability to store the path of full path of the entry to which leading directories
/// are to be created to avoid allocating memory.
///
/// For this to work, it remembers the last 'good' path to a directory and assumes that all components of it
/// are still valid, too.
/// As directories are created, the cache will be adjusted to reflect the latest seen directory.
///
/// The caching is only useful if consecutive calls to create a directory are using a sorted list of entries.
#[derive(Clone)]
pub struct Stack {
    stack: gix_fs::Stack,
    /// tells us what to do as we change paths.
    state: stack::State,
    /// A buffer used when reading attribute or ignore files or their respective objects from the object database.
    buf: Vec<u8>,
    /// If case folding should happen when looking up attributes or exclusions.
    case: gix_glob::pattern::Case,
    /// A lookup table for object ids to read from in some situations when looking up attributes or exclusions.
    id_mappings: Vec<PathIdMapping>,
    statistics: stack::Statistics,
}

pub(crate) type PathIdMapping = (BString, gix_hash::ObjectId);

///
pub mod stack;
