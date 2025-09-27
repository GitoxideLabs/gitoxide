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
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg, doc_auto_cfg))]
#![deny(missing_docs, rust_2018_idioms, unsafe_code)]

use gix_hash::{oid, ObjectId};
pub use gix_object::bstr;
use gix_object::bstr::{BStr, BString};

#[path = "store/mod.rs"]
mod store_impl;
pub use store_impl::{file, packed};

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

        /// Options for use during [initialization](crate::file::Store::at).
        #[derive(Debug, Copy, Clone, Default)]
        pub struct Options {
            /// How to write the ref-log.
            pub write_reflog: super::WriteReflog,
            /// The kind of hash to expect in
            pub object_hash: gix_hash::Kind,
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

    /// A thread-local handle for interacting with a [`Store`][crate::Store] to find and iterate references.
    #[derive(Clone)]
    #[allow(dead_code)]
    pub(crate) struct Handle {
        /// A way to access shared state with the requirement that interior mutability doesn't leak or is incorporated into error types
        /// if it could. The latter can't happen if references to said internal aren't ever returned.
        state: handle::State,
    }

    #[allow(dead_code)]
    pub(crate) enum State {
        Loose { store: file::Store },
    }

    pub(crate) mod general;

    ///
    #[path = "general/handle/mod.rs"]
    mod handle;
    pub use handle::find;

    use crate::file;
}

/// The git reference store.
/// TODO: Figure out if handles are needed at all, which depends on the ref-table implementation.
#[allow(dead_code)]
pub(crate) struct Store {
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
    /// It can be resolved to an id using the [`peel_to_id()`][`crate::file::ReferenceExt::peel_to_id()`] method.
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

#[cfg(test)]
mod display_tests;
