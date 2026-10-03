use std::ffi::OsString;

use gix_error::{Result, bail, validation};

/// The action passed to the credential helper implementation in [`main()`][crate::program::main()].
#[derive(Debug, Copy, Clone)]
pub enum Action {
    /// Get credentials for a url.
    Get,
    /// Store credentials provided in the given context.
    Store,
    /// Erase credentials identified by the given context.
    Erase,
}

impl TryFrom<OsString> for Action {
    type Error = gix_error::Error;

    /// Invalid actions return a [`gix_error::Class::Validation`] error.
    /// The action's [encoded bytes](std::ffi::OsStr::as_encoded_bytes) are retained as `input`
    /// [metadata](gix_error::Error::metadata()).
    fn try_from(value: OsString) -> Result<Self> {
        Ok(match value.to_str() {
            Some("fill" | "get") => Action::Get,
            Some("approve" | "store") => Action::Store,
            Some("reject" | "erase") => Action::Erase,
            _ => {
                bail!(
                    validation("Action is invalid, need 'get', 'store', 'erase' or 'fill', 'approve', 'reject'",)
                        .with_input(value.as_encoded_bytes())
                );
            }
        })
    }
}

impl Action {
    /// Return ourselves as string representation, similar to what would be passed as argument to a credential helper.
    pub fn as_str(&self) -> &'static str {
        match self {
            Action::Get => "get",
            Action::Store => "store",
            Action::Erase => "erase",
        }
    }
}

pub(crate) mod function {
    use gix_error::Result;
    use std::ffi::OsString;

    use gix_error::{OptionExt, ResultExt, bail, validation};

    use crate::{
        program::main::Action,
        protocol::{Context, ContextOptions},
    };

    /// Invoke a custom credentials helper which receives program `args`, with the first argument being the
    /// action to perform (as opposed to the program name).
    /// Then read context information from `stdin` and if the action is `Action::Get`, then write the result to `stdout`.
    /// `credentials` is the API version of such call, where`Ok(Some(context))` returns credentials, and `Ok(None)` indicates
    /// no credentials could be found for `url`, which is always set when called.
    ///
    /// Call this function from a programs `main`, passing `std::env::args_os()`, `stdin()` and `stdout` accordingly, along with
    /// the context encoding `options` and your own helper implementation.
    pub fn main<CredentialsFn>(
        args: impl IntoIterator<Item = OsString>,
        mut stdin: impl std::io::Read,
        stdout: impl std::io::Write,
        options: ContextOptions,
        credentials: CredentialsFn,
    ) -> Result
    where
        CredentialsFn: FnOnce(Action, Context) -> Result<Option<Context>>,
    {
        let action = args
            .into_iter()
            .next()
            .ok_or_raise(|| validation("The first argument must be the action to perform"))?;
        let action = Action::try_from(action)?;
        let mut buf = Vec::<u8>::with_capacity(512);
        stdin.read_to_end(&mut buf).or_error()?;
        let ctx = Context::from_bytes(&buf, options)?;
        if ctx.url.is_none() && (ctx.protocol.is_none() || ctx.host.is_none()) {
            bail!(validation(
                "Either 'url' field or both 'protocol' and 'host' fields must be provided"
            ));
        }
        let res = credentials(action, ctx.clone())?;
        match (action, res) {
            (Action::Get, None) => {
                let ctx_for_error = ctx;
                let url = ctx_for_error
                    .url
                    .clone()
                    .or_else(|| ctx_for_error.to_url())
                    .expect("URL is available either directly or via protocol+host which we checked for");
                bail!("Credentials for {url:?} could not be obtained".unauthenticated());
            }
            (Action::Get, Some(mut ctx)) => {
                ctx.options = options;
                ctx.write_to(stdout).or_error()?;
            }
            (Action::Erase | Action::Store, None) => {}
            (Action::Erase | Action::Store, Some(_)) => {
                panic!("BUG: credentials helper must not return context for erase or store actions")
            }
        }
        Ok(())
    }
}
