//! This crate provides the [`Repository`] abstraction which serves as a hub into all the functionality of git.
//!
//! It's powerful and won't sacrifice performance while still increasing convenience compared to using the sub-crates
//! individually. Sometimes it may hide complexity under the assumption that the performance difference doesn't matter
//! for all but the fewest tools out there, which would be using the underlying crates directly or file an issue.
//!
//! ## Example
//!
//! This is merely an introduction, for more see the respective [`Repository`] methods.
//! ```
//! # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
//! # let repo_dir = doctest::basic_repo_dir()?;
//! # let repo = doctest::open_repo(repo_dir)?;
//! assert!(repo.git_dir().is_dir());
//! assert!(!repo.is_bare());
//! assert!(repo.workdir().is_some());
//! # Ok(()) }
//! ```
//!
//! ### Stability and experimental APIs
//!
//! APIs available without `unstable` form the initial stability baseline. Repository opening,
//! discovery, path access, attached object IDs, configured signatures, selected configuration
//! options and state queries are available. Object decoding, reference names and other
//! plumbing APIs require `features = ["unstable"]` while their underlying types are still
//! evolving. Their documentation gives the reason for each gate.
//! Enabling `unstable` adds access without replacing the stable API; experimental APIs may
//! change incompatibly. Pin the crate version when depending on those APIs.
//!
//! This is preparation for stability, not a 1.0 guarantee. The Cargo manifest declares which
//! plumbing types are provisionally supported; their compatibility commitments must be
//! established before 1.0. Concrete plumbing causes are diagnostic detail
//! unless an API explicitly documents a supported downcast; unstable downcasts require the
//! experimental API. Shared classifications and metadata are the stable recovery interface.
//!
//! ### The Trust Model
//!
//! It is very simple - based on the ownership of the repository compared to the user of the current process [Trust](sec::Trust)
//! is assigned. This can be overridden with `open::Options::with_trust()` as well. Further, git configuration files track their trust level
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
//! ### Errors
//!
//! Fallible APIs defined by this crate use [`Result<T>`], including iterator items and callbacks.
//! [`Error`] implements [`std::error::Error`] and preserves underlying causes, classifications, and diagnostic metadata.
//! Use [`Error::downcast_any_ref()`] to inspect concrete causes. Re-exported plumbing APIs and implementations of
//! external traits retain the error types required by those APIs.
//!
//! Applications can use `gix::Result` and `gix::Error` for their own errors as well. The [`error`] module re-exports
//! error helpers, extension traits, and macros, so no separate `gix-error` dependency is needed:
//!
//! ```
//! use gix::{Result, error::{ResultExt, message}};
//!
//! fn open_repository(path: impl Into<std::path::PathBuf>) -> Result<gix::Repository> {
//!     gix::open(path).or_raise(|| message("Could not open repository"))
//! }
//! ```
//!
//! ### `ThreadSafe` Mode
//!
//! By default, the [`Repository`] isn't `Sync` and thus can't be used in certain contexts which require the `Sync` trait.
//!
//! To help with this, convert it with [`Repository::into_sync()`] into a [`ThreadSafeRepository`].
//!
//! ### Object-Access Performance
//!
//! Accessing objects quickly is the bread-and-butter of working with git, right after accessing references. Hence it's vital
//! to understand which cache levels exist and how to leverage them.
//!
//! When accessing an object, the first cache that's queried is a  memory-capped LRU object cache, mapping their id to data and kind.
//! It has to be specifically enabled on a [`Repository`].
//! On miss, the object is looked up and if a pack is hit, there is a small fixed-size cache for delta-base objects.
//!
//! In scenarios where the same objects are accessed multiple times, the object cache can be useful and is to be configured specifically
//! using the [`Repository::object_cache_size()`] method.
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
//! * [`git2::Repository::submodule_status()`](https://docs.rs/git2/*/git2/struct.Repository.html#method.submodule_status) ➡ `Submodule::state()` - status provides more information and conveniences though, and an actual worktree status isn't performed.
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
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg))]
#![deny(missing_docs, unsafe_code)]

