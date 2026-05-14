//! Low-level access to reading and writing chunk file based formats.
//!
//! See the [git documentation](https://github.com/git/git/blob/seen/Documentation/technical/chunk-format.txt) for details.
//!
//! ## Examples
//!
//! ```
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use std::io::Write;
//!
//! let mut index = gix_chunk::file::Index::for_writing();
//! index.plan_chunk(*b"OIDF", 4);
//! index.plan_chunk(*b"DATA", 3);
//!
//! let mut out = index.into_write(Vec::new(), 0)?;
//! while let Some(kind) = out.next_chunk() {
//!     match kind {
//!         [b'O', b'I', b'D', b'F'] => out.write_all(b"abcd")?,
//!         [b'D', b'A', b'T', b'A'] => out.write_all(b"xyz")?,
//!         _ => unreachable!("planned chunks are known"),
//!     }
//! }
//!
//! let data = out.into_inner();
//! let decoded = gix_chunk::file::Index::from_bytes(&data, 0, 2)?;
//! assert_eq!(decoded.data_by_id(&data, *b"OIDF")?, b"abcd");
//! assert_eq!(decoded.data_by_id(&data, *b"DATA")?, b"xyz");
//! # Ok(()) }
//! ```
#![deny(missing_docs, unsafe_code)]

/// An identifier to describe the kind of chunk, unique within a chunk file, typically in ASCII
pub type Id = [u8; 4];

/// A special value denoting the end of the chunk file table of contents.
pub const SENTINEL: Id = [0u8; 4];

///
pub mod range {
    use std::ops::Range;

    use crate::file;

    /// Turn a u64 Range into a usize range safely, to make chunk ranges useful in memory mapped files.
    pub fn into_usize(Range { start, end }: Range<file::Offset>) -> Option<Range<usize>> {
        let start = start.try_into().ok()?;
        let end = end.try_into().ok()?;
        Some(Range { start, end })
    }

    /// Similar to [`into_usize()`], but panics assuming that the memory map couldn't be created if offsets
    /// stored are too high.
    ///
    /// This is only true for correctly formed files, as it's entirely possible to provide out of bounds offsets
    /// which are checked for separately - we wouldn't be here if that was the case.
    pub fn into_usize_or_panic(range: Range<file::Offset>) -> Range<usize> {
        into_usize(range).expect("memory maps can't be created if files are too large")
    }
}

///
pub mod file;
