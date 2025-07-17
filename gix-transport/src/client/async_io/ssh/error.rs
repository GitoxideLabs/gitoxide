use russh::MethodSet;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("The authentication method failed. Remaining methods: {0:?}")]
    AuthenticationFailed(MethodSet),
    #[error(transparent)]
    Ssh(#[from] russh::Error),
}
