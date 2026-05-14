use crate::{
    Remote,
    remote::{Connection, connection::AuthenticateFn},
};
#[cfg(feature = "async-network-client")]
use gix_transport::client::async_io::Transport;
#[cfg(feature = "blocking-network-client")]
use gix_transport::client::blocking_io::Transport;

/// Builder
impl<'a, T> Connection<'a, '_, T>
where
    T: Transport,
{
    /// Set a custom credentials callback to provide credentials if the remotes require authentication.
    ///
    /// Otherwise, we will use the git configuration to perform the same task as the `git credential` helper program,
    /// which is calling other helper programs in succession while resorting to a prompt to obtain credentials from the
    /// user.
    ///
    /// A custom function may also be used to prevent accessing resources with authentication.
    ///
    /// Use the [`configured_credentials()`](Connection::configured_credentials()) method to obtain the implementation
    /// that would otherwise be used, which can be useful to proxy the default configuration and obtain information about the
    /// URLs to authenticate with.
    pub fn with_credentials(
        mut self,
        helper: impl FnMut(gix_credentials::helper::Action) -> gix_credentials::protocol::Result + 'a,
    ) -> Self {
        self.authenticate = Some(Box::new(helper));
        self
    }

    /// Provide configuration to be used before the first handshake is conducted.
    /// It's typically created by initializing it with [`Repository::transport_options()`](crate::Repository::transport_options()),
    /// which is also the default if this isn't set explicitly. Note that all the default configuration is created from `git`
    /// configuration, which can also be manipulated through overrides to affect the default configuration.
    ///
    /// Use this method to provide transport configuration with custom backend configuration that is not configurable by other means and
    /// custom to the application at hand.
    pub fn with_transport_options(mut self, config: Box<dyn std::any::Any>) -> Self {
        self.transport_options = Some(config);
        self
    }
}

/// Mutation
impl<'a, T> Connection<'a, '_, T>
where
    T: Transport,
{
    /// Like [`with_credentials()`](Self::with_credentials()), but without consuming the connection.
    pub fn set_credentials(
        &mut self,
        helper: impl FnMut(gix_credentials::helper::Action) -> gix_credentials::protocol::Result + 'a,
    ) -> &mut Self {
        self.authenticate = Some(Box::new(helper));
        self
    }

    /// Like [`with_transport_options()`](Self::with_transport_options()), but without consuming the connection.
    pub fn set_transport_options(&mut self, config: Box<dyn std::any::Any>) -> &mut Self {
        self.transport_options = Some(config);
        self
    }
}

/// Access
impl<'repo, T> Connection<'_, 'repo, T>
where
    T: Transport,
{
    /// A utility to return a function that will use this repository's configuration to obtain credentials, similar to
    /// what `git credential` is doing.
    ///
    /// It's meant to be used by users of the [`with_credentials()`](Self::with_credentials()) builder to gain access to the
    /// default way of handling credentials, which they can call as fallback.
    pub fn configured_credentials(
        &self,
        url: gix_url::Url,
    ) -> Result<AuthenticateFn<'static>, crate::config::credential_helpers::Error> {
        let (mut cascade, _action_with_normalized_url, prompt_opts) =
            self.remote.repo.config_snapshot().credential_helpers(url)?;
        Ok(Box::new(move |action| cascade.invoke(action, prompt_opts.clone())) as AuthenticateFn<'_>)
    }
    /// Return the underlying remote that instantiate this connection.
    pub fn remote(&self) -> &Remote<'repo> {
        self.remote
    }

    /// Provide a mutable transport to allow interacting with it according to its actual type.
    /// Note that the caller _should not_ call [`configure()`](gix_protocol::transport::client::TransportWithoutIO::configure())
    /// as we will call it automatically before performing the handshake. Instead, to bring in custom configuration,
    /// call [`with_transport_options()`](Connection::with_transport_options()).
    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport.inner
    }
}
