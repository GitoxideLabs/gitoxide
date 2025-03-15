//! This crate includes the various diffs `git` can do between different representations
//! of the repository state, like comparisons between…
//!
//! * index and working tree
//! * *tree and index*
//!
//! …while also being able to check if the working tree is dirty, quickly, by instructing the operation to stop once the first
//! change was found.
//!
//! ### Tree-Index Status
//!
//! This status is not actually implemented here as it's not implemented directly. Instead, one creates an Index from a tree
//! and then diffs two indices with `gix_diff::index(index_from_tree, usually_dot_git_index)`. This adds about 15% to the runtime
//! and comes at the cost of another index in memory.
//! Once there are generators implementing depth-first tree iteration should become trivial, but for now it's very hard if one
//! wants to return referenced state of the iterator (which is not possible).
//!
//! ### Difference to `gix-diff`
//!
//! Technically, `status` is just another form of diff between different kind of sides, i.e. an index and a working tree.
//! This is the difference to `gix-diff`, which compares only similar items.
//!
//! ### Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg, doc_auto_cfg))]
#![deny(missing_docs, rust_2018_idioms, unsafe_code)]

#[cfg(target_has_atomic = "64")]
use std::sync::atomic::AtomicU64;

#[cfg(not(target_has_atomic = "64"))]
use portable_atomic::AtomicU64;

pub mod index_as_worktree;
pub use index_as_worktree::function::index_as_worktree;

#[cfg(feature = "worktree-rewrites")]
pub mod index_as_worktree_with_renames;
#[cfg(feature = "worktree-rewrites")]
pub use index_as_worktree_with_renames::function::index_as_worktree_with_renames;

/// A stack that validates we are not going through a symlink in a way that is read-only.
///
/// It can efficiently validate paths when these are queried in sort-order, which leads to each component
/// to only be checked once.
pub struct SymlinkCheck {
    /// Supports querying additional information, like the stack root.
    pub inner: gix_fs::Stack,
}

mod stack;

fn is_dir_to_mode(is_dir: bool) -> gix_index::entry::Mode {
    if is_dir {
        gix_index::entry::Mode::DIR
    } else {
        gix_index::entry::Mode::FILE
    }
}
