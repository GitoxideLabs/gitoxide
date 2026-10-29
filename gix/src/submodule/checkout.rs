use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    Result, Submodule,
    bstr::ByteSlice,
    error::{OptionExt, ResultExt, cancelled, ensure, message, unsupported},
    worktree::checkout::{connect, relative_path, validate_destination},
};

impl Submodule<'_> {
    /// Restore an existing local clone's current `HEAD` into its missing or empty worktree.
    ///
    /// Return `None` if no local clone exists; this never clones or fetches. The destination may
    /// contain only its correct `.git` file or directory. A relative gitfile and `core.worktree`
    /// reconnect the clone when needed. `HEAD` and its attachment remain unchanged, while the
    /// index is replaced with the current `HEAD` tree, discarding staged changes. An unborn HEAD
    /// creates an empty index. Nested submodules remain unpopulated, even if recursion is configured.
    /// Checkout filters and sparse-checkout patterns come from the submodule repository.
    ///
    /// The index lock is held throughout restoration. Callers must prevent concurrent worktree
    /// changes. Failure may leave reconnected metadata and partial worktree contents, but the
    /// previous index remains until checkout completes. Cancellation is an error.
    pub fn restore_worktree<P>(
        &self,
        progress: P,
        should_interrupt: &AtomicBool,
    ) -> Result<Option<gix_worktree_state::checkout::Outcome>>
    where
        P: gix_utils::progress::NestedProgress,
        P::SubProgress: gix_utils::progress::NestedProgress + 'static,
    {
        ensure!(
            !should_interrupt.load(Ordering::Relaxed),
            cancelled("Submodule restoration was interrupted")
        );
        let parent = self.state.repo;
        let root = parent
            .workdir()
            .ok_or_raise(|| unsupported("Submodule restoration requires a parent worktree"))?;
        let mut paths = gix_fs::SymlinkCheck::new(parent.current_dir().join(root));
        let work_dir = paths
            .verified_path_allow_nonexisting(self.path()?.as_bstr())
            .or_error()?
            .into_owned();
        let dot_git = work_dir.join(".git");
        let expected_git_dir = if dot_git.is_dir() {
            dot_git
        } else {
            parent.current_dir().join(self.git_dir()?)
        };
        let recreate_git_link = validate_destination(&work_dir, &expected_git_dir)?;
        let Some(repo) = self.open()? else {
            return Ok(None);
        };
        let mut repo = repo.with_absolute_paths()?.into_owned();
        repo.clear_namespace();
        let root_tree_id = repo.head_tree_id_or_empty()?.detach();
        let index_lock = gix_lock::File::acquire(
            repo.index_path(),
            gix_lock::acquire::Fail::Immediately,
            None,
            0,
            Some(&gix_lock::acquire::resolve_symlink),
        )
        .or_raise(|| message("Could not lock the index for submodule restoration"))?;
        let mut config = repo.config_file_mut(repo.config_path(crate::config::Source::Local)?)?;
        connect(&work_dir, repo.git_dir(), recreate_git_link)?;
        let work_dir = gix_path::realpath(&work_dir).or_error()?;
        let git_dir = gix_path::realpath(repo.git_dir()).or_error()?;
        config.set_raw_value(
            "core.worktree",
            gix_path::os_str_into_bstr(relative_path(&work_dir, &git_dir).as_os_str()).or_error()?,
        )?;
        config.commit()?;
        repo.set_workdir(Some(work_dir))?;
        crate::worktree::add::checkout_tree(&repo, &root_tree_id, progress, should_interrupt, Some(index_lock))
            .map(Some)
    }
}
