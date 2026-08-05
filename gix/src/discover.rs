#![allow(clippy::result_large_err)]
use std::path::Path;

pub use gix_discover::*;

use crate::{ThreadSafeRepository, bstr::BString};

/// The error returned by [`crate::discover()`].
pub type Error = gix_error::Error;

impl ThreadSafeRepository {
    /// Try to open a git repository in `directory` and search upwards through its parents until one is found,
    /// using default trust options which matters in case the found repository isn't owned by the current user.
    pub fn discover(directory: impl AsRef<Path>) -> Result<Self, Error> {
        Self::discover_opts(directory, Default::default(), Default::default())
    }

    /// Try to open a git repository in `directory` and search upwards through its parents until one is found,
    /// while applying `options`. Then use the `trust_map` to determine which of our own repository options to use
    /// for instantiations.
    ///
    /// Note that [trust overrides](crate::open::Options::with()) in the `trust_map` are not effective here and we will
    /// always override it with the determined trust value as per [gix_discover::upwards::Options::trust].
    /// This value, however, can be set to [assume a given trust level](gix_discover::upwards::TrustPolicy::Assume) to let
    /// callers control the trust level without re-determining it.
    pub fn discover_opts(
        directory: impl AsRef<Path>,
        options: upwards::Options<'_>,
        trust_map: gix_sec::trust::Mapping<crate::open::Options>,
    ) -> Result<Self, Error> {
        let _span = gix_trace::coarse!("ThreadSafeRepository::discover()");
        let (path, trust) = upwards_opts(directory.as_ref(), options).map_err(gix_error::Error::from_error)?;
        let (git_dir, worktree_dir) = path.into_repository_and_work_tree_directories();
        let mut options = trust_map.into_value_by_level(trust);
        options.git_dir_trust = trust.into();
        // Note that we will adjust the `current_dir` later so it matches the value of `core.precomposeUnicode`.
        options.current_dir = Some(
            gix_fs::current_dir(false).map_err(|err| gix_error::Error::from_error(upwards::Error::CurrentDir(err)))?,
        );
        Self::open_from_paths(git_dir, worktree_dir, options).map_err(gix_error::Error::from_error)
    }

    /// Try to open a git repository directly from the environment.
    /// If that fails, discover upwards from `directory` until one is found,
    /// while applying discovery options from the environment.
    ///
    /// For more, see [`ThreadSafeRepository::discover_with_environment_overrides_opts()`].
    pub fn discover_with_environment_overrides(directory: impl AsRef<Path>) -> Result<Self, Error> {
        Self::discover_with_environment_overrides_opts(directory, Default::default(), Default::default())
    }

    /// Try to open a git repository directly from the environment, which reads `GIT_DIR`
    /// if it is set. If unset, discover upwards from `directory` until one is found,
    /// while applying `options` with overrides from the environment which includes:
    ///
    /// - `GIT_DISCOVERY_ACROSS_FILESYSTEM`
    /// - `GIT_CEILING_DIRECTORIES`
    ///
    /// Finally, use the `trust_map` to determine which of our own repository options to use
    /// based on the trust level of the effective repository directory.
    ///
    /// ### Note
    ///
    /// Consider to set [`match_ceiling_dir_or_error = false`](gix_discover::upwards::Options::match_ceiling_dir_or_error)
    /// to allow discovery if an outside environment variable sets non-matching ceiling directories for greater
    /// compatibility with Git.
    pub fn discover_with_environment_overrides_opts(
        directory: impl AsRef<Path>,
        mut options: upwards::Options<'_>,
        trust_map: gix_sec::trust::Mapping<crate::open::Options>,
    ) -> Result<Self, Error> {
        fn apply_additional_environment(mut opts: upwards::Options<'_>) -> upwards::Options<'_> {
            use crate::bstr::ByteVec;

            if let Some(cross_fs) = std::env::var_os("GIT_DISCOVERY_ACROSS_FILESYSTEM")
                .and_then(|v| Vec::from_os_string(v).ok().map(BString::from))
            {
                if let Ok(b) = gix_config::Boolean::try_from(cross_fs) {
                    opts.cross_fs = b.into();
                }
            }
            opts
        }

        if std::env::var_os("GIT_DIR").is_some() {
            return Self::open_with_environment_overrides(directory.as_ref(), trust_map)
                .map_err(gix_error::Error::from_error);
        }

        options = apply_additional_environment(options.apply_environment());
        Self::discover_opts(directory, options, trust_map)
    }
}
