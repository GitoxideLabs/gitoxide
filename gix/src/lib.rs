//! This crate provides the [`Repository`] abstraction which serves as a hub into all the functionality of git.
//!
//! It's powerful and won't sacrifice performance while still increasing convenience compared to using the sub-crates
//! individually. Sometimes it may hide complexity under the assumption that the performance difference doesn't matter
//! for all but the fewest tools out there, which would be using the underlying crates directly or file an issue.
//!
//! ### The Trust Model
//!
//! It is very simple - based on the ownership of the repository compared to the user of the current process [Trust](sec::Trust)
//! is assigned. This can be [overridden](open::Options::with()) as well. Further, git configuration files track their trust level
//! per section based on and sensitive values like paths to executables or certain values will be skipped if they are from a source
//! that isn't [fully](sec::Trust::Full) trusted.
//!
//! That way, data can safely be obtained without risking to execute untrusted executables.
//!
//! Note that it's possible to let `gix` act like `git` or `git2` by setting the [open::Options::bail_if_untrusted()] option.
//!
//! ### The prelude and extensions
//!
//! With `use git_repository::prelude::*` you should be ready to go as it pulls in various extension traits to make functionality
//! available on objects that may use it.
//!
//! The method signatures are still complex and may require various arguments for configuration and cache control.
//!
//! Most extensions to existing objects provide an `obj_with_extension.attach(&repo).an_easier_version_of_a_method()` for simpler
//! call signatures.
//!
//! ### `ThreadSafe` Mode
//!
//! By default, the [`Repository`] isn't `Sync` and thus can't be used in certain contexts which require the `Sync` trait.
//!
//! To help with this, convert it with [`.into_sync()`][Repository::into_sync()] into a [`ThreadSafeRepository`].
//!
//! ### Object-Access Performance
//!
//! Accessing objects quickly is the bread-and-butter of working with git, right after accessing references. Hence it's vital
//! to understand which cache levels exist and how to leverage them.
//!
//! When accessing an object, the first cache that's queried is a  memory-capped LRU object cache, mapping their id to data and kind.
//! It has to be specifically enabled a [`Repository`].
//! On miss, the object is looked up and if a pack is hit, there is a small fixed-size cache for delta-base objects.
//!
//! In scenarios where the same objects are accessed multiple times, the object cache can be useful and is to be configured specifically
//! using the [`object_cache_size(…)`][crate::Repository::object_cache_size()] method.
//!
//! Use the `cache-efficiency-debug` cargo feature to learn how efficient the cache actually is - it's easy to end up with lowered
//! performance if the cache is not hit in 50% of the time.
//!
//! ### Terminology
//!
//! #### `WorkingTree` and `WorkTree`
//!
//! When reading the documentation of the canonical gix-worktree program one gets the impression work tree and working tree are used
//! interchangeably. We use the term _work tree_ only and try to do so consistently as its shorter and assumed to be the same.
//!
//! ### Plumbing Crates
//!
//! To make using  _sub-crates_ and their types easier, these are re-exported into the root of this crate. Here we list how to access nested plumbing
//! crates which are otherwise harder to discover:
//!
//! **`git_repository::`**
//! * [`odb`]
//!   * [`pack`][odb::pack]
//! * [`protocol`]
//!   * [`transport`][protocol::transport]
//!     * [`packetline`][protocol::transport::packetline]
//!
//! ### `libgit2` API to `gix`
//!
//! This doc-aliases are used to help finding methods under a possibly changed name. Just search in the docs.
//! Entering `git2` into the search field will also surface all methods with such annotations.
//!
//! What follows is a list of methods you might be missing, along with workarounds if available.
//! * [`git2::Repository::open_bare()`](https://docs.rs/git2/*/git2/struct.Repository.html#method.open_bare) ➡ ❌ - use [`open()`] and discard if it is not bare.
//! * [`git2::build::CheckoutBuilder::disable_filters()`](https://docs.rs/git2/*/git2/build/struct.CheckoutBuilder.html#method.disable_filters) ➡ ❌ *(filters are always applied during checkouts)*
//! * [`git2::Repository::submodule_status()`](https://docs.rs/git2/*/git2/struct.Repository.html#method.submodule_status) ➡ [`Submodule::state()`] - status provides more information and conveniences though, and an actual worktree status isn't performed.
//!
//! #### Integrity checks
//!
//! `git2` by default performs integrity checks via [`strict_hash_verification()`](https://docs.rs/git2/latest/git2/opts/fn.strict_hash_verification.html) and
//! [`strict_object_creation`](https://docs.rs/git2/latest/git2/opts/fn.strict_object_creation.html) which `gitoxide` *currently* **does not have**.
//!
//! ### Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg, doc_auto_cfg))]
#![deny(missing_docs, rust_2018_idioms, unsafe_code)]
#![allow(clippy::result_large_err)]

