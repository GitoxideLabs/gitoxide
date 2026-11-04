use std::{collections::BTreeSet, fs, io::Write, sync::atomic::AtomicBool};

use crate::{
    Repository, Result,
    error::{ErrorExt, ResultExt, ensure, message, unsupported},
};
use gix_index::entry::{Flags, Mode};
use gix_ref::transaction::{PreviousValue, RefEdit};
use gix_utils::progress::Progress as _;
use gix_worktree_state::checkout::{Change, Outcome};

use super::checkout::{Disk, inspect_path, interrupted, validate_path};

impl Repository {
    /// Replace the index and tracked worktree contents with `commit_id`, then move `HEAD` to it.
    ///
    /// Like `git reset --hard <commit>`, this discards staged and unstaged changes, including
    /// conflicts, and removes untracked files and directories that obstruct the target tree.
    /// Other untracked contents remain. The current branch stays attached, and `ORIG_HEAD`
    /// records the former HEAD. Reflogs use the full target ID. Reference namespaces are ignored.
    /// Finished cherry-pick/revert and merge state is cleared, preserving unfinished sequences,
    /// rebases, and bisects. Merge autostashes are saved without being applied.
    ///
    /// Sparse indexes, skip-worktree entries, sparse-checkout configuration, and recursive
    /// submodule updates are not yet supported and are rejected before changing any files.
    /// Otherwise submodules are not checked out recursively. Checkout filters and filesystem
    /// settings come from this repository. Paths are anchored to its captured current directory.
    ///
    /// The index lock is held from reading the previous index through the final index write;
    /// a symlinked index is updated through its target. Callers must prevent other concurrent
    /// worktree changes. Cancellation and checkout errors leave HEAD and the on-disk index
    /// unchanged, but may leave partial worktree changes. Failures while updating references or
    /// clearing operation state can occur after the new index and worktree have been installed.
    pub fn reset_hard<P>(
        &self,
        commit_id: impl Into<gix_hash::ObjectId>,
        mut progress: P,
        should_interrupt: &AtomicBool,
    ) -> Result<Outcome>
    where
        P: gix_utils::progress::NestedProgress,
        P::SubProgress: gix_utils::progress::NestedProgress + 'static,
    {
        interrupted(should_interrupt)?;
        let mut repo = self.with_absolute_paths()?.into_owned();
        repo.clear_namespace();
        let workdir = repo
            .workdir()
            .ok_or_else(|| unsupported("Hard reset requires a worktree").raise())?;
        let commit_id = commit_id.into();
        let tree_id = repo.find_commit(commit_id)?.tree_id()?;
        ensure!(
            !repo
                .config_snapshot()
                .try_boolean("core.sparseCheckout")?
                .unwrap_or_default(),
            unsupported("Hard reset does not yet support sparse checkout")
        );
        ensure!(
            !repo
                .config_snapshot()
                .try_boolean("submodule.recurse")?
                .unwrap_or_default(),
            unsupported("Hard reset does not yet support recursive submodule updates")
        );
        let previous_head_id = repo.head()?.id().map(crate::Id::detach);
        let previous_orig_id = repo
            .try_find_reference("ORIG_HEAD")?
            .map(|mut reference| match reference.follow_to_object() {
                Ok(id) => Ok(Some(id.detach())),
                Err(err) if err.is_not_found() => Ok(None),
                Err(err) => Err(err),
            })
            .transpose()?
            .flatten();
        let committer = repo.committer().transpose()?;
        let mut index_lock = gix_lock::File::acquire(
            repo.index_path(),
            gix_lock::acquire::Fail::Immediately,
            None,
            0,
            Some(&gix_lock::acquire::resolve_symlink),
        )
        .or_raise(|| message("Could not lock the index for hard reset"))?;
        let previous = repo.index_or_empty()?;
        ensure!(
            !previous.is_sparse()
                && !previous
                    .entries()
                    .iter()
                    .any(|entry| entry.flags.contains(Flags::SKIP_WORKTREE)),
            unsupported("Hard reset does not yet support sparse indexes or skip-worktree entries")
        );
        let mut index = repo.index_from_tree_with_index(&tree_id, &previous)?;
        let mut options = repo.checkout_options(gix_worktree::stack::state::attributes::Source::IdMapping)?;
        let mut removed = BTreeSet::new();
        for entry in previous.entries() {
            interrupted(should_interrupt)?;
            let path = entry.path(&previous);
            validate_path(path, entry.mode, options.validate)?;
            if index.entry_by_path(path).is_some() {
                continue;
            }
            let Disk::Present(metadata) = inspect_path(workdir, path)? else {
                continue;
            };
            let remove = if entry.mode == Mode::COMMIT {
                metadata.is_dir()
                    && !metadata.is_symlink()
                    && fs::read_dir(workdir.join(gix_path::from_bstr(path)?))
                        .or_error()?
                        .next()
                        .is_none()
            } else {
                !metadata.is_dir() || metadata.is_symlink()
            };
            if remove {
                removed.insert(path.to_owned());
            }
        }
        let removals: Vec<_> = removed.into_iter().map(|path| Change::Remove { path }).collect();
        let mut files = progress.add_child("checkout");
        let mut bytes = progress.add_child("writing");
        files.init(Some(index.entries().len()), crate::progress::count("files"));
        bytes.init(None, crate::progress::bytes());
        let objects = repo
            .objects
            .clone()
            .into_arc()
            .or_raise(|| message("Could not reopen objects for hard reset"))?;
        gix_worktree_state::checkout::apply_changes(
            &mut index,
            &removals,
            workdir,
            objects.clone(),
            &files,
            &bytes,
            should_interrupt,
            options.clone(),
        )?;
        options.overwrite_existing = true;
        let outcome =
            gix_worktree_state::checkout(&mut index, workdir, objects, &files, &bytes, should_interrupt, options)?;
        interrupted(should_interrupt)?;
        ensure!(
            outcome.errors.is_empty()
                && outcome.collisions.is_empty()
                && outcome.delayed_paths_unknown.is_empty()
                && outcome.delayed_paths_unprocessed.is_empty(),
            message!("Hard reset checkout was incomplete: {outcome:?}")
        );
        let mut buffer = std::io::BufWriter::new(&mut index_lock);
        index.write_to(&mut buffer, Default::default())?;
        buffer
            .flush()
            .or_raise(|| message("Could not flush the hard-reset index"))?;
        drop(buffer);
        index_lock
            .commit()
            .or_raise(|| message("Could not install the hard-reset index"))?;

        let expected = |id: Option<gix_hash::ObjectId>| {
            id.map_or(PreviousValue::MustNotExist, |id| {
                PreviousValue::MustExistAndMatch(id.into())
            })
        };
        if let Some(previous_head_id) = previous_head_id {
            repo.edit_references_as(
                Some(
                    RefEdit::update(
                        "ORIG_HEAD".try_into().expect("valid pseudoref"),
                        previous_head_id,
                        expected(previous_orig_id),
                        "reset: updating ORIG_HEAD",
                    )
                    .with_deref(true),
                ),
                committer,
            )?;
        } else if previous_orig_id.is_some() {
            repo.edit_references_as(
                Some(
                    RefEdit::delete(
                        "ORIG_HEAD".try_into().expect("valid pseudoref"),
                        expected(previous_orig_id),
                    )
                    .with_deref(true),
                ),
                committer,
            )?;
        }
        repo.edit_references_as(
            Some(
                RefEdit::update(
                    "HEAD".try_into().expect("valid pseudoref"),
                    commit_id,
                    expected(previous_head_id),
                    format!("reset: moving to {commit_id}"),
                )
                .with_deref(true),
            ),
            committer,
        )?;
        repo.clear_reset_state()?;
        Ok(outcome)
    }
}
