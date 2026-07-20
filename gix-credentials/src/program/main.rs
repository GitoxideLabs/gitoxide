use std::ffi::OsString;

use bstr::BString;

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
    type Error = Error;

    fn try_from(value: OsString) -> Result<Self, Self::Error> {
        Ok(match value.to_str() {
            Some("fill" | "get") => Action::Get,
            Some("approve" | "store") => Action::Store,
            Some("reject" | "erase") => Action::Erase,
            _ => return Err(Error::ActionInvalid { name: value }),
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
#[derive(Debug)]
#[expect(missing_docs)]
pub enum Error {
    ActionInvalid {
        name: OsString,
    },
    ActionMissing,
    Helper {
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },
    Io(std::io::Error),
    Context(crate::protocol::context::decode::Error),
    CredentialsMissing {
        url: BString,
    },
    UrlMissing,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ActionInvalid { name } => write!(
                f,
                "Action named {name:?} is invalid, need 'get', 'store', 'erase' or 'fill', 'approve', 'reject'"
            ),
            Error::ActionMissing => f.write_str("The first argument must be the action to perform"),
            Error::Helper { source } => std::fmt::Display::fmt(source, f),
            Error::Io(err) => std::fmt::Display::fmt(err, f),
            Error::Context(err) => std::fmt::Display::fmt(err, f),
            Error::CredentialsMissing { url } => write!(f, "Credentials for {url:?} could not be obtained"),
            Error::UrlMissing => {
                f.write_str("Either 'url' field or both 'protocol' and 'host' fields must be provided")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Helper { source } => source.source(),
            Error::Io(err) => err.source(),
            Error::Context(err) => err.source(),
            Error::ActionInvalid { .. }
            | Error::ActionMissing
            | Error::CredentialsMissing { .. }
            | Error::UrlMissing => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<crate::protocol::context::decode::Error> for Error {
    fn from(err: crate::protocol::context::decode::Error) -> Self {
        Error::Context(err)
    }
}

pub(crate) mod function {
    use std::ffi::OsString;

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
    pub fn main<CredentialsFn, E>(
        args: impl IntoIterator<Item = OsString>,
        mut stdin: impl std::io::Read,
        stdout: impl std::io::Write,
        options: ContextOptions,
        credentials: CredentialsFn,
    ) -> Result<(), Error>
    where
        CredentialsFn: FnOnce(Action, Context) -> Result<Option<Context>, E>,
        E: std::error::Error + Send + Sync + 'static,
    {
        let action: Action = args.into_iter().next().ok_or(Error::ActionMissing)?.try_into()?;
        let mut buf = Vec::<u8>::with_capacity(512);
        stdin.read_to_end(&mut buf)?;
        let ctx = Context::from_bytes(&buf, options)?;
        if ctx.url.is_none() && (ctx.protocol.is_none() || ctx.host.is_none()) {
            return Err(Error::UrlMissing);
        }
        let res = credentials(action, ctx.clone()).map_err(|err| Error::Helper { source: Box::new(err) })?;
        match (action, res) {
            (Action::Get, None) => {
                let ctx_for_error = ctx;
                let url = ctx_for_error
                    .url
                    .clone()
                    .or_else(|| ctx_for_error.to_url())
                    .expect("URL is available either directly or via protocol+host which we checked for");
                return Err(Error::CredentialsMissing { url });
            }
            (Action::Get, Some(mut ctx)) => {
                ctx.options = options;
                ctx.write_to(stdout)?;
            }
            (Action::Erase | Action::Store, None) => {}
            (Action::Erase | Action::Store, Some(_)) => {
                panic!("BUG: credentials helper must not return context for erase or store actions")
            }
        }
        Ok(())
    }
}
