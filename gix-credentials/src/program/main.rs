use std::ffi::OsString;

use gix_error::ValidationError;

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
    type Error = ValidationError;

    fn try_from(value: OsString) -> Result<Self, Self::Error> {
        Ok(match value.to_str() {
            Some("fill" | "get") => Action::Get,
            Some("approve" | "store") => Action::Store,
            Some("reject" | "erase") => Action::Erase,
            _ => {
                return Err(ValidationError::new_with_input(
                    "Action is invalid, need 'get', 'store', 'erase' or 'fill', 'approve', 'reject'",
                    value.as_encoded_bytes().to_owned(),
                ));
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

/// The error of [`main()`][crate::program::main()].
pub type Error = gix_error::Exn;

pub(crate) mod function {
    use std::ffi::OsString;

    use gix_error::{ErrorExt, ResultExt, ValidationError};

    use crate::{
        program::main::{Action, Error},
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
    ) -> Result<(), Error>
    where
        CredentialsFn: FnOnce(Action, Context) -> Result<Option<Context>, gix_error::Exn>,
    {
        let action = args
            .into_iter()
            .next()
            .ok_or_else(|| ValidationError::new("The first argument must be the action to perform").raise_erased())?;
        let action = Action::try_from(action).or_erased()?;
        let mut buf = Vec::<u8>::with_capacity(512);
        stdin.read_to_end(&mut buf).or_erased()?;
        let ctx = Context::from_bytes(&buf, options).or_erased()?;
        if ctx.url.is_none() && (ctx.protocol.is_none() || ctx.host.is_none()) {
            return Err(ValidationError::new(
                "Either 'url' field or both 'protocol' and 'host' fields must be provided",
            )
            .raise_erased());
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
                return Err(
                    gix_error::NotFoundError::new(format!("Credentials for {url:?} could not be obtained"))
                        .raise_erased(),
                );
            }
            (Action::Get, Some(mut ctx)) => {
                ctx.options = options;
                ctx.write_to(stdout).or_erased()?;
            }
            (Action::Erase | Action::Store, None) => {}
            (Action::Erase | Action::Store, Some(_)) => {
                panic!("BUG: credentials helper must not return context for erase or store actions")
            }
        }
        Ok(())
    }
}
