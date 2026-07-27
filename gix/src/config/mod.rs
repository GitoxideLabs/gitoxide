pub use gix_config::*;
use gix_features::threading::OnceCell;

use crate::{Repository, bstr::BString, repository::identity};

pub(crate) mod cache;
mod snapshot;
#[cfg(feature = "credentials")]
pub use snapshot::credential_helpers;

///
pub mod overrides;

pub mod tree;
pub use tree::root::Tree;

/// A platform to access configuration values as read from disk.
///
/// Note that these values won't update even if the underlying file(s) change.
pub struct Snapshot<'repo> {
    /// The owning repository.
    pub repo: &'repo Repository,
}

/// A platform to access configuration values and modify them in memory, while making them available when this platform is dropped
/// as form of auto-commit.
/// Note that the values will only affect this instance of the parent repository, and not other clones that may exist.
///
/// Note that these values won't update even if the underlying file(s) change.
///
/// Use [`forget()`][Self::forget()] to not apply any of the changes.
// TODO: make it possible to load snapshots with reloading via .config() and write mutated snapshots back to disk which should be the way
//       to affect all instances of a repo, probably via `config_mut()` and `config_mut_at()`.
pub struct SnapshotMut<'repo> {
    /// The owning repository.
    pub repo: Option<&'repo mut Repository>,
    pub(crate) config: gix_config::File,
}

/// A utility structure created by [`SnapshotMut::commit_auto_rollback()`] that restores the previous configuration on drop.
pub struct CommitAutoRollback<'repo> {
    /// The owning repository.
    pub repo: Option<&'repo mut Repository>,
    pub(crate) prev_config: crate::Config,
}

///
pub mod section {
    /// A filter that returns `true` for `meta` if the meta-data attached to a configuration section can be trusted.
    /// This is either the case if its file is fully trusted, or if it's a section from a system-wide file.
    pub fn is_trusted(meta: &gix_config::file::Metadata) -> bool {
        meta.trust == gix_sec::Trust::Full || meta.source.kind() != gix_config::source::Kind::Repository
    }
}

///
pub mod set_value {
    /// The error produced when calling [`SnapshotMut::set(_subsection)?_value()`][crate::config::SnapshotMut::set_value()]
    pub type Error = gix_error::Error;
}