// Re-exports to make this a potential one-stop shop crate avoiding people from having to reference various crates themselves.
// This also means that their major version changes affect our major version, but that's alright as we directly expose their
// APIs/instances anyway.
pub use gix_actor as actor;
#[cfg(feature = "attributes")]
pub use gix_attributes as attrs;
#[cfg(feature = "blame")]
pub use gix_blame as blame;
#[cfg(feature = "command")]
pub use gix_command as command;
pub use gix_commitgraph as commitgraph;
#[cfg(feature = "credentials")]
pub use gix_credentials as credentials;
pub use gix_date as date;
#[cfg(feature = "dirwalk")]
pub use gix_dir as dir;
pub use gix_features as features;
use gix_features::threading::OwnShared;
pub use gix_features::{
    parallel,
    progress::{Count, DynNestedProgress, NestedProgress, Progress},
    threading,
};
pub use gix_fs as fs;
pub use gix_glob as glob;
pub use gix_hash as hash;
pub use gix_hashtable as hashtable;
#[cfg(feature = "excludes")]
pub use gix_ignore as ignore;
#[doc(inline)]
#[cfg(feature = "index")]
pub use gix_index as index;
pub use gix_lock as lock;
#[cfg(feature = "blob-merge")]
pub use gix_merge as merge;
#[cfg(feature = "credentials")]
pub use gix_negotiate as negotiate;
pub use gix_object as objs;
pub use gix_object::bstr;
pub use gix_odb as odb;
#[cfg(feature = "credentials")]
pub use gix_prompt as prompt;
#[cfg(feature = "gix-protocol")]
pub use gix_protocol as protocol;
pub use gix_ref as refs;
pub use gix_refspec as refspec;
pub use gix_revwalk as revwalk;
pub use gix_sec as sec;
pub use gix_tempfile as tempfile;
pub use gix_trace as trace;
pub use gix_traverse as traverse;
pub use gix_url as url;
#[doc(inline)]
pub use gix_url::Url;
pub use gix_utils as utils;
pub use gix_validate as validate;
pub use hash::{oid, ObjectId};

pub mod interrupt;

mod ext;
///
pub mod prelude;

#[cfg(feature = "excludes")]
mod attribute_stack;

///
pub mod path;

/// The standard type for a store to handle git references.
pub type RefStore = gix_ref::file::Store;
/// A handle for finding objects in an object database, abstracting away caches for thread-local use.
pub type OdbHandle = gix_odb::memory::Proxy<gix_odb::Handle>;
/// A handle for finding objects in an object database, abstracting away caches for moving across threads.
pub type OdbHandleArc = gix_odb::memory::Proxy<gix_odb::HandleArc>;

/// A way to access git configuration
pub(crate) type Config = OwnShared<gix_config::File<'static>>;

mod types;
#[cfg(any(feature = "excludes", feature = "attributes"))]
pub use types::AttributeStack;
pub use types::{
    Blob, Commit, Head, Id, Object, ObjectDetached, Reference, Remote, Repository, Tag, ThreadSafeRepository, Tree,
    Worktree,
};
#[cfg(feature = "attributes")]
pub use types::{Pathspec, PathspecDetached, Submodule};

///
pub mod clone;
pub mod commit;
///
#[cfg(feature = "dirwalk")]
pub mod dirwalk;
pub mod head;
pub mod id;
pub mod object;
#[cfg(feature = "attributes")]
pub mod pathspec;
pub mod reference;
pub mod repository;
#[cfg(feature = "attributes")]
pub mod submodule;
pub mod tag;
#[cfg(any(feature = "dirwalk", feature = "status"))]
pub(crate) mod util;

///
pub mod progress;
///
pub mod push;

///
pub mod diff;

