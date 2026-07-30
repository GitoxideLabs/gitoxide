pub use gix_config::*;
use gix_features::threading::OnceCell;

use crate::{Repository, repository::identity};

pub(crate) mod cache;
pub mod file_mut;

mod snapshot;
#[cfg(feature = "credentials")]
pub use snapshot::credential_helpers;

///
pub mod overrides;

pub mod tree;
pub use tree::root::Tree;

/// A locked, mutable physical configuration file.
///
/// Create one with [`crate::config_mut()`] or [`Repository::config_file_mut()`].
/// Includes are not expanded. Dropping this value releases the lock and discards all changes;
/// [`commit()`](Self::commit()) writes them atomically. Existing repository instances are not updated.
pub struct FileTransaction {
    pub(crate) lock: gix_lock::File,
    pub(crate) config: gix_config::File,
}

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
}

/// Configuration-key diagnostics and their shared metadata.
///
/// Errors from configuration conversions expose the logical `key` name and, when available, the
/// offending `input` and a possible `environment_override` through [`crate::Error::metadata()`].
pub(crate) mod key {
    use gix_error::{Message, MetadataValue};

    use super::tree;

    /// Create a validation diagnostic with `message` and the metadata describing `key`.
    ///
    /// `key` is its logical name as a string, including placeholders for parameterized subsections.
    /// `environment_override` is included as a string if an override is declared on this key or a
    /// fallback key. It describes a possible override, not the actual source of the invalid value.
    pub fn error(key: &dyn tree::Key, message: &'static str) -> Message {
        let mut error = gix_error::validation(message).with("key", key.logical_name());
        if let Some(environment) = key.environment_override() {
            error = error.with("environment_override", environment);
        }
        error
    }

    /// Create a validation diagnostic with the metadata from [`error()`] and `value` as `input`.
    ///
    /// Pass raw configuration values as byte strings to preserve non-UTF-8 input. Parsed numbers
    /// retain their numeric type. An empty value is distinct from the absent input in [`error()`].
    pub fn error_with_value(key: &dyn tree::Key, message: &'static str, value: impl Into<MetadataValue>) -> Message {
        error(key, message).with("input", value)
    }

    #[cfg(test)]
    mod tests {
        use super::{MetadataValue, error, error_with_value};
        use crate::{
            Error,
            bstr::ByteSlice,
            config::tree::{Core, Key, Remote, keys},
        };

        #[test]
        fn helpers_cover_key_value_and_environment_metadata() {
            let fallback = keys::Any::new("fallback", &Core).with_fallback(&Core::DELTA_BASE_CACHE_LIMIT);
            for (key, name, environment) in [
                (&Core::BARE as &dyn Key, "core.bare", None),
                (&Remote::URL, "remote.<name>.url", None),
                (
                    &Core::DELTA_BASE_CACHE_LIMIT,
                    "core.deltaBaseCacheLimit",
                    Some("GIX_PACK_CACHE_MEMORY"),
                ),
                (&fallback, "core.fallback", Some("GIX_PACK_CACHE_MEMORY")),
            ] {
                let empty = b"".as_bstr();
                for (message, input) in [
                    (error(key, "Invalid configuration value"), None),
                    (
                        error_with_value(key, "Invalid configuration value", empty),
                        Some(MetadataValue::from(empty)),
                    ),
                ] {
                    let error = Error::from_error(message);
                    assert!(
                        error.is_validation(),
                        "invalid configuration is classified as validation"
                    );
                    let metadata = error.metadata().next().expect("a configuration error has metadata");
                    assert_eq!(
                        metadata.get("key"),
                        Some(&MetadataValue::from(name)),
                        "the context identifies the key"
                    );
                    assert_eq!(
                        metadata.get("input"),
                        input.as_ref(),
                        "the context retains the available input"
                    );
                    assert_eq!(
                        metadata.get("environment_override"),
                        environment.map(MetadataValue::from).as_ref(),
                        "the context identifies the possible environment override"
                    );
                }
            }
            assert_eq!(
                error_with_value(&Core::BARE, "Invalid boolean", b"bad".as_bstr()).to_string(),
                "Invalid boolean, \"input\"=\"bad\", \"key\"=\"core.bare\"",
                "the standard message display includes its diagnostic metadata"
            );
        }
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
    /// The policy for filesystem refreshes caused by object misses.
    pub(crate) object_refresh_mode: gix_odb::store::RefreshMode,
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
    use crate::Result;
    use crate::config::{cache::util::ApplyLeniency, tree::Core};

    pub fn is_replace_refs_enabled(
        config: &gix_config::File,
        lenient: bool,
        mut filter_config_section: fn(&gix_config::file::Metadata) -> bool,
    ) -> Result<Option<bool>> {
        Core::USE_REPLACE_REFS
            .enrich_error(config.boolean_filter("core.useReplaceRefs", &mut filter_config_section))
            .with_leniency(lenient)
    }
}