// Re-exports to make this a potential one-stop shop crate avoiding people from having to reference various crates themselves.
// Plumbing re-exports are experimental until their complete public APIs are stable.
#[gix_macros::unstable(since = "2023-02-11")]
pub use gix_actor as actor;
#[cfg(feature = "attributes")]
#[gix_macros::unstable(since = "2023-02-11")]
pub use gix_attributes as attrs;
#[cfg(feature = "blame")]
#[gix_macros::unstable(since = "2024-12-19")]
pub use gix_blame as blame;
#[cfg(feature = "command")]
#[gix_macros::unstable(since = "2023-12-12")]
pub use gix_command as command;
#[gix_macros::unstable(since = "2023-05-18")]
pub use gix_commitgraph as commitgraph;
#[cfg(feature = "credentials")]
#[gix_macros::unstable(since = "2023-02-11")]
pub use gix_credentials as credentials;
#[gix_macros::unstable(since = "2023-02-11")]
pub use gix_date as date;
#[cfg(feature = "dirwalk")]
#[gix_macros::unstable(since = "2024-02-11")]
pub use gix_dir as dir;
pub use gix_error as error;
#[gix_macros::unstable(since = "2023-04-16")]
pub use gix_fs as fs;
#[gix_macros::unstable(since = "2023-02-11")]
pub use gix_glob as glob;
#[gix_macros::unstable(since = "2023-02-11")]
pub use gix_hash as hash;
#[gix_macros::unstable(since = "2023-06-12")]
pub use gix_hashtable as hashtable;
#[cfg(feature = "excludes")]
#[gix_macros::unstable(since = "2023-04-03")]
pub use gix_ignore as ignore;
#[doc(inline)]
#[cfg(feature = "index")]
#[gix_macros::unstable(since = "2023-02-11")]
pub use gix_index as index;
#[gix_macros::unstable(since = "2023-02-11")]
pub use gix_lock as lock;
#[cfg(feature = "credentials")]
#[gix_macros::unstable(since = "2023-06-06")]
pub use gix_negotiate as negotiate;
#[gix_macros::unstable(since = "2023-02-11")]
pub use gix_object as objs;
pub use gix_object::bstr;
#[gix_macros::unstable(since = "2023-02-11")]
pub use gix_odb as odb;
#[gix_macros::unstable(since = "2026-10-04")]
pub use gix_parallel as parallel;
use gix_parallel::OwnShared;
#[cfg(feature = "credentials")]
#[gix_macros::unstable(since = "2023-02-11")]
pub use gix_prompt as prompt;
#[gix_macros::unstable(since = "2023-02-11")]
pub use gix_protocol as protocol;
#[gix_macros::unstable(since = "2026-08-15")]
pub use gix_quote as quote;
#[gix_macros::unstable(since = "2023-02-11")]
pub use gix_ref as refs;
#[gix_macros::unstable(since = "2023-02-11")]
pub use gix_refspec as refspec;
#[gix_macros::unstable(since = "2023-09-07")]
pub use gix_revwalk as revwalk;
#[gix_macros::unstable(since = "2023-02-11")]
pub use gix_sec as sec;
#[gix_macros::unstable(since = "2023-02-11")]
pub use gix_tempfile as tempfile;
#[gix_macros::unstable(since = "2023-06-16")]
pub use gix_trace as trace;
#[gix_macros::unstable(since = "2023-02-11")]
pub use gix_traverse as traverse;
#[gix_macros::unstable(since = "2023-02-11")]
pub use gix_url as url;
#[doc(inline)]
#[gix_macros::unstable(since = "2023-02-11")]
pub use gix_url::Url;
#[gix_macros::unstable(since = "2023-04-13")]
pub use gix_utils as utils;
#[gix_macros::unstable(since = "2026-10-04")]
pub use gix_utils::progress::{Count, DynNestedProgress, NestedProgress, Progress};
#[gix_macros::unstable(since = "2024-06-24")]
pub use gix_validate as validate;
#[gix_macros::unstable(since = "2026-07-05")]
pub use gix_zlib as zlib;
pub use hash::{ObjectId, oid};

