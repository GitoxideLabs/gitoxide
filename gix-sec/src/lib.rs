//! A shared trust model for `gitoxide` crates.
//!
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
// `unsafe_code` not forbidden because we need to interact with the libc
#![deny(missing_docs, unsafe_code)]

use std::fmt::{Display, Formatter};

/// A way to specify how 'safe' we feel about a resource, typically about a git repository.
#[derive(Copy, Clone, Ord, PartialOrd, PartialEq, Eq, Debug, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Trust {
    /// Caution is warranted when using the resource.
    Reduced,
    /// We have no doubts that this resource means no harm and it can be used at will.
    Full,
}

///
pub mod trust;

/// Allow, deny or forbid using a resource or performing an action.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Permission {
    /// Fail outright when trying to load a resource or performing an action.
    Forbid,
    /// Ignore resources or try to avoid performing an operation.
    Deny,
    /// Allow loading a resource or performing an action.
    Allow,
}

///
pub mod permission;

bitflags::bitflags! {
    /// Whether something can be read or written.
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Debug)]
    pub struct ReadWrite: u8 {
        /// The item can be read.
        const READ = 1 << 0;
        /// The item can be written
        const WRITE = 1 << 1;
    }
}

impl Display for ReadWrite {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

/// Various types to identify entities.
pub mod identity;
