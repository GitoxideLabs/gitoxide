use std::{fmt::Write as _, path::Path, process::Command};

use anyhow::{Context, Result};
use gix::{
    ObjectId,
    bstr::ByteSlice,
    refs::{
        Target,
        transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog},
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

#[derive(Clone)]
pub(super) struct SavedStash {
    pub name: gix::refs::FullName,
    pub target: Target,
    pub warning: Option<String>,
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
    if let Err(err) = repo.edit_references([RefEdit {
        change: Change::Update {
            log: LogChange {
                mode: RefLog::AndReference,
                force_create_reflog: false,
                message: reflog_message.into(),
            },
            expected: PreviousValue::MustNotExist,
            new: target.clone(),
        },
        name: name.clone(),
        deref: false,
    }]) {
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

    let warning = match current(repository_path, bare)? {
        Some(current) if current == id => {
            let output = Command::new("git")
                .arg("-C")
                .arg(workdir)
                .args(["stash", "drop", "--quiet", "stash@{0}"])
                .output()
                .context("could not launch git stash drop")?;
            (!output.status.success()).then(|| {
                format!(
                    "{state_label} was saved, but its ordinary stash entry remains: {}",
                    output.stderr.trim().to_str_lossy()
                )
            })
        }
        _ => Some(format!(
            "{state_label} was saved, but refs/stash changed before its entry could be dropped"
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
    let deletion = repo.edit_references([RefEdit {
        change: Change::Delete {
            expected: PreviousValue::MustExistAndMatch(stash.target),
            log: RefLog::AndReference,
        },
        name: stash.name.clone(),
        deref: false,
    }]);
    let mut notice = if output.status.success() {
        format!("restored {}", stash.name.shorten())
    } else {
        format!(
            "{} restore needs attention: {}",
            stash.name.shorten(),
            output.stderr.trim().to_str_lossy()
        )
    };
    if let Err(err) = deletion {
        write!(notice, "; stash reference remains: {err}").expect("writing to a string cannot fail");
    }
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

        let repo = crate::open_test_repository(fixture.path())?;
        let repository_path = repo.git_dir().to_owned();
        drop(repo);
        let notice = save_manual(&repository_path, false, head)?;
        assert!(notice.contains("stashed changes"));

        let repo = crate::open_test_repository(fixture.path())?;
        assert_eq!(repo.find_reference("refs/stash")?.id().detach(), ordinary);
        let name = reference(head)?;
        assert!(repo.try_find_reference(name.as_ref())?.is_some());
        assert!(
            git(fixture.path(), &["status", "--porcelain=v1", "--untracked-files=all"])?.is_empty(),
            "the tracked and untracked changes were stashed"
        );
        assert_eq!(std::fs::read(fixture.path().join("ignored"))?, b"ignored\n");
        drop(repo);

        let saved = find(&repository_path, false, name.clone())?.expect("the manual stash exists");
        apply(&repository_path, false, fixture.path(), saved)?;
        assert_eq!(
            git(fixture.path(), &["diff", "--cached", "--name-only"])?.trim(),
            b"staged"
        );
        assert_eq!(git(fixture.path(), &["diff", "--name-only"])?.trim(), b"tracked");
        assert_eq!(std::fs::read(fixture.path().join("untracked"))?, b"untracked\n");
        let repo = crate::open_test_repository(fixture.path())?;
        assert!(repo.try_find_reference(name.as_ref())?.is_none());
        assert_eq!(repo.find_reference("refs/stash")?.id().detach(), ordinary);
        Ok(())
    }
}
