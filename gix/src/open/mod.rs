use std::{
    ops::ControlFlow,
    path::{Path, PathBuf},
    sync::Arc,
};

use gix_error::ErrorExt;
use gix_sec::Trust;

use crate::{Repository, Result, bstr::BString};

/// The relationship that led to opening another repository from an existing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RelatedRepositoryKind {
    /// A submodule, including old-form layouts and unlisted gitlinks.
    Submodule,
    /// A linked worktree.
    LinkedWorktree,
    /// The main repository opened from a linked worktree.
    MainRepository,
}

/// The target and parent of an open initiated by an existing repository.
///
/// The target paths are absolute, with their symlink spelling preserved.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct RelatedRepository<'a> {
    /// The relationship through which the target is being opened.
    pub kind: RelatedRepositoryKind,
    /// The target's resolved Git directory.
    pub git_dir: &'a Path,
    /// The target's inferred checkout, which may be absent, inaccessible, or changed by configuration later.
    pub inferred_worktree: Option<&'a Path>,
    /// The trust the target would start with using the opening path's usual ownership or worktree trust rules.
    pub initial_trust: Trust,
    /// The repository initiating the open. Its paths may be relative to its captured working directory.
    pub parent: &'a Repository,
}

type ConfigureRelatedRepository =
    dyn Fn(RelatedRepository<'_>, Options) -> crate::Result<ControlFlow<(), Options>> + Send + Sync;

type ConfigOverridesByTrust = dyn Fn(Trust, &gix_config::File) -> Result<Vec<BString>> + Send + Sync;

pub(crate) fn related_repository_skipped(git_dir: &Path) -> crate::Error {
    gix_error::message!(
        "Opening the repository at \"{}\" was skipped by the callback",
        git_dir.display()
    )
    .raise()
}

#[derive(Copy, Clone)]
pub(crate) enum RepositoryTrust {
    /// Opening may adjust this level after resolving the configuration and worktree.
    Precomputed(Trust),
    /// The caller has made the final trust decision for this repository.
    Enforced(Trust),
}

impl RepositoryTrust {
    pub(crate) fn level(self) -> Trust {
        match self {
            Self::Precomputed(level) | Self::Enforced(level) => level,
        }
    }
}

/// Permissions associated with various resources of a git repository
#[derive(Copy, Clone, Ord, PartialOrd, PartialEq, Eq, Debug, Hash)]
pub struct Permissions {
    /// Control which environment variables may be accessed.
    pub env: permissions::Environment,
    /// Permissions related where git configuration should be loaded from.
    pub config: permissions::Config,
    /// Permissions related to where `gitattributes` should be loaded from.
    pub attributes: permissions::Attributes,
}

/// The options used in [`ThreadSafeRepository::open_opts()`][crate::ThreadSafeRepository::open_opts()].
///
/// ### Replacement Objects for the object database
///
/// The environment variable `GIT_REPLACE_REF_BASE` is mapped to `gitoxide.objects.replaceRefBase`.
/// The presence of `GIT_NO_REPLACE_OBJECTS` sets `core.useReplaceRefs=false`. Replacements are enabled by default.
///
/// Use [Permissions] to control which environment variables can be read, and config-overrides to control these values programmatically.
/// Use [`Repository::reload()`][crate::Repository::reload()] to reopen the same repository with its stored options.
/// Copy options with [`clone_without_trust()`][Self::clone_without_trust()] when opening another repository.
/// [`config_overrides_by_trust()`][Self::config_overrides_by_trust()] controls the configuration applied after trust is known.
pub struct Options {
    pub(crate) object_store_slots: gix_odb::store::init::Slots,
    /// Define what is allowed while opening a repository.
    pub permissions: Permissions,
    /// Trust is specific to this Git directory and must not be reused when opening sub-repositories.
    /// Use `clone_without_trust()` so each sub-repository's trust is determined independently.
    pub(crate) git_dir_trust: Option<RepositoryTrust>,
    /// Warning: this one is copied to config::Cache - don't change it after repo open or keep in sync.
    pub(crate) filter_config_section: Option<fn(&gix_config::file::Metadata) -> bool>,
    pub(crate) lossy_config: bool,
    pub(crate) lenient_config: bool,
    pub(crate) bail_if_untrusted: bool,
    pub(crate) api_config_overrides: Vec<BString>,
    pub(crate) cli_config_overrides: Vec<BString>,
    pub(crate) git_installation_config_path: Option<Box<std::path::Path>>,
    pub(crate) system_config_path: Option<Box<std::path::Path>>,
    /// Whether repository-local environment variables like `GIT_WORK_TREE` and `GIT_INDEX_FILE` may be applied.
    /// This is disabled when reusing these options to enter another repository.
    pub(crate) use_repository_local_environment: bool,
    /// Whether to treat the input path as a git directory without first trying `<path>/.git`.
    /// This only controls how the current call's input is interpreted, so it is reset after path resolution.
    /// Retaining it would make later submodule or worktree opens that clone these options skip their normal
    /// `<path>/.git` lookup as well.
    pub(crate) open_path_as_is: bool,
    /// Submodule checkouts remain usable even if their repository declares `core.bare=true`.
    /// Consumed during initialization so this does not affect later ordinary repository opens.
    pub(crate) retain_inferred_worktree_when_bare: bool,
    /// Internal to pass an already obtained CWD on to where it may also be used.
    /// This avoids the CWD being queried more than once per repo.
    pub(crate) current_dir: Option<PathBuf>,
    /// Callback that can change the opening options or skip a related repository open before its configuration is loaded.
    /// [`Self::clone_without_trust()`] retains it so the policy applies to subsequent related opens.
    /// Set it with [`Self::configure_related_repository()`].
    // Status requires these options to be Send even in serial builds.
    configure_related_repository: Option<Arc<ConfigureRelatedRepository>>,
    config_overrides_by_trust: Option<Arc<ConfigOverridesByTrust>>,
}

mod options;
pub mod permissions;
mod repository;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copying_for_another_repository_clears_repository_specific_state() -> gix_testtools::TestResult {
        let mut options = Options::isolated()
            .with_enforced_trust(gix_sec::Trust::Reduced)
            .open_path_as_is(true);
        options.current_dir = Some("captured-cwd".into());
        options.retain_inferred_worktree_when_bare = true;
        let copied = options.clone_without_trust();
        assert!(
            copied.git_dir_trust.is_none(),
            "ownership must determine the new repository's trust"
        );
        assert!(
            copied.current_dir.is_none(),
            "the opening call must capture its own CWD"
        );
        assert!(
            !copied.open_path_as_is,
            "path interpretation is specific to the original opening call"
        );
        assert!(
            !copied.retain_inferred_worktree_when_bare,
            "checkout retention is a one-shot policy"
        );
        assert!(
            !copied.use_repository_local_environment,
            "another repository must not inherit environment redirection"
        );
        Ok(())
    }

    #[cfg(feature = "sha1")]
    #[test]
    fn retained_bare_submodule_checkout_is_used_for_safe_directory_checks() -> gix_testtools::TestResult {
        let root = gix_testtools::scripted_fixture_read_only("make_unlisted_gitlinks.sh")?.join("listed-gitfile-bare");
        let git_dir = root.join(".git/embedded");
        let worktree = root.join("d");
        let cwd = gix_fs::current_dir(false)?;
        for trust in [gix_sec::Trust::Full, gix_sec::Trust::Reduced] {
            let mut options = Options::isolated().with_trust(trust).bail_if_untrusted(true);
            options.current_dir = Some(cwd.clone());
            options.retain_inferred_worktree_when_bare = true;
            let opened =
                crate::ThreadSafeRepository::open_from_paths(git_dir.clone(), Some(worktree.clone()), options, None);
            if trust == gix_sec::Trust::Full {
                let repo = opened?;
                assert_eq!(
                    repo.work_tree.as_deref(),
                    Some(worktree.as_path()),
                    "the checkout is retained"
                );
                assert!(
                    !repo.linked_worktree_options.retain_inferred_worktree_when_bare,
                    "the submodule-only policy must not leak into later opens"
                );
            } else {
                let err = opened.expect_err("reduced trust requires safe.directory approval");
                assert!(
                    err.to_string()
                        .contains(worktree.canonicalize()?.to_string_lossy().as_ref()),
                    "safe.directory checks must use the retained checkout, not the Git directory: {err}"
                );
            }
        }
        Ok(())
    }
}