pub use gix_error::{Error, Exn, ExnMessageResult, ExnResult, Result};

pub mod interrupt;

#[cfg_attr(not(feature = "unstable"), allow(dead_code, unused_imports))]
mod ext;
#[gix_macros::unstable(since = "2023-02-11")]
pub mod prelude;

#[cfg(feature = "excludes")]
#[cfg_attr(not(feature = "unstable"), allow(dead_code))]
mod attribute_stack;

#[gix_macros::unstable(since = "2023-02-11")]
pub mod path;

/// The standard type for a store to handle git references.
#[gix_macros::unstable(since = "2023-02-11")]
pub type RefStore = gix_ref::file::Store;
/// A handle for finding objects in an object database, abstracting away caches for thread-local use.
#[gix_macros::unstable(since = "2023-02-11")]
pub type OdbHandle = gix_odb::memory::Proxy<gix_odb::Handle>;
/// A handle for finding objects in an object database, abstracting away caches for moving across threads.
#[gix_macros::unstable(since = "2024-09-05")]
pub type OdbHandleArc = gix_odb::memory::Proxy<gix_odb::HandleArc>;

/// A way to access git configuration
pub(crate) type Config = OwnShared<gix_config::File>;

#[cfg_attr(not(feature = "unstable"), allow(dead_code))]
mod types;
#[cfg(any(feature = "excludes", feature = "attributes"))]
#[gix_macros::unstable(since = "2023-09-08")]
pub use types::AttributeStack;
pub use types::Id;
#[gix_macros::unstable(since = "2023-02-11")]
pub use types::{Blob, Commit, Head, Object, ObjectDetached, Reference, Remote, Tag, Tree, Worktree};
#[cfg(feature = "attributes")]
#[gix_macros::unstable(since = "2023-02-11")]
pub use types::{Pathspec, PathspecDetached, Submodule};
pub use types::{Repository, ThreadSafeRepository};

#[gix_macros::unstable(since = "2023-02-11")]
pub mod clone;
#[gix_macros::unstable(since = "2023-02-11")]
pub mod commit;
#[cfg(feature = "dirwalk")]
#[gix_macros::unstable(since = "2024-02-11")]
pub mod dirwalk;
#[gix_macros::unstable(since = "2023-02-11")]
pub mod head;
pub mod id;
#[gix_macros::unstable(since = "2023-02-11")]
pub mod object;
#[cfg(feature = "attributes")]
#[gix_macros::unstable(since = "2023-08-15")]
pub mod pathspec;
#[gix_macros::unstable(since = "2023-02-11")]
pub mod reference;
pub mod repository;
#[cfg(feature = "attributes")]
#[gix_macros::unstable(since = "2023-08-17")]
pub mod submodule;
#[gix_macros::unstable(since = "2023-02-11")]
pub mod tag;
#[cfg(any(feature = "dirwalk", feature = "status"))]
pub(crate) mod util;

#[gix_macros::unstable(since = "2023-02-11")]
pub mod progress;
#[gix_macros::unstable(since = "2023-12-16")]
pub mod push;

#[gix_macros::unstable(since = "2023-02-11")]
pub mod diff;

#[cfg(feature = "merge")]
#[gix_macros::unstable(since = "2024-11-05")]
pub mod merge;