/// See [`ThreadSafeRepository::discover()`], but returns a [`Repository`] instead.
///
/// # Note
///
/// **The discovered repository might not be suitable for any operation that requires authentication with remotes**
/// as it doesn't see the relevant git configuration.
///
/// To achieve that, one has to [enable `git_binary` configuration](https://github.com/GitoxideLabs/gitoxide/blob/9723e1addf52cc336d59322de039ea0537cdca36/src/plumbing/main.rs#L86)
/// in the open-options and use [`ThreadSafeRepository::discover_opts()`] instead. Alternatively, it might be well-known
/// that the tool is going to run in a neatly configured environment without relying on bundled configuration.
#[allow(clippy::result_large_err)]
pub fn discover(directory: impl AsRef<std::path::Path>) -> Result<Repository, discover::Error> {
    ThreadSafeRepository::discover(directory).map(Into::into)
}

/// See [`ThreadSafeRepository::init()`], but returns a [`Repository`] instead.
#[allow(clippy::result_large_err)]
pub fn init(directory: impl AsRef<std::path::Path>) -> Result<Repository, init::Error> {
    ThreadSafeRepository::init(directory, create::Kind::WithWorktree, create::Options::default()).map(Into::into)
}

/// See [`ThreadSafeRepository::init()`], but returns a [`Repository`] instead.
#[allow(clippy::result_large_err)]
pub fn init_bare(directory: impl AsRef<std::path::Path>) -> Result<Repository, init::Error> {
    ThreadSafeRepository::init(directory, create::Kind::Bare, create::Options::default()).map(Into::into)
}

/// Create a platform for configuring a bare clone from `url` to the local `path`, using default options for opening it (but
/// amended with using configuration from the git installation to ensure all authentication options are honored).
///
/// See [`clone::PrepareFetch::new()`] for a function to take full control over all options.
#[allow(clippy::result_large_err)]
pub fn prepare_clone_bare<Url, E>(
    url: Url,
    path: impl AsRef<std::path::Path>,
) -> Result<clone::PrepareFetch, clone::Error>
where
    Url: std::convert::TryInto<gix_url::Url, Error = E>,
    gix_url::parse::Error: From<E>,
{
    clone::PrepareFetch::new(
        url,
        path,
        create::Kind::Bare,
        create::Options::default(),
        open_opts_with_git_binary_config(),
    )
}

/// Create a platform for configuring a clone with main working tree from `url` to the local `path`, using default options for opening it
/// (but amended with using configuration from the git installation to ensure all authentication options are honored).
///
/// See [`clone::PrepareFetch::new()`] for a function to take full control over all options.
#[allow(clippy::result_large_err)]
pub fn prepare_clone<Url, E>(url: Url, path: impl AsRef<std::path::Path>) -> Result<clone::PrepareFetch, clone::Error>
where
    Url: std::convert::TryInto<gix_url::Url, Error = E>,
    gix_url::parse::Error: From<E>,
{
    clone::PrepareFetch::new(
        url,
        path,
        create::Kind::WithWorktree,
        create::Options::default(),
        open_opts_with_git_binary_config(),
    )
}

fn open_opts_with_git_binary_config() -> open::Options {
    use gix_sec::trust::DefaultForLevel;
    let mut opts = open::Options::default_for_level(gix_sec::Trust::Full);
    opts.permissions.config.git_binary = true;
    opts
}

/// See [`ThreadSafeRepository::open()`], but returns a [`Repository`] instead.
#[allow(clippy::result_large_err)]
#[doc(alias = "git2")]
pub fn open(directory: impl Into<std::path::PathBuf>) -> Result<Repository, open::Error> {
    ThreadSafeRepository::open(directory).map(Into::into)
}

/// See [`ThreadSafeRepository::open_opts()`], but returns a [`Repository`] instead.
#[allow(clippy::result_large_err)]
#[doc(alias = "open_ext", alias = "git2")]
pub fn open_opts(directory: impl Into<std::path::PathBuf>, options: open::Options) -> Result<Repository, open::Error> {
    ThreadSafeRepository::open_opts(directory, options).map(Into::into)
}

///
pub mod create;

///
pub mod open;

///
pub mod config;

///
#[cfg(feature = "mailmap")]
pub mod mailmap;

///
pub mod worktree;

pub mod revision;

#[cfg(feature = "attributes")]
pub mod filter;

///
pub mod remote;

///
pub mod init;

/// Not to be confused with 'status'.
pub mod state;

///
#[cfg(feature = "status")]
pub mod status;

///
pub mod shallow;

///
pub mod discover;

pub mod env;

#[cfg(feature = "attributes")]
fn is_dir_to_mode(is_dir: bool) -> gix_index::entry::Mode {
    if is_dir {
        gix_index::entry::Mode::DIR
    } else {
        gix_index::entry::Mode::FILE
    }
}
