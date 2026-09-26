use bstr::BString;
use gix_error::ErrorExt;
use gix_error::Result;

use crate::helper;

/// The outcome of the credentials top-level functions to obtain a complete identity.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Outcome {
    /// The identity provide by the helper.
    pub identity: gix_sec::identity::Account,
    /// A handle to the action to perform next in another call to [`helper::invoke()`][crate::helper::invoke()].
    pub next: helper::NextAction,
}

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
    /// HTTP `WWW-Authenticate` challenges, in server order, passed to helpers as `wwwauth[]`.
    ///
    /// Helpers can use these to select an authentication method or a stored account without prompting.
    /// These values are input to helpers and are discarded once the cascade obtains a complete identity.
    pub www_authenticate: Vec<BString>,
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
pub fn helper_outcome_to_result(outcome: Option<helper::Outcome>, action: helper::Action) -> Result<Option<Outcome>> {
    match (action, outcome) {
        (helper::Action::Get(ctx), None) => Err(identity_missing(ctx).into()),
        (helper::Action::Get(ctx), Some(mut outcome)) => match outcome.consume_identity() {
            Some(identity) => Ok(Some(Outcome {
                identity,
                next: outcome.next,
            })),
            None => Err(if outcome.quit {
                gix_error::message("The handler asked to stop trying to obtain credentials").raise()
            } else {
                identity_missing(ctx).into()
            }),
        },
        (helper::Action::Store(_) | helper::Action::Erase(_), _ignore) => Ok(None),
    }
}

fn identity_missing(context: Context) -> gix_error::Exn {
    let mut buf = Vec::new();
    // Invalid protocol values must not prevent reporting the missing identity.
    context.redacted().write_to(&mut buf).ok();
    gix_error::not_found(format!(
        "Could not obtain identity for context: {}",
        String::from_utf8_lossy(&buf)
    ))
    .raise_erased()
}

///
pub mod context;
