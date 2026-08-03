//! Async server-side transport I/O primitives.

use crate::{
    packetline::{PacketLineRef, async_io::StreamingPeekableIter},
    server::{ConnectRequest, Error, parse_connect_message},
    Protocol, Service,
};
use bstr::BString;
use futures_io::AsyncRead;

/// A server-side connection wrapping an async packetline reader and a raw writer.
///
/// Created by [`accept()`] after parsing the client's initial connect message,
/// or constructed directly for protocols where connection setup is handled
/// externally (e.g. HTTP or SSH).
pub struct Connection<R, W> {
    /// The async packetline reader for incoming client data.
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
    R: AsyncRead + Unpin,
    W: futures_io::AsyncWrite + Unpin,
{
    /// Create a connection directly from an async reader/writer pair.
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

/// Accept a git daemon connection by reading the initial connect message asynchronously.
///
/// Reads the first packetline from `reader`, parses it as a
/// `git-proto-request`, and returns a [`Connection`] ready for async protocol
/// communication along with the full [`ConnectRequest`] metadata.
///
/// This is the async equivalent of [`super::super::blocking_io::accept()`].
pub async fn accept<R, W>(reader: R, writer: W) -> Result<(Connection<R, W>, ConnectRequest), Error>
where
    R: AsyncRead + Unpin,
    W: futures_io::AsyncWrite + Unpin,
{
    let mut line_provider = StreamingPeekableIter::new(reader, &[PacketLineRef::Flush], false);

    let line = line_provider
        .read_line()
        .await
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
    use futures_lite::future::block_on;

    /// Build a packetline data frame: 4-hex-digit length prefix + payload.
    fn pkt_line(data: &[u8]) -> Vec<u8> {
        let len = data.len() + 4;
        let mut buf = format!("{len:04x}").into_bytes();
        buf.extend_from_slice(data);
        buf
    }

    /// A flush packet (0000).
    fn pkt_flush() -> Vec<u8> {
        b"0000".to_vec()
    }

    fn build_connect_packet(message: &[u8]) -> Vec<u8> {
        let mut buf = pkt_line(message);
        buf.extend(pkt_flush());
        buf
    }

    #[test]
    fn accept_upload_pack_v2() {
        block_on(async {
            let input = build_connect_packet(
                b"git-upload-pack /repo.git\0host=example.org\0\0version=2\0",
            );

            let output = futures_lite::io::Cursor::new(Vec::new());
            let (conn, request) = accept(input.as_slice(), output).await.expect("accept succeeds");

            assert_eq!(request.service, Service::UploadPack);
            assert_eq!(request.repository_path, "/repo.git");
            assert_eq!(request.protocol, Protocol::V2);
            assert_eq!(request.virtual_host, Some(("example.org".to_owned(), None)));

            assert_eq!(conn.service, Service::UploadPack);
            assert_eq!(conn.protocol, Protocol::V2);
        });
    }

    #[test]
    fn accept_receive_pack_v1() {
        block_on(async {
            let input = build_connect_packet(
                b"git-receive-pack /project.git\0host=git.example.com:9418\0",
            );

            let output = futures_lite::io::Cursor::new(Vec::new());
            let (conn, request) = accept(input.as_slice(), output).await.expect("accept succeeds");

            assert_eq!(request.service, Service::ReceivePack);
            assert_eq!(conn.protocol, Protocol::V1);
            assert_eq!(request.virtual_host, Some(("git.example.com".to_owned(), Some(9418))));
        });
    }

    #[test]
    fn accept_empty_input_returns_error() {
        block_on(async {
            let input: &[u8] = &[];
            let output = futures_lite::io::Cursor::new(Vec::new());
            let err = accept(input, output).await.unwrap_err();
            assert!(matches!(err, Error::MalformedMessage));
        });
    }

    #[test]
    fn accept_flush_only_returns_error() {
        block_on(async {
            let input = pkt_flush();
            let output = futures_lite::io::Cursor::new(Vec::new());
            let err = accept(input.as_slice(), output).await.unwrap_err();
            assert!(matches!(err, Error::MalformedMessage));
        });
    }

    #[test]
    fn accept_unknown_service_returns_error() {
        block_on(async {
            let input = build_connect_packet(b"git-frobnicate /repo.git\0");
            let output = futures_lite::io::Cursor::new(Vec::new());
            let err = accept(input.as_slice(), output).await.unwrap_err();
            assert!(matches!(err, Error::UnknownService { .. }));
        });
    }

    #[test]
    fn connection_new_sets_fields() {
        let input: &[u8] = &[];
        let output = futures_lite::io::Cursor::new(Vec::new());
        let conn = Connection::new(input, output, Service::ReceivePack, "/project.git", Protocol::V1);
        assert_eq!(conn.service, Service::ReceivePack);
        assert_eq!(conn.repository_path, "/project.git");
        assert_eq!(conn.protocol, Protocol::V1);
    }

    #[test]
    fn subsequent_data_readable_after_accept() {
        block_on(async {
            let mut input = pkt_line(
                b"git-upload-pack /repo.git\0host=h\0\0version=2\0",
            );
            input.extend(pkt_line(b"command=ls-refs\n"));
            input.extend(pkt_flush());

            let output = futures_lite::io::Cursor::new(Vec::new());
            let (mut conn, _) = accept(input.as_slice(), output).await.expect("accept succeeds");

            conn.line_provider.reset();
            let next = conn.line_provider.read_line().await;
            assert!(next.is_some(), "should have subsequent data");
            let line = next.unwrap().expect("io ok").expect("decode ok");
            let text = line.as_text().expect("text line");
            assert_eq!(text.0, b"command=ls-refs".as_slice());
        });
    }

    #[test]
    fn debug_output_shows_metadata() {
        let input: &[u8] = &[];
        let output = futures_lite::io::Cursor::new(Vec::new());
        let conn = Connection::new(input, output, Service::UploadPack, "/repo.git", Protocol::V2);
        let debug = format!("{conn:?}");
        assert!(debug.contains("UploadPack"));
        assert!(debug.contains("/repo.git"));
        assert!(debug.contains("V2"));
    }
}
