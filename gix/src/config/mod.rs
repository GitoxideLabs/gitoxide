pub use gix_config::*;
use gix_features::threading::OnceCell;

use crate::{Repository, repository::identity};

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

/// The error returned when failing to initialize the repository configuration.
///
/// This configuration is on the critical path when opening a repository.
pub type Error = gix_error::Error;

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
        use crate::bstr::BString;

        /// The error produced when obtaining `diff.algorithm`.
        #[derive(Debug)]
        #[expect(missing_docs)]
        pub enum Error {
            Unknown { name: BString },
            Unimplemented { name: BString },
        }

        impl std::fmt::Display for Error {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Error::Unknown { name } => write!(f, "Unknown diff algorithm named '{name}'"),
                    Error::Unimplemented { name } => write!(f, "The '{name}' algorithm is not yet implemented"),
                }
            }
        }

        impl std::error::Error for Error {}
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
    /// The error produced when setting up the attribute stack to query `gitattributes`.
    pub type Error = gix_error::Error;
}

///
pub mod protocol {
    ///
    pub mod allow {
        /// The error returned when obtaining the permission for a particular scheme.
        pub type Error = gix_error::Error;
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
    /// A generic error suitable to produce decent messages for all kinds of configuration errors with config-key granularity.
    ///
    /// This error is meant to be reusable and help produce uniform error messages related to parsing any configuration key.
    #[derive(Debug)]
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

    impl<E, const PREFIX: char, const SUFFIX: char> std::fmt::Display for Error<E, PREFIX, SUFFIX>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{} \"{}{}\"{} {}",
                prefix(PREFIX),
                self.key,
                self.value.as_ref().map(|v| format!("={v}")).unwrap_or_default(),
                self.environment_override
                    .map(|var| format!(" (possibly from {var})"))
                    .unwrap_or_default(),
                suffix(SUFFIX)
            )
        }
    }

    impl<E, const PREFIX: char, const SUFFIX: char> std::error::Error for Error<E, PREFIX, SUFFIX>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            self.source.as_ref().map(|err| err as _)
        }
    }

    impl<E, const PREFIX: char, const SUFFIX: char> From<Error<E, PREFIX, SUFFIX>> for gix_error::Error
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        fn from(err: Error<E, PREFIX, SUFFIX>) -> Self {
            use gix_error::ErrorExt;
            let message = err.to_string();
            err.and_raise(gix_error::ValidationError::new(message)).into()
        }
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
    pub type GenericError<E = gix_error::Error> = Error<E, 'k', 'i'>;

    /// A generic key error which will also contain a value.
    pub type GenericErrorWithValue<E = gix_error::Error> = Error<E, 'v', 'i'>;
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
        pub type Error = config::key::Error<gix_error::Error, 'n', 'd'>;
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
    /// The error produced when obtaining or installing a fallback commit signature.
    pub type Error = gix_error::Error;
}

///
pub mod lock_timeout {
    /// The error produced when failing to parse timeout for locks.
    pub type Error = super::key::Error<gix_error::Error, 'i', 'i'>;
}

///
pub mod duration {
    /// The error produced when failing to parse durations (in milliseconds).
    pub type Error = super::key::Error<gix_error::Error, 'd', 'i'>;
}

///
pub mod boolean {
    /// The error produced when failing to parse time from configuration.
    pub type Error = super::key::Error<gix_error::Error, 'b', 'i'>;
}

///
pub mod unsigned_integer {
    /// The error produced when failing to parse a signed integer from configuration.
    pub type Error = super::key::Error<gix_error::Error, 'k', 'u'>;
}

///
pub mod url {
    /// The error produced when failing to parse a url from the configuration.
    pub type Error = super::key::Error<gix_error::Error, 'u', 'p'>;
}

///
pub mod string {
    /// The error produced when failing to interpret configuration as UTF-8 encoded string.
    pub type Error = super::key::Error<crate::bstr::Utf8Error, 'w', 'd'>;
}

///
pub mod refspec {
    /// The error produced when failing to parse a refspec from the configuration.
    pub type Error = super::key::Error<gix_error::Error, 'r', 'p'>;
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
