use std::{
    ffi::OsString,
    fmt::Write as _,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};
use gix::{
    ObjectId,
    bstr::ByteSlice,
    refs::{
        Target,
        transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog},
    },
};

use crate::{history, open_repository};

pub(crate) enum Perform {
    Complete(Option<String>),
    Conflict(Conflict),
}

impl Perform {
    #[cfg(test)]
    pub(crate) fn complete(self) -> Result<Option<String>> {
        match self {
            Perform::Complete(notice) => Ok(notice),
            Perform::Conflict(_) => anyhow::bail!("time-travel unexpectedly suspended on a conflict"),
        }
    }
}

pub(crate) struct Conflict {
    rebase: super::rebase::Conflict,
    repository_path: PathBuf,
    bare: bool,
    revisions: Vec<OsString>,
    include_worktrees: bool,
}

impl Conflict {
    pub(crate) fn original(&self) -> ObjectId {
        self.rebase.original()
    }

    #[tracing::instrument(skip_all, fields(commit_id = %self.rebase.original()))]
    pub(crate) fn accept(self) -> Result<(String, ObjectId)> {
        let mut conflict = self.rebase.persist()?;
        let notice = move_head(
            &self.repository_path,
            self.bare,
            conflict.commit,
            &self.revisions,
            self.include_worktrees,
        )?
        .unwrap_or_else(|| format!("checked out {}", conflict.commit.to_hex_with_len(7)));
        delete_deferred_refs(&self.repository_path, self.bare, &conflict.deferred_ref_deletions)?;
        conflict.materialize()?;
        Ok((format!("{notice}; ready to resolve conflicts"), conflict.commit))
    }
}

#[tracing::instrument(skip_all, fields(commit_id = %conflict.original()))]
pub(crate) fn materialize_plan_conflict(
    conflict: super::rebase::PlanConflict,
    repository_path: &Path,
    bare: bool,
    revisions: &[OsString],
    include_worktrees: bool,
) -> Result<(String, ObjectId)> {
    let original = conflict.original();
    let mut conflict = conflict.into_conflict().persist()?;
    let notice = move_head(repository_path, bare, conflict.commit, revisions, include_worktrees)?
        .unwrap_or_else(|| format!("checked out {}", conflict.commit.to_hex_with_len(7)));
    delete_deferred_refs(repository_path, bare, &conflict.deferred_ref_deletions)?;
    conflict.materialize()?;
    tracing::warn!(commit_id = %original, rewritten_id = %conflict.commit, "materialized rebase-todo conflict");
    Ok((format!("{notice}; ready to resolve conflicts"), conflict.commit))
}

pub(crate) fn checkout_without_replay(
    repository_path: &Path,
    bare: bool,
    selected: ObjectId,
    revisions: &[OsString],
    include_worktrees: bool,
) -> Result<Option<String>> {
    move_head(repository_path, bare, selected, revisions, include_worktrees)
}

pub(crate) fn checkout_plan(
    repository_path: &Path,
    bare: bool,
    selected: ObjectId,
    reference: Option<&gix::refs::FullName>,
    deferred_ref_deletions: &[(gix::refs::FullName, ObjectId)],
    revisions: &[OsString],
    include_worktrees: bool,
) -> Result<Option<String>> {
    let notice = move_head_to(repository_path, bare, selected, reference, revisions, include_worktrees)?;
    delete_deferred_refs(repository_path, bare, deferred_ref_deletions)?;
    Ok(notice)
}

fn move_head(
    repository_path: &Path,
    bare: bool,
    selected: ObjectId,
    revisions: &[OsString],
    include_worktrees: bool,
) -> Result<Option<String>> {
    move_head_to(repository_path, bare, selected, None, revisions, include_worktrees)
}

fn move_head_to(
    repository_path: &Path,
    bare: bool,
    selected: ObjectId,
    reference: Option<&gix::refs::FullName>,
    revisions: &[OsString],
    include_worktrees: bool,
) -> Result<Option<String>> {
    let repository = open_repository(repository_path, bare, false).context("could not open repository for checkout")?;
    let workdir = repository
        .workdir()
        .context("time-travel requires a worktree")?
        .to_owned();
    let head = repository.head().context("could not read HEAD before time-travel")?;
    let head_id = head
        .id()
        .map(gix::Id::detach)
        .context("cannot time-travel from an unborn HEAD")?;
    let head_ref = head.referent_name().map(ToOwned::to_owned);
    let saved_target = head_ref.clone().map_or(Target::Object(head_id), Target::Symbolic);
    drop(head);
    if selected == head_id && head_ref.as_ref() == reference {
        return Ok(None);
    }
    let destination_pin = selected_pin(&repository, selected)?;
    let provisional = create_or_reuse_pin(&repository, saved_target, head_id)?;
    drop(repository);
    let checkout = match (reference, &destination_pin) {
        (Some(reference), _) => checkout_branch(&workdir, reference),
        (None, Some(pin)) => checkout_pin(&workdir, pin),
        (None, None) => checkout_detached(&workdir, selected),
    };
    if let Err(checkout) = checkout {
        if provisional.1 {
            let cleanup = open_repository(repository_path, bare, false)
                .context("could not reopen repository to remove a provisional pin")
                .and_then(|repository| delete_pin(&repository, &provisional.0));
            if let Err(cleanup) = cleanup {
                return Err(checkout.context(format!(
                    "checkout failed and {} could not be removed: {cleanup:#}",
                    pin_label(&provisional.0)
                )));
            }
        }
        return Err(checkout);
    }
    let mut notice = reference.map_or_else(
        || {
            destination_pin.as_ref().map_or_else(
                || format!("time-travelled to {}", selected.to_hex_with_len(7)),
                |pin| format!("returned from {}", pin_label(pin)),
            )
        },
        |reference| format!("checked out {}", reference.shorten()),
    );
    let repository =
        open_repository(repository_path, bare, false).context("could not reopen repository after time-travel")?;
    if let Some(pin) = destination_pin
        && let Err(err) = delete_pin(&repository, &pin)
    {
        notice = format!("{notice}; destination pin remains: {err:#}");
    }
    let snapshot = history::snapshot_ignoring_pin(
        &repository,
        revisions,
        &[],
        include_worktrees,
        Some(provisional.0.name.as_bstr()),
    )?;
    if snapshot
        .view_tips
        .iter()
        .copied()
        .any(|tip| contains(&repository, head_id, tip))
    {
        if let Err(err) = delete_pin(&repository, &provisional.0) {
            notice = format!("{notice}; redundant {} remains: {err:#}", pin_label(&provisional.0));
        }
    } else {
        notice = format!("{notice}; saved {}", pin_label(&provisional.0));
    }
    Ok(Some(notice))
}

