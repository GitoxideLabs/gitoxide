#![deny(rust_2018_idioms)]
#![forbid(unsafe_code)]

///
pub mod blob;
///
pub mod commit;
pub use commit::function::commit;
///
pub mod tree;
pub use tree::function::tree;
