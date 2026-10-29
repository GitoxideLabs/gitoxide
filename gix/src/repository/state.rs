use gix_error::{ErrorExt, OptionExt, ResultExt, bail, message, validation};
use gix_ref::transaction::{LogChange, PreviousValue, RefEdit};

use crate::{Result, state};

impl crate::Repository {
    /// Returns the status of an in progress operation on a repository or [`None`]
    /// if no operation is currently in progress.
    ///
    /// Note to be confused with the repositories 'status'.
    pub fn state(&self) -> Option<state::InProgress> {
        let git_dir = self.path();

        // This is modeled on the logic from wt_status_get_state in git's wt-status.c and
        // ps1 from gix-prompt.sh.

        if git_dir.join("rebase-apply/applying").is_file() {
            Some(state::InProgress::ApplyMailbox)
        } else if git_dir.join("rebase-apply/rebasing").is_file() {
            Some(state::InProgress::Rebase)
        } else if git_dir.join("rebase-apply").is_dir() {
            Some(state::InProgress::ApplyMailboxRebase)
        } else if git_dir.join("rebase-merge/interactive").is_file() {
            Some(state::InProgress::RebaseInteractive)
        } else if git_dir.join("rebase-merge").is_dir() {
            Some(state::InProgress::Rebase)
        } else if git_dir.join("CHERRY_PICK_HEAD").is_file() {
            if git_dir.join("sequencer/todo").is_file() {
                Some(state::InProgress::CherryPickSequence)
            } else {
                Some(state::InProgress::CherryPick)
            }
        } else if git_dir.join("MERGE_HEAD").is_file() {
            Some(state::InProgress::Merge)
        } else if git_dir.join("BISECT_LOG").is_file() {
            Some(state::InProgress::Bisect)
        } else if git_dir.join("REVERT_HEAD").is_file() {
            if git_dir.join("sequencer/todo").is_file() {
                Some(state::InProgress::RevertSequence)
            } else {
                Some(state::InProgress::Revert)
            }
        } else {
            None
        }
    }
}

impl crate::Repository {
    /// Forget an in-progress merge without changing HEAD, the index, or working-tree files.
    ///
    /// Like `git merge --quit`, remove merge messages, conflict metadata, and `AUTO_MERGE`, preserving
    /// `ORIG_HEAD` and unrelated operation state. An autostash is first saved in `refs/stash` with a reflog
    /// entry, even when ordinary reflog writing is disabled. It is never applied to the working tree.
    /// Invalid autostashes or a failure to save them leave the merge state intact for recovery.
    ///
    /// Missing state is allowed, making successful cleanup repeatable. Reference namespaces are ignored,
    /// like Git's local operation commands. Cleanup affects only this worktree;
    /// source paths are anchored to the directory captured when the repository was opened. Concurrent
    /// changes to merge state must be prevented by the caller. Other I/O failures may leave partial cleanup.
    pub fn clear_merge_state(&self) -> Result<()> {
        let mut repo = self.clone();
        repo.make_paths_absolute()?;
        repo.clear_namespace();
        if let Some(autostash) = repo.try_find_reference("MERGE_AUTOSTASH")? {
            let stash_commit_id = match autostash.target() {
                gix_ref::TargetRef::Object(id) => id.to_owned(),
                gix_ref::TargetRef::Symbolic(_) => {
                    bail!(validation("The merge autostash cannot be a symbolic reference"))
                }
            };
            save_autostash(&repo, stash_commit_id)?;
            delete_reference(&repo, &autostash)?;
        }
        if let Some(reference) = repo.try_find_reference("AUTO_MERGE")? {
            delete_reference(&repo, &reference)?;
        }
        for name in ["MERGE_HEAD", "MERGE_MODE", "MERGE_MSG", "MERGE_RR", "MERGE_LABELS"] {
            remove_state_file(&repo.git_dir().join(name))?;
        }
        Ok(())
    }
}

fn save_autostash(repo: &crate::Repository, stash_commit_id: gix_hash::ObjectId) -> Result<()> {
    // Match `git stash store`'s stash-like check: the working tree and first two parents must
    // resolve to trees. A regular commit must not silently be turned into an unusable stash entry.
    let stash = repo
        .find_commit(stash_commit_id)
        .or_raise(|| message("Could not read the autostash commit"))?;
    stash
        .tree()
        .or_raise(|| message("Could not read the autostash working tree"))?;
    let mut parents = stash.parent_ids();
    for description in ["base", "index"] {
        let parent = parents
            .next()
            .ok_or_raise(|| validation("The autostash is not a stash-like commit"))?;
        parent
            .object()?
            .try_into_commit()?
            .tree()
            .or_raise(|| message!("Could not read the autostash {description} tree"))?;
    }
    let mut writer = repo.clone();
    // Stash reflogs retain older entries, so they must be written even with core.logAllRefUpdates=false.
    writer.refs.write_reflog = gix_ref::store::WriteReflog::Normal;
    writer
        .edit_reference(RefEdit::update_with_log(
            "refs/stash".try_into().expect("valid stash reference name"),
            stash_commit_id,
            PreviousValue::Any,
            LogChange {
                force_create_reflog: true,
                message: "autostash".into(),
                ..Default::default()
            },
        ))
        .or_raise(|| message("Could not preserve the autostash in refs/stash"))?;
    Ok(())
}

fn delete_reference(repo: &crate::Repository, reference: &crate::Reference<'_>) -> Result<()> {
    repo.edit_references_as(
        [RefEdit::delete(
            reference.name().to_owned(),
            PreviousValue::MustExistAndMatch(reference.target().into_owned()),
        )],
        None,
    )
    .or_raise(|| message!("Could not remove operation reference {}", reference.name().as_bstr()))?;
    Ok(())
}

fn remove_state_file(path: &std::path::Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.and_raise(message!("Could not remove operation state at {}", path.display()))),
    }
}
