//! An implementation of the shared parts of git bitmaps used in `gix-pack`, `gix-index` and `gix-worktree`.
//!
//! Note that many tests are performed indirectly by tests in the aforementioned consumer crates.
//!
//! ## Examples
//!
//! ```
//! let encoded = [
//!     0, 0, 0, 64, // 64 bits in total
//!     0, 0, 0, 2,  // two u64 words follow
//!     0, 0, 0, 2, 0, 0, 0, 0, // one RLW word with one literal word
//!     0, 0, 0, 0, 0, 0, 0, 21, // literal bits 0, 2 and 4 set
//!     0, 0, 0, 0, // RLW points at the first word
//! ];
//!
//! let (bitmap, rest) = gix_bitmap::ewah::decode(&encoded).unwrap();
//! let mut set_bits = Vec::new();
//! bitmap.for_each_set_bit(|idx| {
//!     set_bits.push(idx);
//!     Some(())
//! });
//!
//! assert!(rest.is_empty());
//! assert_eq!(bitmap.num_bits(), 64);
//! assert_eq!(set_bits, vec![0, 2, 4]);
//! ```
#![deny(unsafe_code, missing_docs)]

/// Bitmap utilities for the advanced word-aligned hybrid bitmap
pub mod ewah;

pub(crate) mod decode {
    #[inline]
    pub(crate) fn u32(data: &[u8]) -> Option<(u32, &[u8])> {
        data.split_at_checked(4)
            .map(|(num, data)| (u32::from_be_bytes(num.try_into().unwrap()), data))
    }
}
