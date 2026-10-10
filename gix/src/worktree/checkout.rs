use gix_error::validation;

use crate::{Result, Worktree};

impl Worktree<'_> {
    /// Perform the initial checkout of a persisted linked worktree's current `HEAD`.
    ///
    /// Unlike an unpersisted worktree preparation, failure never removes the registration or
    /// partial checkout. The destination must be absent or contain only its correct `.git` link,
    /// and no index may exist. This prevents overwriting user data or a previous checkout.
    /// Missing directories and links are recreated, including relative registrations.
    /// Relative repository paths use the current directory captured when the repository was opened.
    /// Main worktrees are rejected. Callers must prevent concurrent changes to the destination.
    ///
    /// `progress` reports files checked out and bytes written through the `checkout` and `writing`
    /// child progress items, respectively. Set `should_interrupt` to `true` to request cancellation;
    /// it is checked before destination validation, or before and during checkout.
    pub fn checkout<P>(
        &self,
        progress: P,
        should_interrupt: &std::sync::atomic::AtomicBool,
    ) -> Result<gix_worktree_state::checkout::Outcome>
    where
        P: gix_utils::progress::NestedProgress,
        P::SubProgress: gix_utils::progress::NestedProgress + 'static,
    {
        use gix_error::{ErrorExt, ResultExt, bail, ensure, message};
        use std::{fs, sync::atomic::Ordering};
        ensure!(
            !should_interrupt.load(Ordering::Relaxed),
            crate::worktree::add::Error::Interrupted
        );
        let repo = self.parent;
        ensure!(
            !self.is_main(),
            validation("Initial checkout requires a linked worktree")
        );
        ensure!(
            !repo.current_dir().join(repo.index_path()).try_exists().or_error()?,
            validation("Initial checkout refuses an existing index")
        );
        let work_dir = repo.current_dir().join(self.base());
        let git_dir = repo.current_dir().join(repo.git_dir());
        match fs::symlink_metadata(&work_dir) {
            Ok(meta) => {
                ensure!(
                    meta.is_dir() && !meta.file_type().is_symlink(),
                    validation("Initial checkout destination must be a directory, not a symlink")
                );
                for entry in fs::read_dir(&work_dir).or_error()? {
                    ensure!(
                        entry.or_error()?.file_name() == ".git",
                        validation("Initial checkout refuses existing worktree contents")
                    );
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => bail!(err.and_raise(message("Could not inspect the initial checkout destination"))),
        }
        let dot_git = work_dir.join(".git");
        let recreate_git_link = match fs::symlink_metadata(&dot_git) {
            Ok(meta) => {
                ensure!(
                    meta.is_file() && !meta.file_type().is_symlink(),
                    validation("The worktree link must be a regular file")
                );
                let target = gix_discover::path::from_gitdir_file(&dot_git).or_error()?;
                ensure!(
                    gix_path::realpath(&target).or_error()? == gix_path::realpath(&git_dir).or_error()?,
                    validation("The worktree link points to another repository")
                );
                false
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => true,
            Err(err) => bail!(err.and_raise(message("Could not inspect the worktree link"))),
        };
        let repo = repo.with_absolute_paths()?;
        let root_tree_id = repo.head_tree_id()?.detach();
        fs::create_dir_all(&work_dir).or_raise(|| message("Could not create the initial checkout destination"))?;
        if recreate_git_link {
            use std::io::Write;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&dot_git)
                .or_raise(|| message("Could not recreate the worktree link"))?;
            // Relative forward links work independently of the registration's original link style.
            let git_dir = gix_path::realpath(&git_dir).or_error()?;
            let work_dir = gix_path::realpath(&work_dir).or_error()?;
            let relative = match work_dir
                .ancestors()
                .last()
                .and_then(|root| Some((git_dir.strip_prefix(root).ok()?, work_dir.strip_prefix(root).ok()?)))
            {
                Some((target, base)) => gix_path::relativize_with_prefix(target, base).into_owned(),
                None => git_dir,
            };
            file.write_all(b"gitdir: ").or_error()?;
            file.write_all(gix_path::os_str_into_bstr(relative.as_os_str()).or_error()?)
                .or_error()?;
            file.write_all(b"\n").or_error()?;
        }
        crate::worktree::add::checkout_tree(&repo, &root_tree_id, progress, should_interrupt)
    }
}
