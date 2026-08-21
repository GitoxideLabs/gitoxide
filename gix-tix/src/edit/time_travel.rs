use std::{
    collections::HashMap,
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
    Complete {
        notice: Option<String>,
        selected: ObjectId,
        ref_rewrites: Vec<super::rebase::RefRewrite>,
        ref_changes: Vec<super::undo::RefChange>,
    },
    Conflict(Conflict),
}

#[derive(Clone, Debug)]
struct RememberedBranch {
    branch: gix::refs::FullName,
    branch_tip: ObjectId,
}

impl Perform {
    #[cfg(test)]
    pub(crate) fn complete(self) -> Result<Option<String>> {
        match self {
            Perform::Complete { notice, .. } => Ok(notice),
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
    ref_rewrites: Vec<super::rebase::RefRewrite>,
    ref_changes: Vec<super::undo::RefChange>,
}

impl Conflict {
    pub(crate) fn from_rebase(
        rebase: super::rebase::Conflict,
        repository_path: &Path,
        bare: bool,
        revisions: &[OsString],
        include_worktrees: bool,
    ) -> Self {
        Conflict {
            rebase,
            repository_path: repository_path.to_owned(),
            bare,
            revisions: revisions.to_vec(),
            include_worktrees,
            ref_rewrites: Vec::new(),
            ref_changes: Vec::new(),
        }
    }

    pub(crate) fn original(&self) -> ObjectId {
        self.rebase.original()
    }

    pub(crate) fn prepend_ref_changes(&mut self, mut changes: Vec<super::undo::RefChange>) {
        changes.append(&mut self.ref_changes);
        self.ref_changes = changes;
    }

    pub(crate) fn into_ref_changes(self) -> Vec<super::undo::RefChange> {
        self.ref_changes
    }

    #[tracing::instrument(skip_all, fields(commit_id = %self.rebase.original()))]
    pub(crate) fn accept(
        self,
    ) -> Result<(
        String,
        ObjectId,
        Vec<super::rebase::RefRewrite>,
        Vec<super::undo::RefChange>,
    )> {
        let mut conflict = self.rebase.persist()?;
        let mut ref_rewrites = self.ref_rewrites;
        let mut ref_changes = self.ref_changes;
        ref_rewrites.append(&mut conflict.ref_rewrites);
        ref_changes.append(&mut conflict.ref_changes);
        let (notice, mut checkout_changes) = move_head_to_reporting(
            &self.repository_path,
            self.bare,
            conflict.commit,
            None,
            &self.revisions,
            self.include_worktrees,
            |id| conflict.map(id),
        )?;
        ref_changes.append(&mut checkout_changes);
        let mut deletion_changes =
            delete_deferred_refs(&self.repository_path, self.bare, &conflict.deferred_ref_deletions)?;
        ref_changes.append(&mut deletion_changes);
        conflict.materialize()?;
        Ok((
            format!(
                "{}; ready to resolve conflicts",
                notice.unwrap_or_else(|| format!("checked out {}", conflict.commit.to_hex_with_len(7)))
            ),
            conflict.commit,
            ref_rewrites,
            ref_changes,
        ))
    }
}

#[tracing::instrument(skip_all, fields(commit_id = %conflict.original()))]
pub(crate) fn materialize_plan_conflict_reporting(
    conflict: super::rebase::PlanConflict,
    repository_path: &Path,
    bare: bool,
    revisions: &[OsString],
    include_worktrees: bool,
) -> Result<(
    String,
    ObjectId,
    Vec<super::rebase::RefRewrite>,
    Vec<super::undo::RefChange>,
)> {
    let original = conflict.original();
    let mapped_head = conflict
        .repository()
        .head()?
        .id()
        .map(gix::Id::detach)
        .map(|id| (id, conflict.map(id)));
    let mut conflict = conflict.into_conflict().persist()?;
    let mut ref_changes = std::mem::take(&mut conflict.ref_changes);
    let (notice, mut checkout_changes) = move_head_to_reporting(
        repository_path,
        bare,
        conflict.commit,
        None,
        revisions,
        include_worktrees,
        |id| match mapped_head {
            Some((head, mapped)) if head == id => mapped,
            _ => conflict.map(id),
        },
    )?;
    ref_changes.append(&mut checkout_changes);
    let mut deletion_changes = delete_deferred_refs(repository_path, bare, &conflict.deferred_ref_deletions)?;
    ref_changes.append(&mut deletion_changes);
    conflict.materialize()?;
    tracing::warn!(commit_id = %original, rewritten_id = %conflict.commit, "materialized rebase-todo conflict");
    Ok((
        format!(
            "{}; ready to resolve conflicts",
            notice.unwrap_or_else(|| format!("checked out {}", conflict.commit.to_hex_with_len(7)))
        ),
        conflict.commit,
        conflict.ref_rewrites,
        ref_changes,
    ))
}

#[cfg(test)]
pub(crate) fn checkout_without_replay(
    repository_path: &Path,
    bare: bool,
    selected: ObjectId,
    revisions: &[OsString],
    include_worktrees: bool,
) -> Result<Option<String>> {
    Ok(move_head_to_reporting(
        repository_path,
        bare,
        selected,
        None,
        revisions,
        include_worktrees,
        Some,
    )?
    .0)
}

#[cfg(test)]
pub(crate) fn checkout_review_return(
    repository_path: &Path,
    bare: bool,
    name: &gix::refs::FullName,
    revisions: &[OsString],
    include_worktrees: bool,
) -> Result<(ObjectId, Option<String>)> {
    let (selected, notice, _) =
        checkout_review_return_reporting(repository_path, bare, name, revisions, include_worktrees)?;
    Ok((selected, notice))
}

pub(crate) fn checkout_review_return_reporting(
    repository_path: &Path,
    bare: bool,
    name: &gix::refs::FullName,
    revisions: &[OsString],
    include_worktrees: bool,
) -> Result<(ObjectId, Option<String>, Vec<super::undo::RefChange>)> {
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
    let (notice, ref_changes) = move_head_to_reporting(
        repository_path,
        bare,
        selected,
        reference.as_ref(),
        revisions,
        include_worktrees,
        |_| None,
    )?;
    Ok((selected, notice, ref_changes))
}

#[cfg(test)]
pub(crate) fn checkout_plan(
    repository_path: &Path,
    bare: bool,
    outcome: &super::rebase::Outcome,
    revisions: &[OsString],
    include_worktrees: bool,
) -> Result<Option<String>> {
    Ok(checkout_plan_reporting(repository_path, bare, outcome, revisions, include_worktrees)?.0)
}

pub(crate) fn checkout_plan_reporting(
    repository_path: &Path,
    bare: bool,
    outcome: &super::rebase::Outcome,
    revisions: &[OsString],
    include_worktrees: bool,
) -> Result<(Option<String>, Vec<super::undo::RefChange>)> {
    let selected = outcome.selected.context("the rebase plan does not select a checkout")?;
    let mut ref_changes = outcome.ref_changes.clone();
    let (notice, mut checkout_changes) = move_head_to_reporting(
        repository_path,
        bare,
        selected,
        outcome.checkout_reference.as_ref(),
        revisions,
        include_worktrees,
        |id| outcome.map(id),
    )?;
    ref_changes.append(&mut checkout_changes);
    let mut deletion_changes = delete_deferred_refs(repository_path, bare, &outcome.deferred_ref_deletions)?;
    ref_changes.append(&mut deletion_changes);
    Ok((notice, ref_changes))
}

#[cfg(test)]
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
    Ok(move_head_to_reporting(
        repository_path,
        bare,
        selected,
        reference,
        revisions,
        include_worktrees,
        map_departure,
    )?
    .0)
}

fn move_head_to_reporting<F>(
    repository_path: &Path,
    bare: bool,
    selected: ObjectId,
    reference: Option<&gix::refs::FullName>,
    revisions: &[OsString],
    include_worktrees: bool,
    map_departure: F,
) -> Result<(Option<String>, Vec<super::undo::RefChange>)>
where
    F: FnOnce(ObjectId) -> Option<ObjectId>,
{
    let repository = open_repository(repository_path, bare, false).context("could not open repository for checkout")?;
    let head_name: gix::refs::FullName = "HEAD".try_into().expect("valid reference name");
    let head_before = super::undo::state(&repository, head_name.as_ref())?;
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
        return Ok((None, Vec::new()));
    }
    let mut ref_changes = Vec::new();
    let pins = history::all_pins(&repository)?;
    let destination_pin = selected_pin(&pins, selected);
    let direct_head_pin = if reference.is_none() {
        pins.into_iter().find(|pin| {
            pin.is_head()
                && pin.id == selected
                && pin.target.try_name().is_some_and(|branch| {
                    branch.as_bstr().starts_with(b"refs/heads/")
                        && ensure_branch_is_available(&repository, branch).is_ok()
                })
        })
    } else {
        None
    };
    let pin_for_checkout = direct_head_pin.as_ref().or(destination_pin.as_ref());
    let checkout_detaches = reference.is_none() && pin_for_checkout.is_none_or(|pin| pin.target.try_name().is_none());
    let head_pin = match head_ref
        .as_ref()
        .filter(|name| checkout_detaches && name.as_bstr().starts_with(b"refs/heads/"))
    {
        Some(name) => {
            let (pin, mut changes) = create_or_update_head_pin_reporting(&repository, name, head_id)?;
            ref_changes.append(&mut changes);
            Some(pin)
        }
        None => None,
    };
    let provisional = match head_pin
        .is_none()
        .then_some(departure)
        .flatten()
        .filter(|departure| *departure != selected)
        .filter(|departure| !contains(&repository, *departure, selected))
    {
        Some(departure) => {
            let target = head_ref.clone().map_or(Target::Object(departure), Target::Symbolic);
            let (pin, created, mut changes) =
                create_or_reuse_pin_reporting(&repository, target, departure, "tix time-travel")?;
            ref_changes.append(&mut changes);
            Some((pin, created))
        }
        None => None,
    };
    let head_after_checkout = reference.map_or_else(
        || match pin_for_checkout.map(|pin| &pin.target) {
            Some(Target::Symbolic(name)) => super::undo::State::Symbolic(name.clone()),
            Some(Target::Object(id)) => super::undo::State::Object(*id),
            None => super::undo::State::Object(selected),
        },
        |name| super::undo::State::Symbolic(name.clone()),
    );
    drop(repository);
    let checkout = match (reference, pin_for_checkout) {
        (Some(reference), _) if reference.as_bstr().starts_with(b"refs/heads/") => {
            checkout_branch(&workdir, reference.as_ref())
        }
        (Some(reference), _) => checkout_reference(repository_path, bare, &workdir, selected, reference),
        (None, Some(pin)) => checkout_pin(&workdir, pin),
        (None, None) => checkout_detached(&workdir, selected),
    };
    if let Err(checkout) = checkout {
        let cleanup = open_repository(repository_path, bare, false)
            .context("could not reopen repository to restore provisional references")
            .and_then(|repository| super::undo::apply_reversed_changes(&repository, &ref_changes));
        if let Err(cleanup) = cleanup {
            return Err(checkout.context(format!("checkout failed and provisional refs remain: {cleanup:#}")));
        }
        return Err(checkout);
    }
    if head_before != head_after_checkout {
        ref_changes.push(super::undo::RefChange {
            name: head_name,
            before: head_before,
            after: head_after_checkout,
        });
    }
    let mut notice = match (reference, direct_head_pin.as_ref(), destination_pin.as_ref()) {
        (Some(reference), _, _) => format!("checked out {}", reference.shorten()),
        (None, Some(pin), _) => format!(
            "returned to {}",
            pin.target
                .try_name()
                .expect("a direct HEAD return is symbolic")
                .shorten()
        ),
        (None, None, Some(pin)) => format!("returned from {}", pin_label(pin)),
        (None, None, None) => format!("time-travelled to {}", selected.to_hex_with_len(7)),
    };
    let repository = match open_repository(repository_path, bare, false) {
        Ok(repository) => repository,
        Err(err) => {
            notice = format!("{notice}; post-checkout cleanup skipped: {err:#}");
            return Ok((Some(notice), ref_changes));
        }
    };
    if let Some(pin) = destination_pin {
        match delete_pin_reporting(&repository, &pin) {
            Ok(mut changes) => ref_changes.append(&mut changes),
            Err(err) => notice = format!("{notice}; destination pin remains: {err:#}"),
        }
    }
    match reconcile_head_pin_reporting(&repository, &workdir) {
        Ok((addition, mut changes)) => {
            ref_changes.append(&mut changes);
            if let Some(addition) = addition {
                notice = format!("{notice}; {addition}");
            }
        }
        Err(err) => notice = format!("{notice}; HEAD-pin reconciliation failed: {err:#}"),
    }
    if let Some((provisional, _)) = provisional {
        let snapshot = history::snapshot_ignoring_pin(
            &repository,
            revisions,
            &[],
            include_worktrees,
            Some(provisional.name.as_bstr()),
        );
        let snapshot = match snapshot {
            Ok(snapshot) => snapshot,
            Err(err) => {
                notice = format!("{notice}; kept {}: {err:#}", pin_label(&provisional));
                return Ok((Some(notice), ref_changes));
            }
        };
        if snapshot
            .view_tips
            .iter()
            .copied()
            .any(|tip| contains(&repository, provisional.id, tip))
        {
            match delete_pin_reporting(&repository, &provisional) {
                Ok(mut changes) => ref_changes.append(&mut changes),
                Err(err) => notice = format!("{notice}; redundant {} remains: {err:#}", pin_label(&provisional)),
            }
        } else {
            notice = format!("{notice}; saved {}", pin_label(&provisional));
        }
    }
    Ok((Some(notice), ref_changes))
}

