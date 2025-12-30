//! A high-performance SDK for building Git read-only RPC servers on top of gitoxide.
//!
//! This crate provides a connection pool and high-level operations for efficiently
//! serving Git repository data over RPC protocols. It is designed for read-only
//! access patterns typical of Git hosting services.
//!
//! # Features
//!
//! - Repository connection pooling with [`RepoPool`]
//! - Configurable caching and resource limits via [`SdkConfig`]
//! - High-level Git operations in the [`ops`] module
//! - Thread-safe repository handles with [`RepoHandle`]

mod pool;
mod config;
mod error;
mod types;
pub mod ops;

pub use pool::{RepoPool, RepoHandle};
pub use config::SdkConfig;
pub use error::SdkError;
pub use types::PoolStats;
