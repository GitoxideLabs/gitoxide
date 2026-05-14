//! Provide facilities to merge *blobs*, *trees* and *commits*.
//!
//! * [blob-merges](blob) look at file content.
//! * [tree-merges](mod@tree) look at trees and merge them structurally, triggering blob-merges as needed.
//! * [commit-merges](mod@commit) are like tree merges, but compute or create the merge-base on the fly.
#![deny(missing_docs)]
#![forbid(unsafe_code)]

///
pub mod blob;
///
pub mod commit;
pub use commit::function::commit;
///
pub mod tree;
pub use tree::function::tree;
