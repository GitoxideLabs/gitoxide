use gix_error::{ErrorExt, OptionExt, ResultExt, bail, message, validation};
use gix_object::bstr::ByteSlice;
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

    /// Forget all in-progress operations without changing HEAD, the index, or working-tree files.
    ///
    /// Clear merge, rebase, mailbox application, cherry-pick, revert, and bisect metadata. This has
    /// the preservation semantics of Git's `--quit` commands, including saving autostashes to
    /// `refs/stash` without applying them. Bisect state is removed without checking out its starting
    /// revision. `ORIG_HEAD`, unrelated files, and unregistered rebase labels are retained.
    ///
    /// Rebase-generated references listed in `rebase-merge/refs-to-delete` must be valid names
    /// beneath `refs/rewritten/`; malformed state is rejected without deleting other references.
    /// Autostashes are preserved before removing operation directories. Failures can leave partial
    /// cleanup, but an autostash is never discarded if it cannot be saved.
    ///
    /// Cleanup is repeatable, ignores reference namespaces like Git's local operation commands,
    /// and affects only this worktree. The caller must prevent concurrent changes to operation state.
    pub fn clear_operation_state(&self) -> Result<()> {
        let mut repo = self.clone();
        repo.make_paths_absolute()?;
        repo.clear_namespace();
        for name in ["rebase-apply", "rebase-merge", "sequencer"] {
            let path = repo.git_dir().join(name);
            match std::fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.file_type().is_dir() => {}
                Ok(_) => bail!(validation(format!(
                    "Operation state is not a directory: {}",
                    path.display()
                ))),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => {
                    return Err(err.and_raise(message!("Could not inspect operation state at {}", path.display())));
                }
            }
        }
        let mut references = Vec::new();
        if let Some(names) = read_state_file(&repo.git_dir().join("rebase-merge/refs-to-delete"))? {
            for name in names.lines() {
                let full_name = <&gix_ref::FullNameRef>::try_from(name.as_bstr())
                    .or_raise(|| validation("Invalid reference name in rebase cleanup state"))?;
                if !full_name.as_bstr().starts_with(b"refs/rewritten/") {
                    bail!(validation(format!(
                        "Refusing to remove non-rebase reference {}",
                        full_name.as_bstr()
                    )));
                }
                if let Some(reference) = repo.try_find_reference(full_name.as_bstr())? {
                    references.push(reference);
                }
            }
        }
        references.extend(
            repo.references()?
                .prefixed("refs/bisect/")?
                .collect::<Result<Vec<_>>>()?,
        );
        for name in [
            "CHERRY_PICK_HEAD",
            "REVERT_HEAD",
            "REBASE_HEAD",
            "BISECT_HEAD",
            "BISECT_EXPECTED_REV",
        ] {
            if let Some(reference) = repo.try_find_reference(name)? {
                references.push(reference);
            }
        }
        for name in ["rebase-apply", "rebase-merge"] {
            let path = repo.git_dir().join(name).join("autostash");
            if let Some(content) = read_state_file(&path)? {
                let stash_commit_id = gix_hash::ObjectId::from_hex(content.trim())
                    .or_raise(|| validation(format!("Invalid autostash object ID at {}", path.display())))?;
                save_autostash(&repo, stash_commit_id)?;
                remove_state_file(&path)?;
            }
        }
        repo.clear_merge_state()?;
        references.sort_unstable_by(|a, b| a.name().cmp(b.name()));
        references.dedup_by(|a, b| a.name() == b.name());
        for reference in references {
            delete_reference(&repo, &reference)?;
        }
        for name in ["rebase-apply", "rebase-merge", "sequencer"] {
            let path = repo.git_dir().join(name);
            match std::fs::remove_dir_all(&path) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => {
                    return Err(err.and_raise(message!("Could not remove operation state at {}", path.display())));
                }
            }
        }
        for name in [
            "SQUASH_MSG",
            "BISECT_ANCESTORS_OK",
            "BISECT_LOG",
            "BISECT_NAMES",
            "BISECT_RUN",
            "BISECT_TERMS",
            "BISECT_FIRST_PARENT",
            "BISECT_RESET_WHEN_FOUND",
            "BISECT_START",
        ] {
            remove_state_file(&repo.git_dir().join(name))?;
        }
        Ok(())
    }
}

#[cfg(feature = "worktree-mutation")]
impl crate::Repository {
    pub(super) fn clear_reset_state(&self) -> Result<()> {
        let mut finished_pick = false;
        for name in ["CHERRY_PICK_HEAD", "REVERT_HEAD"] {
            if let Some(reference) = self.try_find_reference(name)? {
                delete_reference(self, &reference)?;
                finished_pick = true;
            }
        }
        if finished_pick
            && let Some(todo) = read_state_file(&self.git_dir().join("sequencer/todo"))?
            && todo
                .iter()
                .position(|byte| *byte == b'\n')
                .is_none_or(|position| position + 1 == todo.len())
        {
            std::fs::remove_dir_all(self.git_dir().join("sequencer"))
                .or_raise(|| message("Could not remove the completed sequence after hard reset"))?;
        }
        remove_state_file(&self.git_dir().join("SQUASH_MSG"))?;
        self.clear_merge_state()
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

fn read_state_file(path: &std::path::Path) -> Result<Option<Vec<u8>>> {
    match std::fs::read(path) {
        Ok(content) => Ok(Some(content)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err.and_raise(message!("Could not read operation state at {}", path.display()))),
    }
}
