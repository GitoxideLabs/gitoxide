//! A crate to help setting the worktree to a particular state.
#![deny(missing_docs, unsafe_code)]

///
pub mod checkout;
pub use checkout::function::checkout;
