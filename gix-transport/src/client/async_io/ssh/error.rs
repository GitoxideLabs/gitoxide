use russh::MethodSet;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("The authentication method failed. Remaining methods: {0:?}")]
    AuthenticationFailed(MethodSet),
    #[error(transparent)]
    Ssh(#[from] russh::Error),
    #[error(transparent)]
    Keys(#[from] russh::keys::Error),
    #[error(transparent)]
    Agent(#[from] russh::AgentAuthError),
}

impl From<Error> for crate::client::Error {
    fn from(err: Error) -> Self {
        Self::NativeSshError(err)
    }
}
