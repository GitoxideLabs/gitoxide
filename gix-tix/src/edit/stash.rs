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

#[derive(Clone)]
pub(super) struct SavedStash {
    pub name: gix::refs::FullName,
    pub target: Target,
    pub warning: Option<String>,
}

#[expect(clippy::too_many_arguments)]
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
