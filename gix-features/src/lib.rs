//! A crate providing foundational capabilities to other `git-*` crates with trade-offs between compile time, binary size or speed
//! selectable using cargo feature toggles.
//!
//! It's designed to allow the application level crate to configure feature toggles, affecting all other `git-*` crates using
//! this one.
//!
//! Thus all features provided here commonly have a 'cheap' base implementation, with the option to pull in
//! counterparts with higher performance.
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs)]

///
pub mod cache;
///
pub mod decode;
pub mod fs;
pub mod hash;
pub mod interrupt;
#[cfg(feature = "io-pipe")]
pub mod io;
pub mod parallel;
#[cfg(feature = "progress")]
pub mod progress;
pub mod threading;
pub use gix_trace as trace;

///
#[cfg(feature = "zlib")]
pub mod zlib;

///
pub mod iter {
    /// An iterator over chunks of input, producing `Vec<Item>` with a size of `size`, with the last chunk being the remainder and thus
    /// potentially smaller than `size`.
    pub struct Chunks<I> {
        /// The inner iterator to ask for items.
        pub inner: I,
        /// The size of chunks to produce
        pub size: usize,
    }

    impl<I, Item> Iterator for Chunks<I>
    where
        I: Iterator<Item = Item>,
    {
        type Item = Vec<Item>;

        fn next(&mut self) -> Option<Self::Item> {
            let mut res = Vec::with_capacity(self.size);
            let mut items_left = self.size;
            for item in &mut self.inner {
                res.push(item);
                items_left -= 1;
                if items_left == 0 {
                    break;
                }
            }
            (!res.is_empty()).then_some(res)
        }
    }
}
