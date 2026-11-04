use crate::{
    Remote, Result,
    remote::{Connection, connection::AuthenticateFn, connection::ConnectionDetached},
};
use gix_error::{OptionExt, ResultExt};
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
        helper: impl FnMut(gix_credentials::helper::Action) -> Result<Option<gix_credentials::protocol::Outcome>> + 'b,
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
        configured_credentials_for_current_url(
            repo.clone(),
            self.remote.command_environment.clone(),
            self.remote.working_dir.clone(),
        )
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
        helper: impl FnMut(gix_credentials::helper::Action) -> Result<Option<gix_credentials::protocol::Outcome>> + 'auth,
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
    pub fn configured_credentials(&self, url: gix_url::Url) -> Result<AuthenticateFn<'static>> {
        let (mut cascade, _action_with_normalized_url, mut prompt_opts) =
            self.remote.repo.config_snapshot().credential_helpers(url)?;
        let environment = self.remote.command_environment.clone();
        for program in &mut cascade.programs {
            program.current_dir.clone_from(&self.remote.working_dir);
        }
        prompt_opts.current_dir.clone_from(&self.remote.working_dir);
        apply_prompt_environment(&mut prompt_opts, &environment)?;
        Ok(
            Box::new(move |action| cascade.invoke_with_environment(action, prompt_opts.clone(), &environment))
                as AuthenticateFn<'_>,
        )
    }

    /// A utility to return a function that uses each
    /// [`Get`](gix_credentials::helper::Action::Get) action's context to obtain credentials from this repository's
    /// configuration.
    ///
    /// The transport creates these actions from its current URL, which means authentication naturally follows redirects.
    pub fn configured_credentials_for_current_url(&self) -> AuthenticateFn<'static> {
        configured_credentials_for_current_url(
            self.remote.repo.clone(),
            self.remote.command_environment.clone(),
            self.remote.working_dir.clone(),
        )
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

fn configured_credentials_for_current_url(
    repo: crate::Repository,
    environment: Vec<(std::ffi::OsString, std::ffi::OsString)>,
    working_dir: Option<std::path::PathBuf>,
) -> AuthenticateFn<'static> {
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
                .ok_or_raise(|| {
                    gix_error::validation("Either 'url' field or both 'protocol' and 'host' fields must be provided")
                })?;
            let (mut cascade, _action_with_normalized_url, mut prompt_opts) = repo
                .config_snapshot()
                .credential_helpers(gix_url::parse(&url)?)
                .or_raise(|| gix_error::message("Credential helper configuration is invalid"))?;
            for program in &mut cascade.programs {
                program.current_dir.clone_from(&working_dir);
            }
            prompt_opts.current_dir.clone_from(&working_dir);
            apply_prompt_environment(&mut prompt_opts, &environment)?;
            let outcome = cascade.invoke_with_environment(action, prompt_opts.clone(), &environment);
            previous_cascade_and_prompt = Some((cascade, prompt_opts));
            outcome
        } else {
            match previous_cascade_and_prompt.as_mut() {
                Some((cascade, prompt_opts)) => {
                    cascade.invoke_with_environment(action, prompt_opts.clone(), &environment)
                }
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

pub(super) fn transport_options(
    repo: &crate::Repository,
    url: &crate::bstr::BStr,
    remote_name: Option<&crate::bstr::BStr>,
    environment: &[(std::ffi::OsString, std::ffi::OsString)],
    working_dir: Option<&std::path::Path>,
) -> Result<Option<Box<dyn std::any::Any>>> {
    #[allow(unused_mut)]
    let mut options = repo.transport_options(url, remote_name)?;
    #[cfg(any(
        feature = "blocking-http-transport-reqwest",
        feature = "blocking-http-transport-curl"
    ))]
    if let Some(http) = options
        .as_deref_mut()
        .and_then(|options| options.downcast_mut::<gix_transport::client::blocking_io::http::Options>())
    {
        if let (Some(directory), Some(path)) = (working_dir, http.ssl_ca_info.as_mut()) {
            *path = directory.join(&*path);
        }
        if (working_dir.is_some() || !environment.is_empty())
            && let Some((action, authenticate)) = http.proxy_authenticate.as_mut()
        {
            let url = action
                .context()
                .and_then(|context| context.url.as_ref())
                .expect("proxy configuration supplies a Get action with its URL");
            let (mut cascade, _, mut prompt) = repo.config_snapshot().credential_helpers(gix_url::parse(url)?)?;
            for program in &mut cascade.programs {
                program.current_dir = working_dir.map(std::path::Path::to_owned);
            }
            prompt.current_dir = working_dir.map(std::path::Path::to_owned);
            apply_prompt_environment(&mut prompt, environment)?;
            let environment = environment.to_vec();
            *authenticate = std::sync::Arc::new(std::sync::Mutex::new(move |action| {
                cascade.invoke_with_environment(action, prompt.clone(), &environment)
            }));
        }
    }
    #[cfg(not(any(
        feature = "blocking-http-transport-reqwest",
        feature = "blocking-http-transport-curl"
    )))]
    let _ = (environment, working_dir);
    Ok(options)
}

