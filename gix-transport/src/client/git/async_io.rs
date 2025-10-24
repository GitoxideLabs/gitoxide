use std::{borrow::Cow, error::Error};

use async_trait::async_trait;
use bstr::{BStr, BString, ByteVec};
use futures_io::{AsyncRead, AsyncWrite};
use futures_lite::AsyncWriteExt;

use crate::{
    client::{
        self,
        async_io::{RequestWriter, SetServiceResponse},
        capabilities,
        git::{self, ConnectionState},
        Capabilities,
    },
    packetline::{
        async_io::{StreamingPeekableIter, Writer},
        PacketLineRef,
    },
    Protocol, Service,
};

/// A TCP connection to either a `git` daemon or a spawned `git` process.
///
/// When connecting to a daemon, additional context information is sent with the first line of the handshake.
pub struct Connection<R, W> {
    pub(in crate::client) writer: W,
    pub(in crate::client) line_provider: StreamingPeekableIter<R>,
    pub(in crate::client) state: ConnectionState,
}

impl<R, W> Connection<R, W> {
    /// Optionally set the URL to be returned when asked for it if `Some` or calculate a default for `None`.
    ///
    /// The URL is required as parameter for authentication helpers which are called in transports
    /// that support authentication. Even though plain git transports don't support that, this
    /// may well be the case in custom transports.
    pub fn custom_url(mut self, url: Option<BString>) -> Self {
        self.state.custom_url = url;
        self
    }

    /// Return the inner reader and writer
    pub fn into_inner(self) -> (R, W) {
        (self.line_provider.into_inner(), self.writer)
    }
}

impl<R, W> client::TransportWithoutIO for Connection<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    fn to_url(&self) -> Cow<'_, BStr> {
        self.state.custom_url.as_ref().map_or_else(
            || {
                let mut possibly_lossy_url = self.state.path.clone();
                possibly_lossy_url.insert_str(0, "file://");
                Cow::Owned(possibly_lossy_url)
            },
            |url| Cow::Borrowed(url.as_ref()),
        )
    }

    fn connection_persists_across_multiple_requests(&self) -> bool {
        true
    }

    fn configure(&mut self, _config: &dyn std::any::Any) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
        Ok(())
    }
}

#[async_trait(?Send)]
impl<R, W> client::async_io::Transport for Connection<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    async fn handshake<'a>(
        &mut self,
        service: Service,
        extra_parameters: &'a [(&'a str, Option<&'a str>)],
    ) -> Result<SetServiceResponse<'_>, client::Error> {
        if self.state.mode == git::ConnectMode::Daemon {
            let mut line_writer = Writer::new(&mut self.writer);
            line_writer.enable_binary_mode();
            line_writer
                .write_all(&git::message::connect(
                    service,
                    self.state.desired_version,
                    &self.state.path,
                    self.state.virtual_host.as_ref(),
                    extra_parameters,
                ))
                .await?;
            line_writer.flush().await?;
        }

        let capabilities::recv::Outcome {
            capabilities,
            refs,
            protocol: actual_protocol,
        } = Capabilities::from_lines_with_version_detection(&mut self.line_provider).await?;
        Ok(SetServiceResponse {
            actual_protocol,
            capabilities,
            refs,
        })
    }

    fn request(
        &mut self,
        write_mode: client::WriteMode,
        on_into_read: client::MessageKind,
        trace: bool,
    ) -> Result<RequestWriter<'_>, client::Error> {
        Ok(RequestWriter::new_from_bufread(
            &mut self.writer,
            Box::new(self.line_provider.as_read_without_sidebands()),
            write_mode,
            on_into_read,
            trace,
        ))
    }
}

impl<R, W> Connection<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    /// Create a connection from the given `read` and `write`, asking for `desired_version` as preferred protocol
    /// and the transfer of the repository at `repository_path`.
    ///
    /// `virtual_host` along with a port to which to connect to, while `mode` determines the kind of endpoint to connect to.
    /// If `trace` is `true`, all packetlines received or sent will be passed to the facilities of the `gix-trace` crate.
    pub fn new(
        read: R,
        write: W,
        desired_version: Protocol,
        repository_path: impl Into<BString>,
        virtual_host: Option<(impl Into<String>, Option<u16>)>,
        mode: git::ConnectMode,
        trace: bool,
    ) -> Self {
        Connection {
            writer: write,
            line_provider: StreamingPeekableIter::new(read, &[PacketLineRef::Flush], trace),
            state: ConnectionState {
                path: repository_path.into(),
                virtual_host: virtual_host.map(|(h, p)| (h.into(), p)),
                desired_version,
                custom_url: None,
                mode,
            },
        }
    }
}

#[cfg(feature = "async-std")]
mod async_net {
    use std::time::Duration;

    use async_std::net::TcpStream;

    use crate::client::{git, Error};

    impl git::Connection<TcpStream, TcpStream> {
        /// Create a new TCP connection using the `git` protocol of `desired_version`, and make a connection to `host`
        /// at `port` for accessing the repository at `path` on the server side.
        /// If `trace` is `true`, all packetlines received or sent will be passed to the facilities of the `gix-trace` crate.
        pub async fn new_tcp(
            host: &str,
            port: Option<u16>,
            path: bstr::BString,
            desired_version: crate::Protocol,
            trace: bool,
        ) -> Result<git::Connection<TcpStream, TcpStream>, Error> {
            let read = async_std::io::timeout(
                Duration::from_secs(5),
                TcpStream::connect(&(host, port.unwrap_or(9418))),
            )
            .await?;
            let write = read.clone();
            Ok(git::Connection::new(
                read,
                write,
                desired_version,
                path,
                None::<(String, _)>,
                git::ConnectMode::Daemon,
                trace,
            ))
        }
    }
}
