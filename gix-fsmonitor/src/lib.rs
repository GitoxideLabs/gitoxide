//! Clients and an optional embeddable server for Git's filesystem monitor protocols.
//!
//! Repository configuration, index checkpoints, and status policy belong to the caller.
#![deny(missing_docs)]
#![deny(unsafe_code)]

/// Query filesystem monitor hooks and native daemons.
#[forbid(unsafe_code)]
pub mod client;

/// Serve Git's native filesystem monitor protocol with application-owned watches and shutdown.
#[cfg(feature = "server")]
pub mod server;
