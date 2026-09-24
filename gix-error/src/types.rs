//! Utility types for error-chain interoperability, classification, and diagnostic display.
//!
//! Frequently used error types are exported at the crate root. Exception frames and type erasure live in
//! [`crate::exn`].

pub use crate::concrete::chain::ChainedError;
pub use crate::error::{Classification, Classifications, DisplaySource};
