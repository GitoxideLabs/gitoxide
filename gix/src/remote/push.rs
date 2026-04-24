use crate::bstr::BString;

#[cfg(feature = "blocking-network-client")]
pub use super::connection::push::{Error, SignMode};

/// Per-ref status from a completed push.
#[derive(Debug, Clone)]
pub struct RefStatus {
    /// The local ref name that was pushed (empty for pure-delete commands).
    pub local: BString,
    /// The remote ref name that was updated.
    pub remote: BString,
    /// `Ok(())` when the ref update succeeded on the server; `Err(reason)` with the
    /// server's rejection message otherwise.
    pub result: Result<(), BString>,
    /// The ref's value on the remote before the push — null when the ref was
    /// newly created. Callers rendering porcelain / status lines use this to
    /// distinguish new-branch (null→value) from update (value→value) and
    /// delete (value→null) cases.
    pub old_oid: gix_hash::ObjectId,
    /// The ref's value after the push — null for delete commands.
    pub new_oid: gix_hash::ObjectId,
}

/// Outcome of a
/// [`Prepare::transmit`](crate::remote::connection::push::Prepare::transmit) call.
#[derive(Debug, Clone)]
pub struct Outcome {
    /// Per-ref status in the order the server reported.
    pub status: Vec<RefStatus>,
}
