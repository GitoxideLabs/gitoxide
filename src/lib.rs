//! This is the documentation of the binaries that come with `gitoxide`. These are called `gix` and `ein`.
//!
//! #### `gix`
//!
//! A developer tool to allow using `gitoxide` algorithms and functionality outside of the test suite. It will be unstable as long as
//! the `gix` crate is unstable and is explicitly not to be understood as `git` replacement.
//!
//! #### `ein`
//!
//! A program to eventually become the most convenient way to do typical operations on `git` repositories, with all tooling one typically
//! needs built right into it.
//! For now, it's most useful for its assorted set of `tools` which help to build automations or learn something about `git` repositories.
//!
//! ## Feature Flags
//!
//! Feature configuration can be complex and this document seeks to provide an overview.
//!
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![allow(missing_docs)]
#![deny(unsafe_code)]

pub mod plumbing;
pub mod porcelain;
/// everything in common between the `gix` and `ein` binaries.
pub mod shared;