/// Try to open a git repository in `directory` and search upwards through its parents until one is found,
/// using default trust options which matters in case the found repository isn't owned by the current user.
///
/// For details, see [`ThreadSafeRepository::discover()`].
///
/// # Note
///
/// **The discovered repository might not be suitable for any operation that requires authentication with remotes**
/// as it doesn't see the relevant git configuration.
///
/// To achieve that, one has to [enable `git_binary` configuration](https://github.com/GitoxideLabs/gitoxide/blob/9723e1addf52cc336d59322de039ea0537cdca36/src/plumbing/main.rs#L86)
/// in the open-options and use `ThreadSafeRepository::discover_opts()` instead. Alternatively, it might be well-known
/// that the tool is going to run in a neatly configured environment without relying on bundled configuration.
///
/// # Examples
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
/// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
/// # let repo_dir = doctest::basic_repo_dir()?;
/// let repo = doctest::discover_repo(repo_dir.join("some/very/deeply/nested/subdir"))?;
///
/// assert_eq!(repo.kind(), gix::repository::Kind::Common);
/// assert!(repo.workdir_path("this").expect("non-bare").is_file());
/// # Ok(()) }
/// ```
pub fn discover(directory: impl AsRef<std::path::Path>) -> Result<Repository> {
    ThreadSafeRepository::discover(directory).map(Into::into)
}

/// Try to open a git repository in `directory` and search upwards through its parents until one is found,
/// using `open_options` regardless of the trust level of the discovered repository.
/// The detected trust level is retained, so repositories with reduced trust still restrict their behavior accordingly.
#[gix_macros::unstable(since = "2026-07-22")]
pub fn discover_opts(
    directory: impl AsRef<std::path::Path>,
    options: discover::upwards::Options<'_>,
    open_options: open::Options,
) -> Result<Repository> {
    ThreadSafeRepository::discover_opts(
        directory,
        options,
        sec::trust::Mapping {
            full: open_options.clone_with_trust(),
            reduced: open_options,
        },
    )
    .map(Into::into)
}

/// Try to discover a git repository directly from the environment.
///
/// For details, see `ThreadSafeRepository::discover_with_environment_overrides_opts()`.
pub fn discover_with_environment_overrides(directory: impl AsRef<std::path::Path>) -> Result<Repository> {
    ThreadSafeRepository::discover_with_environment_overrides(directory).map(Into::into)
}

/// Try to open a git repository directly from the environment.
///
/// See `ThreadSafeRepository::open_with_environment_overrides()`.
pub fn open_with_environment_overrides(directory: impl Into<std::path::PathBuf>) -> Result<Repository> {
    ThreadSafeRepository::open_with_environment_overrides(directory, open::Options::default_trust_map()).map(Into::into)
}

/// See `ThreadSafeRepository::init()`, but returns a `Repository` instead.
///
/// # Examples
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
/// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
/// # let dir = doctest::tempdir()?;
/// let repo = gix::init(dir.path())?;
///
/// assert!(repo.git_dir().is_dir());
/// assert!(repo.workdir().is_some());
/// # Ok(()) }
/// ```
pub fn init(directory: impl AsRef<std::path::Path>) -> Result<Repository> {
    ThreadSafeRepository::init(directory, create::Kind::WithWorktree, create::Options::default()).map(Into::into)
}

/// See `ThreadSafeRepository::init()`, but returns a `Repository` instead.
pub fn init_bare(directory: impl AsRef<std::path::Path>) -> Result<Repository> {
    ThreadSafeRepository::init(directory, create::Kind::Bare, create::Options::default()).map(Into::into)
}

