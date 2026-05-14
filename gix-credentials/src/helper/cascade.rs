use crate::{Program, helper, helper::Cascade, protocol, protocol::Context};

impl Default for Cascade {
    fn default() -> Self {
        Cascade {
            programs: Vec::new(),
            stderr: true,
            use_http_path: false,
            query_user_only: false,
        }
    }
}

/// Initialization
impl Cascade {
    /// Return the programs to run for the current platform.
    ///
    /// These are typically used as basis for all credential cascade invocations, with configured programs following afterwards.
    ///
    /// # Note
    ///
    /// These defaults emulate what typical git installations may use these days, as in fact it's a configurable which comes
    /// from installation-specific configuration files which we cannot know (or guess at best).
    /// This seems like an acceptable trade-off as helpers are ignored if they fail or are not existing.
    pub fn platform_builtin() -> Vec<Program> {
        if cfg!(target_os = "macos") {
            Some("osxkeychain")
        } else if cfg!(target_os = "linux") {
            Some("libsecret")
        } else if cfg!(target_os = "windows") {
            Some("manager-core")
        } else {
            None
        }
        .map(|name| vec![Program::from_custom_definition(name)])
        .unwrap_or_default()
    }
}

/// Builder
impl Cascade {
    /// Extend the list of programs to run `programs`.
    pub fn extend(mut self, programs: impl IntoIterator<Item = Program>) -> Self {
        self.programs.extend(programs);
        self
    }
    /// If `toggle` is true, http(s) urls will use the path portions of the url to obtain a credential for.
    ///
    /// Otherwise, they will only take the user name into account.
    pub fn use_http_path(mut self, toggle: bool) -> Self {
        self.use_http_path = toggle;
        self
    }

    /// If `toggle` is true, a bogus password will be provided to prevent any helper program from prompting for it, nor will
    /// we prompt for the password. The resulting identity will have a bogus password and it's expected to not be used by the
    /// consuming transport.
    pub fn query_user_only(mut self, toggle: bool) -> Self {
        self.query_user_only = toggle;
        self
    }
}

/// Finalize
impl Cascade {
    /// Invoke the cascade by `invoking` each program with `action`, and configuring potential prompts with `prompt` options.
    /// The latter can also be used to disable the prompt entirely when setting the `mode` to [`Disable`][gix_prompt::Mode::Disable];=.
    ///
    /// When _getting_ credentials, all programs are asked until the credentials are complete, stopping the cascade.
    /// When _storing_ or _erasing_ all programs are instructed in order.
    #[allow(clippy::result_large_err)]
    pub fn invoke(&mut self, mut action: helper::Action, mut prompt: gix_prompt::Options<'_>) -> protocol::Result {
        let mut url = action
            .context_mut()
            .map(|ctx| {
                #[allow(clippy::manual_inspect)] /* false positive */
                ctx.destructure_url_in_place(self.use_http_path).map(|ctx| {
                    if self.query_user_only && ctx.password.is_none() {
                        ctx.password = Some("".into());
                    }
                    ctx
                })
            })
            .transpose()?
            .and_then(|ctx| ctx.url.take());

        for program in &mut self.programs {
            program.stderr = self.stderr;
            match helper::invoke::raw(program, &action) {
                Ok(None) => {}
                Ok(Some(stdout)) => {
                    let Context {
                        protocol,
                        host,
                        path,
                        username,
                        password,
                        oauth_refresh_token,
                        password_expiry_utc,
                        url: ctx_url,
                        quit,
                    } = Context::from_bytes(&stdout)?;
                    if let Some(dst_ctx) = action.context_mut() {
                        if let Some(src) = path {
                            dst_ctx.path = Some(src);
                        }
                        if let Some(src) = password_expiry_utc {
                            dst_ctx.password_expiry_utc = Some(src);
                        }
                        for (src, dst) in [
                            (protocol, &mut dst_ctx.protocol),
                            (host, &mut dst_ctx.host),
                            (username, &mut dst_ctx.username),
                            (password, &mut dst_ctx.password),
                            (oauth_refresh_token, &mut dst_ctx.oauth_refresh_token),
                        ] {
                            if let Some(src) = src {
                                *dst = Some(src);
                            }
                        }
                        if let Some(src) = ctx_url {
                            dst_ctx.url = Some(src);
                            url = dst_ctx.destructure_url_in_place(self.use_http_path)?.url.take();
                        }
                        if dst_ctx
                            .password_expiry_utc
                            .is_some_and(|expiry_date| expiry_date < gix_date::Time::now_utc().seconds)
                        {
                            dst_ctx.password_expiry_utc = None;
                            dst_ctx.clear_secrets();
                        }
                        if dst_ctx.username.is_some() && dst_ctx.password.is_some() {
                            break;
                        }
                        if quit.unwrap_or_default() {
                            dst_ctx.quit = quit;
                            break;
                        }
                    }
                }
                Err(helper::Error::CredentialsHelperFailed { .. }) => continue, // ignore helpers that we can't call
                Err(err) if action.context().is_some() => return Err(err.into()), // communication errors are fatal when getting credentials
                Err(_) => {} // for other actions, ignore everything, try the operation
            }
        }

        if prompt.mode != gix_prompt::Mode::Disable {
            if let Some(ctx) = action.context_mut() {
                ctx.url = url;
                if ctx.username.is_none() {
                    let message = ctx.to_prompt("Username");
                    prompt.mode = gix_prompt::Mode::Visible;
                    ctx.username = gix_prompt::ask(&message, &prompt)
                        .map_err(|err| protocol::Error::Prompt {
                            prompt: message,
                            source: err,
                        })?
                        .into();
                }
                if ctx.password.is_none() {
                    let message = ctx.to_prompt("Password");
                    prompt.mode = gix_prompt::Mode::Hidden;
                    ctx.password = gix_prompt::ask(&message, &prompt)
                        .map_err(|err| protocol::Error::Prompt {
                            prompt: message,
                            source: err,
                        })?
                        .into();
                }
            }
        }

        protocol::helper_outcome_to_result(
            action.context().map(|ctx| helper::Outcome {
                username: ctx.username.clone(),
                password: ctx.password.clone(),
                oauth_refresh_token: ctx.oauth_refresh_token.clone(),
                quit: ctx.quit.unwrap_or(false),
                next: ctx.to_owned().into(),
            }),
            action,
        )
    }
}
