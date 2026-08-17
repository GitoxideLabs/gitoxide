use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use gix::{
    ObjectId,
    bstr::{BStr, BString, ByteSlice},
    objs::Write,
    prelude::ObjectIdExt,
    refs::{
        Category, Target,
        transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog},
    },
};

use crate::history::HistoryGraph;

const MARKER: &[u8] = b"tix-rebase";
const ORIGINAL_PARENT: &[u8] = b"tix-rebase-parent";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Signature {
    InvalidateExisting,
    RedoIfNeeded,
    Remove,
}

enum CommitState {
    Unmarked(Signature),
    Pending { original_parent: Option<ObjectId> },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Tree {
    #[cfg_attr(not(test), allow(dead_code))]
    LeaveAsIs,
    LeaveAsIsAndMark,
    LeaveAsIsAndMarkDescendants,
    CherryPick,
}

pub(crate) enum Edit {
    Replace {
        target: ObjectId,
        commit: gix::objs::Commit,
    },
    Insert {
        anchor: Option<ObjectId>,
        commit: gix::objs::Commit,
        reset_index: bool,
    },
    Fork {
        anchor: ObjectId,
        commit: gix::objs::Commit,
    },
    Remove {
        target: ObjectId,
    },
    Split {
        target: ObjectId,
        source: gix::objs::Commit,
        upper: gix::objs::Commit,
    },
    Repeat {
        base: ObjectId,
        checkout: ObjectId,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PlanParent {
    Existing(ObjectId),
    Step(usize),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PlanCommit {
    Pick(ObjectId),
    Resolved(ObjectId),
    Empty(BString),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlanStep {
    pub parent: PlanParent,
    pub commit: PlanCommit,
    pub squash: Vec<ObjectId>,
}

#[derive(Clone, Debug)]
pub(crate) struct ExpectedRef {
    pub name: gix::refs::FullName,
    pub old: Option<ObjectId>,
    pub target: ObjectId,
    pub new: Option<ObjectId>,
    pub follows_tip: bool,
    pub editable: bool,
    pub placement: Option<PlanParent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlanCheckout {
    pub target: PlanParent,
    pub reference: Option<gix::refs::FullName>,
}

#[derive(Clone, Debug)]
pub(crate) struct Plan {
    pub base: ObjectId,
    pub scope: Vec<ObjectId>,
    pub steps: Vec<PlanStep>,
    pub checkout: Option<PlanCheckout>,
    pub expected_refs: Vec<ExpectedRef>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Progress {
    pub total: usize,
    pub processed: usize,
    pub cherry_picked: usize,
    pub signed: usize,
    pub cherry_pick_time: Duration,
    pub signing_time: Duration,
}

impl Progress {
    fn for_plan(plan: &Plan) -> Self {
        Progress {
            total: plan.steps.len() + plan.steps.iter().map(|step| step.squash.len()).sum::<usize>(),
            ..Progress::default()
        }
    }
}

pub(crate) enum PlanPerform {
    Complete(Outcome),
    Conflict(PlanConflict),
}

impl PlanPerform {
    #[cfg(test)]
    pub(crate) fn complete(self) -> Result<Outcome> {
        match self {
            PlanPerform::Complete(outcome) => Ok(outcome),
            PlanPerform::Conflict(conflict) => anyhow::bail!("rebase plan conflicts at {}", conflict.original()),
        }
    }
}

pub(crate) struct PlanConflict {
    conflict: Conflict,
    plan: Plan,
    produced: Vec<ObjectId>,
    rewritten: HashMap<ObjectId, Option<ObjectId>>,
    conflict_step: usize,
    continuation_start: usize,
    remaining_squash: Vec<Vec<ObjectId>>,
    final_refs: HashSet<gix::refs::FullName>,
}

impl PlanConflict {
    pub(crate) fn original(&self) -> ObjectId {
        self.conflict.original()
    }

    pub(crate) fn commit(&self) -> ObjectId {
        self.conflict.commit
    }

    pub(crate) fn produced(&self) -> &[ObjectId] {
        &self.produced
    }

    pub(crate) fn repository(&self) -> &gix::Repository {
        &self.conflict.prepared.repo
    }

    pub(crate) fn map(&self, id: ObjectId) -> Option<ObjectId> {
        self.rewritten.get(&id).copied().unwrap_or(Some(id))
    }

    pub(crate) fn continuation_plan(&self) -> Plan {
        let expected_refs = self
            .plan
            .expected_refs
            .iter()
            .filter(|expected| !self.final_refs.contains(&expected.name))
            .map(|expected| ExpectedRef {
                name: expected.name.clone(),
                old: expected.old,
                target: expected.new.expect("continued refs have a target"),
                new: expected.new,
                follows_tip: expected.follows_tip,
                editable: expected.editable,
                placement: expected.placement.map(|target| match target {
                    PlanParent::Existing(id) => PlanParent::Existing(id),
                    PlanParent::Step(index) if index < self.continuation_start => {
                        PlanParent::Existing(self.produced[index])
                    }
                    PlanParent::Step(index) => PlanParent::Step(index - self.continuation_start),
                }),
            })
            .collect();
        let mut scope = self.produced[self.continuation_start..].to_vec();
        scope.extend(self.remaining_squash.iter().flatten().copied());
        let base = match self.plan.steps[self.continuation_start].parent {
            PlanParent::Existing(id) => id,
            PlanParent::Step(parent) => self.produced[parent],
        };
        Plan {
            base,
            scope,
            steps: self
                .plan
                .steps
                .iter()
                .enumerate()
                .skip(self.continuation_start)
                .map(|(index, step)| PlanStep {
                    parent: match step.parent {
                        PlanParent::Existing(id) => PlanParent::Existing(id),
                        PlanParent::Step(parent) if parent < self.continuation_start => {
                            PlanParent::Existing(self.produced[parent])
                        }
                        PlanParent::Step(parent) => PlanParent::Step(parent - self.continuation_start),
                    },
                    commit: if index == self.conflict_step {
                        PlanCommit::Resolved(self.produced[index])
                    } else {
                        PlanCommit::Pick(self.produced[index])
                    },
                    squash: self.remaining_squash[index].clone(),
                })
                .collect(),
            checkout: self.plan.checkout.as_ref().map(|checkout| PlanCheckout {
                target: match checkout.target {
                    PlanParent::Existing(id) => PlanParent::Existing(id),
                    PlanParent::Step(index) if index < self.continuation_start => {
                        PlanParent::Existing(self.produced[index])
                    }
                    PlanParent::Step(index) => PlanParent::Step(index - self.continuation_start),
                },
                reference: checkout.reference.clone(),
            }),
            expected_refs,
        }
    }

    pub(crate) fn into_conflict(self) -> Conflict {
        self.conflict
    }
}

pub(crate) struct Outcome {
    pub selected: Option<ObjectId>,
    pub checkout_reference: Option<gix::refs::FullName>,
    pub deferred_ref_deletions: Vec<(gix::refs::FullName, ObjectId)>,
    rewritten: HashMap<ObjectId, Option<ObjectId>>,
}

impl Outcome {
    pub(crate) fn map(&self, id: ObjectId) -> Option<ObjectId> {
        self.rewritten.get(&id).copied().unwrap_or(Some(id))
    }
}

pub(crate) enum Perform {
    Complete(Outcome),
    Conflict(Conflict),
}

impl Perform {
    pub(crate) fn complete(self) -> Result<Outcome> {
        match self {
            Perform::Complete(outcome) => Ok(outcome),
            Perform::Conflict(_) => anyhow::bail!("an edit unexpectedly produced a merge conflict"),
        }
    }
}

pub(crate) struct Conflict {
    prepared: Prepared,
    conflicts: Vec<gix::merge::tree::Conflict>,
    merged_tree: ObjectId,
    commit: ObjectId,
    original: ObjectId,
}

impl Conflict {
    pub(crate) fn original(&self) -> ObjectId {
        self.original
    }

    pub(crate) fn persist(mut self) -> Result<PersistedConflict> {
        let outcome = self.prepared.finish()?;
        Ok(PersistedConflict {
            repo: self.prepared.repo,
            conflicts: self.conflicts,
            merged_tree: self.merged_tree,
            commit: self.commit,
            deferred_ref_deletions: outcome.deferred_ref_deletions,
            rewritten: outcome.rewritten,
        })
    }
}

pub(crate) struct PersistedConflict {
    repo: gix::Repository,
    conflicts: Vec<gix::merge::tree::Conflict>,
    merged_tree: ObjectId,
    pub(crate) commit: ObjectId,
    pub(crate) deferred_ref_deletions: Vec<(gix::refs::FullName, ObjectId)>,
    rewritten: HashMap<ObjectId, Option<ObjectId>>,
}

impl PersistedConflict {
    pub(crate) fn map(&self, id: ObjectId) -> Option<ObjectId> {
        self.rewritten.get(&id).copied().unwrap_or(Some(id))
    }

    pub(crate) fn materialize(&mut self) -> Result<()> {
        let mut index = self
            .repo
            .index_from_tree(&self.merged_tree)
            .context("could not prepare the conflicting index")?;
        if !gix::merge::tree::apply_index_entries(
            &self.conflicts,
            gix::merge::tree::TreatAsUnresolved::git(),
            &mut index,
            gix::merge::tree::apply_index_entries::RemovalMode::Prune,
        ) {
            anyhow::bail!("could not apply conflict stages to the prepared index");
        }
        index.remove_tree();
        let ours_tree = self
            .repo
            .find_commit(self.commit)
            .context("could not find the conflicting commit")?
            .tree_id()
            .context("could not read the conflicting commit tree")?
            .detach();
        let workdir = self
            .repo
            .workdir()
            .context("materializing a conflict requires a worktree")?;
        super::forget::apply_tree_transition(workdir, ours_tree, self.merged_tree)
            .context("could not check out the conflicting merge result")?;
        if let Err(err) = index
            .write(gix::index::write::Options::default())
            .context("could not write the conflicting index")
        {
            return match super::forget::apply_tree_transition(workdir, self.merged_tree, ours_tree) {
                Ok(()) => Err(err),
                Err(rollback) => Err(err.context(format!("conflict checkout rollback failed: {rollback:#}"))),
            };
        }
        Ok(())
    }
}

struct Prepared {
    repo: gix::Repository,
    root: Option<ObjectId>,
    reset_index: bool,
    reset_index_paths: Option<Vec<BString>>,
    skip_worktree_transitions: bool,
    selected: Option<ObjectId>,
    rewritten: HashMap<ObjectId, Option<ObjectId>>,
    stash_rewritten: HashMap<ObjectId, Option<ObjectId>>,
    removed: HashSet<ObjectId>,
    committer: gix::actor::Signature,
    expected_refs: Option<Vec<ExpectedRef>>,
    checkout_reference: Option<gix::refs::FullName>,
    checkout_after_finish: bool,
    pins: Vec<ObjectId>,
    delete_refs: Vec<(gix::refs::FullName, Target)>,
}

pub(crate) fn capture_refs(repo: &gix::Repository, scope: &[ObjectId], tips: &[ObjectId]) -> Result<Vec<ExpectedRef>> {
    let scope: HashSet<_> = scope.iter().copied().collect();
    let tips: HashSet<_> = tips.iter().copied().collect();
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for reference in repo.references()?.all()? {
        let reference = match reference {
            Ok(reference) => reference,
            Err(err) if is_missing_ref(&*err) => continue,
            Err(err) => anyhow::bail!("could not inspect a reference before editing: {err}"),
        };
        if matches!(
            reference.name().category(),
            Some(Category::Tag | Category::RemoteBranch)
        ) {
            continue;
        }
        let Some(old) = reference.try_id().map(gix::Id::detach) else {
            continue;
        };
        if scope.contains(&old) && seen.insert(reference.name().to_owned()) {
            out.push(ExpectedRef {
                name: reference.name().to_owned(),
                old: Some(old),
                target: old,
                new: Some(old),
                follows_tip: tips.contains(&old),
                editable: !reference.name().as_bstr().starts_with(crate::history::PIN_PREFIX)
                    && !reference.name().as_bstr().starts_with(crate::history::REVIEW_PREFIX),
                placement: None,
            });
        }
    }
    Ok(out)
}

#[tracing::instrument(skip_all, fields(signature = ?signature, tree = ?tree_mode))]
pub(crate) fn perform(
    repo: &gix::Repository,
    graph: &HistoryGraph,
    edit: Edit,
    signature: Signature,
    tree_mode: Tree,
) -> Result<Perform> {
    perform_inner(repo, graph, edit, signature, tree_mode, Vec::new(), None, |_| {})
}

pub(crate) fn perform_with_progress(
    repo: &gix::Repository,
    graph: &HistoryGraph,
    edit: Edit,
    signature: Signature,
    tree_mode: Tree,
    report: impl FnMut(Progress),
) -> Result<Perform> {
    perform_inner(repo, graph, edit, signature, tree_mode, Vec::new(), None, report)
}

pub(super) fn perform_resetting_index_paths(
    repo: &gix::Repository,
    graph: &HistoryGraph,
    edit: Edit,
    signature: Signature,
    tree_mode: Tree,
    paths: Vec<BString>,
) -> Result<Perform> {
    perform_inner(repo, graph, edit, signature, tree_mode, Vec::new(), Some(paths), |_| {})
}

pub(super) fn perform_deleting_refs(
    repo: &gix::Repository,
    graph: &HistoryGraph,
    edit: Edit,
    signature: Signature,
    tree_mode: Tree,
    deletions: Vec<(gix::refs::FullName, Target)>,
) -> Result<Perform> {
    perform_inner(repo, graph, edit, signature, tree_mode, deletions, None, |_| {})
}

#[expect(
    clippy::too_many_arguments,
    reason = "shared edit preparation plus progress reporting"
)]
fn perform_inner(
    repo: &gix::Repository,
    graph: &HistoryGraph,
    edit: Edit,
    signature: Signature,
    tree_mode: Tree,
    delete_refs: Vec<(gix::refs::FullName, Target)>,
    reset_index_paths: Option<Vec<BString>>,
    mut report: impl FnMut(Progress),
) -> Result<Perform> {
    let mut repo = repo.clone();
    let repeat_checkout = match &edit {
        Edit::Repeat { checkout, .. } => Some(*checkout),
        _ => None,
    };
    let (root, replacement, inserted, reset_index, forked, removed, repeat, mut split_upper) = match edit {
        Edit::Replace { target, commit } => (Some(target), Some(commit), false, false, false, false, false, None),
        Edit::Insert {
            anchor,
            commit,
            reset_index,
        } => (anchor, Some(commit), true, reset_index, false, false, false, None),
        Edit::Fork { anchor, commit } => (Some(anchor), Some(commit), false, false, true, false, false, None),
        Edit::Remove { target } => (Some(target), None, false, false, false, true, false, None),
        Edit::Split { target, source, upper } => (
            Some(target),
            Some(source),
            false,
            false,
            false,
            false,
            false,
            Some(upper),
        ),
        Edit::Repeat { base, .. } => (Some(base), None, false, false, false, false, true, None),
    };

    let affected = match root.filter(|_| !forked) {
        Some(root) => graph
            .descendants_in_parent_order(root)
            .context("the edited commit is not in the loaded history")?,
        None => Vec::new(),
    };
    let mut progress = Progress {
        total: affected.len(),
        ..Progress::default()
    };
    report(progress);
    validate(&repo, graph, &affected, removed, repeat, tree_mode)?;

    let signing = repo
        .commit_signing_options_if_enabled()
        .context("could not resolve commit signing configuration")?;
    let committer = repo
        .committer()
        .context("no Git committer is configured")?
        .context("could not resolve the Git committer")?
        .to_owned()
        .context("could not own the Git committer")?;
    repo = repo.with_object_memory();

    let mut rewritten = HashMap::<ObjectId, Option<ObjectId>>::new();
    let mut selected = None;
    let mut conflict = None;
    if inserted || forked {
        let mut commit = replacement.clone().context("an inserted commit is required")?;
        commit.parents = root.into_iter().collect();
        let id = write_commit(
            &repo,
            commit,
            None,
            &committer,
            CommitState::Unmarked(signature),
            signing.clone(),
        )?;
        selected = Some(id);
        if inserted {
            if let Some(root) = root {
                rewritten.insert(root, Some(id));
            } else {
                rewritten.insert(id, Some(id));
            }
        }
    } else if removed {
        let root = root.context("a removed commit is required")?;
        let parent = graph
            .parents_of(root)
            .context("the removed commit is not in the loaded history")?
            .first()
            .copied();
        rewritten.insert(root, parent);
        selected = parent;
    }

    let mut pending = affected;
    if inserted || removed {
        pending.retain(|id| Some(*id) != root);
    }
    for old_id in pending {
        let old_parents = graph.parents_of(old_id).context("an affected commit is incomplete")?;
        let mut commit = if Some(old_id) == root {
            match replacement.clone() {
                Some(commit) => commit,
                None => repo
                    .find_commit(old_id)
                    .context("could not find commit to rewrite")?
                    .decode()
                    .context("could not decode commit to rewrite")?
                    .into_owned()
                    .context("could not own commit to rewrite")?,
            }
        } else {
            repo.find_commit(old_id)
                .context("could not find descendant commit")?
                .decode()
                .context("could not decode descendant commit")?
                .into_owned()
                .context("could not own descendant commit")?
        };
        let new_parents: Vec<_> = old_parents
            .iter()
            .filter_map(|parent| rewritten.get(parent).copied().unwrap_or(Some(*parent)))
            .collect();
        let recorded_parent = has_marker(&commit).then(|| marked_parent(&commit)).transpose()?;
        let original_parents =
            recorded_parent.map_or_else(|| old_parents.clone(), |parent| parent.into_iter().collect::<Vec<_>>());
        let eager =
            repeat && repeat_checkout.is_some_and(|checkout| graph.is_ancestor(old_id, checkout)) && conflict.is_none();
        let commit_tree_mode = if repeat {
            if eager {
                Tree::CherryPick
            } else {
                Tree::LeaveAsIsAndMark
            }
        } else if conflict.is_some() {
            Tree::LeaveAsIsAndMark
        } else {
            tree_mode
        };
        let cherry_pick_started = (commit_tree_mode == Tree::CherryPick).then(Instant::now);
        let rewritten_tree = rewritten_tree(
            &repo,
            &commit,
            if !repeat { &old_parents } else { &original_parents },
            &new_parents,
            commit_tree_mode,
        )?;
        let mut new_conflict = None;
        commit.tree = match rewritten_tree {
            TreeRewrite::Complete(tree) => tree,
            TreeRewrite::Conflict {
                ours,
                merged,
                conflicts,
            } => {
                new_conflict = Some((merged, conflicts));
                ours
            }
        };
        if let Some(started) = cherry_pick_started.filter(|_| new_conflict.is_none()) {
            progress.cherry_picked += 1;
            progress.cherry_pick_time += started.elapsed();
        }
        commit.parents = new_parents.into_iter().collect();
        let pending = commit_tree_mode == Tree::LeaveAsIsAndMark
            || (commit_tree_mode == Tree::LeaveAsIsAndMarkDescendants && Some(old_id) != root)
            || conflict.is_some()
            || new_conflict.is_some();
        let is_conflicting_commit = new_conflict.is_some();
        let signature = if conflict.is_some() || is_conflicting_commit || (repeat && !eager) {
            Signature::InvalidateExisting
        } else if repeat {
            Signature::RedoIfNeeded
        } else {
            signature
        };
        let state = if pending && (repeat || Some(old_id) != root) {
            CommitState::Pending {
                original_parent: recorded_parent.flatten().or_else(|| old_parents.first().copied()),
            }
        } else {
            CommitState::Unmarked(signature)
        };
        let (new_id, signing_time) =
            write_commit_timed(&repo, commit, Some(old_id), &committer, state, signing.clone())?;
        if let Some(elapsed) = signing_time {
            progress.signed += 1;
            progress.signing_time += elapsed;
        }
        progress.processed += 1;
        report(progress);
        rewritten.insert(old_id, Some(new_id));
        if let Some((tree, conflicts)) = new_conflict {
            conflict = Some((old_id, tree, conflicts, new_id));
        }
        if Some(old_id) == root {
            if let Some(mut upper) = split_upper.take() {
                upper.parents = [new_id].into_iter().collect();
                let upper_id = write_commit(
                    &repo,
                    upper,
                    None,
                    &committer,
                    CommitState::Unmarked(Signature::RedoIfNeeded),
                    signing.clone(),
                )?;
                rewritten.insert(old_id, Some(upper_id));
                selected = Some(upper_id);
            } else {
                selected = Some(new_id);
            }
        }
    }

    let marked = (!forked && matches!(tree_mode, Tree::LeaveAsIsAndMark | Tree::LeaveAsIsAndMarkDescendants))
        || conflict.is_some();
    let checkout_after_finish = conflict.is_some();
    let mut prepared = Prepared {
        repo,
        root,
        reset_index: if inserted { reset_index } else { marked },
        reset_index_paths,
        skip_worktree_transitions: inserted
            || forked
            || (matches!(tree_mode, Tree::LeaveAsIsAndMark | Tree::LeaveAsIsAndMarkDescendants) && !removed),
        selected,
        stash_rewritten: rewritten.clone(),
        rewritten,
        removed: if removed {
            root.into_iter().collect()
        } else {
            HashSet::new()
        },
        committer,
        expected_refs: None,
        checkout_reference: None,
        checkout_after_finish,
        pins: if forked {
            selected.into_iter().collect()
        } else {
            Vec::new()
        },
        delete_refs,
    };
    match conflict {
        Some((original, merged_tree, conflicts, commit)) => Ok(Perform::Conflict(Conflict {
            prepared,
            conflicts,
            merged_tree,
            commit,
            original,
        })),
        None => Ok(Perform::Complete(prepared.finish()?)),
    }
}

#[tracing::instrument(skip_all, fields(%review, %tip))]
pub(super) fn finish_review(
    repo: &gix::Repository,
    graph: &HistoryGraph,
    review: ObjectId,
    tip: ObjectId,
    review_ref: gix::refs::FullName,
    delete_refs: Vec<(gix::refs::FullName, Target)>,
    checkout: Option<(ObjectId, Option<gix::refs::FullName>)>,
) -> Result<Outcome> {
    let mut repo = repo.clone();
    let signing = repo
        .commit_signing_options_if_enabled()
        .context("could not resolve commit signing configuration")?;
    let committer = repo
        .committer()
        .context("no Git committer is configured")?
        .context("could not resolve the Git committer")?
        .to_owned()?;
    repo = repo.with_object_memory();

    let review_ids = graph
        .descendants_in_parent_order(review)
        .context("the review commit is not in the loaded history")?;
    let review_set: HashSet<_> = review_ids.iter().copied().collect();
    let natural_ids: Vec<_> = graph
        .descendants_in_parent_order(tip)
        .context("the reviewed commit is not in the loaded history")?
        .into_iter()
        .filter(|id| *id != tip && !review_set.contains(id))
        .collect();
    for id in review_ids.iter().chain(&natural_ids) {
        if graph
            .parents_of(*id)
            .context("a review descendant is incomplete")?
            .len()
            > 1
        {
            anyhow::bail!("review finish cannot rewrite merge descendants");
        }
    }

    let mut rewritten = HashMap::<ObjectId, Option<ObjectId>>::new();
    let mut finished_review = None;
    for old in &review_ids {
        let old_parents = graph.parents_of(*old).context("a review descendant is incomplete")?;
        let mut commit = repo.find_commit(*old)?.decode()?.into_owned()?;
        let new_parents = if *old == review {
            vec![tip]
        } else {
            old_parents
                .iter()
                .filter_map(|parent| rewritten.get(parent).copied().unwrap_or(Some(*parent)))
                .collect()
        };
        commit.parents = new_parents.into_iter().collect();
        if *old == review {
            commit.extra_headers.retain(|(name, value)| {
                !(name.as_slice() == MARKER
                    && value.as_slice().strip_prefix(b"onto ") == Some(review_ref.as_bstr().as_ref()))
                    && name.as_slice() != super::review::RETURN_TO
            });
        }
        let new = write_commit(
            &repo,
            commit,
            Some(*old),
            &committer,
            CommitState::Unmarked(Signature::RedoIfNeeded),
            signing.clone(),
        )?;
        rewritten.insert(*old, Some(new));
        if *old == review {
            finished_review = Some(new);
        }
    }
    let finished_review = finished_review.context("the review commit was not rewritten")?;
    let non_leaves: HashSet<_> = review_ids
        .iter()
        .flat_map(|id| graph.parents_of(*id).unwrap_or_default())
        .filter(|parent| review_set.contains(parent))
        .collect();
    let leaves: Vec<_> = review_ids
        .iter()
        .filter(|id| !non_leaves.contains(*id))
        .copied()
        .collect();
    let insertion = if leaves.len() == 1 {
        rewritten[&leaves[0]].context("the review leaf disappeared")?
    } else {
        finished_review
    };

    rewritten.insert(tip, Some(finished_review));
    for old in natural_ids {
        let old_parents = graph.parents_of(old).context("a reviewed descendant is incomplete")?;
        let mut commit = repo.find_commit(old)?.decode()?.into_owned()?;
        let new_parents: Vec<_> = old_parents
            .iter()
            .filter_map(|parent| {
                if *parent == tip {
                    Some(insertion)
                } else {
                    rewritten.get(parent).copied().unwrap_or(Some(*parent))
                }
            })
            .collect();
        commit.parents = new_parents.into_iter().collect();
        let new = write_commit(
            &repo,
            commit,
            Some(old),
            &committer,
            CommitState::Pending {
                original_parent: old_parents.first().copied(),
            },
            signing.clone(),
        )?;
        rewritten.insert(old, Some(new));
    }

    let (selected, checkout_reference) = checkout.map_or((finished_review, None), |(old, reference)| {
        (rewritten.get(&old).copied().flatten().unwrap_or(old), reference)
    });
    let mut prepared = Prepared {
        repo,
        root: Some(review),
        reset_index: false,
        reset_index_paths: None,
        skip_worktree_transitions: false,
        selected: Some(selected),
        stash_rewritten: rewritten.clone(),
        rewritten,
        removed: HashSet::new(),
        committer,
        expected_refs: None,
        checkout_reference,
        checkout_after_finish: false,
        pins: Vec::new(),
        delete_refs,
    };
    prepared.finish()
}

pub(crate) fn perform_plan(repo: &gix::Repository, graph: &HistoryGraph, plan: Plan) -> Result<PlanPerform> {
    perform_plan_with_progress(repo, graph, plan, |_| {})
}

#[tracing::instrument(skip_all, fields(base = %plan.base, steps = plan.steps.len()))]
pub(crate) fn perform_plan_with_progress(
    repo: &gix::Repository,
    graph: &HistoryGraph,
    mut plan: Plan,
    mut report: impl FnMut(Progress),
) -> Result<PlanPerform> {
    let mut progress = Progress::for_plan(&plan);
    report(progress);
    let mut repo = repo.clone();
    let signing = repo
        .commit_signing_options_if_enabled()
        .context("could not resolve commit signing configuration")?;
    let author = repo
        .author()
        .context("no Git author is configured")?
        .context("could not resolve the Git author")?
        .to_owned()
        .context("could not own the Git author")?;
    let committer = repo
        .committer()
        .context("no Git committer is configured")?
        .context("could not resolve the Git committer")?
        .to_owned()
        .context("could not own the Git committer")?;
    repo = repo.with_object_memory();

    let scope: HashSet<_> = plan.scope.iter().copied().collect();
    let mut picked = HashSet::new();
    for step in &plan.steps {
        let ids = match step.commit {
            PlanCommit::Pick(id) | PlanCommit::Resolved(id) => Some(id).into_iter().chain(step.squash.iter().copied()),
            PlanCommit::Empty(_) => None.into_iter().chain(step.squash.iter().copied()),
        };
        for id in ids {
            if !scope.contains(&id) || !picked.insert(id) {
                anyhow::bail!("a rebase plan contains an invalid or duplicate pick");
            }
            if graph.parents_of(id).context("a picked commit is incomplete")?.len() > 1 {
                anyhow::bail!("merge commits cannot be picked by the rebase editor");
            }
        }
    }

    let mut eager = HashSet::new();
    let mut cursor = plan.checkout.as_ref().map(|checkout| checkout.target);
    while let Some(PlanParent::Step(index)) = cursor {
        if !eager.insert(index) {
            anyhow::bail!("the checkout ancestry contains a cycle");
        }
        cursor = match plan.steps.get(index).context("the checkout step is missing")?.parent {
            parent @ PlanParent::Step(_) => Some(parent),
            PlanParent::Existing(_) => None,
        };
    }

    let mut rewritten = HashMap::<ObjectId, Option<ObjectId>>::new();
    let mut produced = Vec::with_capacity(plan.steps.len());
    let mut delete_refs = Vec::new();
    let mut conflict = None;
    let mut marked = false;
    for (index, step) in plan.steps.iter().enumerate() {
        let parent = match step.parent {
            PlanParent::Existing(id) => {
                repo.find_commit(id).context("could not find a fork target")?;
                id
            }
            PlanParent::Step(parent) => *produced.get(parent).context("a fork points to a later commit")?,
        };
        let eager = conflict.is_none() && (eager.contains(&index) || !step.squash.is_empty());
        let mut commit = match &step.commit {
            PlanCommit::Pick(id) => repo
                .find_commit(*id)
                .context("could not find a picked commit")?
                .decode()
                .context("could not decode a picked commit")?
                .into_owned()
                .context("could not own a picked commit")?,
            PlanCommit::Resolved(_) => {
                let mut commit = repo
                    .head()?
                    .peel_to_commit()
                    .context("could not resolve the conflicted HEAD commit")?
                    .decode()
                    .context("could not decode the conflicted HEAD commit")?
                    .into_owned()
                    .context("could not own the conflicted HEAD commit")?;
                let index = repo
                    .index_or_empty()
                    .context("could not load the resolved conflict index")?;
                if index
                    .entries()
                    .iter()
                    .any(|entry| entry.stage() != gix::index::entry::Stage::Unconflicted)
                {
                    anyhow::bail!("the conflict index still has unresolved entries");
                }
                commit.tree = super::create::index_tree(&repo, &index)?;
                commit
            }
            PlanCommit::Empty(title) => gix::objs::Commit {
                tree: parent_tree(&repo, Some(parent))?,
                parents: [parent].into_iter().collect(),
                author: author.clone(),
                committer: committer.clone(),
                encoding: None,
                message: title.clone(),
                extra_headers: Vec::new(),
            },
        };
        let graph_parents = match step.commit {
            PlanCommit::Pick(id) | PlanCommit::Resolved(id) => {
                graph.parents_of(id).context("a picked commit is incomplete")?
            }
            PlanCommit::Empty(_) => vec![parent],
        };
        let recorded_parent = has_marker(&commit).then(|| marked_parent(&commit)).transpose()?;
        if let PlanCommit::Pick(id) = step.commit
            && step.squash.is_empty()
            && !is_pending(&commit)
            && graph_parents.as_slice() == [parent]
        {
            rewritten.insert(id, Some(id));
            produced.push(id);
            progress.processed += 1;
            report(progress);
            continue;
        }
        let replay_parents = recorded_parent.map_or_else(
            || graph_parents.clone(),
            |parent| parent.into_iter().collect::<Vec<_>>(),
        );
        let mode = if eager {
            Tree::CherryPick
        } else {
            Tree::LeaveAsIsAndMark
        };
        let cherry_pick_started =
            (eager && matches!(step.commit, PlanCommit::Pick(_) | PlanCommit::Resolved(_))).then(Instant::now);
        let mut step_conflict = None;
        commit.tree = match rewritten_tree(&repo, &commit, &replay_parents, &[parent], mode)? {
            TreeRewrite::Complete(tree) => tree,
            TreeRewrite::Conflict {
                ours,
                merged,
                conflicts,
            } => {
                let (PlanCommit::Pick(original) | PlanCommit::Resolved(original)) = step.commit else {
                    unreachable!("empty commits cannot conflict")
                };
                step_conflict = Some((original, merged, conflicts, None));
                ours
            }
        };
        if let Some(started) = cherry_pick_started.filter(|_| step_conflict.is_none()) {
            progress.cherry_picked += 1;
            progress.cherry_pick_time += started.elapsed();
        }
        if matches!(step.commit, PlanCommit::Pick(_) | PlanCommit::Resolved(_)) {
            progress.processed += 1;
            report(progress);
        }
        let mut squashed = Vec::with_capacity(step.squash.len());
        for id in &step.squash {
            let source = repo
                .find_commit(*id)
                .context("could not find a squashed commit")?
                .decode()
                .context("could not decode a squashed commit")?
                .into_owned()
                .context("could not own a squashed commit")?;
            let graph_parents = graph.parents_of(*id).context("a squashed commit is incomplete")?;
            let recorded_parent = has_marker(&source).then(|| marked_parent(&source)).transpose()?;
            let replay_parents = recorded_parent.map_or_else(
                || graph_parents.clone(),
                |parent| parent.into_iter().collect::<Vec<_>>(),
            );
            squashed.push((*id, source, replay_parents));
        }
        if !squashed.is_empty() && conflict.is_none() && step_conflict.is_none() {
            let mut applied = 0;
            for (squash_index, (id, source, replay_parents)) in squashed.iter().enumerate() {
                let old_base = parent_tree(&repo, replay_parents.first().copied())?;
                let started = Instant::now();
                commit.tree = match cherry_pick_tree_outcome(&repo, old_base, commit.tree, source.tree)? {
                    TreeRewrite::Complete(tree) => tree,
                    TreeRewrite::Conflict {
                        ours,
                        merged,
                        conflicts,
                    } => {
                        step_conflict = Some((*id, merged, conflicts, Some(squash_index)));
                        ours
                    }
                };
                applied += 1;
                if step_conflict.is_some() {
                    break;
                }
                progress.cherry_picked += 1;
                progress.cherry_pick_time += started.elapsed();
                progress.processed += 1;
                report(progress);
                delete_refs.extend(super::review::deletions(&repo, source)?);
            }
            squash_message(&repo, &mut commit, &squashed[..applied])?;
        }
        commit.parents = [parent].into_iter().collect();
        let state = if step_conflict.is_some() {
            CommitState::Unmarked(Signature::InvalidateExisting)
        } else if eager {
            CommitState::Unmarked(Signature::RedoIfNeeded)
        } else {
            marked = true;
            CommitState::Pending {
                original_parent: recorded_parent.flatten().or_else(|| graph_parents.first().copied()),
            }
        };
        let predecessor = match step.commit {
            PlanCommit::Pick(id) | PlanCommit::Resolved(id) => Some(id),
            PlanCommit::Empty(_) => None,
        };
        let (new_id, signing_time) =
            write_commit_timed(&repo, commit, predecessor, &committer, state, signing.clone())?;
        if let Some(elapsed) = signing_time {
            progress.signed += 1;
            progress.signing_time += elapsed;
        }
        if matches!(step.commit, PlanCommit::Empty(_)) {
            progress.processed += 1;
        }
        report(progress);
        if let PlanCommit::Pick(old_id) | PlanCommit::Resolved(old_id) = step.commit {
            rewritten.insert(old_id, Some(new_id));
        }
        for old_id in &step.squash {
            rewritten.insert(*old_id, Some(new_id));
        }
        produced.push(new_id);
        if let Some((original, tree, conflicts, squash_index)) = step_conflict {
            let remaining_squash = squash_index.map_or_else(
                || step.squash.clone(),
                |squash_index| step.squash[squash_index + 1..].to_vec(),
            );
            conflict = Some((original, tree, conflicts, new_id, index, remaining_squash));
        }
    }

    let dropped: Vec<_> = plan.scope.iter().copied().filter(|id| !picked.contains(id)).collect();
    let removed: HashSet<_> = dropped.iter().copied().collect();
    for id in &dropped {
        let commit = repo.find_commit(*id)?.decode()?.into_owned()?;
        delete_refs.extend(super::review::deletions(&repo, &commit)?);
    }
    for dropped in dropped {
        let mut ancestor = graph
            .parents_of(dropped)
            .context("a dropped commit is incomplete")?
            .first()
            .copied();
        while let Some(id) = ancestor {
            if let Some(new) = rewritten.get(&id).copied().flatten() {
                ancestor = Some(new);
                break;
            }
            if !scope.contains(&id) {
                break;
            }
            ancestor = graph
                .parents_of(id)
                .context("a dropped ancestor is incomplete")?
                .first()
                .copied();
        }
        rewritten.insert(dropped, ancestor.or(Some(plan.base)));
    }

    let mut primary_children = HashMap::new();
    for (index, step) in plan.steps.iter().enumerate() {
        let parent = match step.parent {
            PlanParent::Existing(id) => id,
            PlanParent::Step(parent) => produced[parent],
        };
        primary_children.entry(parent).or_insert(produced[index]);
    }
    for expected in &mut plan.expected_refs {
        if let Some(target) = expected.placement {
            expected.new = Some(match target {
                PlanParent::Existing(id) => id,
                PlanParent::Step(index) => *produced.get(index).context("a reference points to a missing step")?,
            });
        } else if expected.new.is_some() {
            let mut target = rewritten
                .get(&expected.target)
                .copied()
                .flatten()
                .unwrap_or(expected.target);
            if expected.follows_tip {
                while let Some(child) = primary_children.get(&target) {
                    target = *child;
                }
            }
            expected.new = Some(target);
        }
    }

    let planned_non_leaves: HashSet<_> = plan
        .steps
        .iter()
        .filter_map(|step| match step.parent {
            PlanParent::Step(index) => Some(index),
            PlanParent::Existing(_) => None,
        })
        .collect();
    let mut pins = Vec::new();
    for (index, id) in produced.iter().copied().enumerate() {
        if planned_non_leaves.contains(&index)
            || plan
                .checkout
                .as_ref()
                .is_some_and(|checkout| checkout.target == PlanParent::Step(index))
        {
            continue;
        }
        let referenced = plan.expected_refs.iter().any(|expected| expected.new == Some(id));
        if !referenced {
            pins.push(id);
        }
    }
    let selected = plan.checkout.as_ref().map(|checkout| match checkout.target {
        PlanParent::Existing(id) => id,
        PlanParent::Step(index) => produced[index],
    });
    let mut prepared = Prepared {
        repo,
        root: Some(plan.base),
        reset_index: marked,
        reset_index_paths: None,
        skip_worktree_transitions: false,
        selected,
        rewritten: rewritten.clone(),
        stash_rewritten: rewritten.clone(),
        removed,
        committer,
        expected_refs: Some(plan.expected_refs.clone()),
        checkout_reference: plan.checkout.as_ref().and_then(|checkout| checkout.reference.clone()),
        checkout_after_finish: false,
        pins,
        delete_refs,
    };
    tracing::info!(
        total = progress.total,
        processed = progress.processed,
        cherry_picked = progress.cherry_picked,
        signed = progress.signed,
        cherry_pick_ms = progress.cherry_pick_time.as_millis(),
        signing_ms = progress.signing_time.as_millis(),
        "prepared rebase plan"
    );
    let Some((original, merged_tree, conflicts, commit, conflict_step, conflict_remaining_squash)) = conflict else {
        return Ok(PlanPerform::Complete(prepared.finish()?));
    };

    let continuation_start = plan
        .checkout
        .as_ref()
        .and_then(|checkout| match checkout.target {
            PlanParent::Step(index) if index < conflict_step => Some(index),
            _ => None,
        })
        .map_or(conflict_step, |_| 0);
    let affected_steps: HashSet<_> = (continuation_start..plan.steps.len()).collect();
    let affected_ids: HashSet<_> = affected_steps.iter().map(|index| produced[*index]).collect();
    let final_refs: Vec<_> = plan
        .expected_refs
        .iter()
        .filter(|expected| expected.new.is_none_or(|new| !affected_ids.contains(&new)))
        .cloned()
        .collect();
    let final_ref_names = final_refs.iter().map(|expected| expected.name.clone()).collect();
    let mut remaining_squash = vec![Vec::new(); plan.steps.len()];
    remaining_squash[conflict_step] = conflict_remaining_squash;
    for (index, step) in plan.steps.iter().enumerate().skip(conflict_step + 1) {
        remaining_squash[index].clone_from(&step.squash);
    }
    prepared.selected = None;
    prepared.checkout_reference = None;
    prepared.checkout_after_finish = true;
    prepared.reset_index = false;
    prepared.rewritten = final_refs
        .iter()
        .filter_map(|expected| expected.old.map(|old| (old, expected.new)))
        .collect();
    prepared.expected_refs = Some(final_refs);
    prepared.pins.retain(|id| !affected_ids.contains(id));
    Ok(PlanPerform::Conflict(PlanConflict {
        conflict: Conflict {
            prepared,
            conflicts,
            merged_tree,
            commit,
            original,
        },
        plan,
        produced,
        rewritten,
        conflict_step,
        continuation_start,
        remaining_squash,
        final_refs: final_ref_names,
    }))
}

impl Prepared {
    fn finish(&mut self) -> Result<Outcome> {
        let stash_edits = super::stash::rewrite_edits(&self.repo, &self.stash_rewritten, &self.removed)?;
        let objects = self
            .repo
            .objects
            .take_object_memory()
            .context("candidate object memory was unavailable")?;
        for (id, (kind, data)) in objects.iter() {
            self.repo
                .write_buf_with_known_id(*kind, data, *id)
                .map_err(|err| anyhow::anyhow!("could not persist a prepared rebase object: {err}"))?;
        }

        let transitions = worktree_transitions(
            &self.repo,
            &self.rewritten,
            self.expected_refs.as_deref(),
            self.skip_worktree_transitions,
        )?;
        let index_reset_from = if self.reset_index { self.root } else { None };
        let index_resets = index_reset_from
            .map(|old| index_resets(&self.repo, &self.rewritten, old))
            .transpose()?;
        for transition in &transitions {
            super::forget::preflight_tree_transition(
                &transition.repo,
                &transition.workdir,
                transition.old,
                transition.new,
            )?;
        }
        let current_ref = self.repo.head()?.referent_name().map(ToOwned::to_owned);
        let mut deferred_ref_deletions = Vec::new();
        if let (Some(expected_refs), Some(current_ref)) = (&mut self.expected_refs, current_ref) {
            if expected_refs
                .iter()
                .any(|expected| expected.name == current_ref && expected.old.is_some() && expected.new.is_none())
                && (self.repo.workdir().is_none() || (self.selected.is_none() && !self.checkout_after_finish))
            {
                anyhow::bail!("cannot delete the checked-out branch without selecting another checkout");
            }
            expected_refs.retain(|expected| {
                let defer = expected.name == current_ref && expected.old.is_some() && expected.new.is_none();
                if defer {
                    deferred_ref_deletions.push((expected.name.clone(), expected.old.expect("checked above")));
                }
                !defer
            });
        }
        let rollback_refs = update_refs(
            &self.repo,
            &self.rewritten,
            self.root.is_none(),
            self.selected,
            &self.committer,
            self.expected_refs.take(),
            (&self.pins, &self.delete_refs),
            stash_edits,
        )?;
        for (transitioned, transition) in transitions.iter().enumerate() {
            if let Err(err) = super::forget::apply_tree_transition(&transition.workdir, transition.old, transition.new)
            {
                return rollback(
                    &self.repo,
                    &self.committer,
                    &transitions[..transitioned],
                    &rollback_refs,
                    err,
                );
            }
        }
        let mut index_resets = index_resets.unwrap_or_default();
        for index in 0..index_resets.len() {
            if let Err(mut err) = reset_index(&mut index_resets[index], self.reset_index_paths.as_deref()) {
                for applied in index_resets[..=index].iter().rev() {
                    if let Err(restore) = std::fs::write(&applied.index, &applied.before) {
                        err = err.context(format!("index rollback failed: {restore}"));
                    }
                }
                return rollback(&self.repo, &self.committer, &transitions, &rollback_refs, err);
            }
        }
        Ok(Outcome {
            selected: self.selected,
            checkout_reference: self.checkout_reference.clone(),
            deferred_ref_deletions,
            rewritten: std::mem::take(&mut self.rewritten),
        })
    }
}

fn rollback<T>(
    repo: &gix::Repository,
    committer: &gix::actor::Signature,
    transitions: &[Transition],
    refs: &[RefEdit],
    cause: anyhow::Error,
) -> Result<T> {
    let mut failures = Vec::new();
    for transition in transitions.iter().rev() {
        if let Err(err) = super::forget::apply_tree_transition(&transition.workdir, transition.new, transition.old) {
            failures.push(format!("worktree rollback failed: {err:#}"));
        }
    }
    let mut time = gix::date::parse::TimeBuf::default();
    if let Err(err) = repo.edit_references_as(refs.iter().cloned(), Some(committer.to_ref(&mut time))) {
        failures.push(format!("reference rollback failed: {err}"));
    }
    if failures.is_empty() {
        Err(cause)
    } else {
        Err(cause.context(failures.join("; ")))
    }
}

fn index_resets(
    repo: &gix::Repository,
    rewritten: &HashMap<ObjectId, Option<ObjectId>>,
    reset_from: ObjectId,
) -> Result<Vec<IndexReset>> {
    let mut repos = vec![
        repo.main_repo()
            .context("could not open the main worktree repository")?,
    ];
    for proxy in repo.worktrees().context("could not enumerate linked worktrees")? {
        if let Ok(worktree_repo) = proxy.into_repo_with_possibly_inaccessible_worktree() {
            repos.push(worktree_repo);
        }
    }
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for worktree_repo in repos {
        if !seen.insert(worktree_repo.git_dir().to_owned()) {
            continue;
        }
        let Some(old) = worktree_repo
            .head()
            .ok()
            .and_then(|head| head.id().map(gix::Id::detach))
        else {
            continue;
        };
        if old != reset_from {
            continue;
        }
        let Some(Some(new)) = rewritten.get(&old).copied() else {
            continue;
        };
        if let Some(workdir) = worktree_repo.workdir().filter(|path| path.is_dir()).map(PathBuf::from) {
            let index = worktree_repo.index_path();
            let index_path = index.to_owned();
            let before = std::fs::read(index).context("could not preserve an affected index")?;
            out.push(IndexReset {
                repo: worktree_repo,
                workdir,
                index: index_path,
                before,
                new,
            });
        }
    }
    Ok(out)
}

struct IndexReset {
    repo: gix::Repository,
    workdir: PathBuf,
    index: PathBuf,
    before: Vec<u8>,
    new: ObjectId,
}

fn reset_index(reset: &mut IndexReset, paths: Option<&[BString]>) -> Result<()> {
    if let Some(paths) = paths {
        return reset_index_paths(&reset.repo, reset.new, paths);
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(&reset.workdir)
        .args(["reset", "--mixed", "--quiet"])
        .arg(reset.new.to_string())
        .output()
        .context("could not update the index after inserting a commit")?;
    if output.status.success() {
        Ok(())
    } else {
        anyhow::bail!("git reset failed: {}", String::from_utf8_lossy(&output.stderr).trim())
    }
}

fn reset_index_paths(repo: &gix::Repository, id: ObjectId, paths: &[BString]) -> Result<()> {
    let tree = repo.find_commit(id)?.tree()?;
    let mut index = repo
        .open_index()
        .context("could not load the index to update selected paths")?;
    for path in paths {
        let previous = index
            .entry_by_path(path.as_bstr())
            .map(|entry| (entry.stat, entry.flags));
        index.remove_entries(|_, candidate, _| candidate == path.as_bstr());
        if let Some(entry) = tree.lookup_entry(
            path.split(|byte| *byte == b'/')
                .map(|component| BStr::new(component).to_owned()),
        )? {
            let (stat, flags) =
                previous.unwrap_or((gix::index::entry::Stat::default(), gix::index::entry::Flags::empty()));
            index.dangerously_push_entry(
                stat,
                entry.object_id(),
                flags,
                entry.mode().kind().into(),
                path.as_bstr(),
            );
        }
    }
    index.sort_entries();
    index.remove_tree();
    index
        .write(gix::index::write::Options::default())
        .context("could not update selected index paths")
}

fn validate(
    repo: &gix::Repository,
    graph: &HistoryGraph,
    affected: &[ObjectId],
    removed: bool,
    repeat: bool,
    tree: Tree,
) -> Result<()> {
    for (position, id) in affected.iter().enumerate() {
        let parents = graph.parents_of(*id).context("an affected commit is incomplete")?;
        if parents.len() > 1 && (position > 0 || removed || tree == Tree::CherryPick) {
            anyhow::bail!("descendant merge commits cannot be rebased");
        }
        if repeat && position == 0 {
            let commit = repo.find_commit(*id)?.decode()?.into_owned()?;
            if !is_pending(&commit) {
                anyhow::bail!("the root of a repeated rebase must be pending");
            }
        }
    }
    if repeat
        && let Some(base) = affected.first()
        && let Some(parent) = graph.parents_of(*base).and_then(|parents| parents.first().copied())
        && is_pending(&repo.find_commit(parent)?.decode()?.into_owned()?)
    {
        anyhow::bail!("the parent of a repeated rebase must not be pending");
    }
    Ok(())
}

enum TreeRewrite {
    Complete(ObjectId),
    Conflict {
        ours: ObjectId,
        merged: ObjectId,
        conflicts: Vec<gix::merge::tree::Conflict>,
    },
}

fn squash_message(
    repo: &gix::Repository,
    commit: &mut gix::objs::Commit,
    squashed: &[(ObjectId, gix::objs::Commit, Vec<ObjectId>)],
) -> Result<()> {
    let mut known = HashSet::<(BString, BString)>::new();
    collect_co_authors(&commit.message, &mut known);
    for (_, source, _) in squashed {
        collect_co_authors(&source.message, &mut known);
    }
    known.insert(author_identity(&commit.author));
    let mut additional = Vec::new();
    for (_, source, _) in squashed {
        let author = author_identity(&source.author);
        if known.insert(author.clone()) {
            additional.push(author);
        }
    }

    let mut message = commit.message.to_vec();
    for (id, source, _) in squashed {
        while message.last().is_some_and(|byte| matches!(byte, b'\n' | b'\r')) {
            message.pop();
        }
        let short = id
            .attach(repo)
            .shorten()
            .context("could not shorten a squashed commit ID")?;
        message.extend_from_slice(b"\n\n# ");
        message.extend_from_slice(short.to_string().as_bytes());
        message.push(b' ');
        message.extend_from_slice(
            gix::objs::commit::MessageRef::from_bytes(&source.message)
                .summary()
                .as_ref(),
        );
        message.extend_from_slice(b"\n\n");
        message.extend_from_slice(&source.message);
    }
    if !additional.is_empty() {
        let has_trailers = gix::objs::commit::MessageRef::from_bytes(&message)
            .body()
            .is_some_and(|body| body.trailers().next().is_some());
        while message.last().is_some_and(|byte| matches!(byte, b'\n' | b'\r')) {
            message.pop();
        }
        message.extend_from_slice(if has_trailers { b"\n" } else { b"\n\n" });
        for (name, email) in additional {
            message.extend_from_slice(b"Co-authored-by: ");
            message.extend_from_slice(&name);
            message.extend_from_slice(b" <");
            message.extend_from_slice(&email);
            message.extend_from_slice(b">\n");
        }
    }
    commit.message = message.into();
    Ok(())
}

fn author_identity(author: &gix::actor::Signature) -> (BString, BString) {
    (
        author.name.trim().to_owned().into(),
        author.email.trim().to_owned().into(),
    )
}

fn collect_co_authors(message: &[u8], out: &mut HashSet<(BString, BString)>) {
    let Some(body) = gix::objs::commit::MessageRef::from_bytes(message).body() else {
        return;
    };
    for trailer in body.trailers().co_authored_by() {
        let mut value: &[u8] = trailer.value.as_ref();
        let Ok(identity) = gix::actor::IdentityRef::from_bytes_consuming(&mut value) else {
            continue;
        };
        if value.trim().is_empty() {
            let identity = identity.trim();
            out.insert((identity.name.to_owned(), identity.email.to_owned()));
        }
    }
}

fn rewritten_tree(
    repo: &gix::Repository,
    commit: &gix::objs::Commit,
    old_parents: &[ObjectId],
    new_parents: &[ObjectId],
    mode: Tree,
) -> Result<TreeRewrite> {
    if mode != Tree::CherryPick || old_parents == new_parents {
        return Ok(TreeRewrite::Complete(commit.tree));
    }
    let old_base = parent_tree(repo, old_parents.first().copied())?;
    let new_base = parent_tree(repo, new_parents.first().copied())?;
    if commit.tree == old_base {
        return Ok(TreeRewrite::Complete(new_base));
    }
    if old_base == new_base {
        return Ok(TreeRewrite::Complete(commit.tree));
    }
    cherry_pick_tree_outcome(repo, old_base, new_base, commit.tree)
}

pub(super) fn cherry_pick_tree(
    repo: &gix::Repository,
    old_base: ObjectId,
    new_base: ObjectId,
    tree: ObjectId,
) -> Result<ObjectId> {
    match cherry_pick_tree_outcome(repo, old_base, new_base, tree)? {
        TreeRewrite::Complete(tree) => Ok(tree),
        TreeRewrite::Conflict { .. } => anyhow::bail!("rebasing would cause a merge conflict"),
    }
}

fn cherry_pick_tree_outcome(
    repo: &gix::Repository,
    old_base: ObjectId,
    new_base: ObjectId,
    tree: ObjectId,
) -> Result<TreeRewrite> {
    if tree == old_base {
        return Ok(TreeRewrite::Complete(new_base));
    }
    if old_base == new_base {
        return Ok(TreeRewrite::Complete(tree));
    }
    let labels = gix::merge::blob::builtin_driver::text::Labels {
        ancestor: Some(BStr::new(b"parent")),
        current: Some(BStr::new(b"rebased parent")),
        other: Some(BStr::new(b"commit")),
    };
    let mut outcome = repo
        .merge_trees(old_base, new_base, tree, labels, repo.tree_merge_options()?)
        .context("could not cherry-pick a descendant tree")?;
    let unresolved = outcome.has_unresolved_conflicts(gix::merge::tree::TreatAsUnresolved::git());
    let merged = outcome
        .tree
        .write()
        .context("could not prepare a rebased tree")?
        .detach();
    if unresolved {
        Ok(TreeRewrite::Conflict {
            ours: new_base,
            merged,
            conflicts: outcome.conflicts,
        })
    } else {
        Ok(TreeRewrite::Complete(merged))
    }
}

fn parent_tree(repo: &gix::Repository, parent: Option<ObjectId>) -> Result<ObjectId> {
    match parent {
        Some(parent) => Ok(repo.find_commit(parent)?.tree_id()?.detach()),
        None => Ok(repo.empty_tree().id),
    }
}

fn marker(commit: &mut gix::objs::Commit, add: bool, original_parent: Option<ObjectId>) {
    commit
        .extra_headers
        .retain(|(name, _)| name.as_slice() != ORIGINAL_PARENT);
    if add {
        let parent = original_parent.unwrap_or_else(|| ObjectId::null(commit.tree.kind()));
        commit
            .extra_headers
            .push((ORIGINAL_PARENT.into(), parent.to_hex().to_string().into()));
    }
}

fn marked_parent(commit: &gix::objs::Commit) -> Result<Option<ObjectId>> {
    commit
        .extra_headers
        .iter()
        .find(|(name, _)| name.as_slice() == ORIGINAL_PARENT)
        .map(|(_, value)| parse_marked_parent(value.as_bstr()))
        .transpose()
        .map(Option::flatten)
}

pub(crate) fn marked_parent_ref(commit: &gix::objs::CommitRef<'_>) -> Result<Option<Option<ObjectId>>> {
    commit
        .extra_headers()
        .find("tix-rebase-parent")
        .map(parse_marked_parent)
        .transpose()
}

fn parse_marked_parent(value: &BStr) -> Result<Option<ObjectId>> {
    ObjectId::from_hex(value)
        .context("pending rebase has an invalid original parent")
        .map(|id| (!id.is_null()).then_some(id))
}

pub(super) fn has_marker(commit: &gix::objs::Commit) -> bool {
    commit
        .extra_headers
        .iter()
        .any(|(name, _)| name.as_slice() == ORIGINAL_PARENT)
}

pub(super) fn is_pending(commit: &gix::objs::Commit) -> bool {
    has_marker(commit)
        || commit
            .extra_headers
            .iter()
            .any(|(name, value)| is_signature(name) && value.is_empty())
}

fn write_commit(
    repo: &gix::Repository,
    commit: gix::objs::Commit,
    predecessor: Option<ObjectId>,
    committer: &gix::actor::Signature,
    state: CommitState,
    signing: Option<gix::objs::commit::signature::Options>,
) -> Result<ObjectId> {
    Ok(write_commit_timed(repo, commit, predecessor, committer, state, signing)?.0)
}

fn write_commit_timed(
    repo: &gix::Repository,
    mut commit: gix::objs::Commit,
    predecessor: Option<ObjectId>,
    committer: &gix::actor::Signature,
    state: CommitState,
    signing: Option<gix::objs::commit::signature::Options>,
) -> Result<(ObjectId, Option<Duration>)> {
    if let Some(predecessor) = predecessor {
        crate::change_id::inherit(repo, &mut commit, predecessor)?;
    }
    commit.committer = committer.clone();
    let signature = match state {
        CommitState::Unmarked(signature) => {
            marker(&mut commit, false, None);
            signature
        }
        CommitState::Pending { original_parent } => {
            marker(&mut commit, true, original_parent);
            Signature::InvalidateExisting
        }
    };
    let had_signature = commit.extra_headers.iter().any(|(name, _)| is_signature(name));
    commit.extra_headers.retain(|(name, _)| !is_signature(name));
    let mut signing_time = None;
    commit = match (signature, signing) {
        (Signature::RedoIfNeeded, Some(options)) => {
            let started = Instant::now();
            let signed = commit.sign(options).context("could not sign rebased commit")?;
            signing_time = Some(started.elapsed());
            signed
        }
        (Signature::InvalidateExisting, Some(_)) if had_signature => {
            let field = gix::objs::commit::signature_field_name(commit.tree.kind());
            commit.extra_headers.push((field.into(), BString::default()));
            commit
        }
        (Signature::Remove, _) => commit,
        _ => commit,
    };
    Ok((
        repo.write_object(&commit)
            .context("could not prepare rebased commit")?
            .detach(),
        signing_time,
    ))
}

fn is_signature(name: &BString) -> bool {
    name.as_slice() == gix::objs::commit::SIGNATURE_FIELD_NAME.as_bytes()
        || name.as_slice() == gix::objs::commit::SIGNATURE_FIELD_NAME_SHA256.as_bytes()
}

struct Transition {
    repo: gix::Repository,
    workdir: PathBuf,
    old: ObjectId,
    new: ObjectId,
}

fn worktree_transitions(
    repo: &gix::Repository,
    rewritten: &HashMap<ObjectId, Option<ObjectId>>,
    expected_refs: Option<&[ExpectedRef]>,
    inserted: bool,
) -> Result<Vec<Transition>> {
    if inserted {
        return Ok(Vec::new());
    }
    let mut repos = vec![
        repo.main_repo()
            .context("could not open the main worktree repository")?,
    ];
    for proxy in repo.worktrees().context("could not enumerate linked worktrees")? {
        if let Ok(worktree_repo) = proxy.into_repo_with_possibly_inaccessible_worktree() {
            repos.push(worktree_repo);
        }
    }
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for worktree_repo in repos {
        if !seen.insert(worktree_repo.git_dir().to_owned()) {
            continue;
        }
        let Ok(head) = worktree_repo.head() else {
            continue;
        };
        let Some(old) = head.id().map(gix::Id::detach) else {
            continue;
        };
        let planned = head.referent_name().and_then(|name| {
            expected_refs.and_then(|refs| refs.iter().find(|expected| expected.name.as_bstr() == name.as_bstr()))
        });
        let new = match planned {
            Some(expected) if expected.new.is_none() => {
                if worktree_repo.git_dir() == repo.git_dir() {
                    continue;
                }
                anyhow::bail!(
                    "cannot delete {} because another worktree has it checked out",
                    expected.name.shorten()
                );
            }
            Some(expected) => Some(expected.new),
            None => rewritten.get(&old).copied(),
        };
        let Some(new) = new else { continue };
        if worktree_repo.workdir().is_none() && worktree_repo.is_bare() {
            continue;
        }
        let old_tree = worktree_repo.find_commit(old)?.tree_id()?.detach();
        let new_tree = match new {
            Some(new) => repo.find_commit(new)?.tree_id()?.detach(),
            None if worktree_repo.head()?.referent_name().is_some() => repo.empty_tree().id,
            None => anyhow::bail!("a detached checked-out root commit cannot be removed"),
        };
        if old_tree == new_tree {
            continue;
        }
        let workdir = worktree_repo
            .workdir()
            .filter(|path| path.is_dir())
            .context("an affected worktree is inaccessible")?
            .to_owned();
        out.push(Transition {
            repo: worktree_repo,
            workdir,
            old: old_tree,
            new: new_tree,
        });
    }
    Ok(out)
}

#[expect(clippy::too_many_arguments)]
fn update_refs(
    repo: &gix::Repository,
    rewritten: &HashMap<ObjectId, Option<ObjectId>>,
    unborn: bool,
    inserted: Option<ObjectId>,
    committer: &gix::actor::Signature,
    expected_refs: Option<Vec<ExpectedRef>>,
    resources: (&[ObjectId], &[(gix::refs::FullName, Target)]),
    stash_edits: super::stash::RewriteEdits,
) -> Result<Vec<RefEdit>> {
    let (pins, delete_refs) = resources;
    let mut edits = stash_edits.forward;
    let mut rollback = stash_edits.rollback;
    if let Some(expected_refs) = expected_refs {
        for ExpectedRef { name, old, new, .. } in expected_refs {
            if delete_refs.iter().any(|(delete, _)| delete == &name) {
                continue;
            }
            if old == new {
                continue;
            }
            edits.push(ref_edit(name.clone(), old, new));
            rollback.push(ref_edit(name, new, old));
        }
    } else {
        for reference in repo.references()?.all()? {
            let reference = match reference {
                Ok(reference) => reference,
                Err(err) if is_missing_ref(&*err) => continue,
                Err(err) => anyhow::bail!("could not inspect a reference before rebasing: {err}"),
            };
            if matches!(
                reference.name().category(),
                Some(Category::Tag | Category::RemoteBranch)
            ) {
                continue;
            }
            let Some(old) = reference.try_id().map(gix::Id::detach) else {
                continue;
            };
            let Some(new) = rewritten.get(&old) else { continue };
            let name = reference.name().to_owned();
            if delete_refs.iter().any(|(delete, _)| delete == &name) {
                continue;
            }
            edits.push(ref_edit(name.clone(), Some(old), *new));
            rollback.push(ref_edit(name, *new, Some(old)));
        }
        if let Some(head) = repo.try_find_reference("HEAD")?
            && let Some(old) = head.try_id().map(gix::Id::detach)
            && let Some(new) = rewritten.get(&old)
        {
            let name = head.name().to_owned();
            edits.push(ref_edit(name.clone(), Some(old), *new));
            rollback.push(ref_edit(name, *new, Some(old)));
        }
    }
    if unborn {
        let name = repo
            .head()?
            .referent_name()
            .context("an unborn HEAD must point to a branch")?
            .to_owned();
        let new = inserted.context("an unborn insertion must create a commit")?;
        edits.push(RefEdit {
            name: name.clone(),
            deref: false,
            change: Change::Update {
                log: log_change(),
                expected: PreviousValue::MustNotExist,
                new: Target::Object(new),
            },
        });
        rollback.push(RefEdit {
            name,
            deref: false,
            change: Change::Delete {
                expected: PreviousValue::MustExistAndMatch(Target::Object(new)),
                log: RefLog::AndReference,
            },
        });
    }
    let mut reserved = HashSet::new();
    for id in pins {
        let name = pin_name(repo, *id, &reserved)?;
        reserved.insert(name.clone());
        edits.push(RefEdit {
            name: name.clone(),
            deref: false,
            change: Change::Update {
                log: log_change(),
                expected: PreviousValue::MustNotExist,
                new: Target::Object(*id),
            },
        });
        rollback.push(RefEdit {
            name,
            deref: false,
            change: Change::Delete {
                expected: PreviousValue::MustExistAndMatch(Target::Object(*id)),
                log: RefLog::AndReference,
            },
        });
    }
    for (name, target) in delete_refs {
        edits.push(RefEdit {
            name: name.clone(),
            deref: false,
            change: Change::Delete {
                expected: PreviousValue::MustExistAndMatch(target.clone()),
                log: RefLog::AndReference,
            },
        });
        rollback.push(RefEdit {
            name: name.clone(),
            deref: false,
            change: Change::Update {
                log: log_change(),
                expected: PreviousValue::MustNotExist,
                new: target.clone(),
            },
        });
    }
    if edits.is_empty() {
        return Ok(Vec::new());
    }
    let mut time = gix::date::parse::TimeBuf::default();
    repo.edit_references_as(edits, Some(committer.to_ref(&mut time)))
        .context("could not update references after rebasing")?;
    Ok(rollback)
}

fn pin_name(
    repo: &gix::Repository,
    id: ObjectId,
    reserved: &HashSet<gix::refs::FullName>,
) -> Result<gix::refs::FullName> {
    let hex = id.to_hex().to_string();
    let mut len = 8.min(hex.len());
    let mut number = 2;
    loop {
        let suffix = if len <= hex.len() {
            hex[..len].to_owned()
        } else {
            let suffix = format!("{hex}{number}");
            number += 1;
            suffix
        };
        let name: gix::refs::FullName = format!("{}{}", String::from_utf8_lossy(crate::history::PIN_PREFIX), suffix)
            .try_into()
            .context("generated an invalid tix pin name")?;
        if !reserved.contains(&name) && repo.try_find_reference(name.as_ref())?.is_none() {
            return Ok(name);
        }
        if len < hex.len() {
            len += 1;
        } else {
            len = hex.len() + 1;
        }
    }
}

fn is_missing_ref(mut err: &(dyn std::error::Error + 'static)) -> bool {
    loop {
        if err
            .downcast_ref::<std::io::Error>()
            .is_some_and(|err| err.kind() == std::io::ErrorKind::NotFound)
        {
            return true;
        }
        let Some(source) = err.source() else { return false };
        err = source;
    }
}

fn ref_edit(name: gix::refs::FullName, old: Option<ObjectId>, new: Option<ObjectId>) -> RefEdit {
    RefEdit {
        name,
        deref: false,
        change: match (old, new) {
            (Some(old), Some(new)) => Change::Update {
                log: log_change(),
                expected: PreviousValue::MustExistAndMatch(Target::Object(old)),
                new: Target::Object(new),
            },
            (Some(old), None) => Change::Delete {
                expected: PreviousValue::MustExistAndMatch(Target::Object(old)),
                log: RefLog::AndReference,
            },
            (None, Some(new)) => Change::Update {
                log: log_change(),
                expected: PreviousValue::MustNotExist,
                new: Target::Object(new),
            },
            (None, None) => unreachable!("unchanged absent refs are filtered before editing"),
        },
    }
}

fn log_change() -> LogChange {
    LogChange {
        mode: RefLog::AndReference,
        force_create_reflog: false,
        message: BString::from("tix rebase"),
    }
}

#[cfg(test)]
mod tests {
    use std::{path::Path, process::Command};

    use gix::bstr::ByteSlice;

    use super::*;

    fn open(path: &Path) -> gix_testtools::Result<gix::Repository> {
        Ok(crate::test_repository::open_with(
            path,
            ["user.name=rebasing committer", "user.email=rebasing@example.com"],
        )?)
    }

    #[test]
    fn rewords_a_middle_commit_and_reparents_all_linear_descendants() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let middle = repo.rev_parse_single("HEAD~1")?.detach();
        let old_tip = repo.head_id()?.detach();
        let old_tip_tree = repo.find_commit(old_tip)?.tree_id()?.detach();
        let mut commit = repo.find_commit(middle)?.decode()?.into_owned()?;
        commit.message = "rewritten middle".into();

        let outcome = perform(
            &repo,
            &graph,
            Edit::Replace { target: middle, commit },
            Signature::RedoIfNeeded,
            Tree::LeaveAsIs,
        )?
        .complete()?;
        let new_middle = outcome.selected.expect("replacement selects the rewritten commit");
        let new_tip = repo.head_id()?.detach();
        assert_ne!(new_tip, old_tip, "the descendant is rewritten");
        assert_eq!(
            repo.find_commit(new_tip)?.parent_ids().next().map(gix::Id::detach),
            Some(new_middle),
            "the descendant follows the replacement"
        );
        assert_eq!(
            repo.find_commit(new_tip)?.tree_id()?.detach(),
            old_tip_tree,
            "a reword preserves descendant trees"
        );
        for (id, predecessor) in [(new_middle, middle), (new_tip, old_tip)] {
            let commit = repo.find_commit(id)?.decode()?.into_owned()?;
            assert_eq!(commit.committer.name, b"rebasing committer".as_bstr());
            assert_eq!(commit.committer.email, b"rebasing@example.com".as_bstr());
            assert_eq!(
                commit.committer.time.seconds, 978_307_200,
                "every rewritten commit receives the operation's current committer date"
            );
            assert_eq!(
                crate::change_id::effective(
                    id,
                    commit
                        .extra_headers
                        .iter()
                        .filter_map(|(name, value)| (name == crate::change_id::HEADER).then_some(value.as_ref()))
                ),
                predecessor.into(),
                "each rewritten commit retains the identity of its predecessor"
            );
        }
        insta::assert_snapshot!(
            "reworded-middle-stack",
            gix_testtools::repository::snapshot(fixture.path())?.to_string()
        );
        Ok(())
    }

    #[test]
    fn rewriting_a_stashed_commit_moves_its_stash_association() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let middle = repo.rev_parse_single("HEAD~1")?.detach();
        let saved = repo.write_blob(b"saved worktree state")?.detach();
        let old_name = super::super::stash::reference(middle)?;
        repo.reference(old_name.clone(), saved, PreviousValue::MustNotExist, "test stash")?;
        let mut commit = repo.find_commit(middle)?.decode()?.into_owned()?;
        commit.message = "rewritten middle".into();

        let outcome = perform(
            &repo,
            &graph,
            Edit::Replace { target: middle, commit },
            Signature::RedoIfNeeded,
            Tree::LeaveAsIs,
        )?
        .complete()?;
        let rewritten = outcome.map(middle).expect("the stashed commit is retained");
        let new_name = super::super::stash::reference(rewritten)?;
        assert!(repo.try_find_reference(old_name.as_ref())?.is_none());
        assert_eq!(
            repo.find_reference(new_name.as_ref())?.id().detach(),
            saved,
            "the saved state follows the rewritten commit without changing its target"
        );
        Ok(())
    }

    #[test]
    fn removes_a_middle_commit_by_cherry_picking_its_descendant() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let base = repo.rev_parse_single("HEAD~2")?.detach();
        let middle = repo.rev_parse_single("HEAD~1")?.detach();
        let old_tip_tree = repo.find_commit(repo.head_id()?)?.tree_id()?.detach();

        let outcome = perform(
            &repo,
            &graph,
            Edit::Remove { target: middle },
            Signature::RedoIfNeeded,
            Tree::CherryPick,
        )?
        .complete()?;
        assert_eq!(outcome.selected, Some(base), "removal selects its parent");
        let tip = repo.head_id()?.detach();
        assert_eq!(
            repo.find_commit(tip)?.parent_ids().next().map(gix::Id::detach),
            Some(base),
            "the descendant is transplanted onto the removed commit's parent"
        );
        assert_ne!(
            repo.find_commit(tip)?.tree_id()?.detach(),
            old_tip_tree,
            "the removed commit's tree contribution is absent"
        );
        Ok(())
    }

    #[test]
    fn a_marked_rebase_can_be_repeated_and_clears_its_markers() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let middle = repo.rev_parse_single("HEAD~1")?.detach();
        let mut commit = repo.find_commit(middle)?.decode()?.into_owned()?;
        commit.tree = repo.find_commit(repo.rev_parse_single("HEAD~2")?)?.tree_id()?.detach();
        perform(
            &repo,
            &graph,
            Edit::Replace { target: middle, commit },
            Signature::InvalidateExisting,
            Tree::LeaveAsIsAndMark,
        )?
        .complete()?
        .selected
        .expect("marking rewrites the selected commit");

        let graph = super::super::loaded_graph(&repo)?;
        let tip = repo.head_id()?.detach();
        perform(
            &repo,
            &graph,
            Edit::Repeat {
                base: tip,
                checkout: tip,
            },
            Signature::RedoIfNeeded,
            Tree::CherryPick,
        )?
        .complete()?;
        let mut id = Some(repo.head_id()?.detach());
        while let Some(current) = id {
            let commit = repo.find_commit(current)?.decode()?.into_owned()?;
            assert!(!has_marker(&commit), "repeating clears every pending marker");
            id = commit.parents.first().copied();
        }
        let files = Command::new("git")
            .arg("-C")
            .arg(fixture.path())
            .args(["ls-tree", "-r", "--name-only", "HEAD"])
            .output()?;
        assert!(files.status.success());
        assert_eq!(
            files.stdout, b"base\ntip\n",
            "repeat cherry-picks the descendant against its recorded original parent"
        );
        Ok(())
    }

    #[test]
    fn a_repeat_accepts_ordinary_descendants_above_legacy_signed_pending_commits() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["commit", "--allow-empty", "-q", "-m", "newer"])
                .status()?
                .success(),
            "the fixture gains a second descendant"
        );
        let repo = open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let middle = repo.rev_parse_single("HEAD~2")?.detach();
        let old_tip = repo.rev_parse_single("HEAD~1")?.detach();
        let old_newer = repo.head_id()?.detach();
        let mut commit = repo.find_commit(middle)?.decode()?.into_owned()?;
        commit.tree = repo.find_commit(repo.rev_parse_single("HEAD~3")?)?.tree_id()?.detach();
        let marked = perform(
            &repo,
            &graph,
            Edit::Replace { target: middle, commit },
            Signature::InvalidateExisting,
            Tree::LeaveAsIsAndMark,
        )?
        .complete()?;
        let pending_tip = marked.map(old_tip).expect("the first descendant remains");
        let pending_newer = marked.map(old_newer).expect("the second descendant remains");