/// Create a platform for configuring a bare clone from `url` to the local `path`, using default options for opening it (but
/// amended with using configuration from the git installation to ensure all authentication options are honored).
///
/// Like `git clone --bare`, branches are stored in `refs/heads/`, with no default fetch refspec,
/// remote-tracking `HEAD`, or branch upstream configuration. For a non-bare clone without checkout
/// (like `git clone --no-checkout`), use [`prepare_clone()`] followed by
/// [`fetch_only()`][clone::PrepareFetch::fetch_only()]
/// with the `blocking-network-client` or `async-network-client-async-std` feature enabled.
/// This creates `path/.git` and fetches the repository without populating the working tree.
///
/// See [`clone::PrepareFetch::new()`] for a function to take full control over all options.
#[gix_macros::unstable(since = "2023-02-11")]
pub fn prepare_clone_bare<Url, E>(url: Url, path: impl AsRef<std::path::Path>) -> Result<clone::PrepareFetch>
where
    Url: std::convert::TryInto<gix_url::Url, Error = E>,
    E: std::error::Error + Send + Sync + 'static,
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
#[gix_macros::unstable(since = "2023-02-11")]
pub fn prepare_clone<Url, E>(url: Url, path: impl AsRef<std::path::Path>) -> Result<clone::PrepareFetch>
where
    Url: std::convert::TryInto<gix_url::Url, Error = E>,
    E: std::error::Error + Send + Sync + 'static,
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
    let mut opts = open::Options::default_for_level(gix_sec::Trust::Full);
    opts.permissions.config.git_binary = true;
    opts
}

/// See [`ThreadSafeRepository::open()`], but returns a [`Repository`] instead.
///
/// # Examples
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
/// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
/// # let repo_dir = doctest::basic_repo_dir()?;
/// let repo = doctest::open_repo(repo_dir)?;
///
/// assert_eq!(repo.kind(), gix::repository::Kind::Common);
/// assert!(repo.git_dir().is_dir());
/// # Ok(()) }
/// ```
#[doc(alias = "git2")]
pub fn open(directory: impl Into<std::path::PathBuf>) -> Result<Repository> {
    ThreadSafeRepository::open(directory).map(Into::into)
}

/// See [`ThreadSafeRepository::open_opts()`], but returns a [`Repository`] instead.
#[doc(alias = "open_ext", alias = "git2")]
pub fn open_opts(directory: impl Into<std::path::PathBuf>, options: open::Options) -> Result<Repository> {
    ThreadSafeRepository::open_opts(directory, options).map(Into::into)
}

/// Load configuration available without an existing repository, using the configuration-related portions of `options`.
///
/// `git_dir` supplies context for `includeIf.gitdir` conditions and does not have to exist. Without it, these
/// conditions aren't matched. Repository-local and branch-dependent configuration isn't available at this stage.
#[gix_macros::unstable(since = "2026-07-28")]
pub fn config(git_dir: Option<&std::path::Path>, options: &open::Options) -> Result<config::File> {
    let environment = options.permissions.env;
    let git_install_dir = path::install_dir().ok();
    let home = gix_path::env::home_dir().and_then(|home| environment.home.check_opt(home));
    config::cache::load(
        None,
        &mut Vec::new(),
        git_dir,
        None,
        git_install_dir.as_deref(),
        home.as_deref(),
        options.git_installation_config_path.as_deref(),
        options.system_config_path.as_deref(),
        environment,
        options.permissions.config,
        options.lossy_config,
        options.lenient_config,
        &options.api_config_overrides,
        &options.cli_config_overrides,
        options.use_repository_local_environment,
    )
}

/// Return the path at which a configuration file is expected without a repository, selected by its source.
///
/// Supported sources are [`GitInstallation`](config::Source::GitInstallation), [`System`](config::Source::System),
/// [`Git`](config::Source::Git) (the XDG Git configuration), and [`User`](config::Source::User).
/// Path selection honors the configuration permissions, environment permissions and explicit paths in `options`,
/// just like [`config()`]. Disabled sources and sources without an available path return an error.
/// Relative paths are joined to the current directory when called.
///
/// With a repository, use [`Repository::config_path()`] to also resolve local and worktree configuration paths.
///
/// Use this to inspect, prepare or load the file before calling [`config_mut()`]. The file and its parent directories
/// do not have to exist. No configuration transaction is opened, no lock is acquired, and no directories are created.
/// Discovering the Git installation path, or the system path on Windows, may invoke Git.
#[gix_macros::unstable(since = "2026-09-07")]
pub fn config_path(source: config::Source, options: &open::Options) -> Result<std::path::PathBuf> {
    use gix_error::{OptionExt, ResultExt, bail, message};

    if !matches!(
        source,
        config::Source::GitInstallation | config::Source::System | config::Source::Git | config::Source::User
    ) {
        bail!("Configuration source {source:?} requires a repository or has no physical file");
    }
    let path = config::cache::source_path(
        source,
        options.git_installation_config_path.as_deref(),
        options.system_config_path.as_deref(),
        options.permissions.config,
        &mut config::Cache::make_source_env(options.permissions.env),
    )
    .ok_or_raise(|| message!("Configuration source {source:?} has no available path with these options"))?;
    Ok(if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .or_raise(|| message("Could not obtain the current directory for a relative configuration path"))?
            .join(path)
    })
}

