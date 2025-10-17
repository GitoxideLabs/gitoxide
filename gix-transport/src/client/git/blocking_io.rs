use std::{any::Any, borrow::Cow, error::Error, io::Write};

use bstr::{BStr, BString, ByteVec};
use gix_packetline::PacketLineRef;

use crate::{
    client::{self, capabilities, git, Capabilities, SetServiceResponse},
    Protocol, Service,
};

impl<R, W> client::TransportWithoutIO for git::Connection<R, W>
where
    R: std::io::Read,
    W: std::io::Write,
{
    fn request(
        &mut self,
        write_mode: client::WriteMode,
        on_into_read: client::MessageKind,
        trace: bool,
    ) -> Result<client::RequestWriter<'_>, client::Error> {
        Ok(client::RequestWriter::new_from_bufread(
            &mut self.writer,
            Box::new(self.line_provider.as_read_without_sidebands()),
            write_mode,
            on_into_read,
            trace,
        ))
    }

    fn to_url(&self) -> Cow<'_, BStr> {
        self.custom_url.as_ref().map_or_else(
            || {
                let mut possibly_lossy_url = self.path.clone();
                possibly_lossy_url.insert_str(0, "file://");
                Cow::Owned(possibly_lossy_url)
            },
            |url| Cow::Borrowed(url.as_ref()),
        )
    }

    fn connection_persists_across_multiple_requests(&self) -> bool {
        true
    }

    fn configure(&mut self, _config: &dyn Any) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
        Ok(())
    }
}

impl<R, W> client::Transport for git::Connection<R, W>
where
    R: std::io::Read,
    W: std::io::Write,
{
    fn handshake<'a>(
        &mut self,
        service: Service,
        extra_parameters: &'a [(&'a str, Option<&'a str>)],
    ) -> Result<SetServiceResponse<'_>, client::Error> {
        if self.mode == git::ConnectMode::Daemon {
            let mut line_writer = gix_packetline::Writer::new(&mut self.writer).binary_mode();
            line_writer.write_all(&git::message::connect(
                service,
                self.desired_version,
                &self.path,
                self.virtual_host.as_ref(),
                extra_parameters,
            ))?;
            line_writer.flush()?;
        }

        let capabilities::recv::Outcome {
            capabilities,
            refs,
            protocol: actual_protocol,
        } = Capabilities::from_lines_with_version_detection(&mut self.line_provider)?;
        Ok(SetServiceResponse {
            actual_protocol,
            capabilities,
            refs,
        })
    }
}

impl<R, W> git::Connection<R, W>
where
    R: std::io::Read,
    W: std::io::Write,
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
        git::Connection {
            writer: write,
            line_provider: gix_packetline::read::blocking_io::StreamingPeekableIter::new(
                read,
                &[PacketLineRef::Flush],
                trace,
            ),
            path: repository_path.into(),
            virtual_host: virtual_host.map(|(h, p)| (h.into(), p)),
            desired_version,
            custom_url: None,
            mode,
        }
    }
    pub(crate) fn new_for_spawned_process(
        reader: R,
        writer: W,
        desired_version: Protocol,
        repository_path: impl Into<BString>,
        trace: bool,
    ) -> Self {
        Self::new(
            reader,
            writer,
            desired_version,
            repository_path,
            None::<(&str, _)>,
            git::ConnectMode::Process,
            trace,
        )
    }
}

///
pub mod connect {
    use std::net::{TcpStream, ToSocketAddrs};

    use bstr::BString;

    use crate::client::git;
    /// The error used in [`connect()`].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("An IO error occurred when connecting to the server")]
        Io(#[from] std::io::Error),
        #[error("Could not parse {host:?} as virtual host with format <host>[:port]")]
        VirtualHostInvalid { host: String },
    }

    impl crate::IsSpuriousError for Error {
        fn is_spurious(&self) -> bool {
            match self {
                Error::Io(err) => err.is_spurious(),
                _ => false,
            }
        }
    }

    fn parse_host(input: String) -> Result<(String, Option<u16>), Error> {
        let mut tokens = input.splitn(2, ':');
        Ok(match (tokens.next(), tokens.next()) {
            (Some(host), None) => (host.to_owned(), None),
            (Some(host), Some(port)) => (
                host.to_owned(),
                Some(port.parse().map_err(|_| Error::VirtualHostInvalid { host: input })?),
            ),
            _ => unreachable!("we expect at least one token, the original string"),
        })
    }

    /// Connect to a git daemon running on `host` and optionally `port` and a repository at `path`.
    ///
    /// Use `desired_version` to specify a preferred protocol to use, knowing that it can be downgraded by a server not supporting it.
    /// If `trace` is `true`, all packetlines received or sent will be passed to the facilities of the `gix-trace` crate.
    pub fn connect(
        host: &str,
        path: BString,
        desired_version: crate::Protocol,
        port: Option<u16>,
        trace: bool,
    ) -> Result<git::Connection<TcpStream, TcpStream>, Error> {
        let read = TcpStream::connect_timeout(
            &(host, port.unwrap_or(9418))
                .to_socket_addrs()?
                .next()
                .expect("after successful resolution there is an IP address"),
            std::time::Duration::from_secs(5),
        )?;
        let write = read.try_clone()?;
        let vhost = std::env::var("GIT_OVERRIDE_VIRTUAL_HOST")
            .ok()
            .map(parse_host)
            .transpose()?
            .unwrap_or_else(|| (host.to_owned(), port));
        Ok(git::Connection::new(
            read,
            write,
            desired_version,
            path,
            Some(vhost),
            git::ConnectMode::Daemon,
            trace,
        ))
    }
}

pub use connect::connect;
