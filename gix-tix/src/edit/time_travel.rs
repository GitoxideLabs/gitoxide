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

#[cfg(test)]
use super::stash::SavedStash;

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
        let notice = move_head_to(
            &self.repository_path,
            self.bare,
            conflict.commit,
            None,
            &self.revisions,
            self.include_worktrees,
            |id| conflict.map(id),
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
    let notice = move_head_to(
        repository_path,
        bare,
        conflict.commit,
        None,
        revisions,
        include_worktrees,
        |id| conflict.map(id),
    )?
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

pub(crate) fn checkout_review_return(
    repository_path: &Path,
    bare: bool,
    name: &gix::refs::FullName,
    revisions: &[OsString],
    include_worktrees: bool,
) -> Result<(ObjectId, Option<String>)> {
    let repository = open_repository(repository_path, bare, false).context("could not open review return checkout")?;
    let workdir = repository
        .workdir()
        .context("review cancellation requires a worktree")?
        .to_owned();
    let mut target = repository
        .find_reference(name.as_ref())
        .context("the review return reference is missing")?;
    let reference = if name.as_bstr().starts_with(history::PIN_PREFIX) {
        target.target().try_name().map(ToOwned::to_owned)
    } else {
        Some(name.clone())
    };
    let selected = target
        .peel_to_id()
        .context("the review return reference does not resolve")?
        .detach();
    drop(repository);
    checkout(&workdir, [OsString::from("--force"), OsString::from("HEAD")])
        .context("could not discard the cancelled review checkout")?;
    let notice = move_head_to(
        repository_path,
        bare,
        selected,
        reference.as_ref(),
        revisions,
        include_worktrees,
        |_| None,
    )?;
    Ok((selected, notice))
}

pub(crate) fn checkout_plan(
    repository_path: &Path,
    bare: bool,
    outcome: &super::rebase::Outcome,
    revisions: &[OsString],
    include_worktrees: bool,
) -> Result<Option<String>> {
    let selected = outcome.selected.context("the rebase plan does not select a checkout")?;
    let notice = move_head_to(
        repository_path,
        bare,
        selected,
        outcome.checkout_reference.as_ref(),
        revisions,
        include_worktrees,
        |id| outcome.map(id),
    )?;
    delete_deferred_refs(repository_path, bare, &outcome.deferred_ref_deletions)?;
    Ok(notice)
}

fn move_head(
    repository_path: &Path,
    bare: bool,
    selected: ObjectId,
    revisions: &[OsString],
    include_worktrees: bool,
) -> Result<Option<String>> {
    move_head_to(
        repository_path,
        bare,
        selected,
        None,
        revisions,
        include_worktrees,
        Some,
    )
}

fn move_head_to<F>(
    repository_path: &Path,
    bare: bool,
    selected: ObjectId,
    reference: Option<&gix::refs::FullName>,
    revisions: &[OsString],
    include_worktrees: bool,
    map_departure: F,
) -> Result<Option<String>>
where
    F: FnOnce(ObjectId) -> Option<ObjectId>,
{
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
    let departure = map_departure(head_id);
    drop(head);
    if selected == head_id && head_ref.as_ref() == reference {
        return Ok(None);
    }
    let destination_pin = selected_pin(&repository, selected)?;
    let checkout_detaches = reference.is_none()
        && destination_pin
            .as_ref()
            .is_none_or(|pin| pin.target.try_name().is_none());
    let head_pin = head_ref
        .as_ref()
        .filter(|name| checkout_detaches && name.as_bstr().starts_with(b"refs/heads/"))
        .map(|name| create_or_update_head_pin(&repository, name, head_id))
        .transpose()?;
    let provisional = head_pin
        .is_none()
        .then_some(departure)
        .flatten()
        .filter(|departure| *departure != selected)
        .map(|departure| {
            let target = head_ref.clone().map_or(Target::Object(departure), Target::Symbolic);
            create_or_reuse_pin(&repository, target, departure, "tix time-travel")
        })
        .transpose()?;
    drop(repository);
    let checkout = match (reference, &destination_pin) {
        (Some(reference), _) => checkout_reference(repository_path, bare, &workdir, selected, reference),
        (None, Some(pin)) => checkout_pin(&workdir, pin),
        (None, None) => checkout_detached(&workdir, selected),
    };
    if let Err(checkout) = checkout {
        let cleanup_pin = head_pin
            .as_ref()
            .or_else(|| provisional.as_ref().and_then(|(pin, created)| created.then_some(pin)));
        if let Some(pin) = cleanup_pin {
            let cleanup = open_repository(repository_path, bare, false)
                .context("could not reopen repository to remove a provisional pin")
                .and_then(|repository| delete_pin(&repository, pin));
            if let Err(cleanup) = cleanup {
                return Err(checkout.context(format!(
                    "checkout failed and {} could not be removed: {cleanup:#}",
                    pin_label(pin)
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
    if let Some(addition) = reconcile_head_pin(&repository, &workdir)? {
        notice = format!("{notice}; {addition}");
    }
    if let Some((provisional, _)) = provisional {
        let snapshot = history::snapshot_ignoring_pin(
            &repository,
            revisions,
            &[],
            include_worktrees,
            Some(provisional.name.as_bstr()),
        )?;
        if snapshot
            .view_tips
            .iter()
            .copied()
            .any(|tip| contains(&repository, provisional.id, tip))
        {
            if let Err(err) = delete_pin(&repository, &provisional) {
                notice = format!("{notice}; redundant {} remains: {err:#}", pin_label(&provisional));
            }
        } else {
            notice = format!("{notice}; saved {}", pin_label(&provisional));
        }
    }
    Ok(Some(notice))
}

pub(crate) fn perform(
    repository_path: &Path,
    bare: bool,
    selected: ObjectId,
    graph: &history::HistoryGraph,
    review_roots: &[ObjectId],
    revisions: &[OsString],
    include_worktrees: bool,
) -> Result<Perform> {
    perform_with_progress(
        repository_path,
        bare,
        selected,
        graph,
        review_roots,
        revisions,
        include_worktrees,
        |_| {},
    )
}

#[tracing::instrument(skip_all, fields(commit_id = %selected))]
#[expect(clippy::too_many_arguments, reason = "time travel context plus progress reporting")]
pub(crate) fn perform_with_progress(
    repository_path: &Path,
    bare: bool,
    mut selected: ObjectId,
    graph: &history::HistoryGraph,
    review_roots: &[ObjectId],
    revisions: &[OsString],
    include_worktrees: bool,
    mut report: impl FnMut(super::rebase::Progress),
) -> Result<Perform> {
    let mut repository =
        open_repository(repository_path, bare, false).context("could not open repository for time-travel")?;
    repository.workdir().context("time-travel requires a worktree")?;
    let head = repository.head().context("could not read HEAD before time-travel")?;
    let Some(mut head_id) = head.id().map(gix::Id::detach) else {
        anyhow::bail!("cannot time-travel from an unborn HEAD");
    };
    let head_was_detached = head.is_detached();
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
    let mut completed_progress = super::rebase::Progress::default();
    let mut review_roots = review_roots.to_vec();
    while let Some(base) = pending_base(&repository, selected)? {
        let previous = completed_progress;
        let mut latest = previous;
        let outcome = super::rebase::perform_with_progress(
            &repository,
            completed_graph.as_ref().unwrap_or(graph),
            super::rebase::Edit::Repeat {
                base,
                checkout: selected,
            },
            super::rebase::Signature::RedoIfNeeded,
            super::rebase::Tree::CherryPick,
            |progress| {
                latest = append_progress(previous, progress);
                report(latest);
            },
        )?;
        completed_progress = latest;
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
    let moved = move_head_to(
        repository_path,
        bare,
        selected,
        None,
        revisions,
        include_worktrees,
        |actual| {
            if head_was_detached { Some(head_id) } else { Some(actual) }
        },
    );
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
    if let Some(stash) = super::stash::find(repository_path, bare, super::stash::reference(selected)?)? {
        append_notice(
            &mut notice,
            super::stash::apply(repository_path, bare, &workdir, stash)?,
        );
    }
    Ok(Perform::Complete(notice))
}

fn append_progress(previous: super::rebase::Progress, batch: super::rebase::Progress) -> super::rebase::Progress {
    super::rebase::Progress {
        total: previous.processed + batch.total,
        processed: previous.processed + batch.processed,
        cherry_picked: previous.cherry_picked + batch.cherry_picked,
        signed: previous.signed + batch.signed,
        cherry_pick_time: previous.cherry_pick_time + batch.cherry_pick_time,
        signing_time: previous.signing_time + batch.signing_time,
    }
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

#[tracing::instrument(skip_all, fields(review = %review.reference))]
fn save_review_stash(
    repository_path: &Path,
    bare: bool,
    workdir: &Path,
    review: &ReviewTree,
) -> Result<Option<super::stash::SavedStash>> {
    if !super::review::is_dirty(workdir)? {
        return Ok(None);
    }
    let name = super::review::stash_reference(review.reference.as_bstr())?;
    super::stash::save(
        repository_path,
        bare,
        workdir,
        name,
        format!("tix review {}", review.reference.shorten()),
        "tix review auto-stash",
        "review state",
    )
    .map(Some)
}

fn find_review_stash(
    repository_path: &Path,
    bare: bool,
    review: &ReviewTree,
) -> Result<Option<super::stash::SavedStash>> {
    let name = super::review::stash_reference(review.reference.as_bstr())?;
    super::stash::find(repository_path, bare, name)
}

#[tracing::instrument(skip_all, fields(stash = %stash.name))]
fn apply_review_stash(
    repository_path: &Path,
    bare: bool,
    workdir: &Path,
    stash: super::stash::SavedStash,
) -> Result<String> {
    super::stash::apply(repository_path, bare, workdir, stash)
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
        .filter(|pin| !pin.is_head() && pin.id == selected)
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

fn create_or_update_head_pin(
    repository: &gix::Repository,
    branch: &gix::refs::FullName,
    id: ObjectId,
) -> Result<history::Pin> {
    let name: gix::refs::FullName = history::HEAD_PIN_NAME
        .as_bstr()
        .try_into()
        .context("the HEAD pin name is valid")?;
    let expected = repository
        .try_find_reference(name.as_ref())
        .context("could not read the existing HEAD pin")?
        .map_or(PreviousValue::MustNotExist, |reference| {
            PreviousValue::MustExistAndMatch(reference.target().into_owned())
        });
    let target = Target::Symbolic(branch.clone());
    repository
        .edit_references([RefEdit {
            change: Change::Update {
                log: LogChange {
                    mode: RefLog::AndReference,
                    force_create_reflog: false,
                    message: "tix remember HEAD branch".into(),
                },
                expected,
                new: target.clone(),
            },
            name: name.clone(),
            deref: false,
        }])
        .context("could not remember the branch HEAD was attached to")?;
    Ok(history::Pin { name, target, id })
}

fn reconcile_head_pin(repository: &gix::Repository, workdir: &Path) -> Result<Option<String>> {
    let Some(pin) = history::all_pins(repository)?.into_iter().find(history::Pin::is_head) else {
        return Ok(None);
    };
    let head = repository.head().context("could not read HEAD after time-travel")?;
    let detached = head.is_detached();
    let head_id = head.id().map(gix::Id::detach);
    drop(head);
    if !detached {
        return Ok(delete_pin(repository, &pin)
            .err()
            .map(|err| format!("HEAD pin remains: {err:#}")));
    }
    if head_id != Some(pin.id) {
        return Ok(None);
    }
    let branch = pin.target.try_name().context("the HEAD pin is not symbolic")?;
    if let Err(err) = checkout_branch(workdir, branch) {
        return Ok(Some(format!(
            "could not reattach HEAD to {}: {err:#}; HEAD pin remains",
            branch.shorten()
        )));
    }
    Ok(Some(match delete_pin(repository, &pin) {
        Ok(()) => format!("reattached HEAD to {}", branch.shorten()),
        Err(err) => format!("reattached HEAD to {}; HEAD pin remains: {err:#}", branch.shorten()),
    }))
}

pub(crate) fn create_or_reuse_pin(
    repository: &gix::Repository,
    target: Target,
    id: ObjectId,
    reflog_message: &str,
) -> Result<(history::Pin, bool)> {
    let pins = history::all_pins(repository)?;
    if let Some(pin) = pins.iter().find(|pin| !pin.is_head() && pin.target == target) {
        return Ok((pin.clone(), false));
    }
    Ok((create_pin(repository, target, id, reflog_message)?, true))
}

pub(crate) fn create_pin(
    repository: &gix::Repository,
    target: Target,
    id: ObjectId,
    reflog_message: &str,
) -> Result<history::Pin> {
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
                    message: reflog_message.into(),
                },
                expected: PreviousValue::MustNotExist,
                new: target.clone(),
            },
            name: name.clone(),
            deref: false,
        }])
        .context("could not create tix pin")?;
    Ok(history::Pin { name, target, id })
}

pub(super) fn delete_pin(repository: &gix::Repository, pin: &history::Pin) -> Result<()> {
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
        .filter(|pin| !pin.is_head() && pin.id == selected)
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

fn checkout_branch(workdir: &Path, name: &gix::refs::FullNameRef) -> Result<()> {
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

fn checkout_reference(
    repository_path: &Path,
    bare: bool,
    workdir: &Path,
    selected: ObjectId,
    name: &gix::refs::FullName,
) -> Result<()> {
    checkout_detached(workdir, selected)?;
    open_repository(repository_path, bare, false)
        .context("could not reopen repository to attach HEAD")?
        .edit_reference(RefEdit {
            name: "HEAD".try_into().expect("valid reference name"),
            deref: false,
            change: Change::Update {
                log: LogChange {
                    mode: RefLog::AndReference,
                    force_create_reflog: false,
                    message: "tix attach HEAD".into(),
                },
                expected: PreviousValue::MustExistAndMatch(Target::Object(selected)),
                new: Target::Symbolic(name.clone()),
            },
        })
        .context("could not attach HEAD to the selected reference")?;
    Ok(())
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

pub(crate) fn pin_label(pin: &history::Pin) -> String {
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

    #[test]
    fn progress_accumulates_when_travel_discovers_another_replay_batch() {
        let previous = super::super::rebase::Progress {
            total: 2,
            processed: 2,
            cherry_picked: 1,
            signed: 1,
            cherry_pick_time: std::time::Duration::from_millis(3),
            signing_time: std::time::Duration::from_millis(5),
        };
        let progress = append_progress(
            previous,
            super::super::rebase::Progress {
                total: 3,
                processed: 1,
                cherry_picked: 1,
                signed: 1,
                cherry_pick_time: std::time::Duration::from_millis(7),
                signing_time: std::time::Duration::from_millis(11),
            },
        );
        assert_eq!(progress.total, 5);
        assert_eq!(progress.processed, 3);
        assert_eq!(progress.cherry_picked, 2);
        assert_eq!(progress.signed, 2);
        assert_eq!(progress.cherry_pick_time, std::time::Duration::from_millis(10));
        assert_eq!(progress.signing_time, std::time::Duration::from_millis(16));
    }

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
        let repo = crate::test_repository::open(fixture.path())?;
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
        let repo = crate::test_repository::open(fixture.path())?;
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
        let repo = crate::test_repository::open(fixture.path())?;
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
            crate::test_repository::open(fixture.path())?
                .try_find_reference(stash_name.as_ref())?
                .is_none(),
            "no stash is created inside the review tree"
        );

        perform(&repository_path, false, tip, &graph, &[started.commit], &[], false)?.complete()?;
        let repo = crate::test_repository::open(fixture.path())?;
        assert_eq!(
            repo.head_name()?.map(|name| name.as_bstr().to_owned()),
            Some(b"refs/heads/main".into()),
            "leaving the review returns to the attached branch"
        );
        let snapshot = history::snapshot(&repo, &[], &[], false)?;
        assert_eq!(snapshot.pins.len(), 1, "only the review-tree departure remains pinned");
        assert_eq!(snapshot.pins[0].id, child);
        assert!(
            snapshot.view_tips.contains(&child),
            "a fresh attached-HEAD snapshot retains the review-tree leaf"
        );
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
        let repo = crate::test_repository::open(fixture.path())?;
        assert!(
            history::all_pins(&repo)?.iter().all(history::Pin::is_head),
            "returning consumes the ordinary review-tree pin"
        );
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
        let repo = crate::test_repository::open(fixture.path())?;
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
            crate::test_repository::open(fixture.path())?
                .try_find_reference(stash.name.as_ref())?
                .is_none(),
            "the review stash ref is consumed even when Git cannot apply it"
        );
        Ok(())
    }

    #[test]
    fn nested_review_trees_use_the_nearest_review_root() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
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
        let repository = crate::test_repository::open(fixture.path())?;
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
        assert!(notice.contains("time-travelled"), "{notice}");
        let repository = crate::test_repository::open(fixture.path())?;
        assert!(repository.head()?.is_detached(), "travel detaches HEAD");
        assert_eq!(repository.head_id()?.detach(), root);
        let pins = history::all_pins(&repository)?;
        assert_eq!(pins.len(), 1, "the lost branch tip gets one pin");
        assert_eq!(
            pins[0].name.as_bstr(),
            b"refs/worktree/tix/pins/HEAD".as_bstr(),
            "an attached departure uses the singleton HEAD pin"
        );
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

        let middle = repository.rev_parse_single("main~1")?.detach();
        drop(repository);
        perform(&repository_path, false, middle, &graph, &[], &[], false)?.complete()?;
        let repository = crate::test_repository::open(fixture.path())?;
        assert!(repository.head()?.is_detached(), "further travel remains detached");
        let pins = history::all_pins(&repository)?;
        assert_eq!(pins.len(), 1, "further travel keeps only the singleton HEAD pin");
        assert!(pins[0].is_head());

        repository
            .find_reference("refs/heads/main")?
            .set_target_id(topic, "advance pinned branch")?;
        let advanced = history::snapshot(&repository, &[], &[], false)?;
        assert!(advanced.view_tips.contains(&topic), "a symbolic pin follows its branch");
        drop(repository);

        perform(&repository_path, false, topic, &graph, &[], &[], false)?.complete()?;
        let repository = crate::test_repository::open(fixture.path())?;
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
        let graph = loaded_graph(&crate::test_repository::open(fixture.path())?, &[])?;
        perform(&repository_path, false, root, &graph, &[], &[], false)?.complete()?;
        let pin = history::all_pins(&crate::test_repository::open(fixture.path())?)?
            .pop()
            .context("direct pin is present")?;
        assert_eq!(pin.target.try_id().map(ToOwned::to_owned), Some(main));
        perform(&repository_path, false, main, &graph, &[], &[], false)?.complete()?;
        let repository = crate::test_repository::open(fixture.path())?;
        assert!(
            repository.head()?.is_detached(),
            "a direct pin returns to detached HEAD"
        );
        assert_eq!(repository.head_id()?.detach(), main);
        assert!(history::all_pins(&repository)?.is_empty());
        Ok(())
    }

    #[test]
    fn explicit_attachment_clears_the_head_pin() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
        let repository_path = repository.git_dir().to_owned();
        let root = repository.rev_parse_single("main~2")?.detach();
        let topic = repository.rev_parse_single("topic")?.detach();
        let topic_ref = repository.find_reference("refs/heads/topic")?.name().to_owned();
        let graph = loaded_graph(&repository, &[])?;
        drop(repository);

        perform(&repository_path, false, root, &graph, &[], &[], false)?.complete()?;
        move_head_to(&repository_path, false, topic, Some(&topic_ref), &[], false, Some)?;
        let repository = crate::test_repository::open(fixture.path())?;
        assert_eq!(
            repository.head_name()?.map(|name| name.as_bstr().to_owned()),
            Some(topic_ref.into())
        );
        assert!(
            history::all_pins(&repository)?.iter().all(|pin| !pin.is_head()),
            "an explicit attachment clears the remembered branch"
        );
        Ok(())
    }

    #[test]
    fn failed_reattachment_keeps_the_head_pin() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
        let repository_path = repository.git_dir().to_owned();
        let root = repository.rev_parse_single("main~2")?.detach();
        let main = repository.rev_parse_single("main")?.detach();
        let graph = loaded_graph(&repository, &[])?;
        drop(repository);

        perform(&repository_path, false, root, &graph, &[], &[], false)?.complete()?;
        let linked = fixture.path().join("main-wt");
        let worktree = Command::new("git")
            .arg("-C")
            .arg(fixture.path())
            .args(["worktree", "add", "-q"])
            .arg(&linked)
            .arg("main")
            .status()?;
        assert!(worktree.success(), "another worktree checks out the remembered branch");

        let notice = perform(&repository_path, false, main, &graph, &[], &[], false)?
            .complete()?
            .context("travel reports the failed reattachment")?;
        assert!(notice.contains("could not reattach HEAD to main"), "{notice}");
        let repository = crate::test_repository::open(fixture.path())?;
        assert!(
            repository.head()?.is_detached(),
            "the successful detached checkout is retained"
        );
        assert_eq!(repository.head_id()?.detach(), main);
        assert!(
            history::all_pins(&repository)?.iter().any(history::Pin::is_head),
            "the HEAD pin remains available for a later retry"
        );
        Ok(())
    }

    #[test]
    fn returning_to_a_commit_restores_its_manual_stash() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
        let repository_path = repository.git_dir().to_owned();
        let head = repository.head_id()?.detach();
        let parent = repository
            .find_commit(head)?
            .parent_ids()
            .next()
            .context("the history fixture has a parent")?
            .detach();
        let graph = loaded_graph(&repository, &[])?;
        drop(repository);

        std::fs::write(fixture.path().join("manual-stash"), "saved\n")?;
        super::super::stash::save_manual(&repository_path, false, head)?;
        perform(&repository_path, false, parent, &graph, &[], &[], false)?.complete()?;
        assert!(
            !fixture.path().join("manual-stash").exists(),
            "leaving the stashed commit keeps its worktree clean"
        );

        perform(&repository_path, false, head, &graph, &[], &[], false)?.complete()?;
        assert_eq!(std::fs::read(fixture.path().join("manual-stash"))?, b"saved\n");
        assert!(
            crate::test_repository::open(fixture.path())?
                .try_find_reference(super::super::stash::reference(head)?.as_ref())?
                .is_none(),
            "returning consumes the manual stash association"
        );
        Ok(())
    }

    #[test]
    fn removes_every_pin_at_the_selected_commit() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
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
        let main = repository.find_reference("refs/heads/main")?.name().to_owned();
        create_or_update_head_pin(&repository, &main, selected)?;
        let (manual, created) = create_or_reuse_pin(
            &repository,
            Target::Symbolic(main),
            selected,
            "test ordinary symbolic pin",
        )?;
        assert!(created && !manual.is_head(), "manual pins never reuse the HEAD pin");
        drop(repository);

        assert_eq!(remove_pins(&repository_path, false, selected)?, 3);
        let repository = crate::test_repository::open(fixture.path())?;
        let pins = history::all_pins(&repository)?;
        assert!(pins.iter().any(history::Pin::is_head), "unpin preserves the HEAD pin");
        assert!(pins.iter().any(|pin| pin.id == other), "pins on other commits remain");
        Ok(())
    }

    #[test]
    fn explicitly_created_pins_have_independent_lifetimes() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
        let selected = repository.rev_parse_single("main")?.detach();
        let target = Target::Object(selected);
        let first = create_pin(&repository, target.clone(), selected, "first review")?;
        let second = create_pin(&repository, target, selected, "second review")?;

        assert_ne!(first.name, second.name, "reviews never share ownership of a return pin");
        delete_pin(&repository, &first)?;
        assert!(
            repository.try_find_reference(second.name.as_ref())?.is_some(),
            "consuming one review's pin leaves the other review's return pin intact"
        );
        Ok(())
    }

    #[test]
    fn sideways_travel_preserves_an_unreferenced_departure() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
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
        let repository = crate::test_repository::open(fixture.path())?;
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
        let repository = crate::test_repository::open(fixture.path())?;
        let repository_path = repository.git_dir().to_owned();
        let root = repository.rev_parse_single("main~2")?.detach();
        let main = repository.rev_parse_single("main")?.detach();
        let revisions = [OsString::from("main")];
        let graph = loaded_graph(&repository, &revisions)?;
        drop(repository);

        perform(&repository_path, false, root, &graph, &[], &revisions, false)?.complete()?;
        let pins = history::all_pins(&crate::test_repository::open(fixture.path())?)?;
        assert_eq!(
            pins.len(),
            1,
            "the singleton is retained even when the branch is an explicit tip"
        );
        assert!(pins[0].is_head());

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
        let repository = crate::test_repository::open(fixture.path())?;
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
        let repository = crate::test_repository::open(fixture.path())?;
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
        let repository = crate::test_repository::open(fixture.path())?;
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
        let repository = crate::test_repository::open(fixture.path())?;
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
        let open = || crate::test_repository::open_with(fixture.path(), ["commit.gpgSign=true"]);
        let repository = open()?;
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

        let repository = open()?;
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

        let repository = open()?;
        let graph = super::super::loaded_graph(&repository)?;
        perform(&repository_path, false, root, &graph, &[], &[], false)?.complete()?;
        let repository = open()?;
        assert_eq!(repository.find_reference("refs/heads/main")?.id().detach(), pending_tip);
        assert!(super::super::rebase::is_pending(
            &repository.find_commit(pending_middle)?.decode()?.into_owned()?
        ));
        assert!(super::super::rebase::is_pending(
            &repository.find_commit(pending_tip)?.decode()?.into_owned()?
        ));
        let graph = super::super::loaded_graph(&repository)?;
        drop(repository);

        let mut middle_progress = Vec::new();
        perform_with_progress(
            &repository_path,
            false,
            pending_middle,
            &graph,
            &[],
            &[],
            false,
            |progress| middle_progress.push(progress),
        )?
        .complete()?;
        let progress = middle_progress.last().context("time-travel progress is reported")?;
        assert_eq!(
            progress.total, 2,
            "the selected commit and its pending descendant are processed"
        );
        assert_eq!(progress.processed, 2);
        assert_eq!(
            progress.cherry_picked, 1,
            "only the selected ancestry is replayed eagerly"
        );
        assert_eq!(progress.signed, 1, "the selected commit is signed");
        assert!(progress.cherry_pick_time > std::time::Duration::ZERO);
        assert!(progress.signing_time > std::time::Duration::ZERO);

        let repository = open()?;
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
        assert!(
            history::all_pins(&repository)?
                .iter()
                .all(|pin| pin.id != pending_middle),
            "travelling to a rewritten detached HEAD does not pin its predecessor"
        );
        let graph = super::super::loaded_graph(&repository)?;
        drop(repository);

        let mut tip_progress = Vec::new();
        perform_with_progress(
            &repository_path,
            false,
            still_pending_tip,
            &graph,
            &[],
            &[],
            false,
            |progress| tip_progress.push(progress),
        )?
        .complete()?;
        let progress = tip_progress.last().context("tip time-travel progress is reported")?;
        assert_eq!((progress.total, progress.processed), (1, 1));
        assert_eq!((progress.cherry_picked, progress.signed), (1, 1));
        let repository = open()?;
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
