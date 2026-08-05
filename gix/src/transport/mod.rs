//! Transport implementations specific to the `gix` crate.
//!
//! These live here rather than in `gix-transport` because they depend on higher-level
//! crates (`gix-odb`, `gix-ref`, `gix-protocol`) that would create circular dependencies
//! if placed in a low-level transport crate.

#[cfg(feature = "experimental")]
pub mod builtin_upload_pack;
