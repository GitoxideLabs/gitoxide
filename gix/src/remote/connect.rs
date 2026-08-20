#![allow(clippy::result_large_err)]

use std::borrow::Cow;

use gix_error::{ErrorExt, ResultExt};
#[cfg(feature = "async-network-client")]
use gix_transport::client::async_io::{Transport, connect};
#[cfg(feature = "blocking-network-client")]
use gix_transport::client::blocking_io::{Transport, connect};

use crate::{Remote, config::tree::Protocol, remote::Connection};

/// The error returned by [connect()][crate::Remote::connect()].
pub type Error = gix_error::Error;

/// Establishing connections to remote hosts (without performing a git-handshake).
impl<'repo> Remote<'repo> {
    /// Create a new connection using `transport` to communicate, with `progress` to indicate changes.
    ///
    /// Note that this method expects the `transport` to be created by the user, which would involve the [`url()`](Self::url()).
    /// It's meant to be used when async operation is needed with runtimes of the user's choice.
    pub fn to_connection_with_transport<T>(&self, transport: T) -> Connection<'_, 'static, 'repo, T>
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
    #[gix_protocol::bisync::bisync]
    pub async fn connect(
        &self,
        direction: crate::remote::Direction,
    ) -> Result<Connection<'_, 'static, 'repo, Box<dyn Transport + Send>>, Error> {
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
                    .transpose()
                    .or_raise(|| gix_error::message("Could not obtain options for connecting via ssh"))?
                    .unwrap_or_default(),
                trace: self.repo.config.trace_packet(),
            },
        )
        .await
        .map_err(gix_error::Error::from)?;
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
                    .map_err(|err| {
                        gix_error::Error::from(err.raise(gix_error::message!(
                            "Could not verify that {:?} is a valid git directory before attempting to use it",
                            url.to_bstring()
                        )))
                    })?;
                let (git_dir, _work_dir) = gix_discover::repository::Path::from_dot_git_dir(
                    dir.clone().into_owned(),
                    kind,
                    // precomposed unicode doesn't matter here as long as the produced path is accessible,
                    // which is a given either way.
                    &gix_fs::current_dir(false)
                        .or_raise(|| gix_error::message("Could not obtain the current directory"))?,
                )
                .ok_or_else(|| {
                    gix_error::Error::from_error(gix_error::ValidationError::new_with_input(
                        "Could not access remote repository",
                        gix_path::into_bstr(dir.clone().into_owned()).into_owned(),
                    ))
                })?
                .into_repository_and_work_tree_directories();
                url.path = gix_path::into_bstr(git_dir).into_owned();
            }
            Ok(url)
        }

        let version = crate::config::tree::Protocol::VERSION
            .try_into_protocol_version(self.repo.config.resolved.integer(Protocol::VERSION))
            .map_err(|err| {
                gix_error::Error::from(err.and_raise(gix_error::ValidationError::new(
                    "The given protocol version was invalid. Choose between 1 and 2",
                )))
            })?;

        let url = self
            .url(direction)
            .ok_or_else(|| {
                gix_error::Error::from_error(gix_error::ValidationError::new(format!(
                    "The {} url was missing - don't know where to establish a connection to",
                    direction.as_str()
                )))
            })?
            .to_owned();
        if !self.repo.config.url_scheme().or_erased()?.allow(&url.scheme) {
            return Err(gix_error::Error::from_error(
                gix_error::ValidationError::new_with_input(
                    format!("Protocol {:?} is denied per configuration", url.scheme),
                    url.to_bstring(),
                ),
            ));
        }
        Ok((sanitize(url)?, version))
    }
}