#[tracing::instrument(skip_all, fields(commit_id = %selected))]
pub(crate) fn perform(
    repository_path: &Path,
    bare: bool,
    mut selected: ObjectId,
    graph: &history::HistoryGraph,
    review_roots: &[ObjectId],
    revisions: &[OsString],
    include_worktrees: bool,
) -> Result<Perform> {
    let mut repository =
        open_repository(repository_path, bare, false).context("could not open repository for time-travel")?;
    repository.workdir().context("time-travel requires a worktree")?;
    let head = repository.head().context("could not read HEAD before time-travel")?;
    let Some(mut head_id) = head.id().map(gix::Id::detach) else {
        anyhow::bail!("cannot time-travel from an unborn HEAD");
    };
    drop(head);
    if repository
        .index_or_empty()
        .context("could not inspect the index before time-travel")?
        .entries()
        .iter()
        .any(|entry| entry.stage() != gix::index::entry::Stage::Unconflicted)
    {
        anyhow::bail!("cannot time-travel with unresolved index conflicts");
    }
    let mut completed_graph = None;
    let mut review_roots = review_roots.to_vec();
    while let Some(base) = pending_base(&repository, selected)? {
        let outcome = super::rebase::perform(
            &repository,
            completed_graph.as_ref().unwrap_or(graph),
            super::rebase::Edit::Repeat {
                base,
                checkout: selected,
            },
            super::rebase::Signature::RedoIfNeeded,
            super::rebase::Tree::CherryPick,
        )?;
        let outcome = match outcome {
            super::rebase::Perform::Complete(outcome) => outcome,
            super::rebase::Perform::Conflict(rebase) => {
                return Ok(Perform::Conflict(Conflict {
                    rebase,
                    repository_path: repository_path.to_owned(),
                    bare,
                    revisions: revisions.to_vec(),
                    include_worktrees,
                }));
            }
        };
        selected = outcome
            .map(selected)
            .context("the time-travel destination disappeared while completing its rebase")?;
        head_id = outcome
            .map(head_id)
            .context("HEAD disappeared while completing its rebase")?;
        review_roots = review_roots.into_iter().filter_map(|id| outcome.map(id)).collect();
        repository = open_repository(repository_path, bare, false)
            .context("could not reopen repository after completing a pending rebase")?;
        completed_graph = Some(super::loaded_graph(&repository)?);
    }
    let graph = completed_graph.as_ref().unwrap_or(graph);
    let source_review = review_tree(&repository, graph, &review_roots, head_id)?;
    let destination_review = review_tree(&repository, graph, &review_roots, selected)?;
    let crosses_review_boundary =
        source_review.as_ref().map(|review| review.root) != destination_review.as_ref().map(|review| review.root);
    let workdir = repository
        .workdir()
        .context("time-travel requires a worktree")?
        .to_owned();
    drop(repository);

    let saved = if crosses_review_boundary {
        source_review
            .as_ref()
            .map(|review| save_review_stash(repository_path, bare, &workdir, review))
            .transpose()?
            .flatten()
    } else {
        None
    };
    let moved = move_head(repository_path, bare, selected, revisions, include_worktrees);
    let mut notice = match moved {
        Ok(notice) => notice,
        Err(err) => {
            let err = match saved {
                Some(stash) => match apply_review_stash(repository_path, bare, &workdir, stash) {
                    Ok(notice) => err.context(format!("source review stash restoration: {notice}")),
                    Err(restore) => err.context(format!("source review stash could not be restored: {restore:#}")),
                },
                None => err,
            };
            return Err(err);
        }
    };
    if let Some(saved) = saved
        && let Some(warning) = saved.warning
    {
        append_notice(&mut notice, warning);
    }
    if crosses_review_boundary
        && let Some(review) = destination_review
        && let Some(stash) = find_review_stash(repository_path, bare, &review)?
    {
        append_notice(&mut notice, apply_review_stash(repository_path, bare, &workdir, stash)?);
    }
    Ok(Perform::Complete(notice))
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReviewTree {
    root: ObjectId,
    reference: gix::refs::FullName,
}

fn review_tree(
    repo: &gix::Repository,
    graph: &history::HistoryGraph,
    roots: &[ObjectId],
    commit: ObjectId,
) -> Result<Option<ReviewTree>> {
    let mut nearest = None;
    for root in roots.iter().copied().filter(|root| graph.is_ancestor(*root, commit)) {
        nearest = match nearest {
            None => Some(root),
            Some(current) if graph.is_ancestor(current, root) => Some(root),
            Some(current) if graph.is_ancestor(root, current) => Some(current),
            Some(_) => anyhow::bail!("commit belongs to multiple unrelated review trees"),
        };
    }
    let Some(root) = nearest else { return Ok(None) };
    let commit = repo.find_commit(root)?.decode()?.into_owned()?;
    let reference = super::review::reference(&commit)?.context("review root lost its review identity")?;
    Ok(Some(ReviewTree { root, reference }))
}

#[derive(Clone)]
struct SavedStash {
    name: gix::refs::FullName,
    target: Target,
    warning: Option<String>,
}

#[tracing::instrument(skip_all, fields(review = %review.reference))]
fn save_review_stash(
    repository_path: &Path,
    bare: bool,
    workdir: &Path,
    review: &ReviewTree,
) -> Result<Option<SavedStash>> {
    if !super::review::is_dirty(workdir)? {
        return Ok(None);
    }
    let name = super::review::stash_reference(review.reference.as_bstr())?;
    let repo =
        open_repository(repository_path, bare, false).context("could not open repository to save review state")?;
    if repo.try_find_reference(name.as_ref())?.is_some() {
        anyhow::bail!("review {} already has saved worktree state", review.reference.shorten());
    }
    let previous = repo
        .try_find_reference("refs/stash")?
        .and_then(|mut reference| reference.peel_to_id().ok().map(gix::Id::detach));
    drop(repo);

    let output = Command::new("git")
        .arg("-C")
        .arg(workdir)
        .args(["stash", "push", "--include-untracked", "--quiet", "--message"])
        .arg(format!("tix review {}", review.reference.shorten()))
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
                message: "tix review auto-stash".into(),
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
                "could not retain review stash; original state was restored".to_owned()
            }
            Ok(output) => format!(
                "could not retain review stash and git stash pop failed: {}",
                output.stderr.trim().to_str_lossy()
            ),
            Err(restore) => format!("could not retain review stash and could not launch git stash pop: {restore}"),
        });
    }
    drop(repo);

    let warning = match current_stash(repository_path, bare)? {
        Some(current) if current == id => {
            let output = Command::new("git")
                .arg("-C")
                .arg(workdir)
                .args(["stash", "drop", "--quiet", "stash@{0}"])
                .output()
                .context("could not launch git stash drop")?;
            (!output.status.success()).then(|| {
                format!(
                    "review state was saved, but its ordinary stash entry remains: {}",
                    output.stderr.trim().to_str_lossy()
                )
            })
        }
        _ => Some("review state was saved, but refs/stash changed before its entry could be dropped".to_owned()),
    };
    tracing::info!(review = %review.reference, stash = %name, %id, "saved review worktree state");
    Ok(Some(SavedStash { name, target, warning }))
}