fn remembered_branch(repository: &gix::Repository) -> Result<RememberedBranch> {
    let pin = history::all_pins(repository)?
        .into_iter()
        .find(history::Pin::is_head)
        .context("attaching requires a valid HEAD pin")?;
    let branch = pin
        .target
        .try_name()
        .context("the HEAD pin must point to a local branch")?
        .to_owned();
    if !branch.as_bstr().starts_with(b"refs/heads/") {
        anyhow::bail!("the HEAD pin must point to a local branch");
    }
    Ok(RememberedBranch {
        branch,
        branch_tip: pin.id,
    })
}

fn validate_attach(repository: &gix::Repository, head_id: ObjectId, remembered: &RememberedBranch) -> Result<()> {
    let head = repository.head().context("could not read HEAD before attaching")?;
    if !head.is_detached() || head.id().map(gix::Id::detach) != Some(head_id) {
        anyhow::bail!("HEAD changed while preparing to attach");
    }
    drop(head);
    let pin = history::all_pins(repository)?
        .into_iter()
        .find(history::Pin::is_head)
        .context("the HEAD pin disappeared while preparing to attach")?;
    if pin.target.try_name() != Some(remembered.branch.as_ref()) || pin.id != remembered.branch_tip {
        anyhow::bail!("the HEAD pin changed while preparing to attach");
    }
    let branch_id = repository
        .find_reference(remembered.branch.as_ref())
        .context("the remembered branch disappeared while preparing to attach")?
        .try_id()
        .context("the remembered branch must be a direct reference")?
        .detach();
    if branch_id != remembered.branch_tip {
        anyhow::bail!("the remembered branch changed while preparing to attach");
    }
    ensure_branch_is_available(repository, remembered.branch.as_ref())
}

