//! Convert worktree encodings using [`gix_encoding`], with Git's byte-order mark validation and optional round-trip checks.
//!
//! The buffer helpers retain allocations across calls. Streaming conversions are available through [`gix_encoding`].

///
pub mod encoding;

///
pub mod encode_to_git;
pub use encode_to_git::function::encode_to_git;

///
pub mod encode_to_worktree;
pub use encode_to_worktree::function::encode_to_worktree;
