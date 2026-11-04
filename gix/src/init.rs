use std::path::{Path, PathBuf};

use gix_ref::{
    Category, FullName,
    store::WriteReflog,
    transaction::{PreviousValue, RefEdit},
};

use crate::error::{ResultExt, message, validation};
use crate::{Result, ThreadSafeRepository, bstr::ByteSlice, config::tree::Init};

/// The name of the branch to use if non is configured via git configuration.
///
/// # Deviation
///
/// We use `main` instead of `master`.
pub const DEFAULT_BRANCH_NAME: &str = "main";

impl ThreadSafeRepository {
    /// Create a repository with work-tree within `directory`, creating intermediate directories as needed.
    ///
    /// Fails without action if the destination directory isn't empty unless
    /// [`create::Options::destination_must_be_empty`][crate::create::Options::destination_must_be_empty] is `None`
    /// or `Some(false)`. Note that initialization still fails if a `.git` directory already exists in
    /// the destination.
    pub fn init(
        directory: impl AsRef<Path>,
        kind: crate::create::Kind,
        options: crate::create::Options,
    ) -> Result<Self> {
        use gix_sec::trust::DefaultForLevel;
        let open_options = crate::open::Options::default_for_level(gix_sec::Trust::Full);
        Self::init_opts(directory, kind, options, open_options)
    }

    /// Similar to [`init`][Self::init()], but allows to determine how exactly to open the newly created repository.
    /// Invalid `init.defaultBranch` values include their bytes as `input` [metadata](gix_error::Error::metadata()).
    /// Templates are selected by `GIT_TEMPLATE_DIR`, then trusted `init.templateDir`, then the built-in templates.
    /// Relative paths use the process's current directory. An empty path disables templates.
    /// A template's `HEAD` takes precedence over `init.defaultBranch`.
    /// Missing templates and templates with unsupported repository formats are skipped.
    ///
    /// # Deviation
    ///
    /// Instead of naming the default branch `master`, we name it `main` unless configured explicitly using the `init.defaultBranch`
    /// configuration key.
    ///
    /// Required repository directories are created before copying templates, and updating template config replaces
    /// a copied symlink rather than modifying its target.
    pub fn init_opts(
        directory: impl AsRef<Path>,
        kind: crate::create::Kind,
        mut create_options: crate::create::Options,
        open_options: crate::open::Options,
    ) -> Result<Self> {
        let directory = directory.as_ref();
        if create_options.object_hash.is_none() {
            create_options.object_hash = default_object_format(&open_options)?;
        }
        Self::create_and_open(directory, kind, create_options, open_options)
    }

    /// Like [`init_opts()`][Self::init_opts()], but without applying `init.defaultObjectFormat`.
    pub(crate) fn create_and_open(
        directory: &Path,
        kind: crate::create::Kind,
        create_options: crate::create::Options,
        mut open_options: crate::open::Options,
    ) -> Result<Self> {
        let template = template_directory(&open_options)?;
        let (path, capabilities, created_head) =
            crate::create::into_with_capabilities(directory, kind, create_options, template.as_deref())?;
        if !capabilities.symlink {
            open_options.api_config_overrides.push("core.symlinks=false".into());
        }
        let (git_dir, worktree_dir) = path.into_repository_and_work_tree_directories();
        open_options.git_dir_trust = Some(gix_sec::Trust::Full);
        // The repo will use `core.precomposeUnicode` to adjust the value as needed.
        open_options.current_dir = gix_fs::current_dir(false)
            .or_raise(|| message("Could not obtain the current directory"))?
            .into();
        let repo = ThreadSafeRepository::open_from_paths(git_dir, worktree_dir, open_options, None)?;

        let branch_name = repo
            .config
            .resolved
            .string(Init::DEFAULT_BRANCH)
            .unwrap_or_else(|| DEFAULT_BRANCH_NAME.into());
        if created_head && branch_name.as_bstr() != DEFAULT_BRANCH_NAME {
            let configured_branch_name = branch_name;
            let sym_ref: FullName = Category::LocalBranch
                .to_full_name(configured_branch_name.as_bstr())
                .or_raise(|| validation("Invalid default branch name").with_input(configured_branch_name.clone()))?;
            gix_validate::reference::branch_name(sym_ref.as_bstr())
                .or_raise(|| validation("Invalid default branch name").with_input(configured_branch_name))?;
            let mut repo = repo.to_thread_local();
            let prev_write_reflog = repo.refs.write_reflog;
            repo.refs.write_reflog = WriteReflog::Disable;
            repo.edit_reference(RefEdit::update(
                "HEAD".try_into().expect("valid"),
                sym_ref,
                PreviousValue::Any,
                "",
            ))
            .or_raise(|| message("Could not edit HEAD reference with new default name"))?;
            repo.refs.write_reflog = prev_write_reflog;
        }

        Ok(repo)
    }
}

fn template_directory(options: &crate::open::Options) -> Result<Option<PathBuf>> {
    if options.permissions.env.git_prefix != gix_sec::Permission::Deny
        && let Some(path) = std::env::var_os("GIT_TEMPLATE_DIR")
        && let Some(path) = options.permissions.env.git_prefix.check(path).or_error()?
    {
        return Ok(Some(path.into()));
    }
    let config = crate::config(None, options)?;
    let filter = options
        .filter_config_section
        .unwrap_or(crate::config::section::is_trusted);
    let Some(path) = config.path_filter(Init::TEMPLATE_DIR, filter) else {
        return Ok(None);
    };
    let install_dir = crate::path::install_dir().ok();
    let home = crate::config::cache::access::home_dir(options.permissions.env);
    path.resolve(crate::config::cache::interpolate_context(
        install_dir.as_deref(),
        home.as_deref(),
    ))
}

/// Read `init.defaultObjectFormat` without repository context, like Git's format selection.
fn default_object_format(options: &crate::open::Options) -> Result<Option<gix_hash::Kind>> {
    let config = crate::config(None, options)?;
    let key = &Init::DEFAULT_OBJECT_FORMAT;
    let filter = options
        .filter_config_section
        .unwrap_or(crate::config::section::is_trusted);
    let value = config.string_implicit_filter(key, filter);
    value
        .map(|value| {
            let value = value
                .ok_or_else(|| crate::config::key::error(key, "Missing configuration value").validation_error())?;
            key.try_into_object_format(value.as_bstr())
        })
        .transpose()
}