#[cfg(test)]
pub(crate) fn attach(
    repository_path: &Path,
    bare: bool,
    revisions: &[OsString],
    include_worktrees: bool,
) -> Result<String> {
    Ok(attach_reporting(repository_path, bare, revisions, include_worktrees)?.0)
}

pub(crate) fn attach_reporting(
    repository_path: &Path,
    bare: bool,
    revisions: &[OsString],
    include_worktrees: bool,
) -> Result<(String, Vec<super::undo::RefChange>)> {
    let repository = open_repository(repository_path, bare, false)
        .context("could not open repository to attach the remembered branch")?;
    repository.workdir().context("attaching requires a worktree")?;
    let head = repository.head().context("could not read HEAD before attaching")?;
    if !head.is_detached() {
        anyhow::bail!("attaching requires detached HEAD");
    }
    let head_id = head
        .id()
        .map(gix::Id::detach)
        .context("attaching requires an existing HEAD commit")?;
    drop(head);
    let remembered = remembered_branch(&repository)?;
    validate_attach(&repository, head_id, &remembered)?;

    let pins = history::all_pins(&repository)?;
    let destination_pin = selected_pin(&pins, head_id);
    let mut ref_changes = Vec::new();
    let provisional = if remembered.branch_tip != head_id && !contains(&repository, remembered.branch_tip, head_id) {
        let (pin, created, mut changes) = create_or_reuse_pin_reporting(
            &repository,
            Target::Object(remembered.branch_tip),
            remembered.branch_tip,
            "tix attach departure",
        )?;
        ref_changes.append(&mut changes);
        Some((pin, created))
    } else {
        None
    };
    let mut edits = Vec::with_capacity(2);
    if remembered.branch_tip != head_id {
        edits.push(checked_ref_edit(
            remembered.branch.clone(),
            Target::Object(remembered.branch_tip),
            Target::Object(head_id),
            "tix attach",
        ));
    }
    edits.push(checked_ref_edit(
        "HEAD".try_into().expect("valid reference name"),
        Target::Object(head_id),
        Target::Symbolic(remembered.branch.clone()),
        "tix attach",
    ));
    let applied = match repository
        .edit_references(edits)
        .context("could not move and attach the remembered branch")
    {
        Ok(applied) => applied,
        Err(err) => {
            return Err(match provisional.as_ref() {
                Some(pin) => cleanup_new_pins(&repository, std::slice::from_ref(pin), err),
                None => err,
            });
        }
    };
    let mut attach_changes = super::undo::changes_from_edits(applied)?;
    ref_changes.append(&mut attach_changes);

    let mut notice = format!(
        "attached {} at {}",
        remembered.branch.shorten(),
        head_id.to_hex_with_len(7)
    );
    if let Some(pin) = destination_pin {
        match delete_pin_reporting(&repository, &pin) {
            Ok(mut changes) => ref_changes.append(&mut changes),
            Err(err) => notice = format!("{notice}; destination pin remains: {err:#}"),
        }
    }
    if let Some((pin, _)) = provisional {
        let snapshot =
            history::snapshot_ignoring_pin(&repository, revisions, &[], include_worktrees, Some(pin.name.as_bstr()));
        let snapshot = match snapshot {
            Ok(snapshot) => snapshot,
            Err(err) => {
                notice = format!("{notice}; kept {}: {err:#}", pin_label(&pin));
                return Ok((notice, ref_changes));
            }
        };
        if snapshot
            .view_tips
            .iter()
            .copied()
            .any(|tip| contains(&repository, pin.id, tip))
        {
            match delete_pin_reporting(&repository, &pin) {
                Ok(mut changes) => ref_changes.append(&mut changes),
                Err(err) => notice = format!("{notice}; redundant {} remains: {err:#}", pin_label(&pin)),
            }
        } else {
            notice = format!("{notice}; saved {}", pin_label(&pin));
        }
    }
    Ok((notice, ref_changes))
}

