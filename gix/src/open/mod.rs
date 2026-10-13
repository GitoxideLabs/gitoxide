use std::path::PathBuf;

use crate::bstr::BString;

/// Permissions associated with various resources of a git repository
#[derive(Copy, Clone, Ord, PartialOrd, PartialEq, Eq, Debug, Hash)]
#[gix_macros::unstable(since = "2023-04-25")]
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
/// The environment variables `GIT_REPLACE_REF_BASE` and `GIT_ALLOC_LIMIT` are mapped to
/// `gitoxide.objects.replaceRefBase` and `gitoxide.objects.allocLimit` respectively.
/// The presence of `GIT_NO_REPLACE_OBJECTS` sets `core.useReplaceRefs=false`. Replacements are enabled by default.
///
/// With `unstable`, use `Permissions` to control which environment variables can be read;
/// use config-overrides to control these values programmatically.
/// Use [`Repository::reload()`][crate::Repository::reload()] to reopen the same repository with its stored options.
/// Copy options with [`clone_without_trust()`][Self::clone_without_trust()] when opening another repository.
#[gix_macros::unstable_fields]
pub struct Options {
    pub(crate) object_store_slots: gix_odb::store::init::Slots,
    /// Define what is allowed while opening a repository.
    #[unstable(since = "2022-11-26")]
    pub permissions: Permissions,
    /// Trust is specific to this Git directory and must not be reused when opening sub-repositories.
    /// Use `clone_without_trust()` so each sub-repository's trust is determined independently.
    pub(crate) git_dir_trust: Option<gix_sec::Trust>,
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
}

mod options;
#[gix_macros::unstable(since = "2023-04-25")]
pub mod permissions;
mod repository;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copying_for_another_repository_clears_repository_specific_state() -> gix_testtools::TestResult {
        let mut options = Options::isolated()
            .with_trust(gix_sec::Trust::Reduced)
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

    #[test]
    fn size_of_options() {
        let actual = std::mem::size_of::<Options>();
        let limit = 160;
        assert!(
            actual <= limit,
            "{actual} <= {limit}: size shouldn't change without us knowing (on windows, it's bigger)"
        );
    }
}
