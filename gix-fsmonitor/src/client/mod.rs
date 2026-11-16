/// Git's native filesystem monitor IPC client.
pub mod daemon;
/// Filesystem monitor hook execution and protocol negotiation.
pub mod hook;

#[cfg(unix)]
mod transport;
