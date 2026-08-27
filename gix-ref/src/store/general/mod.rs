mod access;
mod error;
/// Reference lookup and its errors.
pub mod find;
mod init;
/// Backend-neutral reference iteration.
pub mod iter;
/// Backend-neutral reflog access.
pub mod log;
/// Verification and physical maintenance for a reference store.
pub mod maintenance;
mod reference;
/// Coordinated views of a reference store.
pub mod snapshot;
/// Backend-neutral reference transactions.
pub mod transaction;

pub use error::BackendError;
pub use reference::{ReferenceExt, peel};
