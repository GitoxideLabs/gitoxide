use std::{io, path::Path};

use gix_error::{OptionExt, Result, ResultExt, message};

use super::{protocol, transport};

/// Probe `endpoint` without requesting filesystem changes or starting a server.
/// Missing endpoints and refused connections return `false`; other connection errors are reported.
/// Windows resolves the endpoint's parent directory before deriving Git's named-pipe name.
pub fn is_running(endpoint: &Path) -> Result<bool> {
    match transport::connect(endpoint) {
        Ok(_) => Ok(true),
        Err(err) if matches!(err.kind(), io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused) => Ok(false),
        Err(err) => Err(err).or_raise(|| message("probing the filesystem monitor endpoint")),
    }
}

/// A native filesystem monitor control command or arbitrary IPC message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request<'a> {
    /// Stop the service at the endpoint, receiving an empty acknowledgment.
    Quit,
    /// Invalidate the service's token history and request a fresh baseline response.
    Flush,
    /// Send arbitrary bytes, typically interpreted as an opaque filesystem monitor checkpoint token.
    ///
    /// Empty or unknown tokens request a full baseline. The payload is sent unchanged, so literal
    /// `quit` and `flush` messages still invoke the corresponding native control commands.
    Message(&'a [u8]),
}

/// Send `request` to `endpoint` and return the server's raw response.
///
/// [`Request::Quit`] stops the service at this endpoint, and [`Request::Flush`] invalidates its token history.
/// Use [`Handle::shutdown()`](super::Handle::shutdown) to stop only an application-owned instance.
/// Each exchange has a five-second deadline and accepts at most 32 MiB of response data.
/// No daemon is started if the endpoint is unavailable.
pub fn query(endpoint: &Path, request: Request<'_>) -> Result<Vec<u8>> {
    let mut stream = transport::connect(endpoint).or_raise(|| message("connecting to the filesystem monitor"))?;
    let request = match request {
        Request::Quit => b"quit".as_slice(),
        Request::Flush => b"flush".as_slice(),
        Request::Message(message) => message,
    };
    protocol::write_message(&mut stream, request).or_raise(|| message("sending an IPC query"))?;
    protocol::read_message(&mut stream, 32 * 1024 * 1024)
        .or_raise(|| message("reading the filesystem monitor response"))?
        .ok_or_raise(|| message("filesystem monitor closed without a response"))
}
