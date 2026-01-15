use std::ops::Range;

use crate::file::Index;
use crate::Id;
use gix_error::bstr::ByteSlice;
use gix_error::{message, Message};

/// An entry of a chunk file index
pub struct Entry {
    /// The kind of the chunk file
    pub kind: Id,
    /// The offset, relative to the beginning of the file, at which to find the chunk and its end.
    pub offset: Range<crate::file::Offset>,
}

impl Index {
    /// The size of a single index entry in bytes
    pub const ENTRY_SIZE: usize = std::mem::size_of::<u32>() + std::mem::size_of::<u64>();
    /// The smallest possible size of an index, consisting only of the sentinel value pointing past itself.
    pub const EMPTY_SIZE: usize = Index::ENTRY_SIZE;

    /// Returns the size in bytes an index with `num_entries` would take.
    pub const fn size_for_entries(num_entries: usize) -> usize {
        Self::ENTRY_SIZE * (num_entries + 1/*sentinel*/)
    }

    /// Find a chunk of `kind` and return its offset into the data if found
    pub fn offset_by_id(&self, kind: Id) -> Result<Range<crate::file::Offset>, Message> {
        self.chunks
            .iter()
            .find_map(|c| (c.kind == kind).then(|| c.offset.clone()))
            .ok_or_else(make_message(kind))
    }

    /// Find a chunk of `kind` and return its offset as usize range into the data if found.
    ///
    ///
    /// # Panics
    ///
    /// - if the usize conversion fails, which isn't expected as memory maps can't be created if files are too large
    ///   to require such offsets.
    pub fn usize_offset_by_id(&self, kind: Id) -> Result<Range<usize>, Message> {
        self.chunks
            .iter()
            .find_map(|c| (c.kind == kind).then(|| crate::range::into_usize_or_panic(c.offset.clone())))
            .ok_or_else(make_message(kind))
    }

    /// Like [`Index::usize_offset_by_id()`] but with support for validation and transformation using a function.
    pub fn validated_usize_offset_by_id<T>(
        &self,
        kind: Id,
        validate: impl FnOnce(Range<usize>) -> T,
    ) -> Result<T, Message> {
        self.chunks
            .iter()
            .find_map(|c| (c.kind == kind).then(|| crate::range::into_usize_or_panic(c.offset.clone())))
            .map(validate)
            .ok_or_else(make_message(kind))
    }

    /// Find a chunk of `kind` and return its data slice based on its offset.
    pub fn data_by_id<'a>(&self, data: &'a [u8], kind: Id) -> Result<&'a [u8], Message> {
        let offset = self.offset_by_id(kind)?;
        Ok(&data[crate::range::into_usize(offset)
            .ok_or_else(|| message!("The offsets into the file couldn't be represented by usize"))?])
    }

    /// Return the end offset of the last chunk, which is the highest offset as well.
    /// It's definitely available as we have one or more chunks.
    pub fn highest_offset(&self) -> crate::file::Offset {
        self.chunks.last().expect("at least one chunk").offset.end
    }
}

fn make_message(kind: Id) -> impl FnOnce() -> Message {
    move || message!("Chunk named '{}' was not found in chunk file index", kind.as_bstr())
}
