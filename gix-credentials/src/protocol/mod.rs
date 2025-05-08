use bstr::BString;

use crate::helper;

/// The outcome of the credentials top-level functions to obtain a complete identity.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Outcome {
    /// The identity provide by the helper.
    pub identity: gix_sec::identity::Account,
    /// A handle to the action to perform next in another call to [`helper::invoke()`][crate::helper::invoke()].
    pub next: helper::NextAction,
}

/// The Result type used in credentials top-level functions to obtain a complete identity.
pub type Result = std::result::Result<Option<Outcome>, Error>;

/// The error returned top-level credential functions.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    UrlParse(#[from] gix_url::parse::Error),
    #[error("The 'url' field must be set when performing a 'get/fill' action")]
    UrlMissing,
    #[error(transparent)]
    ContextDecode(#[from] context::decode::Error),
    #[error(transparent)]
    InvokeHelper(#[from] helper::Error),
    #[error("Could not obtain identity for context: {}", { let mut buf = Vec::<u8>::new(); context.write_to(&mut buf).ok(); String::from_utf8_lossy(&buf).into_owned() })]
    IdentityMissing { context: Context },
    #[error("The handler asked to stop trying to obtain credentials")]
    Quit,
    #[error("Couldn't obtain {prompt}")]
    Prompt { prompt: String, source: gix_prompt::Error },
}

/// Additional context to be passed to the credentials helper.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct Context {
    /// The protocol over which the credential will be used (e.g., https).
    pub protocol: Option<String>,
    /// The remote hostname for a network credential. This includes the port number if one was specified (e.g., "example.com:8088").
    pub host: Option<String>,
    /// The path with which the credential will be used. E.g., for accessing a remote https repository, this will be the repository’s path on the server.
    /// It can also be a path on the file system.
    pub path: Option<BString>,
    /// The credential’s username, if we already have one (e.g., from a URL, the configuration, the user, or from a previously run helper).
    pub username: Option<String>,
    /// The credential’s password, if we are asking it to be stored.
    pub password: Option<String>,
    /// An OAuth refresh token that may accompany a password. It is to be treated confidentially, just like the password.
    pub oauth_refresh_token: Option<String>,
    /// The expiry date of OAuth tokens as seconds from Unix epoch.
    pub password_expiry_utc: Option<gix_date::SecondsSinceUnixEpoch>,
    /// When this special attribute is read by git credential, the value is parsed as a URL and treated as if its constituent
    /// parts were read (e.g., url=<https://example.com> would behave as if
    /// protocol=https and host=example.com had been provided). This can help callers avoid parsing URLs themselves.
    pub url: Option<BString>,
    /// If true, the caller should stop asking for credentials immediately without calling more credential helpers in the chain.
    pub quit: Option<bool>,
}

/// Convert the outcome of a helper invocation to a helper result, assuring that the identity is complete in the process.
#[allow(clippy::result_large_err)]
pub fn helper_outcome_to_result(outcome: Option<helper::Outcome>, action: helper::Action) -> Result {
    match (action, outcome) {
        (helper::Action::Get(ctx), None) => Err(Error::IdentityMissing {
            context: ctx.redacted(),
        }),
        (helper::Action::Get(ctx), Some(mut outcome)) => match outcome.consume_identity() {
            Some(identity) => Ok(Some(Outcome {
                identity,
                next: outcome.next,
            })),
            None => Err(if outcome.quit {
                Error::Quit
            } else {
                Error::IdentityMissing {
                    context: ctx.redacted(),
                }
            }),
        },
        (helper::Action::Store(_) | helper::Action::Erase(_), _ignore) => Ok(None),
    }
}

///
pub mod context;
