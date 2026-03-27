use bstr::BString;
use gix_packetline::PacketLineRef;

use crate::{packetline::blocking_io::StreamingPeekableIter, Protocol, Service};

/// A server-side connection over a reader/writer pair.
///
/// Wraps a packetline reader and raw writer, holding the parsed
/// service and repository information. Suitable for SSH-tunneled
/// connections or HTTP `--stateless-rpc` mode.
pub struct Connection<R, W> {
    /// The packetline reader for incoming client data.
    pub line_provider: StreamingPeekableIter<R>,
    /// The writer for outgoing server responses.
    pub writer: W,
    /// The service the client requested.
    pub service: Service,
    /// The repository path the client wants to access.
    pub repository_path: BString,
    /// The protocol version to use.
    pub protocol: Protocol,
}

impl<R, W> Connection<R, W>
where
    R: std::io::Read,
    W: std::io::Write,
{
    /// Create a new server-side connection from the given `reader` and `writer`.
    pub fn new(
        reader: R,
        writer: W,
        service: Service,
        repository_path: impl Into<BString>,
        protocol: Protocol,
        trace: bool,
    ) -> Self {
        Connection {
            line_provider: StreamingPeekableIter::new(reader, &[PacketLineRef::Flush], trace),
            writer,
            service,
            repository_path: repository_path.into(),
            protocol,
        }
    }
}