fn current_stash(repository_path: &Path, bare: bool) -> Result<Option<ObjectId>> {
    let repo = open_repository(repository_path, bare, false).context("could not inspect refs/stash")?;
    let Some(mut reference) = repo.try_find_reference("refs/stash")? else {
        return Ok(None);
    };
    Ok(Some(reference.peel_to_id()?.detach()))
}

fn find_review_stash(repository_path: &Path, bare: bool, review: &ReviewTree) -> Result<Option<SavedStash>> {
    let repo = open_repository(repository_path, bare, false).context("could not inspect saved review state")?;
    let name = super::review::stash_reference(review.reference.as_bstr())?;
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
fn apply_review_stash(repository_path: &Path, bare: bool, workdir: &Path, stash: SavedStash) -> Result<String> {
    let repo =
        open_repository(repository_path, bare, false).context("could not open repository before applying stash")?;
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
    tracing::info!(stash = %stash.name, success = output.status.success(), "applied review worktree state");
    Ok(notice)
}

fn append_notice(notice: &mut Option<String>, addition: String) {
    match notice {
        Some(notice) => write!(notice, "; {addition}").expect("writing to a string cannot fail"),
        None => *notice = Some(addition),
    }
}

fn pending_base(repository: &gix::Repository, selected: ObjectId) -> Result<Option<ObjectId>> {
    let mut current = selected;
    let mut base = None;
    loop {
        let commit = repository
            .find_commit(current)
            .context("could not inspect a time-travel destination for a pending rebase")?
            .decode()?
            .into_owned()?;
        if !super::rebase::is_pending(&commit) {
            break;
        }
        base = Some(current);
        let Some(parent) = commit.parents.first().copied() else {
            break;
        };
        current = parent;
    }
    Ok(base)
}

fn selected_pin(repository: &gix::Repository, selected: ObjectId) -> Result<Option<history::Pin>> {
    let mut pins: Vec<_> = history::all_pins(repository)?
        .into_iter()
        .filter(|pin| pin.id == selected)
        .collect();
    pins.sort_by(|a, b| {
        a.target
            .try_name()
            .is_none()
            .cmp(&b.target.try_name().is_none())
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(pins.into_iter().next())
}

fn create_or_reuse_pin(repository: &gix::Repository, target: Target, id: ObjectId) -> Result<(history::Pin, bool)> {
    let pins = history::all_pins(repository)?;
    if let Some(pin) = pins.iter().find(|pin| pin.target == target) {
        return Ok((pin.clone(), false));
    }
    let hex = id.to_hex().to_string();
    let mut suffix_len = 8.min(hex.len());
    let mut number = 2;
    let name = loop {
        let suffix = if suffix_len <= hex.len() {
            hex[..suffix_len].to_owned()
        } else {
            let suffix = format!("{hex}{number}");
            number += 1;
            suffix
        };
        let name: gix::refs::FullName = format!("{}{}", String::from_utf8_lossy(history::PIN_PREFIX), suffix)
            .try_into()
            .context("generated an invalid tix pin name")?;
        if repository
            .try_find_reference(name.as_ref())
            .context("could not check for a colliding tix pin")?
            .is_none()
        {
            break name;
        }
        if suffix_len < hex.len() {
            suffix_len += 1;
        } else {
            suffix_len = hex.len() + 1;
        }
    };
    repository
        .edit_references([RefEdit {
            change: Change::Update {
                log: LogChange {
                    mode: RefLog::AndReference,
                    force_create_reflog: false,
                    message: "tix time-travel".into(),
                },
                expected: PreviousValue::MustNotExist,
                new: target.clone(),
            },
            name: name.clone(),
            deref: false,
        }])
        .context("could not create tix pin")?;
    Ok((history::Pin { name, target, id }, true))
}

fn delete_pin(repository: &gix::Repository, pin: &history::Pin) -> Result<()> {
    repository
        .edit_references([delete_pin_edit(pin)])
        .context("could not remove tix pin")?;
    Ok(())
}

pub(crate) fn remove_pins(repository_path: &Path, bare: bool, selected: ObjectId) -> Result<usize> {
    let repository =
        open_repository(repository_path, bare, false).context("could not open repository to remove pins")?;
    let pins: Vec<_> = history::all_pins(&repository)?
        .into_iter()
        .filter(|pin| pin.id == selected)
        .collect();
    if pins.is_empty() {
        return Ok(0);
    }
    repository
        .edit_references(pins.iter().map(delete_pin_edit))
        .context("could not remove tix pins")?;
    Ok(pins.len())
}

fn delete_pin_edit(pin: &history::Pin) -> RefEdit {
    RefEdit {
        change: Change::Delete {
            expected: PreviousValue::MustExistAndMatch(pin.target.clone()),
            log: RefLog::AndReference,
        },
        name: pin.name.clone(),
        deref: false,
    }
}

fn delete_deferred_refs(repository_path: &Path, bare: bool, refs: &[(gix::refs::FullName, ObjectId)]) -> Result<()> {
    if refs.is_empty() {
        return Ok(());
    }
    let repository = open_repository(repository_path, bare, false)
        .context("could not reopen repository to finish reference deletions")?;
    repository
        .edit_references(refs.iter().map(|(name, old)| RefEdit {
            name: name.clone(),
            deref: false,
            change: Change::Delete {
                expected: PreviousValue::MustExistAndMatch(Target::Object(*old)),
                log: RefLog::AndReference,
            },
        }))
        .context("could not delete the branch HEAD left during rebase")?;
    Ok(())
}

fn checkout_branch(workdir: &Path, name: &gix::refs::FullName) -> Result<()> {
    let branch = name
        .as_bstr()
        .strip_prefix(b"refs/heads/")
        .context("the rebase checkout target is not a local branch")?;
    checkout(
        workdir,
        [
            OsString::from("--no-guess"),
            gix::path::from_bstr(branch.as_bstr()).into_owned().into_os_string(),
        ],
    )
}

fn checkout_pin(workdir: &Path, pin: &history::Pin) -> Result<()> {
    match pin.target.try_name() {
        Some(name) => {
            let branch = name
                .as_bstr()
                .strip_prefix(b"refs/heads/")
                .context("a symbolic tix pin does not point to a local branch")?;
            checkout(
                workdir,
                [
                    OsString::from("--no-guess"),
                    gix::path::from_bstr(branch.as_bstr()).into_owned().into_os_string(),
                ],
            )
        }
        None => checkout_detached(workdir, pin.id),
    }
}

pub(super) fn checkout_detached(workdir: &Path, id: ObjectId) -> Result<()> {
    checkout(
        workdir,
        [OsString::from("--detach"), OsString::from(id.to_hex().to_string())],
    )
}

pub(super) fn checkout(workdir: &Path, args: impl IntoIterator<Item = OsString>) -> Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(workdir)
        .arg("checkout")
        .args(args)
        .output()
        .context("could not launch git checkout")?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = output.stderr.trim().to_str_lossy();
    if stderr.is_empty() {
        anyhow::bail!("git checkout failed with {}", output.status)
    }
    anyhow::bail!("git checkout failed with {}: {}", output.status, stderr)
}

fn contains(repository: &gix::Repository, ancestor: ObjectId, descendant: ObjectId) -> bool {
    ancestor == descendant
        || repository
            .merge_base(ancestor, descendant)
            .is_ok_and(|base| base.as_ref() == ancestor)
}

fn pin_label(pin: &history::Pin) -> String {
    format!(
        "pin:{}",
        pin.name
            .as_bstr()
            .strip_prefix(history::PIN_PREFIX)
            .unwrap_or(pin.name.as_bstr())
            .to_str_lossy()
    )
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicBool;

    use super::*;

    fn git(path: &Path, args: &[&str]) -> gix_testtools::Result<Vec<u8>> {
        let output = Command::new("git").arg("-C").arg(path).args(args).output()?;
        if !output.status.success() {
            return Err(format!("git {} failed: {}", args.join(" "), output.stderr.trim().to_str_lossy()).into());
        }
        Ok(output.stdout)
    }

    fn review_stash_fixture() -> gix_testtools::Result<(gix_testtools::tempfile::TempDir, PathBuf, SavedStash)> {
        let fixture = gix_testtools::tempfile::tempdir()?;
        git(fixture.path(), &["init", "-q", "-b", "main"])?;
        git(fixture.path(), &["config", "user.name", "reviewer"])?;
        git(fixture.path(), &["config", "user.email", "reviewer@example.com"])?;
        std::fs::write(fixture.path().join("file"), "base\n")?;
        git(fixture.path(), &["add", "file"])?;
        git(
            fixture.path(),
            &["-c", "commit.gpgSign=false", "commit", "-q", "-m", "base"],
        )?;
        std::fs::write(fixture.path().join("file"), "stashed\n")?;
        git(fixture.path(), &["stash", "push", "-q", "-m", "review"])?;
        let name: gix::refs::FullName = "refs/worktree/tix/review/stashes/1".try_into()?;
        git(
            fixture.path(),
            &["update-ref", name.as_bstr().to_str_lossy().as_ref(), "refs/stash"],
        )?;
        git(fixture.path(), &["stash", "drop", "-q", "stash@{0}"])?;
        let repo = crate::open_test_repository(fixture.path())?;
        let repository_path = repo.git_dir().to_owned();
        let target = repo.find_reference(name.as_ref())?.target().into_owned();
        Ok((
            fixture,
            repository_path,
            SavedStash {
                name,
                target,
                warning: None,
            },
        ))
    }

    fn loaded_graph(repository: &gix::Repository, revisions: &[OsString]) -> Result<history::HistoryGraph> {
        let authors = gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(
            history::Authors::default(),
        ));
        let mut graph = None;
        history::load(
            repository,
            revisions,
            &[],
            false,
            &authors,
            &AtomicBool::new(false),
            |event| {
                if let history::Event::Complete(value) = event {
                    graph = Some(value);
                }
                true
            },
        )?;
        graph.context("history traversal did not produce a graph")
    }

    #[test]
    fn review_state_is_stashed_only_when_crossing_its_tree_boundary() -> gix_testtools::Result {
        let fixture = gix_testtools::tempfile::tempdir()?;
        git(fixture.path(), &["init", "-q", "-b", "main"])?;
        git(fixture.path(), &["config", "user.name", "reviewer"])?;
        git(fixture.path(), &["config", "user.email", "reviewer@example.com"])?;
        for (name, contents) in [("staged", "base\n"), ("unstaged", "base\n")] {
            std::fs::write(fixture.path().join(name), contents)?;
        }
        git(fixture.path(), &["add", "."])?;
        git(
            fixture.path(),
            &["-c", "commit.gpgSign=false", "commit", "-q", "-m", "base"],
        )?;
        let base = ObjectId::from_hex(git(fixture.path(), &["rev-parse", "HEAD"])?.trim())?;
        for name in ["staged", "unstaged"] {
            std::fs::write(fixture.path().join(name), "tip\n")?;
        }
        git(fixture.path(), &["-c", "commit.gpgSign=false", "commit", "-qam", "tip"])?;
        let tip = ObjectId::from_hex(git(fixture.path(), &["rev-parse", "HEAD"])?.trim())?;

        std::fs::write(fixture.path().join("existing"), "user stash\n")?;
        git(
            fixture.path(),
            &["stash", "push", "--include-untracked", "-q", "-m", "existing"],
        )?;
        let existing_stash = ObjectId::from_hex(git(fixture.path(), &["rev-parse", "refs/stash"])?.trim())?;
        let repo = crate::open_test_repository(fixture.path())?;
        let graph = loaded_graph(&repo, &[])?;
        drop(repo);
        let started = super::super::review::start(fixture.path(), false, &graph, tip, base)?;

        git(fixture.path(), &["add", "staged"])?;
        std::fs::write(fixture.path().join("untracked"), "new\n")?;
        let before = git(fixture.path(), &["status", "--porcelain=v1", "--untracked-files=all"])?;
        let child = ObjectId::from_hex(
            git(
                fixture.path(),
                &[
                    "-c",
                    "commit.gpgSign=false",
                    "commit-tree",
                    &format!("{}^{{tree}}", started.commit),
                    "-p",
                    &started.commit.to_string(),
                    "-m",
                    "review child",
                ],
            )?
            .trim(),
        )?;
        git(
            fixture.path(),
            &["update-ref", "refs/worktree/tix/pins/child", &child.to_string()],
        )?;
        let repo = crate::open_test_repository(fixture.path())?;
        let repository_path = repo.git_dir().to_owned();
        let graph = loaded_graph(&repo, &[])?;
        assert_eq!(
            review_tree(&repo, &graph, &[started.commit], started.commit)?.map(|tree| tree.root),
            Some(started.commit)
        );
        assert_eq!(
            review_tree(&repo, &graph, &[started.commit], child)?.map(|tree| tree.root),
            Some(started.commit)
        );
        drop(repo);

        perform(&repository_path, false, child, &graph, &[started.commit], &[], false)?.complete()?;
        assert_eq!(
            git(fixture.path(), &["status", "--porcelain=v1", "--untracked-files=all"])?,
            before,
            "moving within one review tree leaves index and worktree handling to checkout"
        );
        let stash_name = super::super::review::stash_reference(started.reference.as_bstr())?;
        assert!(
            crate::open_test_repository(fixture.path())?
                .try_find_reference(stash_name.as_ref())?
                .is_none(),
            "no stash is created inside the review tree"
        );

        perform(&repository_path, false, tip, &graph, &[started.commit], &[], false)?.complete()?;
        let repo = crate::open_test_repository(fixture.path())?;
        assert!(repo.try_find_reference(stash_name.as_ref())?.is_some());
        assert_eq!(repo.find_reference("refs/stash")?.id().detach(), existing_stash);
        assert!(
            git(fixture.path(), &["status", "--porcelain=v1", "--untracked-files=all"])?.is_empty(),
            "crossing out leaves the destination clean"
        );
        drop(repo);

        perform(&repository_path, false, child, &graph, &[started.commit], &[], false)?.complete()?;
        assert_eq!(
            git(fixture.path(), &["status", "--porcelain=v1", "--untracked-files=all"])?,
            before,
            "returning through any review descendant restores exact review state"
        );
        let repo = crate::open_test_repository(fixture.path())?;
        assert!(repo.try_find_reference(stash_name.as_ref())?.is_none());
        assert_eq!(repo.find_reference("refs/stash")?.id().detach(), existing_stash);
        Ok(())
    }

    #[test]
    fn review_stash_references_are_consumed_after_any_git_apply_result() -> gix_testtools::Result {
        let (fixture, repository_path, stash) = review_stash_fixture()?;
        std::fs::write(fixture.path().join("file"), "destination\n")?;
        git(
            fixture.path(),
            &["-c", "commit.gpgSign=false", "commit", "-qam", "destination"],
        )?;
        let notice = apply_review_stash(&repository_path, false, fixture.path(), stash.clone())?;
        assert!(notice.contains("needs attention"), "the conflict is reported: {notice}");
        let repo = crate::open_test_repository(fixture.path())?;
        assert!(repo.try_find_reference(stash.name.as_ref())?.is_none());
        assert!(
            repo.index_or_empty()?
                .entries()
                .iter()
                .any(|entry| entry.stage() != gix::index::entry::Stage::Unconflicted),
            "Git's ordinary stash conflict remains in the index"
        );

        let (fixture, repository_path, stash) = review_stash_fixture()?;
        std::fs::write(fixture.path().join(".git/index.lock"), "locked")?;
        let notice = apply_review_stash(&repository_path, false, fixture.path(), stash.clone())?;
        assert!(
            notice.contains("needs attention"),
            "the fatal apply failure is reported: {notice}"
        );
        assert!(
            crate::open_test_repository(fixture.path())?
                .try_find_reference(stash.name.as_ref())?
                .is_none(),
            "the review stash ref is consumed even when Git cannot apply it"
        );
        Ok(())
    }

    #[test]
    fn nested_review_trees_use_the_nearest_review_root() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::open_test_repository(fixture.path())?;
        let original = repo.head_id()?.detach();
        let mut commit = repo.find_commit(original)?.decode()?.into_owned()?;
        commit.parents = [original].into_iter().collect();
        commit.message = "outer review".into();
        commit
            .extra_headers
            .push(("tix-rebase".into(), "onto refs/worktree/tix/review/1".into()));
        let outer = repo.write_object(&commit)?.detach();
        commit.parents = [outer].into_iter().collect();
        commit.message = "middle".into();
        commit.extra_headers.clear();
        let middle = repo.write_object(&commit)?.detach();
        commit.parents = [middle].into_iter().collect();
        commit.message = "inner review".into();
        commit
            .extra_headers
            .push(("tix-rebase".into(), "onto refs/worktree/tix/review/2".into()));
        let inner = repo.write_object(&commit)?.detach();
        commit.parents = [inner].into_iter().collect();
        commit.message = "tip".into();
        commit.extra_headers.clear();
        let tip = repo.write_object(&commit)?.detach();
        repo.reference(
            "refs/heads/main",
            tip,
            PreviousValue::ExistingMustMatch(Target::Object(original)),
            "prepare nested reviews",
        )?;
        let graph = loaded_graph(&repo, &[])?;

        assert_eq!(
            review_tree(&repo, &graph, &[outer, inner], middle)?.map(|tree| tree.root),
            Some(outer)
        );
        assert_eq!(
            review_tree(&repo, &graph, &[outer, inner], tip)?.map(|tree| tree.root),
            Some(inner)
        );
        Ok(())
    }

    #[test]
    fn travels_with_symbolic_and_direct_pins_and_returns() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = crate::open_test_repository(fixture.path())?;
        let repository_path = repository.git_dir().to_owned();
        let root = repository.rev_parse_single("main~2")?.detach();
        let main = repository.rev_parse_single("main")?.detach();
        let topic = repository.rev_parse_single("topic")?.detach();
        let graph = loaded_graph(&repository, &[])?;
        assert!(graph.is_ancestor(root, main), "the selected root is known ancestry");
        assert!(history::all_pins(&repository)?.is_empty());
        assert_eq!(
            open_repository(&repository_path, false, false)?.head_id()?.detach(),
            main
        );
        assert!(!contains(&repository, main, root));
        drop(repository);

        let notice = perform(&repository_path, false, root, &graph, &[], &[], false)?
            .complete()?
            .context("time-travel changed HEAD")?;
        assert!(notice.contains("saved pin:"), "{notice}");
        let repository = crate::open_test_repository(fixture.path())?;
        assert!(repository.head()?.is_detached(), "travel detaches HEAD");
        assert_eq!(repository.head_id()?.detach(), root);
        let pins = history::all_pins(&repository)?;
        assert_eq!(pins.len(), 1, "the lost branch tip gets one pin");
        assert_eq!(
            pins[0].target.try_name().map(gix::refs::FullNameRef::as_bstr),
            Some(b"refs/heads/main".as_bstr())
        );
        assert_eq!(pins[0].id, main);
        assert!(
            history::snapshot(&repository, &[], &[], false)?
                .view_tips
                .contains(&main)
        );

        repository
            .find_reference("refs/heads/main")?
            .set_target_id(topic, "advance pinned branch")?;
        let advanced = history::snapshot(&repository, &[], &[], false)?;
        assert!(advanced.view_tips.contains(&topic), "a symbolic pin follows its branch");
        drop(repository);

        perform(&repository_path, false, topic, &graph, &[], &[], false)?.complete()?;
        let repository = crate::open_test_repository(fixture.path())?;
        assert_eq!(
            repository.head_name()?.map(|name| name.as_bstr().to_owned()),
            Some(b"refs/heads/main".into()),
            "returning through a symbolic pin reattaches HEAD"
        );
        assert!(history::all_pins(&repository)?.is_empty(), "the used pin is removed");

        let detach = Command::new("git")
            .arg("-C")
            .arg(fixture.path())
            .args(["checkout", "--detach", &main.to_hex().to_string()])
            .status()?;
        assert!(detach.success());
        let graph = loaded_graph(&crate::open_test_repository(fixture.path())?, &[])?;
        perform(&repository_path, false, root, &graph, &[], &[], false)?.complete()?;
        let pin = history::all_pins(&crate::open_test_repository(fixture.path())?)?
            .pop()
            .context("direct pin is present")?;
        assert_eq!(pin.target.try_id().map(ToOwned::to_owned), Some(main));
        perform(&repository_path, false, main, &graph, &[], &[], false)?.complete()?;
        let repository = crate::open_test_repository(fixture.path())?;
        assert!(
            repository.head()?.is_detached(),
            "a direct pin returns to detached HEAD"
        );
        assert_eq!(repository.head_id()?.detach(), main);
        assert!(history::all_pins(&repository)?.is_empty());
        Ok(())
    }

    #[test]
    fn removes_every_pin_at_the_selected_commit() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = crate::open_test_repository(fixture.path())?;
        let repository_path = repository.git_dir().to_owned();
        let selected = repository.rev_parse_single("main")?.detach();
        let other = repository.rev_parse_single("topic")?.detach();
        for (name, target) in [
            ("refs/worktree/tix/pins/first", selected),
            ("refs/worktree/tix/pins/second", selected),
            ("refs/worktree/tix/pins/other", other),
        ] {
            repository.reference(name, target, PreviousValue::MustNotExist, "test pin removal")?;
        }
        drop(repository);

        assert_eq!(remove_pins(&repository_path, false, selected)?, 2);
        let repository = crate::open_test_repository(fixture.path())?;
        assert_eq!(
            history::all_pins(&repository)?
                .into_iter()
                .map(|pin| pin.id)
                .collect::<Vec<_>>(),
            [other],
            "pins on other commits remain"
        );
        Ok(())
    }

    #[test]
    fn sideways_travel_preserves_an_unreferenced_departure() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = crate::open_test_repository(fixture.path())?;
        let repository_path = repository.git_dir().to_owned();
        let main = repository.rev_parse_single("main")?.detach();
        let topic = repository.rev_parse_single("topic")?.detach();
        let graph = loaded_graph(&repository, &[])?;
        drop(repository);
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["checkout", "--detach", &main.to_string()])
                .status()?
                .success()
        );
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["branch", "-D", "main"])
                .status()?
                .success()
        );

        perform(&repository_path, false, topic, &graph, &[], &[], false)?.complete()?;
        let repository = crate::open_test_repository(fixture.path())?;
        assert_eq!(repository.head_id()?.detach(), topic);
        let pins = history::all_pins(&repository)?;
        assert_eq!(pins.len(), 1, "sideways travel retains the otherwise lost departure");
        assert_eq!(pins[0].id, main);
        assert!(
            pins[0].target.try_name().is_none(),
            "the detached departure gets a direct pin"
        );
        Ok(())
    }

    #[test]
    fn explicit_tips_avoid_redundant_pins_and_failed_checkouts_clean_up() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = crate::open_test_repository(fixture.path())?;
        let repository_path = repository.git_dir().to_owned();
        let root = repository.rev_parse_single("main~2")?.detach();
        let main = repository.rev_parse_single("main")?.detach();
        let revisions = [OsString::from("main")];
        let graph = loaded_graph(&repository, &revisions)?;
        drop(repository);

        perform(&repository_path, false, root, &graph, &[], &revisions, false)?.complete()?;
        assert!(
            history::all_pins(&crate::open_test_repository(fixture.path())?)?.is_empty(),
            "an explicit tip already retains the former HEAD"
        );

        let checkout = Command::new("git")
            .arg("-C")
            .arg(fixture.path())
            .args(["checkout", "--no-guess", "main"])
            .status()?;
        assert!(checkout.success());
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["update-ref", "refs/worktree/tix/pins/destination", &root.to_string(),])
                .status()?
                .success()
        );
        std::fs::write(fixture.path().join("main"), "dirty\n")?;
        let err = perform(&repository_path, false, root, &graph, &[], &[], false)
            .and_then(Perform::complete)
            .expect_err("Git rejects a conflicting checkout");
        assert!(format!("{err:#}").contains("git checkout failed"));
        let repository = crate::open_test_repository(fixture.path())?;
        assert_eq!(repository.head_id()?.detach(), main, "failed checkout retains HEAD");
        let pins = history::all_pins(&repository)?;
        assert_eq!(pins.len(), 1, "the destination pin survives a failed checkout");
        assert_eq!(pins[0].id, root);
        Ok(())
    }

    #[test]
    fn conflicting_pending_rebases_are_unobservable_until_accepted() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_conflict.sh")?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["config", "gitoxide.commit.committerDate", "2001-01-01T00:00:00 +0000",])
                .status()?
                .success()
        );
        let repository = crate::open_test_repository(fixture.path())?;
        let repository_path = repository.git_dir().to_owned();
        let middle = repository.rev_parse_single("HEAD~1")?.detach();
        std::fs::write(fixture.path().join("after"), "after\n")?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["add", "after"])
                .status()?
                .success()
        );
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["commit", "-q", "-m", "after"])
                .env("GIT_AUTHOR_DATE", "2000-01-04T00:00:00 +0000")
                .env("GIT_COMMITTER_DATE", "2000-01-04T00:00:00 +0000")
                .status()?
                .success()
        );
        let graph = super::super::loaded_graph(&repository)?;
        super::super::rebase::perform(
            &repository,
            &graph,
            super::super::rebase::Edit::Remove { target: middle },
            super::super::rebase::Signature::RedoIfNeeded,
            super::super::rebase::Tree::LeaveAsIsAndMark,
        )?
        .complete()?;
        let graph = super::super::loaded_graph(&repository)?;
        let root = repository.rev_parse_single("HEAD~2")?.detach();
        let tip = repository.find_reference("refs/heads/main")?.id().detach();
        drop(repository);
        perform(&repository_path, false, root, &graph, &[], &[], false)?.complete()?;
        let repository = crate::open_test_repository(fixture.path())?;
        let graph = super::super::loaded_graph(&repository)?;
        let before = gix_testtools::repository::snapshot(fixture.path())?;

        let Perform::Conflict(conflict) = perform(&repository_path, false, tip, &graph, &[], &[], false)? else {
            return Err("the pending rebase should suspend at its conflicting cherry-pick".into());
        };
        assert_eq!(
            gix_testtools::repository::snapshot(fixture.path())?,
            before,
            "preparing the exact merge result changes no repository state"
        );

        let (_notice, conflict_id) = conflict.accept()?;
        let repository = crate::open_test_repository(fixture.path())?;
        assert_eq!(
            repository.head_id()?.detach(),
            conflict_id,
            "the conflicting commit is checked out"
        );
        let conflict_commit = repository.find_commit(conflict_id)?;
        assert_eq!(
            conflict_commit.tree_id()?.detach(),
            conflict_commit
                .parent_ids()
                .next()
                .expect("a cherry-picked commit has a parent")
                .object()?
                .peel_to_tree()?
                .id,
            "the conflicting commit records the ours tree"
        );
        assert!(repository.head()?.is_detached(), "conflict resolution detaches HEAD");
        let branch = repository.find_reference("refs/heads/main")?.id().detach();
        assert_ne!(
            branch, conflict_id,
            "the remaining descendant stays on the saved branch"
        );
        assert!(
            super::super::rebase::has_marker(&repository.find_commit(branch)?.decode()?.into_owned()?),
            "remaining descendants stay as lazy rewrites"
        );
        let index = repository.index_or_empty()?;
        assert!(
            index
                .entries()
                .iter()
                .any(|entry| entry.stage() != gix::index::entry::Stage::Unconflicted),
            "the retained merge outcome supplies unresolved index stages"
        );
        assert!(
            std::fs::read(fixture.path().join("file"))?
                .as_bstr()
                .contains_str("<<<<<<<"),
            "the checked-out merge tree contains conflict markers"
        );
        insta::assert_snapshot!(
            "accepted-pending-rebase-conflict",
            gix_testtools::repository::snapshot(fixture.path())?
                .to_string()
                .replace("\n  \n", "\n\n")
        );
        let err = perform(&repository_path, false, root, &graph, &[], &[], false)
            .and_then(Perform::complete)
            .expect_err("time-travel is disabled until the index conflict is resolved");
        assert!(format!("{err:#}").contains("unresolved index conflicts"));
        Ok(())
    }

    #[test]
    fn time_travel_materializes_only_the_pending_path_to_the_destination() -> gix_testtools::Result {
        if !gix_testtools::signature::program_available("ssh-keygen") {
            return Ok(());
        }
        let (_key_home, key) = gix_testtools::signature::ssh_private_key()?;
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let allowed_signers = gix_testtools::signature::fixture("ssh-allowed-signers");
        git(fixture.path(), &["config", "commit.gpgSign", "true"])?;
        git(fixture.path(), &["config", "gpg.format", "ssh"])?;
        git(
            fixture.path(),
            &["config", "user.signingKey", key.to_string_lossy().as_ref()],
        )?;
        git(
            fixture.path(),
            &[
                "config",
                "gpg.ssh.allowedSignersFile",
                allowed_signers.to_string_lossy().as_ref(),
            ],
        )?;
        let repository = crate::open_test_repository(fixture.path())?;
        let repository_path = repository.git_dir().to_owned();
        let middle = repository.rev_parse_single("HEAD~1")?.detach();
        let root = repository.rev_parse_single("HEAD~2")?.detach();
        let graph = super::super::loaded_graph(&repository)?;
        let commit = repository.find_commit(middle)?.decode()?.into_owned()?;
        let signed_middle = super::super::rebase::perform(
            &repository,
            &graph,
            super::super::rebase::Edit::Replace { target: middle, commit },
            super::super::rebase::Signature::RedoIfNeeded,
            super::super::rebase::Tree::LeaveAsIs,
        )?
        .complete()?
        .selected
        .expect("signing rewrites the middle commit");
        drop(repository);
        git(
            fixture.path(),
            &["checkout", "-q", "--detach", &signed_middle.to_string()],
        )?;

        let repository = crate::open_test_repository(fixture.path())?;
        let graph = super::super::loaded_graph(&repository)?;
        let pending_middle =
            super::super::head::perform(repository.clone(), &graph, super::super::head::Kind::Spill, None)?
                .expect("spilling changes the middle commit");
        let pending_tip = repository.find_reference("refs/heads/main")?.id().detach();
        let middle_commit = repository.find_commit(pending_middle)?.decode()?.into_owned()?;
        let tip_commit = repository.find_commit(pending_tip)?.decode()?.into_owned()?;
        assert!(super::super::rebase::is_pending(&middle_commit));
        assert!(
            !super::super::rebase::has_marker(&middle_commit),
            "the authoritative spilled tree needs no original-parent marker"
        );
        let authors = gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(
            history::Authors::default(),
        ));
        assert_eq!(
            history::load_metadata(&repository, pending_middle, &authors)?
                .0
                .signature,
            crate::app::SignatureState::PendingRebase,
            "an empty signature keeps the authoritative commit visibly pending"
        );
        assert!(super::super::rebase::has_marker(&tip_commit));
        drop(repository);

        let repository = crate::open_test_repository(fixture.path())?;
        let graph = super::super::loaded_graph(&repository)?;
        perform(&repository_path, false, root, &graph, &[], &[], false)?.complete()?;
        let repository = crate::open_test_repository(fixture.path())?;
        assert_eq!(repository.find_reference("refs/heads/main")?.id().detach(), pending_tip);
        assert!(super::super::rebase::is_pending(
            &repository.find_commit(pending_middle)?.decode()?.into_owned()?
        ));
        assert!(super::super::rebase::is_pending(
            &repository.find_commit(pending_tip)?.decode()?.into_owned()?
        ));
        let graph = super::super::loaded_graph(&repository)?;
        drop(repository);

        perform(&repository_path, false, pending_middle, &graph, &[], &[], false)?.complete()?;

        let repository = crate::open_test_repository(fixture.path())?;
        let materialized_middle = repository.head_id()?.detach();
        let still_pending_tip = repository.find_reference("refs/heads/main")?.id().detach();
        assert_ne!(materialized_middle, pending_middle);
        assert_ne!(still_pending_tip, pending_tip);
        let middle_commit = repository.find_commit(materialized_middle)?;
        assert!(!super::super::rebase::is_pending(
            &middle_commit.decode()?.into_owned()?
        ));
        assert!(
            middle_commit
                .verify_signature()?
                .expect("returning to the spilled commit adds its configured signature")
                .is_valid()
        );
        drop(middle_commit);
        let tip_commit = repository.find_commit(still_pending_tip)?.decode()?.into_owned()?;
        assert_eq!(tip_commit.parents.first().copied(), Some(materialized_middle));
        assert!(super::super::rebase::is_pending(&tip_commit));
        assert!(super::super::rebase::has_marker(&tip_commit));
        let graph = super::super::loaded_graph(&repository)?;
        drop(repository);

        perform(&repository_path, false, still_pending_tip, &graph, &[], &[], false)?.complete()?;
        let repository = crate::open_test_repository(fixture.path())?;
        let tip_commit = repository.find_commit(repository.head_id()?)?;
        assert!(!super::super::rebase::is_pending(&tip_commit.decode()?.into_owned()?));
        assert!(
            tip_commit
                .verify_signature()?
                .expect("travelling to the remaining descendant signs it")
                .is_valid()
        );
        Ok(())
    }
}
