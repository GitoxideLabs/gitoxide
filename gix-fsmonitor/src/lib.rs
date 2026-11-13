//! Clients, daemon control, and an optional embeddable server for Git's filesystem monitor protocols.
//!
//! Repository configuration, index checkpoints, and status policy belong to the caller.
#![deny(missing_docs)]
#![deny(unsafe_code)]

/// Query filesystem monitor hooks and native daemons.
#[forbid(unsafe_code)]
pub mod client;

#[cfg(feature = "async")]
mod asynchronous;

/// Serve Git's native filesystem monitor protocol with application-owned watches and shutdown.
#[cfg(feature = "server")]
pub mod server;

/// Control foreground and detached filesystem monitor daemons from caller-resolved server contexts.
/// Available with the `daemon` feature, which also enables `server`.
#[cfg(feature = "daemon")]
pub mod daemon;

#[cfg(any(windows, feature = "server"))]
fn normalize_endpoint(path: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    use std::io;

    // Resolve only the parent: the endpoint may not exist and must not follow a stale socket symlink.
    let (parent, name) = path.parent().zip(path.file_name()).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "filesystem monitor endpoint must name a file",
        )
    })?;
    Ok(parent.canonicalize()?.join(name))
}
