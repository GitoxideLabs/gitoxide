/// Options controlling physical reference-store optimization.
pub use gix_ref::store::maintenance::Options;

/// The error returned by [`Repository::optimize_references()`](crate::Repository::optimize_references).
#[derive(Debug, thiserror::Error)]
#[expect(missing_docs)]
pub enum Error {
    #[error(
        "Could not interpret core.filesRefLockTimeout or core.packedRefsTimeout, it must be the number in milliseconds to wait for locks or negative to wait forever"
    )]
    LockTimeoutConfiguration(#[from] crate::config::lock_timeout::Error),
    #[error(transparent)]
    Store(#[from] gix_ref::store::BackendError),
}
