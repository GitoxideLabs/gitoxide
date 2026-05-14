use bstr::{BStr, BString};

use crate::{Program, protocol, protocol::Context};

/// A list of helper programs to run in order to obtain credentials.
#[allow(dead_code)]
#[derive(Debug)]
pub struct Cascade {
    /// The programs to run in order to obtain credentials
    pub programs: Vec<Program>,
    /// If true, stderr is enabled when `programs` are run, which is the default.
    pub stderr: bool,
    /// If true, http(s) urls will take their path portion into account when obtaining credentials. Default is false.
    /// Other protocols like ssh will always use the path portion.
    pub use_http_path: bool,
    /// If true, default false, when getting credentials, we will set a bogus password to only obtain the user name.
    /// Storage and cancellation work the same, but without a password set.
    pub query_user_only: bool,
}

/// The outcome of the credentials helper [invocation][crate::helper::invoke()].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Outcome {
    /// The username to use in the identity, if set.
    pub username: Option<String>,
    /// The password to use in the identity, if set.
    pub password: Option<String>,
    /// An OAuth refresh token that may accompany a password. It is to be treated confidentially, just like the password.
    pub oauth_refresh_token: Option<String>,
    /// If set, the helper asked to stop the entire process, whether the identity is complete or not.
    pub quit: bool,
    /// A handle to the action to perform next in another call to [`helper::invoke()`][crate::helper::invoke()].
    pub next: NextAction,
}

impl Outcome {
    /// Try to fetch username _and_ password to form an identity. This will fail if one of them is not set.
    ///
    /// This does nothing if only one of the fields is set, or consume both.
    pub fn consume_identity(&mut self) -> Option<gix_sec::identity::Account> {
        if self.username.is_none() || self.password.is_none() {
            return None;
        }
        self.username
            .take()
            .zip(self.password.take())
            .map(|(username, password)| gix_sec::identity::Account {
                username,
                password,
                oauth_refresh_token: self.oauth_refresh_token.take(),
            })
    }
}

/// The Result type used in [`invoke()`][crate::helper::invoke()].
pub type Result = std::result::Result<Option<Outcome>, Error>;

/// The error used in the [credentials helper invocation][crate::helper::invoke()].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    ContextDecode(#[from] protocol::context::decode::Error),
    #[error("An IO error occurred while communicating to the credentials helper")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    CredentialsHelperFailed { source: std::io::Error },
}

/// The action to perform by the credentials [helper][`crate::helper::invoke()`].
#[derive(Clone, Debug)]
pub enum Action {
    /// Provide credentials using the given repository context, which must include the repository url.
    Get(Context),
    /// Approve the credentials as identified by the previous input provided as `BString`, containing information from [`Context`].
    Store(BString),
    /// Reject the credentials as identified by the previous input provided as `BString`. containing information from [`Context`].
    Erase(BString),
}

/// Initialization
impl Action {
    /// Create a `Get` action with context containing the given URL.
    /// Note that this creates an `Action` suitable for the credential helper cascade only.
    pub fn get_for_url(url: impl Into<BString>) -> Action {
        Action::Get(Context {
            url: Some(url.into()),
            ..Default::default()
        })
    }
}

/// Access
impl Action {
    /// Return the payload of store or erase actions.
    pub fn payload(&self) -> Option<&BStr> {
        use bstr::ByteSlice;
        match self {
            Action::Get(_) => None,
            Action::Store(p) | Action::Erase(p) => Some(p.as_bstr()),
        }
    }
    /// Return the context of a get operation, or `None`.
    ///
    /// The opposite of [`payload`][Action::payload()].
    pub fn context(&self) -> Option<&Context> {
        match self {
            Action::Get(ctx) => Some(ctx),
            Action::Erase(_) | Action::Store(_) => None,
        }
    }

    /// Return the mutable context of a get operation, or `None`.
    pub fn context_mut(&mut self) -> Option<&mut Context> {
        match self {
            Action::Get(ctx) => Some(ctx),
            Action::Erase(_) | Action::Store(_) => None,
        }
    }

    /// Returns true if this action expects output from the helper.
    pub fn expects_output(&self) -> bool {
        matches!(self, Action::Get(_))
    }

    /// The name of the argument to describe this action. If `is_external` is true, the target program is
    /// a custom credentials helper, not a built-in one.
    pub fn as_arg(&self, is_external: bool) -> &str {
        match self {
            Action::Get(_) if is_external => "get",
            Action::Get(_) => "fill",
            Action::Store(_) if is_external => "store",
            Action::Store(_) => "approve",
            Action::Erase(_) if is_external => "erase",
            Action::Erase(_) => "reject",
        }
    }
}

/// A handle to [store](NextAction::store()) or [erase](NextAction::erase()) the outcome of the initial action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NextAction {
    previous_output: BString,
}

impl TryFrom<&NextAction> for Context {
    type Error = protocol::context::decode::Error;

    fn try_from(value: &NextAction) -> std::result::Result<Self, Self::Error> {
        Context::from_bytes(value.previous_output.as_ref())
    }
}

impl From<Context> for NextAction {
    fn from(ctx: Context) -> Self {
        let mut buf = Vec::<u8>::new();
        ctx.write_to(&mut buf).expect("cannot fail");
        NextAction {
            previous_output: buf.into(),
        }
    }
}

impl NextAction {
    /// Approve the result of the previous [Action] and store for lookup.
    pub fn store(self) -> Action {
        Action::Store(self.previous_output)
    }
    /// Reject the result of the previous [Action] and erase it as to not be returned when being looked up.
    pub fn erase(self) -> Action {
        Action::Erase(self.previous_output)
    }
}

mod cascade;
pub(crate) mod invoke;

pub use invoke::invoke;
