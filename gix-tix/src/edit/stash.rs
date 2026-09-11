use std::{
    collections::{HashMap, HashSet},
    fmt::Write as _,
    path::Path,
    process::Command,
};

use anyhow::{Context, Result};
use gix::{
    ObjectId,
    bstr::{BStr, ByteSlice},
    refs::{
        Target,
        transaction::{PreviousValue, RefEdit},
    },
};

use crate::open_repository;

pub(crate) fn reference(id: ObjectId) -> Result<gix::refs::FullName> {
    format!(
        "{}{}",
        String::from_utf8_lossy(crate::history::STASH_PREFIX),
        id.to_hex()
    )
    .try_into()
    .context("generated an invalid tix stash reference")
}

pub(crate) fn associated_commit(name: &BStr) -> Result<Option<ObjectId>> {
    let Some(suffix) = name.strip_prefix(crate::history::STASH_PREFIX) else {
        return Ok(None);
    };
    let id = ObjectId::from_hex(suffix).context("tix stash reference has an invalid commit ID")?;
    if id.to_hex().to_string().as_bytes() != suffix {
        anyhow::bail!("tix stash reference does not use a canonical full commit ID");
    }
    Ok(Some(id))
}

pub(super) struct RewriteEdits {
    pub forward: Vec<RefEdit>,
    pub rollback: Vec<RefEdit>,
}

pub(super) fn rewrite_edits(
    repo: &gix::Repository,
    rewritten: &HashMap<ObjectId, Option<ObjectId>>,
    removed: &HashSet<ObjectId>,
) -> Result<RewriteEdits> {
    let mut moves = Vec::new();
    let mut destinations = HashMap::<ObjectId, ObjectId>::new();
    for reference in repo.references()?.all()? {
        let reference = match reference {
            Ok(reference) => reference,
            Err(err) => {
                return Err(anyhow::anyhow!(
                    "could not inspect a stash reference before rebasing: {err}"
                ));
            }
        };
        let old = match associated_commit(reference.name().as_bstr()) {
            Ok(Some(id)) => id,
            Ok(None) => continue,
            Err(err) => {
                tracing::warn!(name = %reference.name(), error = %err, "ignored malformed tix stash reference");
                continue;
            }
        };
        let Some(new) = rewritten.get(&old).copied() else {
            continue;
        };
        if removed.contains(&old) {
            anyhow::bail!("cannot drop stashed commit {}", old.to_hex_with_len(7));
        }
        let new = new.context("a stashed commit cannot disappear during a rewrite")?;
        if new == old {
            continue;
        }
        if let Some(other) = destinations.insert(new, old) {
            anyhow::bail!(
                "stashes at {} and {} would converge on {}",
                other.to_hex_with_len(7),
                old.to_hex_with_len(7),
                new.to_hex_with_len(7)
            );
        }
        moves.push((reference.name().to_owned(), reference.target().into_owned(), old, new));
    }

    let mut forward = Vec::with_capacity(moves.len() * 2);
    let mut rollback = Vec::with_capacity(moves.len() * 2);
    for (old_name, target, old, new) in moves {
        let new_name = reference(new)?;
        if repo.try_find_reference(new_name.as_ref())?.is_some() {
            anyhow::bail!(
                "rewritten commit {} already has saved worktree state",
                new.to_hex_with_len(7)
            );
        }
        forward.push(delete_edit(old_name.clone(), target.clone()));
        forward.push(create_edit(new_name.clone(), target.clone()));
        rollback.push(delete_edit(new_name, target.clone()));
        rollback.push(create_edit(old_name, target));
        tracing::debug!(old = %old, new = %new, "prepared tix stash association rewrite");
    }
    Ok(RewriteEdits { forward, rollback })
}

fn create_edit(name: gix::refs::FullName, target: Target) -> RefEdit {
    RefEdit::update(name, target, PreviousValue::MustNotExist, "tix commit stash rewrite")
}

fn delete_edit(name: gix::refs::FullName, target: Target) -> RefEdit {
    RefEdit::delete(name, PreviousValue::MustExistAndMatch(target))
}