fn checked_ref_edit(name: gix::refs::FullName, old: Target, new: Target, message: &str) -> RefEdit {
    RefEdit {
        name,
        deref: false,
        change: Change::Update {
            log: LogChange {
                mode: RefLog::AndReference,
                force_create_reflog: false,
                message: message.into(),
            },
            expected: PreviousValue::MustExistAndMatch(old),
            new,
        },
    }
}

fn cleanup_new_pins(
    repository: &gix::Repository,
    pins: &[(history::Pin, bool)],
    mut cause: anyhow::Error,
) -> anyhow::Error {
    let edits: Vec<_> = pins
        .iter()
        .filter(|(_, created)| *created)
        .map(|(pin, _)| delete_pin_edit(pin))
        .collect();
    if !edits.is_empty()
        && let Err(err) = repository
            .edit_references(edits)
            .context("could not remove provisional attach pins")
    {
        cause = cause.context(format!("provisional pin cleanup failed: {err:#}"));
    }
    cause
}

fn ensure_branch_is_available(repository: &gix::Repository, branch: &gix::refs::FullNameRef) -> Result<()> {
    let current = repository.worktree().context("attaching requires a current worktree")?;
    let current_id = current.id().map(ToOwned::to_owned);
    if current_id.is_some() {
        ensure_worktree_does_not_own_branch(
            repository
                .main_repo()
                .context("could not open the main worktree while checking the remembered branch")?,
            branch,
        )?;
    }
    for proxy in repository
        .worktrees()
        .context("could not enumerate worktrees while checking the remembered branch")?
    {
        if current_id
            .as_ref()
            .is_some_and(|current| current.as_slice() == proxy.id().as_bytes())
        {
            continue;
        }
        ensure_worktree_does_not_own_branch(
            proxy
                .into_repo_with_possibly_inaccessible_worktree()
                .context("could not inspect a linked worktree while checking the remembered branch")?,
            branch,
        )?;
    }
    Ok(())
}

