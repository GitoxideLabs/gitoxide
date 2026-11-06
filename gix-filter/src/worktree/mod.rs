//! Worktree encodings are powered by the `encoding_rs` crate, which has a narrower focus than the `iconv` library. Thus this implementation
//! is inherently more limited but will handle the common cases. As `encoding_rs` can't encode to UTF-16, we do that ourselves.
//! Only explicit `UTF-16LE` and `UTF-16BE` labels without a BOM are supported; BOM-required `UTF-16` is rejected.
//!  
//! Note that for encoding to legacy formats, [additional normalization steps](https://docs.rs/encoding_rs/0.8.32/encoding_rs/#preparing-text-for-the-encoders)
//! can be taken, which we do not yet take unless there is specific examples or problems to solve.

///
pub mod encoding;

///
pub mod encode_to_git;
pub use encode_to_git::function::encode_to_git;

///
pub mod encode_to_worktree;
pub use encode_to_worktree::function::encode_to_worktree;