#[tracing::instrument(skip_all, fields(commit_id = %id))]
pub(crate) fn save_manual(repository_path: &Path, bare: bool, id: ObjectId) -> Result<String> {
    let repo = open_repository(repository_path, bare, false).context("could not open repository to stash changes")?;
    let workdir = repo
        .workdir()
        .context("stashing changes requires a worktree")?
        .to_owned();
    let head = repo
        .head_id()
        .context("stashing changes requires a born HEAD")?
        .detach();
    if head != id {
        anyhow::bail!("changes can only be stashed at the current HEAD");
    }
    if repo
        .index_or_empty()
        .context("could not inspect the index before stashing")?
        .entries()
        .iter()
        .any(|entry| entry.stage() != gix::index::entry::Stage::Unconflicted)
    {
        anyhow::bail!("cannot stash changes with unresolved index conflicts");
    }
    let name = reference(id)?;
    if repo.try_find_reference(name.as_ref())?.is_some() {
        anyhow::bail!("{} already has saved worktree state", id.to_hex_with_len(7));
    }
    drop(repo);
    if !super::review::is_dirty(&workdir)? {
        anyhow::bail!("there are no worktree or index changes to stash");
    }
    let saved = save(
        repository_path,
        bare,
        &workdir,
        name,
        format!("tix {}", id.to_hex_with_len(7)),
        "tix commit stash",
        "commit state",
    )?;
    let mut notice = format!("stashed changes at {}", id.to_hex_with_len(7));
    if let Some(warning) = saved.warning {
        write!(notice, "; {warning}").expect("writing to a string cannot fail");
    }
    Ok(notice)
}

#[tracing::instrument(skip_all, fields(commit_id = %id))]
pub(crate) fn restore_manual(repository_path: &Path, bare: bool, id: ObjectId) -> Result<String> {
    let repo = open_repository(repository_path, bare, false).context("could not open repository to unstash changes")?;
    let workdir = repo
        .workdir()
        .context("unstashing changes requires a worktree")?
        .to_owned();
    if repo
        .head_id()
        .context("unstashing changes requires a born HEAD")?
        .detach()
        != id
    {
        anyhow::bail!("changes can only be unstashed at the current HEAD");
    }
    let name = reference(id)?;
    drop(repo);
    let saved = find(repository_path, bare, name)?.context("the selected commit has no saved worktree state")?;
    apply(repository_path, bare, &workdir, saved)
}

#[derive(Clone)]
pub(super) struct SavedStash {
    pub name: gix::refs::FullName,
    pub target: Target,
    pub warning: Option<String>,
}

pub(super) fn save_if_dirty(
    repository_path: &Path,
    bare: bool,
    workdir: &Path,
    name: gix::refs::FullName,
) -> Result<Option<SavedStash>> {
    if let Some(commit_id) = associated_commit(name.as_bstr())? {
        let repo = open_repository(repository_path, bare, false).context("could not verify the stash departure")?;
        anyhow::ensure!(
            repo.head_id()? == commit_id,
            "changes can only be stashed at the current HEAD"
        );
    }
    if !super::review::is_dirty(workdir)? {
        return Ok(None);
    }
    let message = format!("tix travel {}", name.shorten());
    save(
        repository_path,
        bare,
        workdir,
        name,
        message,
        "tix travel auto-stash",
        "departure state",
    )
    .map(Some)
}

pub(super) fn remap(saved: &mut SavedStash, map: impl FnOnce(ObjectId) -> Option<ObjectId>) -> Result<()> {
    if let Some(commit_id) = associated_commit(saved.name.as_bstr())? {
        saved.name = reference(map(commit_id).context("the stashed departure disappeared during replay")?)?;
    }
    Ok(())
}

pub(super) fn restore_after_failure(
    repository_path: &Path,
    bare: bool,
    workdir: &Path,
    saved: SavedStash,
    cause: anyhow::Error,
) -> anyhow::Error {
    let saved_id = saved.target.try_id().map(|id| id.to_string());
    let restore = (|| -> Result<String> {
        let repo =
            open_repository(repository_path, bare, false).context("could not inspect the rolled-back departure")?;
        let stash_commit_id = saved.target.try_id().context("a saved stash must point to an object")?;
        let departure_commit_id = repo
            .find_commit(stash_commit_id)?
            .parent_ids()
            .next()
            .context("a saved stash must have a departure parent")?;
        anyhow::ensure!(
            repo.head_id()? == departure_commit_id,
            "HEAD was not restored to the departure"
        );
        anyhow::ensure!(
            !super::review::is_dirty(workdir)?,
            "the departure is not clean after rollback"
        );
        anyhow::ensure!(
            saved.target == repo.find_reference(saved.name.as_ref())?.target(),
            "the departure stash reference changed"
        );
        drop(repo);
        apply(repository_path, bare, workdir, saved)
    })();
    match restore {
        Ok(notice) => cause.context(format!("departure stash restoration: {notice}")),
        Err(restore) => cause.context(format!(
            "departure stash could not be restored: {restore:#}; saved stash object: {}",
            saved_id.as_deref().unwrap_or("unknown")
        )),
    }
}