fn ensure_worktree_does_not_own_branch(worktree: gix::Repository, branch: &gix::refs::FullNameRef) -> Result<()> {
    let head = worktree
        .head()
        .context("could not inspect another worktree HEAD while checking the remembered branch")?;
    if head.referent_name() == Some(branch) {
        anyhow::bail!("{} is checked out in another worktree", branch.shorten());
    }
    Ok(())
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
    perform_reporting_rebased(
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
#[expect(
    clippy::too_many_arguments,
    reason = "time travel context plus rebased-commit reporting"
)]
pub(crate) fn perform_reporting_rebased(
    repository_path: &Path,
    bare: bool,
    mut selected: ObjectId,
    graph: &history::HistoryGraph,
    review_roots: &[ObjectId],
    revisions: &[OsString],
    include_worktrees: bool,
    mut report: impl FnMut(ObjectId),
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
    let source_review = review_tree(&repository, graph, review_roots, head_id)?;
    let destination_review = review_tree(&repository, graph, review_roots, selected)?;
    let crosses_review_boundary =
        source_review.as_ref().map(|review| review.root) != destination_review.as_ref().map(|review| review.root);
    let mut completed_graph = None;
    let mut original_ids = HashMap::new();
    let mut ref_rewrites = Vec::new();
    let mut ref_changes = Vec::new();
    let mut pending = pending_base(&repository, selected)?;
    while let Some(base) = pending {
        let graph = completed_graph.as_ref().unwrap_or(graph);
        let mut rebased = Vec::new();
        let outcome = super::rebase::perform_reporting_rebased(
            &repository,
            graph,
            super::rebase::Edit::Repeat {
                base,
                checkout: selected,
            },
            super::rebase::Signature::RedoIfNeeded,
            super::rebase::Tree::CherryPick,
            |id| {
                let original = original_ids.get(&id).copied().unwrap_or(id);
                rebased.push((id, original));
                report(original);
            },
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
                    ref_rewrites,
                    ref_changes,
                }));
            }
        };
        ref_rewrites.extend(outcome.ref_rewrites.iter().cloned());
        ref_changes.extend(outcome.ref_changes.iter().cloned());
        for &(old, original) in &rebased {
            if let Some(new) = outcome.map(old) {
                original_ids.insert(new, original);
            }
        }
        selected = outcome
            .map(selected)
            .context("the time-travel destination disappeared while completing its rebase")?;
        head_id = outcome
            .map(head_id)
            .context("HEAD disappeared while completing its rebase")?;
        repository = open_repository(repository_path, bare, false)
            .context("could not reopen repository after completing a pending rebase")?;
        pending = pending_base(&repository, selected)?;
        if pending.is_some() {
            let affected = rebased
                .into_iter()
                .map(|(id, _original)| {
                    outcome
                        .map(id)
                        .context("a pending rebase commit disappeared while completing time-travel")
                })
                .collect::<Result<Vec<_>>>()?;
            completed_graph = Some(history::HistoryGraph::for_commits(&repository, &affected)?);
        }
    }
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
    let moved = move_head_to_reporting(
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
    let (mut notice, mut checkout_changes) = match moved {
        Ok(outcome) => outcome,
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
    ref_changes.append(&mut checkout_changes);
    if let Some(saved) = saved
        && let Some(warning) = saved.warning
    {
        append_notice(&mut notice, warning);
    }
    if crosses_review_boundary && let Some(review) = destination_review {
        match find_review_stash(repository_path, bare, &review) {
            Ok(Some(stash)) => match apply_review_stash(repository_path, bare, &workdir, stash) {
                Ok(message) => append_notice(&mut notice, message),
                Err(err) => append_notice(&mut notice, format!("review stash remains: {err:#}")),
            },
            Ok(None) => {}
            Err(err) => append_notice(&mut notice, format!("could not inspect the review stash: {err:#}")),
        }
    }
    match super::stash::reference(selected).and_then(|name| super::stash::find(repository_path, bare, name)) {
        Ok(Some(stash)) => match super::stash::apply(repository_path, bare, &workdir, stash) {
            Ok(message) => append_notice(&mut notice, message),
            Err(err) => append_notice(&mut notice, format!("commit stash remains: {err:#}")),
        },
        Ok(None) => {}
        Err(err) => append_notice(&mut notice, format!("could not inspect the commit stash: {err:#}")),
    }
    Ok(Perform::Complete {
        notice,
        selected,
        ref_rewrites,
        ref_changes,
    })
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

fn selected_pin(pins: &[history::Pin], selected: ObjectId) -> Option<history::Pin> {
    pins.iter()
        .filter(|pin| !pin.is_head() && pin.id == selected)
        .min_by(|a, b| {
            a.target
                .try_name()
                .is_none()
                .cmp(&b.target.try_name().is_none())
                .then_with(|| a.name.cmp(&b.name))
        })
        .cloned()
}

#[cfg(test)]
fn create_or_update_head_pin(
    repository: &gix::Repository,
    branch: &gix::refs::FullName,
    id: ObjectId,
) -> Result<history::Pin> {
    Ok(create_or_update_head_pin_reporting(repository, branch, id)?.0)
}

fn create_or_update_head_pin_reporting(
    repository: &gix::Repository,
    branch: &gix::refs::FullName,
    id: ObjectId,
) -> Result<(history::Pin, Vec<super::undo::RefChange>)> {
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
    let edit = RefEdit {
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
    };
    let applied = repository
        .edit_references([edit])
        .context("could not remember the branch HEAD was attached to")?;
    let changes = super::undo::changes_from_edits(applied)?;
    Ok((history::Pin { name, target, id }, changes))
}

fn reconcile_head_pin_reporting(
    repository: &gix::Repository,
    workdir: &Path,
) -> Result<(Option<String>, Vec<super::undo::RefChange>)> {
    let Some(pin) = history::all_pins(repository)?.into_iter().find(history::Pin::is_head) else {
        return Ok((None, Vec::new()));
    };
    let head = repository.head().context("could not read HEAD after time-travel")?;
    let detached = head.is_detached();
    let head_id = head.id().map(gix::Id::detach);
    drop(head);
    if !detached {
        return Ok(match delete_pin_reporting(repository, &pin) {
            Ok(changes) => (None, changes),
            Err(err) => (Some(format!("HEAD pin remains: {err:#}")), Vec::new()),
        });
    }
    if head_id != Some(pin.id) {
        return Ok((None, Vec::new()));
    }
    let branch = pin.target.try_name().context("the HEAD pin is not symbolic")?;
    if let Err(err) = checkout_branch(workdir, branch) {
        return Ok((
            Some(format!(
                "could not reattach HEAD to {}: {err:#}; HEAD pin remains",
                branch.shorten()
            )),
            Vec::new(),
        ));
    }
    let mut head_change = vec![super::undo::RefChange {
        name: "HEAD".try_into().expect("valid reference name"),
        before: super::undo::State::Object(pin.id),
        after: super::undo::State::Symbolic(branch.to_owned()),
    }];
    Ok(match delete_pin_reporting(repository, &pin) {
        Ok(mut changes) => {
            head_change.append(&mut changes);
            (Some(format!("reattached HEAD to {}", branch.shorten())), head_change)
        }
        Err(err) => (
            Some(format!(
                "reattached HEAD to {}; HEAD pin remains: {err:#}",
                branch.shorten()
            )),
            head_change,
        ),
    })
}

pub(crate) fn create_or_reuse_pin(
    repository: &gix::Repository,
    target: Target,
    id: ObjectId,
    reflog_message: &str,
) -> Result<(history::Pin, bool)> {
    let (pin, created, _) = create_or_reuse_pin_reporting(repository, target, id, reflog_message)?;
    Ok((pin, created))
}

pub(crate) fn create_or_reuse_pin_reporting(
    repository: &gix::Repository,
    target: Target,
    id: ObjectId,
    reflog_message: &str,
) -> Result<(history::Pin, bool, Vec<super::undo::RefChange>)> {
    let pins = history::all_pins(repository)?;
    if let Some(pin) = pins.iter().find(|pin| !pin.is_head() && pin.target == target) {
        return Ok((pin.clone(), false, Vec::new()));
    }
    let (pin, changes) = create_pin_reporting(repository, target, id, reflog_message)?;
    Ok((pin, true, changes))
}

pub(crate) fn create_pin(
    repository: &gix::Repository,
    target: Target,
    id: ObjectId,
    reflog_message: &str,
) -> Result<history::Pin> {
    Ok(create_pin_reporting(repository, target, id, reflog_message)?.0)
}

pub(crate) fn create_pin_reporting(
    repository: &gix::Repository,
    target: Target,
    id: ObjectId,
    reflog_message: &str,
) -> Result<(history::Pin, Vec<super::undo::RefChange>)> {
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
    let edit = RefEdit {
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
    };
    let applied = repository.edit_references([edit]).context("could not create tix pin")?;
    let changes = super::undo::changes_from_edits(applied)?;
    Ok((history::Pin { name, target, id }, changes))
}

pub(super) fn delete_pin(repository: &gix::Repository, pin: &history::Pin) -> Result<()> {
    delete_pin_reporting(repository, pin).map(drop)
}

pub(crate) fn delete_pin_reporting(
    repository: &gix::Repository,
    pin: &history::Pin,
) -> Result<Vec<super::undo::RefChange>> {
    let edit = delete_pin_edit(pin);
    let applied = repository.edit_references([edit]).context("could not remove tix pin")?;
    let changes = super::undo::changes_from_edits(applied)?;
    Ok(changes)
}

#[cfg(test)]
pub(crate) fn remove_pins(repository_path: &Path, bare: bool, selected: ObjectId) -> Result<usize> {
    Ok(remove_pins_reporting(repository_path, bare, selected)?.0)
}

pub(crate) fn remove_pins_reporting(
    repository_path: &Path,
    bare: bool,
    selected: ObjectId,
) -> Result<(usize, Vec<super::undo::RefChange>)> {
    let repository =
        open_repository(repository_path, bare, false).context("could not open repository to remove pins")?;
    let pins: Vec<_> = history::all_pins(&repository)?
        .into_iter()
        .filter(|pin| !pin.is_head() && pin.id == selected)
        .collect();
    if pins.is_empty() {
        return Ok((0, Vec::new()));
    }
    let edits: Vec<_> = pins.iter().map(delete_pin_edit).collect();
    let applied = repository.edit_references(edits).context("could not remove tix pins")?;
    let changes = super::undo::changes_from_edits(applied)?;
    Ok((pins.len(), changes))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PinToggle {
    Created,
    Removed(usize),
}

pub(crate) fn toggle_pin_reporting(
    repository_path: &Path,
    bare: bool,
    selected: ObjectId,
) -> Result<(PinToggle, Vec<super::undo::RefChange>)> {
    let repository =
        open_repository(repository_path, bare, false).context("could not open repository to toggle a pin")?;
    let pins: Vec<_> = history::all_pins(&repository)?
        .into_iter()
        .filter(|pin| !pin.is_head() && pin.id == selected)
        .collect();
    if pins.is_empty() {
        let (_, _, changes) =
            create_or_reuse_pin_reporting(&repository, Target::Object(selected), selected, "tix TUI pin")?;
        return Ok((PinToggle::Created, changes));
    }
    drop(repository);
    let (removed, changes) = remove_pins_reporting(repository_path, bare, selected)?;
    Ok((PinToggle::Removed(removed), changes))
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

fn delete_deferred_refs(
    repository_path: &Path,
    bare: bool,
    refs: &[(gix::refs::FullName, ObjectId)],
) -> Result<Vec<super::undo::RefChange>> {
    if refs.is_empty() {
        return Ok(Vec::new());
    }
    let repository = open_repository(repository_path, bare, false)
        .context("could not reopen repository to finish reference deletions")?;
    let edits: Vec<_> = refs
        .iter()
        .map(|(name, old)| RefEdit {
            name: name.clone(),
            deref: false,
            change: Change::Delete {
                expected: PreviousValue::MustExistAndMatch(Target::Object(*old)),
                log: RefLog::AndReference,
            },
        })
        .collect();
    let applied = repository
        .edit_references(edits)
        .context("could not delete the branch HEAD left during rebase")?;
    let changes = super::undo::changes_from_edits(applied)?;
    Ok(changes)
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

    fn pending_conflict_fixture() -> gix_testtools::Result<(
        gix_testtools::tempfile::TempDir,
        PathBuf,
        ObjectId,
        ObjectId,
        history::HistoryGraph,
    )> {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_conflict.sh")?;
        git(
            fixture.path(),
            &["config", "gitoxide.commit.committerDate", "2001-01-01T00:00:00 +0000"],
        )?;
        let repository = crate::test_repository::open(fixture.path())?;
        let repository_path = repository.git_dir().to_owned();
        let middle = repository.rev_parse_single("HEAD~1")?.detach();
        std::fs::write(fixture.path().join("after"), "after\n")?;
        git(fixture.path(), &["add", "after"])?;
        let commit = Command::new("git")
            .arg("-C")
            .arg(fixture.path())
            .args(["commit", "-q", "-m", "after"])
            .env("GIT_AUTHOR_DATE", "2000-01-04T00:00:00 +0000")
            .env("GIT_COMMITTER_DATE", "2000-01-04T00:00:00 +0000")
            .status()?;
        assert!(commit.success(), "the fixture descendant commit is created");
        let root = repository.rev_parse_single("HEAD~3")?.detach();
        git(fixture.path(), &["checkout", "-q", "--detach", &root.to_string()])?;
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
        let tip = repository.find_reference("refs/heads/main")?.id().detach();
        drop(repository);
        git(fixture.path(), &["checkout", "-q", "main"])?;
        Ok((fixture, repository_path, root, tip, graph))
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
        create_pin(&repository, Target::Object(topic), topic, "overlap with the HEAD pin")?;
        assert_eq!(history::all_pins(&repository)?.len(), 2, "both return paths coexist");
        drop(repository);

        let reflog_entries = git(fixture.path(), &["reflog", "show", "--format=%H", "HEAD"])?
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .count();
        let returned = perform(&repository_path, false, topic, &graph, &[], &[], false)?;
        let Perform::Complete { ref_changes, .. } = returned else {
            return Err("returning through the HEAD pin must complete".into());
        };
        let returned_reflog_entries = git(fixture.path(), &["reflog", "show", "--format=%H", "HEAD"])?
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .count();
        assert_eq!(
            returned_reflog_entries,
            reflog_entries + 1,
            "returning through the HEAD pin performs one checkout"
        );
        let repository = crate::test_repository::open(fixture.path())?;
        assert_eq!(
            repository.head_name()?.map(|name| name.as_bstr().to_owned()),
            Some(b"refs/heads/main".into()),
            "returning through a symbolic pin reattaches HEAD"
        );
        assert!(history::all_pins(&repository)?.is_empty(), "the used pin is removed");
        super::super::undo::record(&repository, "return to branch", &ref_changes)?;
        super::super::undo::plan_undo(&repository)?
            .expect("the return can be undone")
            .apply_with_worktrees(&repository)?;
        assert!(repository.head()?.is_detached(), "undo restores the detached HEAD");
        assert_eq!(repository.head_id()?.detach(), middle);
        let pins = history::all_pins(&repository)?;
        assert_eq!(pins.len(), 2, "undo restores both return paths");
        assert!(
            pins.iter().any(history::Pin::is_head),
            "undo restores the symbolic HEAD pin"
        );
        super::super::undo::plan_redo(&repository)?
            .expect("the return can be redone")
            .apply_with_worktrees(&repository)?;
        assert_eq!(
            repository.head_name()?.map(|name| name.as_bstr().to_owned()),
            Some(b"refs/heads/main".into()),
            "redo reattaches HEAD"
        );
        assert!(history::all_pins(&repository)?.is_empty(), "redo consumes the HEAD pin");

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
    fn attach_moves_and_attaches_the_remembered_branch_without_touching_files() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
        let repository_path = repository.git_dir().to_owned();
        let middle = repository.rev_parse_single("main~1")?.detach();
        let main = repository.rev_parse_single("main")?.detach();
        let graph = loaded_graph(&repository, &[])?;
        drop(repository);

        perform(&repository_path, false, middle, &graph, &[], &[], false)?.complete()?;
        std::fs::write(fixture.path().join("root"), "dirty root\n")?;
        let before = gix_testtools::repository::snapshot(fixture.path())?;

        let notice = attach(&repository_path, false, &[], false)?;
        assert!(notice.contains("attached main at"), "{notice}");
        let repository = crate::test_repository::open(fixture.path())?;
        assert_eq!(
            repository.head_name()?.map(|name| name.as_bstr().to_owned()),
            Some(b"refs/heads/main".into()),
            "HEAD attaches to the remembered branch"
        );
        assert_eq!(repository.head_id()?.detach(), middle);
        assert_eq!(repository.find_reference("refs/heads/main")?.id().detach(), middle);
        let pins = history::all_pins(&repository)?;
        assert!(
            pins.iter().any(|pin| pin.is_head() && pin.id == middle),
            "the symbolic HEAD pin remains and follows the moved branch"
        );
        assert!(
            pins.iter().any(|pin| !pin.is_head() && pin.id == main),
            "the departed branch tip remains visible through an ordinary pin"
        );
        let after = gix_testtools::repository::snapshot(fixture.path())?;
        assert_eq!(after.index_tree, before.index_tree, "attach does not update the index");
        assert_eq!(after.worktree, before.worktree, "attach does not update worktree files");
        Ok(())
    }

    #[test]
    fn attach_does_not_pin_a_tip_retained_by_another_branch() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        git(fixture.path(), &["branch", "keep", "main"])?;
        let repository = crate::test_repository::open(fixture.path())?;
        let repository_path = repository.git_dir().to_owned();
        let middle = repository.rev_parse_single("main~1")?.detach();
        let main = repository.rev_parse_single("main")?.detach();
        let graph = loaded_graph(&repository, &[])?;
        drop(repository);

        perform(&repository_path, false, middle, &graph, &[], &[], false)?.complete()?;
        attach(&repository_path, false, &["keep".into()], false)?;
        let repository = crate::test_repository::open(fixture.path())?;
        assert!(
            history::all_pins(&repository)?
                .iter()
                .all(|pin| pin.is_head() || pin.id != main),
            "the keep branch makes an ordinary departure pin redundant"
        );
        Ok(())
    }

    #[test]
    fn attach_rejects_a_branch_owned_by_another_worktree() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
        let repository_path = repository.git_dir().to_owned();
        let middle = repository.rev_parse_single("main~1")?.detach();
        let main = repository.rev_parse_single("main")?.detach();
        let graph = loaded_graph(&repository, &[])?;
        drop(repository);

        perform(&repository_path, false, middle, &graph, &[], &[], false)?.complete()?;
        let linked = fixture.path().join("main-wt");
        let worktree = Command::new("git")
            .arg("-C")
            .arg(fixture.path())
            .args(["worktree", "add", "-q"])
            .arg(&linked)
            .arg("main")
            .status()?;
        assert!(worktree.success(), "the remembered branch moves to another worktree");

        let err =
            attach(&repository_path, false, &[], false).expect_err("a branch cannot become attached in two worktrees");
        assert!(
            format!("{err:#}").contains("checked out in another worktree"),
            "{err:#}"
        );
        let repository = crate::test_repository::open(fixture.path())?;
        assert!(repository.head()?.is_detached(), "failed attach retains detached HEAD");
        assert_eq!(
            repository.head_id()?.detach(),
            middle,
            "failed attach retains its departure"
        );
        assert_eq!(
            repository.find_reference("refs/heads/main")?.id().detach(),
            main,
            "the other worktree's branch is not moved"
        );
        let pins = history::all_pins(&repository)?;
        assert_eq!(pins.len(), 1, "failure creates no provisional pins");
        assert!(pins[0].is_head(), "the remembered branch remains available");
        Ok(())
    }

    #[test]
    fn attach_accepts_the_branch_of_the_current_linked_worktree() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let linked = fixture.path().join("topic-wt");
        let worktree = Command::new("git")
            .arg("-C")
            .arg(fixture.path())
            .args(["worktree", "add", "-q"])
            .arg(&linked)
            .arg("topic")
            .status()?;
        assert!(worktree.success(), "the linked worktree checks out topic");
        let git_dir = crate::test_repository::open(&linked)?.git_dir().to_owned();
        let repository = open_repository(&git_dir, false, false)?;
        let branch = repository.find_reference("refs/heads/topic")?.name().to_owned();
        let root = repository.rev_parse_single("topic~1")?.detach();
        let graph = loaded_graph(&repository, &[])?;
        drop(repository);

        perform(&git_dir, false, root, &graph, &[], &[], false)?.complete()?;
        let repository = open_repository(&git_dir, false, false)?;
        assert!(
            repository.head()?.is_detached(),
            "ordinary travel detaches the linked worktree"
        );
        assert!(
            history::all_pins(&repository)?.iter().any(history::Pin::is_head),
            "ordinary travel remembers the linked worktree branch"
        );
        drop(repository);

        attach(&git_dir, false, &[], false)?;
        let repository = open_repository(&git_dir, false, false)?;
        assert_eq!(
            repository.head_name()?.map(|name| name.as_bstr().to_owned()),
            Some(branch.as_bstr().to_owned()),
            "attach reattaches the current linked worktree branch"
        );
        assert_eq!(
            repository.find_reference(branch.as_ref())?.id().detach(),
            root,
            "attach moves the linked worktree branch"
        );
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
    fn toggles_one_direct_pin_without_touching_the_head_pin() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
        let repository_path = repository.git_dir().to_owned();
        let selected = repository.rev_parse_single("main")?.detach();
        let main = repository.find_reference("refs/heads/main")?.name().to_owned();
        create_or_update_head_pin(&repository, &main, selected)?;
        drop(repository);

        let (toggle, created_changes) = toggle_pin_reporting(&repository_path, false, selected)?;
        assert_eq!(toggle, PinToggle::Created);
        let [created] = created_changes.as_slice() else {
            return Err("creating one pin must report one reference change".into());
        };
        assert_eq!(created.before, super::super::undo::State::Missing);
        assert_eq!(created.after, super::super::undo::State::Object(selected));
        let pin_name = created.name.clone();
        let repository = crate::test_repository::open(fixture.path())?;
        let pins = history::all_pins(&repository)?;
        assert_eq!(
            pins.iter().filter(|pin| !pin.is_head() && pin.id == selected).count(),
            1,
            "pin creates one direct pin for the selected commit"
        );
        drop(repository);

        let (toggle, removed_changes) = toggle_pin_reporting(&repository_path, false, selected)?;
        assert_eq!(toggle, PinToggle::Removed(1));
        let [removed] = removed_changes.as_slice() else {
            return Err("removing one pin must report one reference change".into());
        };
        assert_eq!(removed.name, pin_name);
        assert_eq!(removed.before, super::super::undo::State::Object(selected));
        assert_eq!(removed.after, super::super::undo::State::Missing);
        let pins = history::all_pins(&crate::test_repository::open(fixture.path())?)?;
        assert!(pins.iter().any(history::Pin::is_head), "unpin preserves the HEAD pin");
        assert!(
            pins.iter().all(|pin| pin.is_head() || pin.id != selected),
            "unpin removes every ordinary pin at the selected commit"
        );
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
        let topic = repository.rev_parse_single("topic")?.detach();
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
        git(
            fixture.path(),
            &["symbolic-ref", "refs/worktree/tix/pins/HEAD", "refs/heads/topic"],
        )?;
        let repository = crate::test_repository::open(fixture.path())?;
        let head_pin_before = repository
            .find_reference(history::HEAD_PIN_NAME.as_bstr())?
            .target()
            .into_owned();
        drop(repository);
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
        assert_eq!(
            repository
                .find_reference(history::HEAD_PIN_NAME.as_bstr())?
                .target()
                .into_owned(),
            head_pin_before,
            "failed checkout restores the existing HEAD pin exactly"
        );
        let pins = history::all_pins(&repository)?;
        assert_eq!(pins.len(), 2, "failed checkout neither loses nor creates pins");
        assert!(
            pins.iter().any(|pin| pin.is_head() && pin.id == topic),
            "the reversed provisional update follows the original remembered branch"
        );
        assert!(
            pins.iter().any(|pin| !pin.is_head() && pin.id == root),
            "the destination pin survives a failed checkout"
        );
        Ok(())
    }

    #[test]
    fn conflicting_pending_rebases_are_unobservable_until_accepted() -> gix_testtools::Result {
        let (fixture, repository_path, root, tip, graph) = pending_conflict_fixture()?;
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

        let (_notice, conflict_id, _, _) = conflict.accept()?;
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
    fn pending_time_travel_does_not_load_unrelated_ref_history() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
        let repository_path = repository.git_dir().to_owned();
        let graph = super::super::loaded_graph(&repository)?;
        let middle = repository.rev_parse_single("HEAD~1")?.detach();
        let root = repository.rev_parse_single("HEAD~2")?.detach();
        let old_tip = repository.head_id()?.detach();
        let mut commit = repository.find_commit(middle)?.decode()?.into_owned()?;
        commit.tree = repository.find_commit(root)?.tree_id()?.detach();
        git(fixture.path(), &["checkout", "-q", "--detach", &root.to_string()])?;
        let marked = super::super::rebase::perform(
            &repository,
            &graph,
            super::super::rebase::Edit::Replace { target: middle, commit },
            super::super::rebase::Signature::InvalidateExisting,
            super::super::rebase::Tree::LeaveAsIsAndMark,
        )?
        .complete()?;
        let pending_tip = marked.map(old_tip).context("the marked tip is retained")?;
        assert!(super::super::rebase::is_pending(
            &repository.find_commit(pending_tip)?.decode()?.into_owned()?
        ));

        let missing_parent = ObjectId::Sha1([0x42; 20]);
        let mut unrelated = repository.find_commit(pending_tip)?.decode()?.into_owned()?;
        unrelated.parents = [missing_parent].into_iter().collect();
        unrelated.message = "unrelated incomplete history".into();
        let unrelated = repository.write_object(&unrelated)?.detach();
        drop(repository);
        git(
            fixture.path(),
            &["update-ref", "refs/heads/unrelated", &unrelated.to_string()],
        )?;

        let repository = crate::test_repository::open(fixture.path())?;
        let graph = loaded_graph(&repository, &[OsString::from("main")])?;
        drop(repository);
        let mut rebased = Vec::new();
        let outcome = perform_reporting_rebased(&repository_path, false, pending_tip, &graph, &[], &[], false, |id| {
            rebased.push(id);
        })?;
        let Perform::Complete { selected, .. } = outcome else {
            return Err("the pending rebase must complete".into());
        };
        assert_eq!(rebased, [pending_tip], "only the selected pending path is replayed");

        let repository = crate::test_repository::open(fixture.path())?;
        assert!(!super::super::rebase::is_pending(
            &repository.find_commit(selected)?.decode()?.into_owned()?
        ));
        assert_eq!(
            repository.find_reference("refs/heads/unrelated")?.id().detach(),
            unrelated,
            "time-travel leaves the unrelated incomplete history untouched"
        );
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
        let spilled_middle =
            super::super::head::perform(repository.clone(), &graph, super::super::head::Kind::Spill, None)?
                .expect("spilling changes the middle commit");
        let pending_tip = repository.find_reference("refs/heads/main")?.id().detach();
        let middle_commit = repository.find_commit(spilled_middle)?.decode()?.into_owned()?;
        let tip_commit = repository.find_commit(pending_tip)?.decode()?.into_owned()?;
        assert!(
            !super::super::rebase::is_pending(&middle_commit),
            "the fully spilled commit has its final tree and parent"
        );
        assert!(
            !super::super::rebase::has_marker(&middle_commit),
            "the authoritative spilled tree needs no original-parent marker"
        );
        assert!(
            repository
                .find_commit(spilled_middle)?
                .verify_signature()?
                .expect("the fully spilled commit is signed immediately")
                .is_valid()
        );
        assert!(super::super::rebase::has_marker(&tip_commit));
        drop(repository);

        let repository = open()?;
        let graph = super::super::loaded_graph(&repository)?;
        perform(&repository_path, false, root, &graph, &[], &[], false)?.complete()?;
        let repository = open()?;
        assert_eq!(repository.find_reference("refs/heads/main")?.id().detach(), pending_tip);
        assert!(!super::super::rebase::is_pending(
            &repository.find_commit(spilled_middle)?.decode()?.into_owned()?
        ));
        assert!(super::super::rebase::is_pending(
            &repository.find_commit(pending_tip)?.decode()?.into_owned()?
        ));
        let graph = super::super::loaded_graph(&repository)?;
        drop(repository);

        let mut rebased = Vec::new();
        perform_reporting_rebased(&repository_path, false, spilled_middle, &graph, &[], &[], false, |id| {
            rebased.push(id);
        })?
        .complete()?;
        assert!(
            rebased.is_empty(),
            "travelling to a final commit does not replay its pending descendant"
        );

        let repository = open()?;
        let materialized_middle = repository.head_id()?.detach();
        let still_pending_tip = repository.find_reference("refs/heads/main")?.id().detach();
        assert_eq!(materialized_middle, spilled_middle);
        assert_eq!(still_pending_tip, pending_tip);
        let middle_commit = repository.find_commit(materialized_middle)?;
        assert!(!super::super::rebase::is_pending(
            &middle_commit.decode()?.into_owned()?
        ));
        assert!(
            middle_commit
                .verify_signature()?
                .expect("the fully spilled commit retained its configured signature")
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
                .all(|pin| pin.id != spilled_middle),
            "travelling to a rewritten detached HEAD does not pin its predecessor"
        );
        let graph = super::super::loaded_graph(&repository)?;
        drop(repository);

        let mut rebased = Vec::new();
        perform_reporting_rebased(
            &repository_path,
            false,
            still_pending_tip,
            &graph,
            &[],
            &[],
            false,
            |id| rebased.push(id),
        )?
        .complete()?;
        assert_eq!(rebased, [still_pending_tip]);
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
