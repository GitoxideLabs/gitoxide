use crate::{
    Remote,
    remote::{Connection, connection::AuthenticateFn, connection::ConnectionDetached},
};
use gix_error::{OptionExt, ResultExt, ValidationError, message};
#[cfg(feature = "async-network-client")]
use gix_transport::client::async_io::Transport;
#[cfg(feature = "blocking-network-client")]
use gix_transport::client::blocking_io::Transport;

/// Builder
impl<'remote, 'repo, T> Connection<'remote, '_, 'repo, T>
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
    pub fn with_credentials<'b>(
        self,
        helper: impl FnMut(gix_credentials::helper::Action) -> gix_credentials::protocol::Result + 'b,
    ) -> Connection<'remote, 'b, 'repo, T> {
        Connection {
            remote: self.remote,
            authenticate: Some(Box::new(helper)),
            transport_options: self.transport_options,
            transport: self.transport,
            handshake: self.handshake,
            trace: self.trace,
        }
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
impl<T> ConnectionDetached<'_, T>
where
    T: Transport,
{
    pub(crate) fn configured_credentials_for_current_url(&self, repo: &crate::Repository) -> AuthenticateFn<'static> {
        configured_credentials_for_current_url(repo.clone())
    }
}

/// Mutation
impl<'auth, T> Connection<'_, 'auth, '_, T>
where
    T: Transport,
{
    /// Like [`with_credentials()`](Self::with_credentials()), but without consuming the connection.
    pub fn set_credentials(
        &mut self,
        helper: impl FnMut(gix_credentials::helper::Action) -> gix_credentials::protocol::Result + 'auth,
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
impl<'auth, 'repo, T> Connection<'_, 'auth, 'repo, T>
where
    T: Transport,
{
    /// A utility to return a function that will use this repository's configuration to obtain credentials for `url`,
    /// similar to what `git credential` is doing.
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

    /// A utility to return a function that uses each
    /// [`Get`](gix_credentials::helper::Action::Get) action's context to obtain credentials from this repository's
    /// configuration.
    ///
    /// The transport creates these actions from its current URL, which means authentication naturally follows redirects.
    pub fn configured_credentials_for_current_url(&self) -> AuthenticateFn<'static> {
        configured_credentials_for_current_url(self.remote.repo.clone())
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

    pub(crate) fn into_detached(self) -> ConnectionDetached<'auth, T> {
        ConnectionDetached {
            remote: self.remote.detached(),
            authenticate: self.authenticate,
            transport_options: self.transport_options,
            transport: self.transport,
            handshake: self.handshake,
            trace: self.trace,
        }
    }
}

fn configured_credentials_for_current_url(repo: crate::Repository) -> AuthenticateFn<'static> {
    let mut previous_cascade_and_prompt = None;
    Box::new(move |action| {
        if matches!(&action, gix_credentials::helper::Action::Get(_)) {
            // The handshake creates the `Get` action from the transport's current URL. That URL may
            // differ from the initial remote URL after redirects, and credential configuration can be
            // URL-specific. Configure the cascade for this action URL, then keep it for the matching
            // `Store` or `Erase` follow-up actions whose context is carried as an encoded payload,
            // and is less convenient to use.
            let url = action
                .context()
                .and_then(|ctx| ctx.url.clone().or_else(|| ctx.to_url()))
                .ok_or_raise_erased(|| {
                    ValidationError::new("Either 'url' field or both 'protocol' and 'host' fields must be provided")
                })?;
            let (mut cascade, _action_with_normalized_url, prompt_opts) = repo
                .config_snapshot()
                .credential_helpers(gix_url::parse(&url).or_erased()?)
                .or_raise_erased(|| message("Could not configure credential helpers"))?;
            let outcome = cascade.invoke(action, prompt_opts.clone());
            previous_cascade_and_prompt = Some((cascade, prompt_opts));
            outcome
        } else {
            match previous_cascade_and_prompt.as_mut() {
                Some((cascade, prompt_opts)) => cascade.invoke(action, prompt_opts.clone()),
                None => {
                    gix_trace::warn!(
                        "credential Store/Erase follow-up was invoked without a preceding Get; ignoring advisory action"
                    );
                    Ok(None)
                }
            }
        }
    }) as AuthenticateFn<'_>
}
