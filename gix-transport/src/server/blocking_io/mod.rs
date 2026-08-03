//! Blocking server-side transport I/O primitives.

use crate::{
    packetline::{PacketLineRef, blocking_io::StreamingPeekableIter},
    server::{ConnectRequest, Error, parse_connect_message},
    Protocol, Service,
};
use bstr::BString;

/// A server-side connection wrapping a packetline reader and a raw writer.
///
/// Created by [`accept()`] after parsing the client's initial connect message,
/// or constructed directly for protocols where connection setup is handled
/// externally (e.g. HTTP or SSH).
pub struct Connection<R, W> {
    /// The packetline reader for incoming client data.
    pub line_provider: StreamingPeekableIter<R>,
    /// The writer for outgoing server responses.
    pub writer: W,
    /// The service the client requested.
    pub service: Service,
    /// The repository path the client wants to access.
    pub repository_path: BString,
    /// The negotiated protocol version.
    pub protocol: Protocol,
}

impl<R, W> std::fmt::Debug for Connection<R, W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("service", &self.service)
            .field("repository_path", &self.repository_path)
            .field("protocol", &self.protocol)
            .finish_non_exhaustive()
    }
}

impl<R, W> Connection<R, W>
where
    R: std::io::Read,
    W: std::io::Write,
{
    /// Create a connection directly from a reader/writer pair.
    ///
    /// Use this when connection setup is handled externally (HTTP, SSH)
    /// and you already know the service, path, and protocol version.
    pub fn new(
        reader: R,
        writer: W,
        service: Service,
        repository_path: impl Into<BString>,
        protocol: Protocol,
    ) -> Self {
        Connection {
            line_provider: StreamingPeekableIter::new(reader, &[PacketLineRef::Flush], false),
            writer,
            service,
            repository_path: repository_path.into(),
            protocol,
        }
    }
}

/// Accept a git daemon connection by reading the initial connect message.
///
/// Reads the first packetline from `reader`, parses it as a
/// `git-proto-request`, and returns a [`Connection`] ready for protocol
/// communication along with the full [`ConnectRequest`] metadata.
///
/// This is the entry point for implementing a `git-daemon` style server.
/// For HTTP or SSH transports where connection setup is handled externally,
/// use [`Connection::new()`] directly.
pub fn accept<R, W>(reader: R, writer: W) -> Result<(Connection<R, W>, ConnectRequest), Error>
where
    R: std::io::Read,
    W: std::io::Write,
{
    let mut line_provider = StreamingPeekableIter::new(reader, &[PacketLineRef::Flush], false);

    let line = line_provider
        .read_line()
        .ok_or(Error::MalformedMessage)?
        .map_err(|_| Error::MalformedMessage)?
        .map_err(|_| Error::MalformedMessage)?;

    let data = match line {
        PacketLineRef::Data(d) => d,
        _ => return Err(Error::MalformedMessage),
    };

    let request = parse_connect_message(data)?;

    let connection = Connection {
        line_provider,
        writer,
        service: request.service,
        repository_path: request.repository_path.clone(),
        protocol: request.protocol,
    };

    Ok((connection, request))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packetline::blocking_io::encode;

    #[test]
    fn accept_upload_pack_connection() -> Result<(), Box<dyn std::error::Error>> {
        let mut input = Vec::new();
        encode::data_to_write(b"git-upload-pack /repo.git\0host=example.org\0\0version=2\0", &mut input)?;
        // Simulate a subsequent flush (end of ref advertisement request)
        encode::flush_to_write(&mut input)?;

        let output = Vec::new();
        let (connection, request) = accept(input.as_slice(), output)?;

        assert_eq!(request.service, Service::UploadPack);
        assert_eq!(request.repository_path, "/repo.git");
        assert_eq!(request.protocol, Protocol::V2);
        assert_eq!(request.virtual_host, Some(("example.org".to_owned(), None)));

        assert_eq!(connection.service, Service::UploadPack);
        assert_eq!(connection.repository_path, "/repo.git");
        assert_eq!(connection.protocol, Protocol::V2);
        Ok(())
    }

    #[test]
    fn accept_receive_pack_v1() -> Result<(), Box<dyn std::error::Error>> {
        let mut input = Vec::new();
        encode::data_to_write(b"git-receive-pack /project.git\0host=git.example.com:9418\0", &mut input)?;
        encode::flush_to_write(&mut input)?;

        let output = Vec::new();
        let (connection, request) = accept(input.as_slice(), output)?;

        assert_eq!(request.service, Service::ReceivePack);
        assert_eq!(connection.protocol, Protocol::V1);
        assert_eq!(request.virtual_host, Some(("git.example.com".to_owned(), Some(9418))));
        Ok(())
    }

    #[test]
    fn accept_empty_input_fails() {
        let input: &[u8] = &[];
        let output = Vec::new();
        let err = accept(input, output).unwrap_err();
        assert!(matches!(err, Error::MalformedMessage));
    }

    #[test]
    fn connection_new_for_http_style_setup() {
        let input: &[u8] = &[];
        let output = Vec::new();
        let conn = Connection::new(input, output, Service::UploadPack, "/repo.git", Protocol::V2);
        assert_eq!(conn.service, Service::UploadPack);
        assert_eq!(conn.repository_path, "/repo.git");
        assert_eq!(conn.protocol, Protocol::V2);
    }
}