        let mut legacy_tip = repo.find_commit(pending_tip)?.decode()?.into_owned()?;
        legacy_tip
            .extra_headers
            .push(("gpgsig".into(), "legacy signature".into()));
        let legacy_tip = repo.write_object(&legacy_tip)?.detach();
        let mut legacy_newer = repo.find_commit(pending_newer)?.decode()?.into_owned()?;
        legacy_newer.parents = [legacy_tip].into_iter().collect();
        legacy_newer
            .extra_headers
            .push(("gpgsig".into(), "legacy signature".into()));
        let legacy_newer = repo.write_object(&legacy_newer)?.detach();
        repo.find_reference("refs/heads/main")?
            .set_target_id(legacy_newer, "test legacy pending commits")?;
        drop(repo);

        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["commit", "--allow-empty", "-q", "-m", "ordinary descendant"])
                .status()?
                .success(),
            "an ordinary commit can be created above pending history"
        );
        let repo = open(fixture.path())?;
        let ordinary = repo.head_id()?.detach();
        let graph = super::super::loaded_graph(&repo)?;
        let outcome = perform(
            &repo,
            &graph,
            Edit::Repeat {
                base: legacy_tip,
                checkout: legacy_tip,
            },
            Signature::RedoIfNeeded,
            Tree::CherryPick,
        )?
        .complete()?;
        let selected = outcome
            .map(legacy_tip)
            .expect("the selected pending commit is retained");
        assert!(!has_marker(&repo.find_commit(selected)?.decode()?.into_owned()?));
        for id in [legacy_newer, ordinary].into_iter().map(|id| {
            outcome
                .map(id)
                .expect("every descendant is retained while repeating the rebase")
        }) {
            let commit = repo.find_commit(id)?.decode()?.into_owned()?;
            assert!(has_marker(&commit), "later descendants become or remain pending");
            assert!(
                commit
                    .extra_headers
                    .iter()
                    .filter(|(name, _)| name == "gpgsig" || name == "gpgsig-sha256")
                    .all(|(_, value)| value.is_empty()),
                "later pending descendants have no usable signature"
            );
        }
        Ok(())
    }

    #[test]
    fn authoritative_replacements_do_not_record_an_original_parent() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let root = repo.rev_parse_single("HEAD~2")?.detach();
        let commit = repo.find_commit(root)?.decode()?.into_owned()?;
        let marked = perform(
            &repo,
            &graph,
            Edit::Replace { target: root, commit },
            Signature::InvalidateExisting,
            Tree::LeaveAsIsAndMark,
        )?
        .complete()?
        .selected
        .expect("the replacement selects its rewritten root");
        let commit = repo.find_commit(marked)?.decode()?.into_owned()?;
        assert!(
            !has_marker(&commit),
            "the replacement tree and unchanged parent need no later cherry-pick"
        );
        Ok(())
    }

    #[test]
    fn rewrites_every_fork_without_flattening_it() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = open(fixture.path())?;
        let middle = repo.rev_parse_single("HEAD~1")?.detach();
        let tree = repo.find_commit(middle)?.tree_id()?.detach();
        let tree_hex = tree.to_string();
        let middle_hex = middle.to_string();
        let side = Command::new("git")
            .arg("-C")
            .arg(fixture.path())
            .args([
                "-c",
                "commit.gpgSign=false",
                "commit-tree",
                &tree_hex,
                "-p",
                &middle_hex,
                "-m",
                "side",
            ])
            .env("GIT_AUTHOR_DATE", "2000-01-04T00:00:00 +0000")
            .env("GIT_COMMITTER_DATE", "2000-01-04T00:00:00 +0000")
            .output()?;
        assert!(side.status.success(), "the side commit fixture is created");
        let side = ObjectId::from_hex(side.stdout.trim())?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["update-ref", "refs/heads/side", &side.to_string()])
                .status()?
                .success(),
            "the side tip is made visible to history loading"
        );
        let graph = super::super::loaded_graph(&repo)?;
        let mut commit = repo.find_commit(middle)?.decode()?.into_owned()?;
        commit.message = "fork point".into();
        let rewritten_middle = perform(
            &repo,
            &graph,
            Edit::Replace { target: middle, commit },
            Signature::RedoIfNeeded,
            Tree::LeaveAsIs,
        )?
        .complete()?
        .selected
        .expect("the fork point is rewritten");

        let main = repo.head_id()?.detach();
        let side = repo.find_reference("refs/heads/side")?.id().detach();
        assert_ne!(main, side, "the two descendant lines remain distinct");
        for (name, tip) in [("main", main), ("side", side)] {
            assert_eq!(
                repo.find_commit(tip)?.parent_ids().next().map(gix::Id::detach),
                Some(rewritten_middle),
                "the {name} fork follows the rewritten fork point"
            );
        }
        Ok(())
    }

    #[test]
    fn a_conflicting_cherry_pick_leaves_repository_state_unmodified() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_conflict.sh")?;
        let repo = open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let middle = repo.rev_parse_single("HEAD~1")?.detach();
        let before = gix_testtools::repository::snapshot(fixture.path())?;
        let objects_before = Command::new("git")
            .arg("-C")
            .arg(fixture.path())
            .args(["count-objects", "-v"])
            .output()?
            .stdout;

        let Perform::Conflict(conflict) = perform(
            &repo,
            &graph,
            Edit::Remove { target: middle },
            Signature::RedoIfNeeded,
            Tree::CherryPick,
        )?
        else {
            return Err("the unresolved cherry-pick should suspend the complete rebase".into());
        };
        drop(conflict);
        assert_eq!(
            gix_testtools::repository::snapshot(fixture.path())?,
            before,
            "refs, index, and worktree remain unchanged"
        );
        assert_eq!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["count-objects", "-v"])
                .output()?
                .stdout,
            objects_before,
            "failed in-memory rebases write no objects"
        );
        Ok(())
    }

    #[test]
    fn a_conflicting_todo_aborts_at_the_original_commit_without_observable_changes() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_conflict.sh")?;
        let repo = open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let base = repo.rev_parse_single("HEAD~2")?.detach();
        let middle = repo.rev_parse_single("HEAD~1")?.detach();
        let tip = repo.head_id()?.detach();
        let before = gix_testtools::repository::snapshot(fixture.path())?;
        let objects_before = Command::new("git")
            .arg("-C")
            .arg(fixture.path())
            .args(["count-objects", "-v"])
            .output()?
            .stdout;

        let PlanPerform::Conflict(conflict) = perform_plan(
            &repo,
            &graph,
            Plan {
                base,
                scope: vec![middle, tip],
                steps: vec![PlanStep {
                    parent: PlanParent::Existing(base),
                    commit: PlanCommit::Pick(tip),
                    squash: Vec::new(),
                }],
                checkout: Some(PlanCheckout {
                    target: PlanParent::Step(0),
                    reference: None,
                }),
                expected_refs: capture_refs(&repo, &[middle, tip], &[tip])?,
            },
        )?
        else {
            return Err("the conflicting todo should abort before persistence".into());
        };
        assert_eq!(conflict.original(), tip, "the diagnostic identifies the source delta");
        assert_eq!(
            gix_testtools::repository::snapshot(fixture.path())?,
            before,
            "refs, index, checkout, and worktree remain unchanged"
        );
        assert_eq!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["count-objects", "-v"])
                .output()?
                .stdout,
            objects_before,
            "the discarded object-memory transaction writes no objects"
        );
        Ok(())
    }

    #[test]
    fn a_todo_rebase_eagerly_replays_only_the_checkout_ancestry_and_keeps_the_branch_at_the_leaf()
    -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let base = repo.rev_parse_single("HEAD~2")?.detach();
        let middle = repo.rev_parse_single("HEAD~1")?.detach();
        let tip = repo.head_id()?.detach();
        let old_middle_tree = repo.find_commit(middle)?.tree_id()?.detach();
        let expected_refs = capture_refs(&repo, &[middle, tip], &[tip])?;

        let mut progress = Vec::new();
        let outcome = perform_plan_with_progress(
            &repo,
            &graph,
            Plan {
                base,
                scope: vec![middle, tip],
                steps: vec![
                    PlanStep {
                        parent: PlanParent::Existing(base),
                        commit: PlanCommit::Pick(tip),
                        squash: Vec::new(),
                    },
                    PlanStep {
                        parent: PlanParent::Step(0),
                        commit: PlanCommit::Pick(middle),
                        squash: Vec::new(),
                    },
                    PlanStep {
                        parent: PlanParent::Step(1),
                        commit: PlanCommit::Empty("checkpoint".into()),
                        squash: Vec::new(),
                    },
                ],
                checkout: Some(PlanCheckout {
                    target: PlanParent::Step(0),
                    reference: None,
                }),
                expected_refs,
            },
            |update| progress.push(update),
        )?
        .complete()?;
        let progress = progress.last().context("rebase progress is reported")?;
        assert_eq!(progress.total, 3, "every todo command contributes to the total");
        assert_eq!(progress.processed, 3, "eager, lazy, and empty commands are processed");
        assert_eq!(progress.cherry_picked, 1, "only the checkout ancestry is eager");
        assert_eq!(progress.signed, 0, "signing is counted only when configured");
        assert!(
            progress.cherry_pick_time > Duration::ZERO,
            "cherry-pick time accumulates"
        );
        let new_tip = outcome.map(tip).context("the picked tip is retained")?;
        let new_middle = outcome.map(middle).context("the picked middle is retained")?;
        assert_eq!(
            outcome.selected,
            Some(new_tip),
            "the checkout still follows its picked commit"
        );
        let tip_commit = repo.find_commit(new_tip)?.decode()?.into_owned()?;
        assert!(!has_marker(&tip_commit), "the checkout ancestry is eagerly replayed");
        assert_eq!(tip_commit.parents.first().copied(), Some(base));
        let middle_commit = repo.find_commit(new_middle)?.decode()?.into_owned()?;
        assert!(
            has_marker(&middle_commit),
            "history outside the checkout ancestry stays lazy"
        );
        assert_eq!(
            middle_commit.tree, old_middle_tree,
            "a lazy rewrite keeps its original tree"
        );
        assert_eq!(middle_commit.parents.first().copied(), Some(new_tip));

        let empty_id = repo.head_id()?.detach();
        let empty = repo.find_commit(empty_id)?.decode()?.into_owned()?;
        assert_eq!(
            empty.message, b"checkpoint",
            "the complete empty-commit title becomes its message"
        );
        assert!(has_marker(&empty), "an empty commit outside @ remains lazy");
        assert_eq!(empty.parents.first().copied(), Some(new_middle));
        assert_eq!(
            empty.tree, middle_commit.tree,
            "the empty commit reuses its parent tree"
        );
        assert!(
            !repo.head()?.is_detached(),
            "the branch remains checked out after rebasing"
        );
        assert!(
            crate::history::all_pins(&repo)?.is_empty(),
            "the referenced leaf needs no pin"
        );
        let repository_path = repo.git_dir().to_owned();
        drop(repo);
        super::super::time_travel::checkout_without_replay(&repository_path, false, new_tip, &[], false)?;
        let repo = open(fixture.path())?;
        assert!(
            repo.head()?.is_detached(),
            "moving @ below the branch tip detaches HEAD"
        );
        assert_eq!(repo.head_id()?.detach(), new_tip, "HEAD follows the checkout marker");
        assert_eq!(
            repo.find_reference("refs/heads/main")?.id().detach(),
            empty_id,
            "the branch remains at the resulting leaf"
        );
        let pins = crate::history::all_pins(&repo)?;
        assert_eq!(pins.len(), 1, "the detached view retains the branch through a pin");
        assert_eq!(
            pins[0].target.try_name().map(gix::refs::FullNameRef::as_bstr),
            Some(b"refs/heads/main".as_bstr()),
            "the visibility pin follows the branch symbolically"
        );
        Ok(())
    }

    #[test]
    fn a_pending_checkout_rewrites_only_from_the_first_pending_commit() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = open(fixture.path())?;
        let base = repo.rev_parse_single("HEAD~2")?.detach();
        let clean = repo.rev_parse_single("HEAD~1")?.detach();
        let old_tip = repo.head_id()?.detach();

        let mut checkout = repo.find_commit(old_tip)?.decode()?.into_owned()?;
        checkout.extra_headers.retain(|(name, _)| !is_signature(name));
        checkout.extra_headers.push((
            gix::objs::commit::signature_field_name(checkout.tree.kind()).into(),
            BString::default(),
        ));
        let checkout = repo.write_object(&checkout)?.detach();

        let mut descendant = repo.find_commit(checkout)?.decode()?.into_owned()?;
        descendant.parents = [checkout].into_iter().collect();
        descendant.message = "pending descendant".into();
        descendant
            .extra_headers
            .push((ORIGINAL_PARENT.into(), checkout.to_hex().to_string().into()));
        let descendant = repo.write_object(&descendant)?.detach();
        repo.reference(
            "refs/heads/main",
            descendant,
            PreviousValue::MustExistAndMatch(Target::Object(old_tip)),
            "test pending tip",
        )?;

        let graph = super::super::loaded_graph(&repo)?;
        let mut progress = Vec::new();
        let outcome = perform_plan_with_progress(
            &repo,
            &graph,
            Plan {
                base,
                scope: vec![clean, checkout, descendant],
                steps: vec![
                    PlanStep {
                        parent: PlanParent::Existing(base),
                        commit: PlanCommit::Pick(clean),
                        squash: Vec::new(),
                    },
                    PlanStep {
                        parent: PlanParent::Step(0),
                        commit: PlanCommit::Pick(checkout),
                        squash: Vec::new(),
                    },
                    PlanStep {
                        parent: PlanParent::Step(1),
                        commit: PlanCommit::Pick(descendant),
                        squash: Vec::new(),
                    },
                ],
                checkout: Some(PlanCheckout {
                    target: PlanParent::Step(1),
                    reference: None,
                }),
                expected_refs: capture_refs(&repo, &[clean, checkout, descendant], &[descendant])?,
            },
            |update| progress.push(update),
        )?
        .complete()?;

        assert_eq!(
            outcome.map(clean),
            Some(clean),
            "the clean prefix retains its commit ID"
        );
        let rewritten_checkout = outcome.map(checkout).context("the pending checkout is retained")?;
        assert_ne!(rewritten_checkout, checkout, "the empty signature is materialized");
        assert_eq!(outcome.selected, Some(rewritten_checkout));
        assert!(
            !is_pending(&repo.find_commit(rewritten_checkout)?.decode()?.into_owned()?),
            "the selected commit is no longer pending"
        );
        let rewritten_descendant = outcome.map(descendant).context("the descendant is retained")?;
        let descendant = repo.find_commit(rewritten_descendant)?.decode()?.into_owned()?;
        assert!(has_marker(&descendant), "history above the checkout remains lazy");
        assert_eq!(descendant.parents.as_slice(), [rewritten_checkout]);
        assert_eq!(
            repo.find_reference("refs/heads/main")?.id().detach(),
            rewritten_descendant,
            "the branch follows the lazily rewritten tip"
        );
        let progress = progress.last().context("rebase progress is reported")?;
        assert_eq!(progress.processed, 3);
        assert_eq!(
            progress.cherry_picked, 1,
            "only the pending checkout is replayed eagerly"
        );
        Ok(())
    }

    #[test]
    fn a_todo_squash_materializes_one_commit_and_maps_every_source_to_it() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let base = repo.rev_parse_single("HEAD~2")?.detach();
        let middle = repo.rev_parse_single("HEAD~1")?.detach();
        let tip = repo.head_id()?.detach();
        let tip_tree = repo.find_commit(tip)?.tree_id()?.detach();

        let mut progress = Vec::new();
        let outcome = perform_plan_with_progress(
            &repo,
            &graph,
            Plan {
                base,
                scope: vec![middle, tip],
                steps: vec![PlanStep {
                    parent: PlanParent::Existing(base),
                    commit: PlanCommit::Pick(middle),
                    squash: vec![tip],
                }],
                checkout: None,
                expected_refs: capture_refs(&repo, &[middle, tip], &[tip])?,
            },
            |update| progress.push(update),
        )?
        .complete()?;
        let progress = progress.last().context("squash progress is reported")?;
        assert_eq!(progress.total, 2, "both squash source commits contribute to the total");
        assert_eq!(progress.processed, 2);
        assert_eq!(progress.cherry_picked, 2, "both source trees are replayed eagerly");
        assert_eq!(progress.signed, 0, "the fixture has no signer configured");
        let combined = outcome.map(middle).context("the first commit is retained")?;
        assert_eq!(outcome.map(tip), Some(combined), "both source IDs map to one commit");
        let commit = repo.find_commit(combined)?.decode()?.into_owned()?;
        assert_eq!(commit.parents.as_slice(), [base]);
        assert_eq!(commit.tree, tip_tree, "all source tree changes are present");
        assert_eq!(commit.author.name, b"author".as_bstr());
        assert_eq!(commit.committer.name, b"rebasing committer".as_bstr());
        assert_eq!(
            crate::change_id::effective(combined, commit.extra_headers().find_all(crate::change_id::HEADER)),
            middle.into(),
            "the primary picked commit supplies the squash identity"
        );
        assert!(!has_marker(&commit), "squash is eager even without a checkout marker");
        assert_eq!(
            commit.message,
            format!("middle\n\n# {} tip\n\ntip\n", tip.to_hex_with_len(7)).as_bytes(),
            "the source title permanently identifies the appended full message"
        );
        assert_eq!(
            repo.head_id()?.detach(),
            combined,
            "the branch follows the combined tip"
        );
        assert_eq!(
            repo.find_reference("refs/patches/middle")?.id().detach(),
            combined,
            "a ref on the first source follows the combined commit"
        );
        Ok(())
    }

    #[test]
    fn a_squash_group_reports_one_configured_signature() -> gix_testtools::Result {
        if !gix_testtools::signature::program_available("ssh-keygen") {
            return Ok(());
        }
        let (_key_home, key) = gix_testtools::signature::ssh_private_key()?;
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open_with(
            fixture.path(),
            [
                "user.name=rebasing committer".to_owned(),
                "user.email=rebasing@example.com".to_owned(),
                "gitoxide.commit.committerDate=2001-01-01T00:00:00 +0000".to_owned(),
                "commit.gpgSign=true".to_owned(),
                "gpg.format=ssh".to_owned(),
                format!("user.signingKey={}", key.display()),
                format!(
                    "gpg.ssh.allowedSignersFile={}",
                    gix_testtools::signature::fixture("ssh-allowed-signers").display()
                ),
            ],
        )?;
        let graph = super::super::loaded_graph(&repo)?;
        let base = repo.rev_parse_single("HEAD~2")?.detach();
        let middle = repo.rev_parse_single("HEAD~1")?.detach();
        let tip = repo.head_id()?.detach();
        let mut updates = Vec::new();

        let outcome = perform_plan_with_progress(
            &repo,
            &graph,
            Plan {
                base,
                scope: vec![middle, tip],
                steps: vec![PlanStep {
                    parent: PlanParent::Existing(base),
                    commit: PlanCommit::Pick(middle),
                    squash: vec![tip],
                }],
                checkout: None,
                expected_refs: capture_refs(&repo, &[middle, tip], &[tip])?,
            },
            |progress| updates.push(progress),
        )?
        .complete()?;
        let progress = updates.last().context("signed progress is reported")?;
        assert_eq!(progress.total, 2);
        assert_eq!(progress.processed, 2);
        assert_eq!(progress.cherry_picked, 2);
        assert_eq!(progress.signed, 1, "the combined result is signed only once");
        assert!(progress.signing_time > Duration::ZERO, "signing time accumulates");
        assert!(
            repo.find_commit(outcome.map(middle).context("the squash result is retained")?)?
                .verify_signature()?
                .expect("the squash result is signed")
                .is_valid(),
            "the reported signature is valid"
        );
        Ok(())
    }

    #[test]
    fn squash_messages_add_distinct_raw_authors_after_permanent_source_sections() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = open(fixture.path())?;
        let base = repo.rev_parse_single("HEAD~2")?.detach();
        let middle = repo.rev_parse_single("HEAD~1")?.detach();
        let tip = repo.head_id()?.detach();
        let mut first = repo.find_commit(base)?.decode()?.into_owned()?;
        first.author.name = "Alice".into();
        first.author.email = "alice@example.com".into();
        first.message = "first title\n\nfirst body".into();

        let mut bob = repo.find_commit(base)?.decode()?.into_owned()?;
        bob.author.name = "Bob".into();
        bob.author.email = "bob@example.com".into();
        bob.message = "bob title\n\nbob body".into();
        let mut carol = repo.find_commit(middle)?.decode()?.into_owned()?;
        carol.author.name = "Carol".into();
        carol.author.email = "carol@example.com".into();
        carol.message = "carol title\n\nCo-authored-by: Bob <bob@example.com>".into();
        let mut repeated_carol = repo.find_commit(tip)?.decode()?.into_owned()?;
        repeated_carol.author = carol.author.clone();
        repeated_carol.message = "carol follow-up".into();

        squash_message(
            &repo,
            &mut first,
            &[
                (base, bob, Vec::new()),
                (middle, carol, Vec::new()),
                (tip, repeated_carol, Vec::new()),
            ],
        )?;
        assert_eq!(
            first.message,
            format!(
                "first title\n\nfirst body\n\n# {} bob title\n\nbob title\n\nbob body\n\n# {} carol title\n\ncarol title\n\nCo-authored-by: Bob <bob@example.com>\n\n# {} carol follow-up\n\ncarol follow-up\n\nCo-authored-by: Carol <carol@example.com>\n",
                base.to_hex_with_len(7),
                middle.to_hex_with_len(7),
                tip.to_hex_with_len(7)
            )
            .as_bytes(),
            "existing trailers suppress duplicates and repeated raw authors appear once"
        );
        Ok(())
    }

    #[test]
    fn tip_refs_follow_the_primary_continuation_and_other_leaves_are_pinned() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let base = repo.rev_parse_single("HEAD~2")?.detach();
        let middle = repo.rev_parse_single("HEAD~1")?.detach();
        let tip = repo.head_id()?.detach();
        repo.reference(
            "refs/custom/tip",
            tip,
            PreviousValue::MustNotExist,
            "test custom tip reference",
        )?;

        let outcome = perform_plan(
            &repo,
            &graph,
            Plan {
                base,
                scope: vec![middle, tip],
                steps: vec![
                    PlanStep {
                        parent: PlanParent::Existing(base),
                        commit: PlanCommit::Pick(tip),
                        squash: Vec::new(),
                    },
                    PlanStep {
                        parent: PlanParent::Step(0),
                        commit: PlanCommit::Pick(middle),
                        squash: Vec::new(),
                    },
                    PlanStep {
                        parent: PlanParent::Step(0),
                        commit: PlanCommit::Empty("side".into()),
                        squash: Vec::new(),
                    },
                ],
                checkout: Some(PlanCheckout {
                    target: PlanParent::Step(0),
                    reference: None,
                }),
                expected_refs: capture_refs(&repo, &[middle, tip], &[tip])?,
            },
        )?
        .complete()?;
        let primary = outcome.map(middle).context("the primary leaf is retained")?;
        for name in ["refs/heads/main", "refs/custom/tip"] {
            assert_eq!(
                repo.find_reference(name)?.id().detach(),
                primary,
                "{name} follows the first continuation"
            );
        }
        let pins = crate::history::all_pins(&repo)?;
        assert_eq!(pins.len(), 1, "the secondary leaf receives one pin");
        assert_eq!(
            repo.find_commit(pins[0].id)?.parent_ids().next().map(gix::Id::detach),
            outcome.map(tip),
            "the pin retains the secondary fork"
        );
        Ok(())
    }

    #[test]
    fn dropping_a_referenced_tip_advances_its_refs_to_the_primary_leaf() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let base = repo.rev_parse_single("HEAD~2")?.detach();
        let middle = repo.rev_parse_single("HEAD~1")?.detach();
        let tip = repo.head_id()?.detach();

        perform_plan(
            &repo,
            &graph,
            Plan {
                base,
                scope: vec![middle, tip],
                steps: vec![
                    PlanStep {
                        parent: PlanParent::Existing(base),
                        commit: PlanCommit::Pick(middle),
                        squash: Vec::new(),
                    },
                    PlanStep {
                        parent: PlanParent::Step(0),
                        commit: PlanCommit::Empty("replacement tip".into()),
                        squash: Vec::new(),
                    },
                ],
                checkout: None,
                expected_refs: capture_refs(&repo, &[middle, tip], &[tip])?,
            },
        )?
        .complete()?;
        assert_eq!(
            repo.find_commit(repo.head_id()?)?.message_raw()?,
            b"replacement tip",
            "the branch advances past the retained ancestor"
        );
        assert!(
            crate::history::all_pins(&repo)?.is_empty(),
            "the referenced replacement leaf needs no pin"
        );
        Ok(())
    }

    #[test]
    fn a_todo_rebase_can_update_onto_a_hidden_branch_without_moving_it() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let base = repo.rev_parse_single("HEAD~2")?.detach();
        let middle = repo.rev_parse_single("HEAD~1")?.detach();
        let tip = repo.head_id()?.detach();
        let mut hidden = repo.find_commit(base)?.decode()?.into_owned()?;
        hidden.parents = [base].into_iter().collect();
        hidden.message = "updated hidden base".into();
        let onto = repo.write_object(&hidden)?.detach();
        repo.reference(
            "refs/heads/hidden",
            onto,
            PreviousValue::MustNotExist,
            "test hidden update target",
        )?;

        let outcome = perform_plan(
            &repo,
            &graph,
            Plan {
                base: onto,
                scope: vec![middle, tip],
                steps: vec![
                    PlanStep {
                        parent: PlanParent::Existing(onto),
                        commit: PlanCommit::Pick(middle),
                        squash: Vec::new(),
                    },
                    PlanStep {
                        parent: PlanParent::Step(0),
                        commit: PlanCommit::Pick(tip),
                        squash: Vec::new(),
                    },
                ],
                checkout: Some(PlanCheckout {
                    target: PlanParent::Step(1),
                    reference: None,
                }),
                expected_refs: capture_refs(&repo, &[middle, tip], &[tip])?,
            },
        )?
        .complete()?;
        let new_middle = outcome.map(middle).context("the middle commit is retained")?;
        assert_eq!(
            repo.find_commit(new_middle)?.parent_ids().next().map(gix::Id::detach),
            Some(onto),
            "the visible stack starts at the latest hidden branch tip"
        );
        assert_eq!(
            repo.find_reference("refs/heads/hidden")?.id().detach(),
            onto,
            "the hidden branch itself remains untouched"
        );
        Ok(())
    }

    #[test]
    fn a_todo_rebase_uses_the_reference_snapshot_from_before_editing() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let base = repo.rev_parse_single("HEAD~2")?.detach();
        let middle = repo.rev_parse_single("HEAD~1")?.detach();
        let tip = repo.head_id()?.detach();
        let expected_refs = capture_refs(&repo, &[middle, tip], &[tip])?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["update-ref", "refs/heads/main", &base.to_string(), &tip.to_string()])
                .status()?
                .success(),
            "the branch moves while the editor is open"
        );

        let err = match perform_plan(
            &repo,
            &graph,
            Plan {
                base,
                scope: vec![middle, tip],
                steps: vec![PlanStep {
                    parent: PlanParent::Existing(base),
                    commit: PlanCommit::Pick(tip),
                    squash: Vec::new(),
                }],
                checkout: None,
                expected_refs,
            },
        ) {
            Ok(_) => return Err("the stale reference snapshot must fail its compare-and-swap".into()),
            Err(err) => err,
        };
        assert_eq!(
            repo.find_reference("refs/heads/main")?.id().detach(),
            base,
            "a concurrent branch update wins"
        );
        assert!(
            format!("{err:#}").contains("reference"),
            "the failure identifies reference persistence: {err:#}"
        );
        Ok(())
    }

    #[test]
    fn checked_out_linked_worktree_refs_move_but_cannot_be_deleted() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let linked_root = gix_testtools::tempfile::tempdir()?;
        let linked = linked_root.path().join("linked");
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["worktree", "add", "-q", "-b", "linked"])
                .arg(&linked)
                .arg("HEAD")
                .status()?
                .success(),
            "the linked worktree is created"
        );
        let repo = open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let base = repo.rev_parse_single("HEAD~2")?.detach();
        let middle = repo.rev_parse_single("HEAD~1")?.detach();
        let tip = repo.head_id()?.detach();
        let steps = || {
            vec![
                PlanStep {
                    parent: PlanParent::Existing(base),
                    commit: PlanCommit::Pick(middle),
                    squash: Vec::new(),
                },
                PlanStep {
                    parent: PlanParent::Step(0),
                    commit: PlanCommit::Pick(tip),
                    squash: Vec::new(),
                },
            ]
        };

        let mut deleting = capture_refs(&repo, &[middle, tip], &[tip])?;
        let linked_ref = deleting
            .iter_mut()
            .find(|reference| reference.name.as_bstr() == b"refs/heads/linked")
            .expect("the linked branch is captured");
        linked_ref.new = None;
        let err = match perform_plan(
            &repo,
            &graph,
            Plan {
                base,
                scope: vec![middle, tip],
                steps: steps(),
                checkout: None,
                expected_refs: deleting,
            },
        ) {
            Ok(_) => return Err("a checked-out linked branch was deleted".into()),
            Err(err) => err,
        };
        assert!(
            format!("{err:#}").contains("another worktree has it checked out"),
            "the refusal identifies the linked-worktree constraint"
        );

        let mut moving = capture_refs(&repo, &[middle, tip], &[tip])?;
        let linked_ref = moving
            .iter_mut()
            .find(|reference| reference.name.as_bstr() == b"refs/heads/linked")
            .expect("the linked branch is captured");
        linked_ref.new = None;
        linked_ref.placement = Some(PlanParent::Step(0));
        let outcome = perform_plan(
            &repo,
            &graph,
            Plan {
                base,
                scope: vec![middle, tip],
                steps: steps(),
                checkout: None,
                expected_refs: moving,
            },
        )?
        .complete()?;
        assert_eq!(
            repo.find_reference("refs/heads/linked")?.id().detach(),
            outcome.map(middle).expect("the middle commit is retained"),
            "the linked branch moves to its explicit todo destination"
        );
        assert!(
            linked.join("middle").is_file(),
            "the linked worktree receives the new tree"
        );
        assert!(
            !linked.join("tip").exists(),
            "files beyond the moved branch are removed"
        );
        Ok(())
    }

    #[test]
    fn dropping_a_pick_moves_its_mutable_refs_to_the_nearest_retained_ancestor() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let base = repo.rev_parse_single("HEAD~2")?.detach();
        let middle = repo.rev_parse_single("HEAD~1")?.detach();
        let tip = repo.head_id()?.detach();
        repo.reference(
            "refs/patches/example",
            middle,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "test reference",
        )?;
        let expected_refs = capture_refs(&repo, &[middle, tip], &[tip])?;

        perform_plan(
            &repo,
            &graph,
            Plan {
                base,
                scope: vec![middle, tip],
                steps: vec![PlanStep {
                    parent: PlanParent::Existing(base),
                    commit: PlanCommit::Pick(tip),
                    squash: Vec::new(),
                }],
                checkout: None,
                expected_refs,
            },
        )?
        .complete()?;
        assert_eq!(
            repo.find_reference("refs/patches/example")?.id().detach(),
            base,
            "the dropped commit's ref does not keep the old history alive"
        );
        Ok(())
    }

    #[test]
    fn dropping_a_stashed_commit_is_rejected_before_refs_change() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let base = repo.rev_parse_single("HEAD~2")?.detach();
        let middle = repo.rev_parse_single("HEAD~1")?.detach();
        let tip = repo.head_id()?.detach();
        let saved = repo.write_blob(b"saved worktree state")?.detach();
        let name = super::super::stash::reference(middle)?;
        repo.reference(name.clone(), saved, PreviousValue::MustNotExist, "test stash")?;

        let err = match perform_plan(
            &repo,
            &graph,
            Plan {
                base,
                scope: vec![middle, tip],
                steps: vec![PlanStep {
                    parent: PlanParent::Existing(base),
                    commit: PlanCommit::Pick(tip),
                    squash: Vec::new(),
                }],
                checkout: None,
                expected_refs: capture_refs(&repo, &[middle, tip], &[tip])?,
            },
        ) {
            Ok(_) => panic!("dropping a commit with saved state must fail before producing an outcome"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("cannot drop stashed commit"));
        assert_eq!(repo.head_id()?.detach(), tip, "HEAD remains at the original tip");
        assert_eq!(
            repo.find_reference(name.as_ref())?.id().detach(),
            saved,
            "the stash association remains unchanged"
        );
        Ok(())
    }

    #[test]
    fn dropping_a_review_commit_removes_all_review_resources() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = open(fixture.path())?;
        let base = repo.rev_parse_single("HEAD~2")?.detach();
        let old_middle = repo.rev_parse_single("HEAD~1")?.detach();
        let old_tip = repo.head_id()?.detach();
        let mut middle = repo.find_commit(old_middle)?.decode()?.into_owned()?;
        middle
            .extra_headers
            .push(("tix-rebase".into(), "onto refs/worktree/tix/review/1".into()));
        let middle = repo.write_object(&middle)?.detach();
        let mut tip = repo.find_commit(old_tip)?.decode()?.into_owned()?;
        tip.parents = [middle].into_iter().collect();
        let tip = repo.write_object(&tip)?.detach();
        repo.reference(
            "refs/worktree/tix/review/1",
            old_middle,
            PreviousValue::MustNotExist,
            "test review resource",
        )?;
        repo.reference(
            "refs/worktree/tix/review/stashes/1",
            old_tip,
            PreviousValue::MustNotExist,
            "test review stash resource",
        )?;
        repo.reference(
            "refs/heads/main",
            tip,
            PreviousValue::ExistingMustMatch(Target::Object(old_tip)),
            "prepare review history",
        )?;
        let graph = super::super::loaded_graph(&repo)?;

        perform_plan(
            &repo,
            &graph,
            Plan {
                base,
                scope: vec![middle, tip],
                steps: vec![PlanStep {
                    parent: PlanParent::Existing(base),
                    commit: PlanCommit::Pick(tip),
                    squash: Vec::new(),
                }],
                checkout: None,
                expected_refs: capture_refs(&repo, &[middle, tip], &[tip])?,
            },
        )?
        .complete()?;
        assert!(
            repo.try_find_reference("refs/worktree/tix/review/1")?.is_none(),
            "dropping the review commit removes its associated resource"
        );
        assert!(
            repo.try_find_reference("refs/worktree/tix/review/stashes/1")?.is_none(),
            "dropping the review commit also removes its saved worktree state"
        );
        Ok(())
    }

    #[test]
    fn squashing_a_later_review_commit_removes_its_resources() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = open(fixture.path())?;
        let base = repo.rev_parse_single("HEAD~2")?.detach();
        let old_middle = repo.rev_parse_single("HEAD~1")?.detach();
        let old_tip = repo.head_id()?.detach();
        let old_tip_tree = repo.find_commit(old_tip)?.tree_id()?.detach();
        let mut middle = repo.find_commit(old_middle)?.decode()?.into_owned()?;
        middle
            .extra_headers
            .push(("tix-rebase".into(), "onto refs/worktree/tix/review/1".into()));
        let middle = repo.write_object(&middle)?.detach();
        let mut tip = repo.find_commit(old_tip)?.decode()?.into_owned()?;
        tip.parents = [middle].into_iter().collect();
        let tip = repo.write_object(&tip)?.detach();
        repo.reference(
            "refs/worktree/tix/review/1",
            old_middle,
            PreviousValue::MustNotExist,
            "test review resource",
        )?;
        repo.reference(
            "refs/worktree/tix/review/stashes/1",
            old_tip,
            PreviousValue::MustNotExist,
            "test review stash resource",
        )?;
        repo.reference(
            "refs/heads/main",
            tip,
            PreviousValue::ExistingMustMatch(Target::Object(old_tip)),
            "prepare review history",
        )?;
        let graph = super::super::loaded_graph(&repo)?;

        let outcome = perform_plan(
            &repo,
            &graph,
            Plan {
                base,
                scope: vec![middle, tip],
                steps: vec![PlanStep {
                    parent: PlanParent::Existing(base),
                    commit: PlanCommit::Pick(tip),
                    squash: vec![middle],
                }],
                checkout: None,
                expected_refs: capture_refs(&repo, &[middle, tip], &[tip])?,
            },
        )?
        .complete()?;
        let combined = outcome.map(tip).context("the first member remains")?;
        assert_eq!(outcome.map(middle), Some(combined));
        assert_eq!(
            repo.find_commit(combined)?.tree_id()?.detach(),
            old_tip_tree,
            "reordered source deltas are composed into the combined tree"
        );
        assert!(
            repo.try_find_reference("refs/worktree/tix/review/1")?.is_none(),
            "the consumed review resource is removed"
        );
        assert!(
            repo.try_find_reference("refs/worktree/tix/review/stashes/1")?.is_none(),
            "the consumed review stash is removed"
        );
        Ok(())
    }
}
