//! Interact with git revisions by parsing them from rev-specs and describing them in terms of reference names.
//!
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs, unsafe_code)]

///
#[cfg(feature = "describe")]
pub mod describe;
#[cfg(feature = "describe")]
pub use describe::function::describe;
/// Find common ancestors of commits, or check if a commit is an ancestor of another one.
#[cfg(feature = "merge_base")]
pub mod merge_base;
#[cfg(feature = "merge_base")]
pub use merge_base::function::merge_base;

///
pub mod spec;
pub use gix_revwalk::{Graph, PriorityQueue, graph};
pub use spec::types::Spec;
