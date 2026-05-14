#![allow(clippy::result_large_err)]

use std::borrow::Cow;

#[cfg(feature = "async-network-client")]
use gix_transport::client::async_io::{Transport, connect};
#[cfg(feature = "blocking-network-client")]
use gix_transport::client::blocking_io::{Transport, connect};

use crate::{Remote, config::tree::Protocol, remote::Connection};

mod error {
    use super::connect;
    use crate::{bstr::BString, config, remote};

    /// The error returned by [connect()][crate::Remote::connect()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Could not obtain options for connecting via ssh")]
        SshOptions(#[from] config::ssh_connect_options::Error),
        #[error("Could not obtain the current directory")]
        CurrentDir(#[from] std::io::Error),
        #[error("Could not access remote repository at \"{}\"", directory.display())]
        InvalidRemoteRepositoryPath { directory: std::path::PathBuf },
        #[error(transparent)]
        SchemePermission(#[from] config::protocol::allow::Error),
        #[error("Protocol {scheme:?} of url {url:?} is denied per configuration")]
        ProtocolDenied { url: BString, scheme: gix_url::Scheme },
        #[error(transparent)]
        Connect(#[from] connect::Error),
        #[error("The {} url was missing - don't know where to establish a connection to", direction.as_str())]
        MissingUrl { direction: remote::Direction },
        #[error("The given protocol version was invalid. Choose between 1 and 2")]
        UnknownProtocol { source: config::key::GenericErrorWithValue },
        #[error("Could not verify that \"{}\" url is a valid git directory before attempting to use it", url.to_bstring())]
        FileUrl {
            source: Box<gix_discover::is_git::Error>,
            url: gix_url::Url,
        },
    }

    impl gix_protocol::transport::IsSpuriousError for Error {
        /// Return `true` if retrying might result in a different outcome due to IO working out differently.
        fn is_spurious(&self) -> bool {
            match self {
                Error::Connect(err) => err.is_spurious(),
                _ => false,
            }
        }
    }
}
pub use error::Error;

/// Establishing connections to remote hosts (without performing a git-handshake).
impl<'repo> Remote<'repo> {
    /// Create a new connection using `transport` to communicate, with `progress` to indicate changes.
    ///
    /// Note that this method expects the `transport` to be created by the user, which would involve the [`url()`](Self::url()).
    /// It's meant to be used when async operation is needed with runtimes of the user's choice.
    pub fn to_connection_with_transport<T>(&self, transport: T) -> Connection<'_, 'repo, T>
    where
        T: Transport,
    {
        let trace = self.repo.config.trace_packet();
        Connection {
            remote: self,
            authenticate: None,
            transport_options: None,
            handshake: None,
            transport: gix_protocol::SendFlushOnDrop::new(transport, trace),
            trace,
        }
    }

    /// Connect to the url suitable for `direction` and return a handle through which operations can be performed.
    ///
    /// Note that the `protocol.version` configuration key affects the transport protocol used to connect,
    /// with `2` being the default.
    ///
    /// The transport used for connection can be configured via `transport_mut().configure()` assuming the actually
    /// used transport is well known. If that's not the case, the transport can be created by hand and passed to
    /// [to_connection_with_transport()][Self::to_connection_with_transport()].
    #[cfg(any(feature = "blocking-network-client", feature = "async-network-client-async-std"))]
    #[gix_protocol::maybe_async::maybe_async]
    pub async fn connect(
        &self,
        direction: crate::remote::Direction,
    ) -> Result<Connection<'_, 'repo, Box<dyn Transport + Send>>, Error> {
        let (url, version) = self.sanitized_url_and_version(direction)?;
        #[cfg(feature = "blocking-network-client")]
        let scheme_is_ssh = url.scheme == gix_url::Scheme::Ssh;
        let transport = connect::connect(
            url,
            connect::Options {
                version,
                #[cfg(feature = "blocking-network-client")]
                ssh: scheme_is_ssh
                    .then(|| self.repo.ssh_connect_options())
                    .transpose()?
                    .unwrap_or_default(),
                trace: self.repo.config.trace_packet(),
            },
        )
        .await?;
        Ok(self.to_connection_with_transport(transport))
    }

    /// Produce the sanitized URL and protocol version to use as obtained by querying the repository configuration.
    ///
    /// This can be useful when using custom transports to allow additional configuration.
    pub fn sanitized_url_and_version(
        &self,
        direction: crate::remote::Direction,
    ) -> Result<(gix_url::Url, gix_protocol::transport::Protocol), Error> {
        fn sanitize(mut url: gix_url::Url) -> Result<gix_url::Url, Error> {
            if url.scheme == gix_url::Scheme::File {
                let mut dir = gix_path::to_native_path_on_windows(Cow::Borrowed(url.path.as_ref()));
                let kind = gix_discover::is_git(dir.as_ref())
                    .or_else(|_| {
                        dir.to_mut().push(gix_discover::DOT_GIT_DIR);
                        gix_discover::is_git(dir.as_ref())
                    })
                    .map_err(|err| Error::FileUrl {
                        source: err.into(),
                        url: url.clone(),
                    })?;
                let (git_dir, _work_dir) = gix_discover::repository::Path::from_dot_git_dir(
                    dir.clone().into_owned(),
                    kind,
                    // precomposed unicode doesn't matter here as long as the produced path is accessible,
                    // which is a given either way.
                    &gix_fs::current_dir(false)?,
                )
                .ok_or_else(|| Error::InvalidRemoteRepositoryPath {
                    directory: dir.into_owned(),
                })?
                .into_repository_and_work_tree_directories();
                url.path = gix_path::into_bstr(git_dir).into_owned();
            }
            Ok(url)
        }

        let version = crate::config::tree::Protocol::VERSION
            .try_into_protocol_version(self.repo.config.resolved.integer(Protocol::VERSION))
            .map_err(|err| Error::UnknownProtocol { source: err })?;

        let url = self.url(direction).ok_or(Error::MissingUrl { direction })?.to_owned();
        if !self.repo.config.url_scheme()?.allow(&url.scheme) {
            return Err(Error::ProtocolDenied {
                url: url.to_bstring(),
                scheme: url.scheme,
            });
        }
        Ok((sanitize(url)?, version))
    }
}
