use std::process::Stdio;

use bstr::BStr;
use gix_error::{Result, ResultExt, bail, message};

/// A filesystem monitor hook protocol version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    /// Timestamp-based requests with NUL-separated paths in the response.
    One,
    /// Opaque-token requests with a token followed by NUL-separated paths in the response.
    Two,
}

mod blocking {
    use super::*;
    use gix_macros::sync as maybe_async;
    fn prepare_command(command: gix_command::Prepare) -> Result<std::process::Command> {
        std::process::Command::try_from(command)
    }
    include!("hook_query.rs");
}
pub use blocking::query;

#[cfg(feature = "async")]
mod asynchronous {
    use super::*;
    use gix_macros::keep as maybe_async;
    fn prepare_command(command: gix_command::Prepare) -> Result<tokio::process::Command> {
        let mut command = tokio::process::Command::from(std::process::Command::try_from(command)?);
        command.kill_on_drop(true);
        Ok(command)
    }
    include!("hook_query.rs");
}
/// Tokio hook execution and protocol negotiation. Requires Tokio process support.
/// Cancelling a query kills the active child process and abandons its response.
#[cfg(feature = "async")]
pub use asynchronous::query as query_async;