#[tracing::instrument(skip_all, fields(stash = %name))]
pub(super) fn save(
    repository_path: &Path,
    bare: bool,
    workdir: &Path,
    name: gix::refs::FullName,
    message: String,
    reflog_message: &'static str,
    state_label: &'static str,
) -> Result<SavedStash> {
    let repo = open_repository(repository_path, bare, false)
        .with_context(|| format!("could not open repository to save {state_label}"))?;
    if repo.try_find_reference(name.as_ref())?.is_some() {
        anyhow::bail!("{state_label} is already saved");
    }
    let previous = repo
        .try_find_reference("refs/stash")?
        .and_then(|mut reference| reference.peel_to_id().ok().map(gix::Id::detach));
    drop(repo);

    let output = Command::new("git")
        .arg("-C")
        .arg(workdir)
        .args(["stash", "push", "--include-untracked", "--quiet", "--message"])
        .arg(message)
        .output()
        .context("could not launch git stash push")?;
    if !output.status.success() {
        anyhow::bail!("git stash push failed: {}", output.stderr.trim().to_str_lossy());
    }

    let repo = open_repository(repository_path, bare, false).context("could not reopen repository after stashing")?;
    let mut stash = repo
        .try_find_reference("refs/stash")?
        .context("git stash push did not create refs/stash")?;
    let id = stash.peel_to_id()?.detach();
    if previous == Some(id) {
        anyhow::bail!("git stash push did not create a new stash");
    }
    let target = Target::Object(id);
    if let Err(err) = repo.edit_references([RefEdit::update(
        name.clone(),
        target.clone(),
        PreviousValue::MustNotExist,
        reflog_message,
    )]) {
        drop(repo);
        let restore = Command::new("git")
            .arg("-C")
            .arg(workdir)
            .args(["stash", "pop", "--index", "--quiet"])
            .output();
        return Err(anyhow::anyhow!(err)).context(match restore {
            Ok(output) if output.status.success() => {
                format!("could not retain {state_label}; original state was restored")
            }
            Ok(output) => format!(
                "could not retain {state_label} and git stash pop failed: {}",
                output.stderr.trim().to_str_lossy()
            ),
            Err(restore) => {
                format!("could not retain {state_label} and could not launch git stash pop: {restore}")
            }
        });
    }
    drop(repo);

    let warning = match current(repository_path, bare) {
        Ok(Some(current)) if current == id => {
            match Command::new("git")
                .arg("-C")
                .arg(workdir)
                .args(["stash", "drop", "--quiet", "stash@{0}"])
                .output()
            {
                Ok(output) => (!output.status.success()).then(|| {
                    format!(
                        "{state_label} was saved, but its ordinary stash entry remains: {}",
                        output.stderr.trim().to_str_lossy()
                    )
                }),
                Err(err) => Some(format!(
                    "{state_label} was saved, but could not launch git stash drop: {err}"
                )),
            }
        }
        Ok(_) => Some(format!(
            "{state_label} was saved, but refs/stash changed before its entry could be dropped"
        )),
        Err(err) => Some(format!(
            "{state_label} was saved, but could not inspect its ordinary stash entry: {err:#}"
        )),
    };
    tracing::info!(stash = %name, %id, "saved worktree state");
    Ok(SavedStash { name, target, warning })
}

fn current(repository_path: &Path, bare: bool) -> Result<Option<ObjectId>> {
    let repo = open_repository(repository_path, bare, false).context("could not inspect refs/stash")?;
    let Some(mut reference) = repo.try_find_reference("refs/stash")? else {
        return Ok(None);
    };
    Ok(Some(reference.peel_to_id()?.detach()))
}

pub(super) fn find(repository_path: &Path, bare: bool, name: gix::refs::FullName) -> Result<Option<SavedStash>> {
    let repo = open_repository(repository_path, bare, false).context("could not inspect saved worktree state")?;
    let Some(reference) = repo.try_find_reference(name.as_ref())? else {
        return Ok(None);
    };
    Ok(Some(SavedStash {
        name,
        target: reference.target().into_owned(),
        warning: None,
    }))
}

