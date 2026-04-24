//! Git stores all of its data as _Objects_, which are data along with a hash over all data. Storing objects efficiently
//! is what git packs are concerned about.
//!
//! Packs consist of [data files][data::File] and [index files][index::File]. The latter can be generated from a data file
//! and make accessing objects within a pack feasible.
//!
//! A [Bundle] conveniently combines a data pack alongside its index to allow [finding][Find] objects or verifying the pack.
//! Objects returned by `.find(…)` are [objects][gix_object::Data] which know their pack location in order to speed up
//! various common operations like creating new packs from existing ones.
//!
//! When traversing all objects in a pack, a _delta tree acceleration structure_ can be built from pack data or an index
//! in order to decompress packs in parallel and without any waste.
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs, rust_2018_idioms, unsafe_code)]

use std::{borrow::Cow, ops::Deref, path::Path};

/// The default memory-backed storage for pack data and index files.
pub use memmap2::Mmap as MMap;

/// A byte-oriented backing store for pack data and indices.
pub trait FileData: Deref<Target = [u8]> {}

impl<T> FileData for T where T: Deref<Target = [u8]> {}

///
pub mod bundle;
/// A bundle of pack data and the corresponding pack index
pub struct Bundle {
    /// The pack file corresponding to `index`
    pub pack: data::File,
    /// The index file corresponding to `pack`
    pub index: index::File,
}

///
pub mod find;

///
pub mod cache;
///
pub mod data;

mod find_traits;
pub use find_traits::{Find, FindExt};

///
pub mod index;
///
pub mod multi_index;

///
pub mod verify;

mod mmap {
    use std::path::Path;

    pub fn read_only(path: &Path) -> std::io::Result<memmap2::Mmap> {
        let file = std::fs::File::open(path)?;
        // SAFETY: we have to take the risk of somebody changing the file underneath. Git never writes into the same file.
        #[allow(unsafe_code)]
        unsafe {
            memmap2::MmapOptions::new().map_copy_read_only(&file)
        }
    }
}

/// Return a display-friendly name for pack- or index-related progress messages.
///
/// Prefer the file name, but fall back to the full path for paths without a terminal component.
fn source_name(path: &Path) -> Cow<'_, str> {
    if path.as_os_str().is_empty() {
        Cow::Borrowed("<memory>")
    } else if let Some(name) = path.file_name() {
        name.to_string_lossy()
    } else {
        path.as_os_str().to_string_lossy()
    }
}

#[inline]
fn read_u32(b: &[u8]) -> u32 {
    u32::from_be_bytes(b.try_into().unwrap())
}

#[inline]
fn read_u64(b: &[u8]) -> u64 {
    u64::from_be_bytes(b.try_into().unwrap())
}

fn exact_vec<T>(capacity: usize) -> Vec<T> {
    let mut v = Vec::new();
    v.reserve_exact(capacity);
    v
}

#[inline]
fn fan_is_monotonically_increasing(fan: &[u32]) -> bool {
    !fan.windows(2).any(|window| window[0] > window[1])
}
