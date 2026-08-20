use bstr::BString;
use gix_error::ErrorExt;

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
pub type Error = gix_error::Exn;

/// Additional context to be passed to the credentials helper.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct Context {
    /// Options controlling how this context is encoded and decoded.
    pub options: ContextOptions,
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

/// Options for encoding and decoding a [`Context`].
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ContextOptions {
    /// If true, carriage returns in credential values are rejected to protect credential-protocol parsing.
    ///
    /// NUL bytes and newlines are always rejected.
    pub protect_protocol: bool,
}

impl Default for ContextOptions {
    fn default() -> Self {
        ContextOptions { protect_protocol: true }
    }
}

/// Convert the outcome of a helper invocation to a helper result, assuring that the identity is complete in the process.
pub fn helper_outcome_to_result(outcome: Option<helper::Outcome>, action: helper::Action) -> Result {
    match (action, outcome) {
        (helper::Action::Get(ctx), None) => Err(identity_missing(ctx)),
        (helper::Action::Get(ctx), Some(mut outcome)) => match outcome.consume_identity() {
            Some(identity) => Ok(Some(Outcome {
                identity,
                next: outcome.next,
            })),
            None => Err(if outcome.quit {
                gix_error::message("The handler asked to stop trying to obtain credentials").raise_erased()
            } else {
                identity_missing(ctx)
            }),
        },
        (helper::Action::Store(_) | helper::Action::Erase(_), _ignore) => Ok(None),
    }
}

fn identity_missing(context: Context) -> Error {
    gix_error::NotFoundError::new(format!(
        "Could not obtain identity for context: {}",
        String::from_utf8_lossy(&context.redacted().to_bstring())
    ))
    .raise_erased()
}

///
pub mod context;
