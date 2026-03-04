//! Read and write Git reftables.
//!
//! This crate provides a Rust implementation of Git's reftable storage format.

#![deny(missing_docs, rust_2018_idioms)]
#![forbid(unsafe_code)]

///
pub mod basics;
///
pub mod block;
///
pub mod blocksource;
///
pub mod constants;
///
pub mod error;
///
pub mod merged;
///
pub mod pq;
///
pub mod record;
///
pub mod stack;
///
pub mod table;
///
pub mod tree;
///
pub mod writer;