fn apply_prompt_environment(
    options: &mut gix_prompt::Options,
    environment: &[(std::ffi::OsString, std::ffi::OsString)],
) -> Result<()> {
    let value = |name: &str| {
        environment
            .iter()
            .rev()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    };
    if let Some(askpass) = value("GIT_ASKPASS") {
        options.askpass = Some(askpass.into());
    } else if options.askpass.is_none()
        && let Some(askpass) = value("SSH_ASKPASS")
    {
        options.askpass = Some(askpass.into());
    }
    if let Some(allow) = value("GIT_TERMINAL_PROMPT") {
        options.mode = if gix_config::Boolean::try_from(allow.clone()).or_error()?.0 {
            gix_prompt::Mode::Hidden
        } else {
            gix_prompt::Mode::Disable
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn operation_environment_can_reenable_terminal_prompts() -> gix_testtools::TestResult {
        let mut options = gix_prompt::Options {
            mode: gix_prompt::Mode::Disable,
            ..Default::default()
        };
        super::apply_prompt_environment(&mut options, &[("GIT_TERMINAL_PROMPT".into(), "true".into())])?;
        assert_eq!(options.mode, gix_prompt::Mode::Hidden);
        super::apply_prompt_environment(&mut options, &[("GIT_TERMINAL_PROMPT".into(), "false".into())])?;
        assert_eq!(options.mode, gix_prompt::Mode::Disable);
        Ok(())
    }

    #[cfg(all(
        unix,
        any(
            feature = "blocking-http-transport-reqwest",
            feature = "blocking-http-transport-curl"
        )
    ))]
    #[test]
    fn operation_directory_reaches_http_ca_paths_and_proxy_credentials() -> gix_testtools::TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let directory = gix_testtools::tempfile::tempdir()?;
        gix_testtools::git(directory.path(), "init --bare")?;
        std::fs::write(directory.path().join("marker"), "")?;
        let absolute = directory.path().join("absolute.pem");
        for path in [std::path::Path::new("relative.pem"), absolute.as_path()] {
            let repo = crate::open_opts(
                directory.path(),
                crate::open::Options::isolated().config_overrides([
                    format!("http.sslCAInfo={}", path.display()),
                    "http.proxy=http://user@proxy.example.com".into(),
                    "credential.helper=!f() { test -f marker && printf 'password=%s\\n' \"$GIX_CWD_PASSWORD\"; }; f"
                        .into(),
                ]),
            )?;
            let get_options = |environment: &[(std::ffi::OsString, std::ffi::OsString)], working_dir| {
                super::transport_options(&repo, "https://example.com".into(), None, environment, working_dir)
            };
            let defaults = get_options(&[], None)?.expect("HTTP options are available");
            let defaults = defaults
                .downcast_ref::<gix_transport::client::blocking_io::http::Options>()
                .expect("HTTP options");
            assert_eq!(
                defaults.ssl_ca_info.as_deref(),
                Some(path),
                "the default keeps ambient-directory behavior"
            );
            let configured = get_options(
                &[("GIX_CWD_PASSWORD".into(), "operation-secret".into())],
                Some(directory.path()),
            )?
            .expect("HTTP options are available");
            let configured = configured
                .downcast_ref::<gix_transport::client::blocking_io::http::Options>()
                .expect("HTTP options");
            assert_eq!(
                configured.ssl_ca_info,
                Some(directory.path().join(path)),
                "relative CA files use the operation directory; absolute paths remain absolute"
            );
            let (action, authenticate) = configured
                .proxy_authenticate
                .as_ref()
                .expect("proxy authentication is configured");
            let outcome = authenticate.lock().expect("no concurrent authentication")(action.clone())?
                .expect("the relative helper supplies the proxy password");
            assert_eq!(outcome.identity.password, "operation-secret");
        }
        Ok(())
    }
}