/// Lock and open one configuration file available without a repository, selected by its source.
///
/// The path is selected by [`config_path()`], which can be used to inspect or prepare the file before opening it.
///
/// The lock timeout comes from `core.configLockTimeout` in [`config(None, options)`](config()), honoring section filtering
/// and leniency, and defaults to one second. Includes and runtime overrides can affect the timeout, but the returned file
/// contains only the selected physical file, parsed strictly and losslessly even with `lossy_config` enabled.
///
/// Missing files start empty; their parent directories must already exist. Existing file permissions are preserved, and
/// new files apply `core.sharedRepository` from the same resolved configuration, honoring section filtering.
/// Invalid sharing policies return an error.
/// Dropping the transaction discards edits and releases its lock; [`commit()`](config::FileTransaction::commit()) writes
/// the file atomically without updating existing repository instances.
///
/// ```no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut file = gix::config_mut(gix::config::Source::User, &gix::open::Options::default())?;
/// file.set_raw_value("user.name", "Ada Lovelace")?;
/// file.commit()?;
/// # Ok(()) }
/// ```
#[gix_macros::unstable(since = "2026-09-06")]
pub fn config_mut(source: config::Source, options: &open::Options) -> Result<config::FileTransaction> {
    use gix_error::{ResultExt, message};

    let path = config_path(source, options)?;
    let resolved = config(None, options).or_raise(|| message("Could not load global configuration"))?;
    let filter = options.filter_config_section.unwrap_or(config::section::is_trusted);
    let lock_mode = config::cache::access::config_lock_timeout(&resolved, options.lenient_config, filter)?;
    let shared_repository_permissions = config::file_mut::shared_repository_permissions(&resolved, filter)?;
    config::FileTransaction::open(
        path,
        source,
        gix_sec::Trust::Full,
        lock_mode,
        shared_repository_permissions,
    )
}

#[gix_macros::unstable(since = "2023-02-11")]
pub mod create;

///
pub mod open;

#[gix_macros::unstable(since = "2023-02-11")]
pub mod config;

#[cfg(feature = "mailmap")]
#[gix_macros::unstable(since = "2023-02-11")]
pub mod mailmap;

#[cfg(feature = "notes")]
#[gix_macros::unstable(since = "2026-07-30")]
pub mod note;

#[gix_macros::unstable(since = "2023-02-11")]
pub mod worktree;

#[gix_macros::unstable(since = "2023-02-11")]
pub mod revision;

#[cfg(feature = "attributes")]
#[gix_macros::unstable(since = "2023-07-04")]
pub mod filter;

#[gix_macros::unstable(since = "2023-02-11")]
pub mod remote;

#[gix_macros::unstable(since = "2020-06-12")]
pub mod init;

#[gix_macros::unstable(since = "2023-02-11")]
pub mod state;

#[cfg(feature = "status")]
#[gix_macros::unstable(since = "2023-10-05")]
pub mod status;

#[gix_macros::unstable(since = "2023-03-05")]
pub mod shallow;

#[gix_macros::unstable(since = "2023-02-11")]
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
