use std::{io, path::Path};

use gix_error::{OptionExt, Result, ResultExt, message};

use super::{protocol, transport};

/// Probe an endpoint without requesting filesystem changes or starting a server.
/// Missing endpoints and refused connections return `false`; other connection errors are reported.
pub fn is_running(endpoint: &Path) -> Result<bool> {
    match transport::connect(endpoint) {
        Ok(_) => Ok(true),
        Err(err) if matches!(err.kind(), io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused) => Ok(false),
        Err(err) => Err(err).or_raise(|| message("probing the filesystem monitor endpoint")),
    }
}

/// Send a token or native control request to an existing server and return its raw response.
///
/// `quit` stops the service at this endpoint, and `flush` invalidates its token history.
/// Use [`Handle::shutdown()`](super::Handle::shutdown) to stop only an application-owned instance.
/// Each exchange has a five-second deadline and accepts at most 32 MiB of response data.
/// No daemon is started if the endpoint is unavailable.
pub fn query(endpoint: &Path, request: &[u8]) -> Result<Vec<u8>> {
    let mut stream = transport::connect(endpoint).or_raise(|| message("connecting to the filesystem monitor"))?;
    protocol::write_message(&mut stream, request).or_raise(|| message("sending an IPC query"))?;
    protocol::read_message(&mut stream, 32 * 1024 * 1024)
        .or_raise(|| message("reading the filesystem monitor response"))?
        .ok_or_raise(|| message("filesystem monitor closed without a response"))
}