#[tracing::instrument(skip_all, fields(stash = %stash.name))]
pub(super) fn apply(repository_path: &Path, bare: bool, workdir: &Path, stash: SavedStash) -> Result<String> {
    let repo = open_repository(repository_path, bare, false)
        .context("could not open repository before applying saved worktree state")?;
    let output = Command::new("git")
        .arg("-C")
        .arg(workdir)
        .args(["stash", "apply", "--index", "--quiet"])
        .arg(stash.name.as_bstr().to_str_lossy().as_ref())
        .output()
        .context("could not launch git stash apply")?;
    let notice = if output.status.success() {
        let mut notice = format!("restored {}", stash.name.shorten());
        if let Err(err) = repo.edit_references([RefEdit::delete(
            stash.name.clone(),
            PreviousValue::MustExistAndMatch(stash.target),
        )]) {
            write!(notice, "; stash reference remains: {err}").expect("writing to a string cannot fail");
        }
        notice
    } else {
        format!(
            "{} restore needs attention; stash reference remains: {}",
            stash.name.shorten(),
            output.stderr.trim().to_str_lossy()
        )
    };
    tracing::info!(stash = %stash.name, success = output.status.success(), "applied saved worktree state");
    Ok(notice)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(path: &Path, args: &[&str]) -> gix_testtools::Result<Vec<u8>> {
        let output = Command::new("git").arg("-C").arg(path).args(args).output()?;
        if !output.status.success() {
            return Err(format!(
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        Ok(output.stdout)
    }

    #[test]
    fn manual_stashes_preserve_git_stashes_and_restore_index_and_worktree_state() -> gix_testtools::Result {
        let fixture = gix_testtools::tempfile::tempdir()?;
        git(fixture.path(), &["init", "-q", "-b", "main"])?;
        crate::test_repository::disable_autocrlf(fixture.path())?;
        git(fixture.path(), &["config", "user.name", "user"])?;
        git(fixture.path(), &["config", "user.email", "user@example.com"])?;
        std::fs::write(fixture.path().join("tracked"), "base\n")?;
        std::fs::write(fixture.path().join(".gitignore"), "ignored\n")?;
        git(fixture.path(), &["add", "."])?;
        git(fixture.path(), &["-c", "commit.gpgSign=false", "commit", "-qm", "base"])?;
        let head = ObjectId::from_hex(git(fixture.path(), &["rev-parse", "HEAD"])?.trim())?;

        std::fs::write(fixture.path().join("ordinary"), "ordinary stash\n")?;
        git(
            fixture.path(),
            &["stash", "push", "--include-untracked", "-qm", "ordinary"],
        )?;
        let ordinary = ObjectId::from_hex(git(fixture.path(), &["rev-parse", "refs/stash"])?.trim())?;

        std::fs::write(fixture.path().join("staged"), "staged\n")?;
        git(fixture.path(), &["add", "staged"])?;
        std::fs::write(fixture.path().join("tracked"), "unstaged\n")?;
        std::fs::write(fixture.path().join("untracked"), "untracked\n")?;
        std::fs::write(fixture.path().join("ignored"), "ignored\n")?;

        let repo = crate::test_repository::open(fixture.path())?;
        let repository_path = repo.git_dir().to_owned();
        drop(repo);
        let notice = save_manual(&repository_path, false, head)?;
        assert!(notice.contains("stashed changes"));

        let repo = crate::test_repository::open(fixture.path())?;
        assert_eq!(repo.find_reference("refs/stash")?.id(), ordinary);
        let name = reference(head)?;
        assert!(repo.try_find_reference(name.as_ref())?.is_some());
        assert!(
            git(fixture.path(), &["status", "--porcelain=v1", "--untracked-files=all"])?.is_empty(),
            "the tracked and untracked changes were stashed"
        );
        assert_eq!(std::fs::read(fixture.path().join("ignored"))?, b"ignored\n");
        drop(repo);

        std::fs::write(fixture.path().join("local"), "existing worktree change\n")?;
        restore_manual(&repository_path, false, head)?;
        assert_eq!(
            git(fixture.path(), &["diff", "--cached", "--name-only"])?.trim(),
            b"staged"
        );
        assert_eq!(git(fixture.path(), &["diff", "--name-only"])?.trim(), b"tracked");
        assert_eq!(std::fs::read(fixture.path().join("untracked"))?, b"untracked\n");
        assert_eq!(
            std::fs::read(fixture.path().join("local"))?,
            b"existing worktree change\n",
            "unstashing leaves unrelated worktree changes intact"
        );
        let repo = crate::test_repository::open(fixture.path())?;
        assert!(repo.try_find_reference(name.as_ref())?.is_none());
        assert_eq!(repo.find_reference("refs/stash")?.id(), ordinary);
        Ok(())
    }

    #[test]
    fn manual_stash_survives_an_index_conflict_and_can_restore_all_saved_files() -> gix_testtools::Result {
        let fixture = gix_testtools::tempfile::tempdir()?;
        git(fixture.path(), &["init", "-q", "-b", "main"])?;
        crate::test_repository::disable_autocrlf(fixture.path())?;
        git(fixture.path(), &["config", "user.name", "user"])?;
        git(fixture.path(), &["config", "user.email", "user@example.com"])?;
        std::fs::write(fixture.path().join("Cargo.lock"), "base\n")?;
        std::fs::write(fixture.path().join("tracked"), "base\n")?;
        git(fixture.path(), &["add", "."])?;
        git(fixture.path(), &["-c", "commit.gpgSign=false", "commit", "-qm", "base"])?;
        let repo = crate::test_repository::open(fixture.path())?;
        let repository_path = repo.git_dir().to_owned();
        let head_commit_id = repo.head_id()?.detach();
        drop(repo);

        std::fs::write(fixture.path().join("Cargo.lock"), "staged\n")?;
        git(fixture.path(), &["add", "Cargo.lock"])?;
        std::fs::write(fixture.path().join("tracked"), "unstaged\n")?;
        std::fs::write(fixture.path().join("untracked"), "untracked\n")?;
        save_manual(&repository_path, false, head_commit_id)?;
        let name = reference(head_commit_id)?;
        let stash_commit_id = crate::test_repository::open(fixture.path())?
            .find_reference(name.as_ref())?
            .id()
            .detach();

        // A conflicting staged change makes --index fail before Git restores the other files.
        std::fs::write(fixture.path().join("Cargo.lock"), "local\n")?;
        git(fixture.path(), &["add", "Cargo.lock"])?;
        let notice = restore_manual(&repository_path, false, head_commit_id)?;
        assert!(
            notice.contains("needs attention"),
            "the index conflict is reported: {notice}"
        );
        assert_eq!(
            std::fs::read(fixture.path().join("Cargo.lock"))?,
            b"local\n",
            "the conflicting local change is preserved"
        );
        assert_eq!(
            std::fs::read(fixture.path().join("tracked"))?,
            b"base\n",
            "the early index failure prevents restoring unstaged changes"
        );
        assert!(
            !fixture.path().join("untracked").exists(),
            "the early index failure also prevents restoring untracked files"
        );
        assert_eq!(
            crate::test_repository::open(fixture.path())?
                .find_reference(name.as_ref())?
                .id(),
            stash_commit_id,
            "a failed apply retains the complete original stash"
        );
        assert!(
            notice.contains("stash reference remains"),
            "the notice explains that saved state remains available: {notice}"
        );

        git(
            fixture.path(),
            &["restore", "--source=HEAD", "--staged", "--worktree", "Cargo.lock"],
        )?;
        restore_manual(&repository_path, false, head_commit_id)?;
        assert_eq!(
            git(fixture.path(), &["show", ":Cargo.lock"])?,
            b"staged\n",
            "retry restores the saved index"
        );
        assert_eq!(
            std::fs::read(fixture.path().join("Cargo.lock"))?,
            b"staged\n",
            "retry restores the staged file's worktree contents"
        );
        assert_eq!(
            std::fs::read(fixture.path().join("tracked"))?,
            b"unstaged\n",
            "retry restores the previously skipped tracked change"
        );
        assert_eq!(
            std::fs::read(fixture.path().join("untracked"))?,
            b"untracked\n",
            "retry restores the previously skipped untracked file"
        );
        assert_eq!(
            git(fixture.path(), &["diff", "--cached", "--name-only"])?.trim(),
            b"Cargo.lock",
            "only the original staged path is staged"
        );
        assert_eq!(
            git(fixture.path(), &["diff", "--name-only"])?.trim(),
            b"tracked",
            "the original unstaged change stays unstaged"
        );
        assert!(
            crate::test_repository::open(fixture.path())?
                .try_find_reference(name.as_ref())?
                .is_none(),
            "only a complete restoration consumes the stash"
        );
        Ok(())
    }
}