// TODO(review): kept concrete. Callers match its variants directly: `gix/tests/gix/repository/
//                open.rs:358` (`PathInterpolation { .. }`), `:458` (`ObjectFormatRequiresV1`), `:477`
//                (`UnsupportedRepositoryFormatVersion { version: 2 }`), and `gix/tests/gix/id.rs:47`
//                (`CoreAbbrev(_)`). Separately, one parent already has an erased slot filled and
//                would gain a second if this type were erased too: `clone::fetch::Error::ApplyConfig`
//                (`gix/src/clone/fetch/mod.rs`, erased slot `ParseConfig`).
/// The error returned when failing to initialize the repository configuration.
///
/// This configuration is on the critical path when opening a repository.
#[derive(Debug, thiserror::Error)]
#[expect(missing_docs)]
pub enum Error {
    #[error(transparent)]
    ConfigBoolean(#[from] boolean::Error),
    #[error(transparent)]
    ConfigUnsigned(#[from] unsigned_integer::Error),
    #[error(transparent)]
    ConfigTypedString(#[from] key::GenericErrorWithValue),
    #[error(transparent)]
    ConfigCompression(#[from] key::GenericError),
    #[error(transparent)]
    RefsNamespace(#[from] refs_namespace::Error),
    #[error("Cannot handle objects formatted as {:?}", .name)]
    UnsupportedObjectFormat { name: BString },
    #[error(
        "extensions.objectFormat is a v1-only extension, but the repository format version is 0; \
         set core.repositoryFormatVersion=1 to use it, or remove extensions.objectFormat to fall back to the default Sha1 format (if supported by this build)"
    )]
    ObjectFormatRequiresV1,
    #[error("Unsupported repository format version {version}; only versions 0 and 1 are supported")]
    UnsupportedRepositoryFormatVersion { version: usize },
    // TODO(review): embeds an erased `gix_error::Error` via `abbrev::Error`; erasing any other
    //                `#[from]` member of this enum — e.g. `ConfigBoolean` (`boolean::Error`), or
    //                any `gix_config`-wrapping variant such as `Init` (`gix_config::file::init::Error`),
    //                still concrete pending a deferred `gix-config` erasure batch — would give it
    //                a second `From<gix_error::Error>` impl and fail to compile (E0119).
    #[error(transparent)]
    CoreAbbrev(#[from] abbrev::Error),
    #[error("Could not read configuration file at \"{}\"", path.display())]
    Io {
        source: std::io::Error,
        path: std::path::PathBuf,
    },
    #[error(transparent)]
    Init(#[from] gix_config::file::init::Error),
    #[error(transparent)]
    ResolveIncludes(#[from] gix_config::file::includes::Error),
    #[error(transparent)]
    Span(#[from] gix_config::parse::span::Error),
    #[error(transparent)]
    ConfigValue(#[from] gix_config::file::section::value::Error),
    #[error(transparent)]
    FromEnv(#[from] gix_config::file::init::from_env::Error),
    #[error("The path {path:?} at the 'core.worktree' configuration could not be interpolated")]
    PathInterpolation {
        path: BString,
        source: gix_config::path::interpolate::Error,
    },
    #[error("{source:?} configuration overrides at open or init time could not be applied.")]
    ConfigOverrides {
        #[source]
        err: overrides::Error,
        source: gix_config::Source,
    },
}

///
pub mod merge {
    ///
    pub mod pipeline_options {
        /// The error produced when obtaining options needed to fill in [gix_merge::blob::pipeline::Options].
        pub type Error = gix_error::Error;
    }

    ///
    pub mod drivers {
        /// The error produced when obtaining a list of [Drivers](gix_merge::blob::Driver).
        pub type Error = gix_error::Error;
    }
}

///
pub mod diff {
    ///
    pub mod algorithm {
        /// The error produced when obtaining `diff.algorithm`.
        pub type Error = gix_error::Error;
    }

    ///
    pub mod pipeline_options {
        /// The error produced when obtaining options needed to fill in [gix_diff::blob::pipeline::Options].
        pub type Error = gix_error::Error;
    }

    ///
    pub mod drivers {
        /// The error produced when obtaining a list of [Drivers](gix_diff::blob::Driver).
        pub type Error = gix_error::Error;
    }
}

///
pub mod stat_options {
    /// The error produced when collecting stat information, and returned by [Repository::stat_options()](crate::Repository::stat_options()).
    pub type Error = gix_error::Error;
}

///
#[cfg(feature = "attributes")]
pub mod checkout_options {
    /// The error produced when collecting all information needed for checking out files into a worktree.
    pub type Error = gix_error::Error;
}

///
#[cfg(feature = "attributes")]
pub mod command_context {
    /// The error produced when collecting all information relevant to spawned commands,
    /// obtained via [Repository::command_context()](crate::Repository::command_context()).
    pub type Error = gix_error::Error;
}

///
pub mod exclude_stack {
    /// The error produced when setting up a stack to query `gitignore` information.
    pub type Error = gix_error::Error;
}

///
pub mod attribute_stack {
    // TODO(review): kept concrete due to an E0119 collision. `repository::diff_resource_cache::Error`
    //                (`gix/src/repository/mod.rs`, `diff_resource_cache` module) embeds both this type,
    //                via `AttributeStack(#[from] config::attribute_stack::Error)`, and the already-erased
    //                `crate::diff::resource_cache::Error`, via `ResourceCache(#[from] ...)`. Erasing
    //                `attribute_stack::Error` would give `repository::diff_resource_cache::Error` two
    //                `From<gix_error::Error>` impls.
    /// The error produced when setting up the attribute stack to query `gitattributes`.
    #[derive(Debug, thiserror::Error)]
    #[expect(missing_docs)]
    pub enum Error {
        #[error("An attribute file could not be read")]
        Io(#[from] std::io::Error),
        #[error("Failed to interpolate the attribute file configured at `core.attributesFile`")]
        AttributesFileInterpolation(#[from] gix_config::path::interpolate::Error),
    }
}

///
pub mod protocol {
    ///
    pub mod allow {
        use crate::bstr::BString;

        // TODO(review): kept concrete due to an E0119 collision. `crate::remote::connect::Error`
        //                (`gix/src/remote/connect.rs`) embeds both this type, via
        //                `SchemePermission(#[from] config::protocol::allow::Error)`, and the
        //                already-erased `config::ssh_connect_options::Error`, via
        //                `SshOptions(#[from] ...)`. Erasing `protocol::allow::Error` would give
        //                `remote::connect::Error` two `From<gix_error::Error>` impls.
        /// The error returned when obtaining the permission for a particular scheme.
        #[derive(Debug, thiserror::Error)]
        #[expect(missing_docs)]
        #[error("The value {value:?} must be allow|deny|user in configuration key protocol{0}.allow", scheme.as_ref().map(|s| format!(".{s}")).unwrap_or_default())]
        pub struct Error {
            pub scheme: Option<String>,
            pub value: BString,
        }
    }
}

///
pub mod ssh_connect_options {
    /// The error produced when obtaining ssh connection configuration.
    pub type Error = gix_error::Error;
}

///
pub mod key {
    use crate::bstr::BString;

    const fn prefix(kind: char) -> &'static str {
        match kind {
            'n' => "",                         // nothing
            'k' => "The value of key",         // generic key
            't' => "The date format at key",   // time
            'i' => "The timeout at key",       // timeout
            'd' => "The duration [ms] at key", // duration
            'b' => "The boolean at key",       // boolean
            'v' => "The key",                  // generic key with value
            'r' => "The refspec at",           // refspec
            's' => "The ssl version at",       // ssl-version
            'u' => "The url at",               // url
            'w' => "The utf-8 string at",      // string
            _ => panic!("BUG: invalid prefix kind - add a case for it here"),
        }
    }
    const fn suffix(kind: char) -> &'static str {
        match kind {
            'd' => "could not be decoded",                    // decoding
            'i' => "was invalid",                             // invalid
            'u' => "could not be parsed as unsigned integer", // unsigned integer
            'p' => "could not be parsed",                     // parsing
            _ => panic!("BUG: invalid suffix kind - add a case for it here"),
        }
    }
    // TODO(review): kept concrete. Generic over the caller-supplied source error `E` and the const
    //                `PREFIX`/`SUFFIX` parameters that select this key's message wording (see
    //                `prefix()`/`suffix()` above); a `pub type Error = gix_error::Error` alias can't
    //                carry any of them, and doing so would collapse the ~14 differently-worded
    //                aliases built on top of it (`boolean::Error`, `GenericErrorWithValue`,
    //                `time::Error`, …) into one indistinguishable type, losing both the per-key
    //                message and the caller's original `source`. The `impl<T, E, const PREFIX: char,
    //                const SUFFIX: char> From<&'static T> for Error<E, PREFIX, SUFFIX>` below would
    //                also become an orphan impl (`std::convert::From` on `gix_error::Error`, both
    //                foreign to this crate) if the parameters were dropped to force an alias — E0117.
    /// A generic error suitable to produce decent messages for all kinds of configuration errors with config-key granularity.
    ///
    /// This error is meant to be reusable and help produce uniform error messages related to parsing any configuration key.
    #[derive(Debug, thiserror::Error)]
    #[error("{} \"{key}{}\"{} {}", prefix(PREFIX), value.as_ref().map(|v| format!("={v}")).unwrap_or_default(), environment_override.as_deref().map(|var| format!(" (possibly from {var})")).unwrap_or_default(), suffix(SUFFIX))]
    pub struct Error<E: std::error::Error + Send + Sync + 'static, const PREFIX: char, const SUFFIX: char> {
        /// The configuration key that contained the value.
        pub key: BString,
        /// The value that was assigned to `key`.
        pub value: Option<BString>,
        /// The associated environment variable that would override this value.
        pub environment_override: Option<&'static str>,
        /// The source of the error if there was one.
        pub source: Option<E>,
    }

    /// Initialization
    /// Instantiate a new error from the given `key`.
    ///
    /// Note that specifics of the error message are defined by the `PREFIX` and `SUFFIX` which is usually defined by a typedef.
    impl<T, E, const PREFIX: char, const SUFFIX: char> From<&'static T> for Error<E, PREFIX, SUFFIX>
    where
        E: std::error::Error + Send + Sync + 'static,
        T: super::tree::Key,
    {
        fn from(key: &'static T) -> Self {
            Error {
                key: key.logical_name().into(),
                value: None,
                environment_override: key.environment_override(),
                source: None,
            }
        }
    }

    /// Initialization
    impl<E, const PREFIX: char, const SUFFIX: char> Error<E, PREFIX, SUFFIX>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        /// Instantiate an error with all data from `key` along with the `value` of the key.
        pub fn from_value(key: &'static impl super::tree::Key, value: BString) -> Self {
            Error::from(key).with_value(value)
        }
    }

    /// Builder
    impl<E, const PREFIX: char, const SUFFIX: char> Error<E, PREFIX, SUFFIX>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        /// Attach the given `err` as source.
        pub fn with_source(mut self, err: E) -> Self {
            self.source = Some(err);
            self
        }

        /// Attach the given `value` as value we observed when the error was produced.
        pub fn with_value(mut self, value: BString) -> Self {
            self.value = Some(value);
            self
        }
    }

    /// A generic key error for use when it doesn't seem worth it say more than 'key is invalid' along with meta-data.
    pub type GenericError<E = gix_config::value::Error> = Error<E, 'k', 'i'>;

    /// A generic key error which will also contain a value.
    pub type GenericErrorWithValue<E = gix_config::value::Error> = Error<E, 'v', 'i'>;
}

///
pub mod encoding {
    /// The error produced when failing to parse the `core.checkRoundTripEncoding` key.
    pub type Error = gix_error::Error;
}

///
pub mod checkout {
    ///
    pub mod workers {
        use crate::config;

        /// The error produced when failing to parse the `checkout.workers` key.
        pub type Error = config::key::Error<gix_config::value::Error, 'n', 'd'>;
    }
}

///
pub mod abbrev {
    /// The error describing an incorrect `core.abbrev` value.
    pub type Error = gix_error::Error;
}

///
pub mod remote {
    ///
    pub mod symbolic_name {
        /// The error produced when failing to produce a symbolic remote name from configuration.
        pub type Error = super::super::key::Error<crate::remote::name::Error, 'v', 'i'>;
    }
}

///
pub mod time {
    /// The error produced when failing to parse time from configuration.
    pub type Error = super::key::Error<gix_date::Error, 't', 'i'>;
}

///
pub mod commit_signature {
    // TODO(review): kept concrete due to an E0119 collision. `crate::clone::fetch::Error`
    //                (`gix/src/clone/fetch/mod.rs`) embeds both this type, via
    //                `CommitterOrFallback(#[from] crate::config::commit_signature::Error)`, and the
    //                already-erased `config::overrides::Error`, via `ParseConfig(#[from] ...)`
    //                (see the comment on `remote::save::AsError` in `gix/src/remote/save.rs`,
    //                which stays concrete for the identical reason). Erasing
    //                `commit_signature::Error` would give `clone::fetch::Error` two
    //                `From<gix_error::Error>` impls.
    /// The error produced when obtaining or installing a fallback commit signature.
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        Time(#[from] super::time::Error),
        #[error(transparent)]
        Span(#[from] gix_config::parse::span::Error),
    }
}

///
pub mod lock_timeout {
    /// The error produced when failing to parse timeout for locks.
    pub type Error = super::key::Error<gix_config::value::Error, 'i', 'i'>;
}

///
pub mod duration {
    /// The error produced when failing to parse durations (in milliseconds).
    pub type Error = super::key::Error<gix_config::value::Error, 'd', 'i'>;
}

///
pub mod boolean {
    /// The error produced when failing to parse time from configuration.
    pub type Error = super::key::Error<gix_config::value::Error, 'b', 'i'>;
}

///
pub mod unsigned_integer {
    /// The error produced when failing to parse a signed integer from configuration.
    pub type Error = super::key::Error<gix_config::value::Error, 'k', 'u'>;
}

///
pub mod url {
    /// The error produced when failing to parse a url from the configuration.
    pub type Error = super::key::Error<gix_url::parse::Error, 'u', 'p'>;
}

///
pub mod string {
    /// The error produced when failing to interpret configuration as UTF-8 encoded string.
    pub type Error = super::key::Error<crate::bstr::Utf8Error, 'w', 'd'>;
}

///
pub mod refspec {
    /// The error produced when failing to parse a refspec from the configuration.
    pub type Error = super::key::Error<gix_refspec::parse::Error, 'r', 'p'>;
}

///
pub mod refs_namespace {
    /// The error produced when failing to parse a refspec from the configuration.
    pub type Error = super::key::Error<gix_validate::reference::name::Error, 'v', 'i'>;
}

///
pub mod ssl_version {
    /// The error produced when failing to parse a refspec from the configuration.
    pub type Error = super::key::Error<std::convert::Infallible, 's', 'i'>;
}

///
pub mod transport {
    /// The error produced when configuring a transport for a particular protocol.
    // `InvalidInteger` and `ConfigValue`, two of the former variants of this now-erased
    // type, were never constructed anywhere in the workspace and carried no message-preservation
    // obligation.
    pub type Error = gix_error::Error;

    ///
    pub mod http {
        /// The error produced when configuring a HTTP transport.
        pub type Error = gix_error::Error;
    }
}

/// Utility type to keep pre-obtained configuration values, only for those required during initial setup
/// and other basic operations that are common enough to warrant a permanent cache.
///
/// All other values are obtained lazily using `OnceCell`.
#[derive(Clone)]
pub(crate) struct Cache {
    pub resolved: crate::Config,
    /// The hex-length to assume when shortening object ids. If `None`, it should be computed based on the approximate object count.
    pub hex_len: Option<usize>,
    /// `true` if the repository is designated as 'bare', without work tree. If `None`, the value wasn't configured.
    pub is_bare: Option<bool>,
    /// The type of hash to use.
    pub object_hash: gix_hash::Kind,
    /// If true, multi-pack indices, whether present or not, may be used by the object database.
    pub use_multi_pack_index: bool,
    /// The representation of `core.logallrefupdates`, or `None` if the variable wasn't set.
    pub reflog: Option<gix_ref::store::WriteReflog>,
    /// The representation of `gitoxide.core.refsNamespace`, or `None` if the variable wasn't set.
    pub refs_namespace: Option<gix_ref::Namespace>,
    /// The configured user agent for presentation to servers.
    pub(crate) user_agent: OnceCell<String>,
    /// identities for later use, lazy initialization.
    pub(crate) personas: OnceCell<identity::Personas>,
    /// A lazily loaded rewrite list for remote urls
    pub(crate) url_rewrite: OnceCell<crate::remote::url::Rewrite>,
    /// The lazy-loaded rename information for diffs.
    #[cfg(feature = "blob-diff")]
    pub(crate) diff_renames: OnceCell<(Option<crate::diff::Rewrites>, bool)>,
    /// A lazily loaded mapping to know which url schemes to allow
    #[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
    pub(crate) url_scheme: OnceCell<crate::remote::url::SchemePermission>,
    /// The algorithm to use when diffing blobs
    #[cfg(feature = "blob-diff")]
    pub(crate) diff_algorithm: OnceCell<gix_diff::blob::Algorithm>,
    /// The amount of bytes to use for a memory backed delta pack cache. If `Some(0)`, no cache is used, if `None`
    /// a standard cache is used which costs near to nothing and always pays for itself.
    pub(crate) pack_cache_bytes: Option<usize>,
    /// The amount of bytes to use for caching whole objects, or 0 to turn it off entirely.
    pub(crate) object_cache_bytes: usize,
    /// The maximum size of a single allocation caused by user-controlled on-disk packed object data.
    pub(crate) alloc_limit_bytes: Option<usize>,
    /// The compression level to use when writing loose objects, from `core.looseCompression` or `core.compression`.
    pub(crate) loose_compression: gix_zlib::Compression,
    /// The amount of bytes we can hold in our static LRU cache. Otherwise, go with the defaults.
    pub(crate) static_pack_cache_limit_bytes: Option<usize>,
    /// The config section filter from the options used to initialize this instance. Keep these in sync!
    filter_config_section: fn(&gix_config::file::Metadata) -> bool,
    /// The object kind to pick if a prefix is ambiguous.
    #[cfg(feature = "revision")]
    pub object_kind_hint: Option<crate::revision::spec::parse::ObjectKindHint>,
    /// If true, we are on a case-insensitive file system.
    pub ignore_case: bool,
    /// If true, we should default what's possible if something is misconfigured, on case by case basis, to be more resilient.
    /// Also, available in options! Keep in sync!
    pub lenient_config: bool,
    #[cfg_attr(not(feature = "worktree-mutation"), allow(dead_code))]
    attributes: crate::open::permissions::Attributes,
    environment: crate::open::permissions::Environment,
    // TODO: make core.precomposeUnicode available as well.
}

/// Utilities shared privately across the crate, for lack of a better place.
pub(crate) mod shared {
    use crate::{
        config,
        config::{cache::util::ApplyLeniency, tree::Core},
    };

    pub fn is_replace_refs_enabled(
        config: &gix_config::File,
        lenient: bool,
        mut filter_config_section: fn(&gix_config::file::Metadata) -> bool,
    ) -> Result<Option<bool>, config::boolean::Error> {
        Core::USE_REPLACE_REFS
            .enrich_error(config.boolean_filter("core.useReplaceRefs", &mut filter_config_section))
            .with_leniency(lenient)
    }
}
