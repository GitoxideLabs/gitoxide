use std::borrow::Cow;

use crate::error::{ResultExt, bail, message};
#[cfg(feature = "async-network-client")]
use gix_transport::client::async_io::{Transport, connect};
#[cfg(feature = "blocking-network-client")]
use gix_transport::client::blocking_io::{Transport, connect};

use crate::{Error, Remote, Result, config::tree::Protocol, remote::Connection};

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
    /// with `2` being the default. Trusted `remote.<name>.uploadpack` and `receivepack`
    /// override the service command for local and SSH transports.
    ///
    /// The transport used for connection can be configured via `transport_mut().configure()` assuming the actually
    /// used transport is well known. If that's not the case, the transport can be created by hand and passed to
    /// [to_connection_with_transport()][Self::to_connection_with_transport()].
    #[cfg(any(feature = "blocking-network-client", feature = "async-network-client-async-std"))]
    #[gix_protocol::bisync::bisync]
    pub async fn connect(
        &self,
        direction: crate::remote::Direction,
    ) -> Result<Connection<'_, 'static, 'repo, Box<dyn Transport + Send>>> {
        let (url, version) = self.sanitized_url_and_version(direction)?;
        #[cfg(feature = "blocking-network-client")]
        let scheme_is_ssh = url.scheme == gix_url::Scheme::Ssh;
        #[cfg(feature = "blocking-network-client")]
        let service_command = if scheme_is_ssh || url.scheme == gix_url::Scheme::File {
            self.service_command(direction)?
        } else {
            None
        };
        let options = connect::Options {
            version,
            working_dir: self.working_dir.clone(),
            #[cfg(feature = "blocking-network-client")]
            ssh: scheme_is_ssh
                .then(|| self.ssh_connect_options())
                .transpose()
                .or_raise(|| message("Could not obtain options for connecting via ssh"))?
                .unwrap_or_default(),
            trace: self.repo.config.trace_packet(),
        };
        #[cfg(feature = "blocking-network-client")]
        let mut transport = connect::connect_with_environment(url, options, &self.command_environment)?;
        #[cfg(feature = "blocking-network-client")]
        if let Some(command) = service_command {
            transport.configure(&gix_transport::client::blocking_io::file::Options { command: Some(command) })?;
        }
        #[cfg(feature = "async-network-client")]
        let transport = connect::connect(url, options).await?;
        Ok(self.to_connection_with_transport(transport))
    }

    /// Produce the sanitized URL and protocol version to use as obtained by querying the repository configuration.
    ///
    /// This can be useful when using custom transports to allow additional configuration.
    /// Repository path resolution failures and denied protocols include the path or URL bytes as `input`
    /// [metadata](gix_error::Error::metadata()). A local URL with a configured service command
    /// is passed to that command without requiring it to name an existing repository.
    pub fn sanitized_url_and_version(
        &self,
        direction: crate::remote::Direction,
    ) -> Result<(gix_url::Url, gix_protocol::transport::Protocol)> {
        fn sanitize(mut url: gix_url::Url, working_dir: Option<&std::path::Path>) -> Result<gix_url::Url> {
            if url.scheme == gix_url::Scheme::File {
                let mut dir = gix_path::to_native_path_on_windows(Cow::Borrowed(url.path.as_ref()))?;
                let current_dir = match working_dir {
                    Some(directory) => Cow::Borrowed(directory),
                    None => Cow::Owned(
                        gix_fs::current_dir(false).or_raise(|| message("Could not obtain the current directory"))?,
                    ),
                };
                if working_dir.is_some() {
                    dir = Cow::Owned(current_dir.join(dir.as_ref()));
                }
                let kind = gix_discover::is_git(dir.as_ref())
                    .or_else(|_| {
                        dir.to_mut().push(gix_discover::DOT_GIT_DIR);
                        gix_discover::is_git(dir.as_ref())
                    })
                    .or_raise(|| {
                        message!(
                            "Could not verify that {:?} is a valid git directory before attempting to use it",
                            url.to_bstring()
                        )
                    })?;
                let Some(repository_path) = gix_discover::repository::Path::from_dot_git_dir(
                    dir.clone().into_owned(),
                    kind,
                    // precomposed unicode doesn't matter here as long as the produced path is accessible,
                    // which is a given either way.
                    &current_dir,
                ) else {
                    return Err(message("Could not access remote repository")
                        .with_input(gix_path::into_bstr(dir.into_owned())?.into_owned())
                        .validation_error());
                };
                let (git_dir, _work_dir) = repository_path.into_repository_and_work_tree_directories();
                url.path = gix_path::into_bstr(git_dir)?.into_owned();
            }
            Ok(url)
        }

        let version = crate::config::tree::Protocol::VERSION
            .try_into_protocol_version(self.repo.config.resolved.integer(Protocol::VERSION))?;

        let url = self
            .url(direction)
            .ok_or_else(|| {
                Error::from_error(gix_error::validation(format!(
                    "The {} url was missing - don't know where to establish a connection to",
                    direction.as_str()
                )))
            })?
            .to_owned();
        if !self.repo.config.url_scheme()?.allow(&url.scheme) {
            bail!(
                "Protocol {:?} is denied per configuration"
                    .permission_denied()
                    .with_input(url.to_bstring()),
                url.scheme
            );
        }
        let url = if url.scheme == gix_url::Scheme::File && self.service_command(direction)?.is_some() {
            url
        } else {
            sanitize(url, self.working_dir.as_deref())?
        };
        Ok((url, version))
    }
}

impl Remote<'_> {
    pub(crate) fn service_command(&self, direction: crate::remote::Direction) -> Result<Option<std::ffi::OsString>> {
        use crate::config::tree::{Key, Remote};

        let Some(name) = self.name() else {
            return Ok(None);
        };
        let key = match direction {
            crate::remote::Direction::Fetch => &Remote::UPLOAD_PACK,
            crate::remote::Direction::Push => &Remote::RECEIVE_PACK,
        };
        self.repo
            .config
            .resolved
            .string_filter_by(
                "remote",
                Some(name.as_bstr()),
                key.name(),
                self.repo.filter_config_section(),
            )
            .map(|command| gix_path::from_bstr(&command).map(|path| path.into_owned().into_os_string()))
            .transpose()
    }
}

#[cfg(feature = "blocking-network-client")]
impl Remote<'_> {
    fn ssh_connect_options(&self) -> Result<gix_transport::client::blocking_io::ssh::connect::Options> {
        let mut options = self.repo.ssh_connect_options()?;
        let value = |name: &str| {
            self.command_environment
                .iter()
                .rev()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value)
        };
        if let Some(command) = value("GIT_SSH_COMMAND") {
            options.command = Some(command.clone());
            options.disallow_shell = false;
        } else if (options.command.is_none() || options.disallow_shell)
            && let Some(command) = value("GIT_SSH")
        {
            options.command = Some(command.clone());
            options.disallow_shell = true;
        }
        if let Some(variant) = value("GIT_SSH_VARIANT") {
            options.kind = crate::config::tree::Ssh::VARIANT
                .try_into_variant(gix_path::into_bstr(std::path::Path::new(variant))?)?;
        }
        Ok(options)
    }
}
