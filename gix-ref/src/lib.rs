//! A crate for handling the references stored in various formats in a git repository.
//!
//! References are also called _refs_ which are used interchangeably.
//!
//! Refs are the way to keep track of objects and come in two flavors.
//!
//! * symbolic refs are pointing to another reference
//! * peeled refs point to the an object by its [`ObjectId`]
//!
//! They can be identified by a relative path and stored in various flavors.
//!
//! * **files**
//!   * **[loose][file::Store]**
//!     * one reference maps to a file on disk
//!   * **packed**
//!     * references are stored in a single human-readable file, along with their targets if they are symbolic.
//!
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs, unsafe_code)]

use gix_hash::{ObjectId, oid};
pub use gix_object::bstr;
use gix_object::bstr::{BStr, BString};

#[path = "store/mod.rs"]
mod store_impl;
pub use store_impl::{file, packed};

mod compare;
mod fullname;
///
pub mod name;
///
pub mod namespace;
///
pub mod transaction;

mod parse;
mod raw;

pub use raw::Reference;

mod target;

///
pub mod log;

///
pub mod peel;

///
pub mod store {
    ///
    pub mod init {

        /// Options for use during [initialization](crate::Store::at).
        #[derive(Debug, Copy, Clone, Default)]
        pub struct Options {
            /// How to write the ref-log.
            pub write_reflog: super::WriteReflog,
            /// The equivalent of `core.precomposeUnicode`.
            pub precompose_unicode: bool,
            /// If `true`, we will avoid reading from or writing to references that contains Windows device names
            /// to avoid side effects. This only needs to be `true` on Windows, but can be `true` on other platforms
            /// if they need to remain compatible with Windows.
            pub prohibit_windows_device_names: bool,
        }
    }
    /// The way a file store handles the reflog
    #[derive(Default, Debug, PartialOrd, PartialEq, Ord, Eq, Hash, Clone, Copy)]
    pub enum WriteReflog {
        /// Always write the reflog for all references for ref edits, unconditionally.
        Always,
        /// Write a ref log for ref edits according to the standard rules.
        #[default]
        Normal,
        /// Never write a ref log.
        Disable,
    }

    #[derive(Debug, Clone)]
    pub(crate) enum State {
        Files { store: file::Store },
    }

    pub(crate) mod general;

    pub use general::{BackendError, ReferenceExt, find, iter, log, maintenance, peel, snapshot, transaction};

    use crate::file;
}

/// An opaque Git reference store whose implementation is selected when it is created.
///
/// Its interface describes logical reference operations and deliberately hides the
/// representation used by the built-in storage adapters.
#[derive(Debug, Clone)]
pub struct Store {
    inner: store::State,
}

/// A validated complete and fully qualified reference name, safe to use for all operations.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FullName(pub(crate) BString);

/// A validated complete and fully qualified reference name, safe to use for all operations.
#[derive(Hash, Debug, PartialEq, Eq, Ord, PartialOrd)]
#[repr(transparent)]
pub struct FullNameRef(BStr);

/// A validated and potentially partial reference name, safe to use for common operations.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd)]
#[repr(transparent)]
pub struct PartialNameRef(BStr);

/// A validated and potentially partial reference name, safe to use for common operations.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PartialName(BString);

/// A _validated_ prefix for references to act as a namespace.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Namespace(BString);

/// Denotes the kind of reference.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Kind {
    /// A ref that points to an object id directly.
    Object,
    /// A ref that points to another reference, adding a level of indirection.
    ///
    /// It can be resolved to an id using the [`peel_to_id()`][`crate::store::ReferenceExt::peel_to_id()`] method.
    Symbolic,
}

/// The various known categories of references.
///
/// This translates into a prefix containing all references of a given category.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Category<'a> {
    /// A tag in `refs/tags`
    Tag,
    /// A branch in `refs/heads`
    LocalBranch,
    /// A branch in `refs/remotes`
    RemoteBranch,
    /// A tag in `refs/notes`
    Note,
    /// Something outside `ref/` in the current worktree, typically `HEAD`.
    PseudoRef,
    /// A `PseudoRef`, but referenced so that it will always refer to the main worktree by
    /// prefixing it with `main-worktree/`.
    MainPseudoRef,
    /// Any reference that is prefixed with `main-worktree/refs/`
    MainRef,
    /// A `PseudoRef` in another _linked_ worktree, never in the main one, like `worktrees/<id>/HEAD`.
    LinkedPseudoRef {
        /// The name of the worktree.
        #[cfg_attr(feature = "serde", serde(borrow))]
        name: &'a BStr,
    },
    /// Any reference that is prefixed with `worktrees/<id>/refs/`.
    LinkedRef {
        /// The name of the worktree.
        name: &'a BStr,
    },
    /// A ref that is private to each worktree (_linked_ or _main_), with `refs/bisect/` prefix
    Bisect,
    /// A ref that is private to each worktree (_linked_ or _main_), with `refs/rewritten/` prefix
    Rewritten,
    /// A ref that is private to each worktree (_linked_ or _main_), with `refs/worktree/` prefix
    WorktreePrivate,
    // REF_TYPE_NORMAL,	  /* normal/shared refs inside refs/        */
}

/// Denotes a ref target, equivalent to [`Kind`], but with mutable data.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Target {
    /// A ref that points directly to an object id.
    Object(ObjectId),
    /// A ref that points to another reference by its validated name, adding a level of indirection.
    ///
    /// Note that this is an extension of gitoxide which will be helpful in logging all reference changes.
    Symbolic(FullName),
}

/// Denotes a ref target, equivalent to [`Kind`], but with immutable data.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
pub enum TargetRef<'a> {
    /// A ref that points directly to an object id.
    Object(&'a oid),
    /// A ref that points to another reference by its validated name, adding a level of indirection.
    Symbolic(&'a FullNameRef),
}
