use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    process::Command,
};

use anyhow::{Context, Result};
use gix::{
    ObjectId,
    bstr::{BStr, BString},
    objs::Write,
    refs::{
        Category, Target,
        transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog},
    },
};

use crate::history::HistoryGraph;

const MARKER: &[u8] = b"tix-rebase";
const ORIGINAL_PARENT: &[u8] = b"tix-rebase-parent";
const PENDING: &[u8] = b"pending";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Signature {
    InvalidateExisting,
    RedoIfNeeded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Tree {
    LeaveAsIs,
    LeaveAsIsAndMark,
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
    },
}

pub(crate) struct Outcome {
    pub selected: Option<ObjectId>,
    rewritten: HashMap<ObjectId, Option<ObjectId>>,
}

impl Outcome {
    pub(crate) fn map(&self, id: ObjectId) -> Option<ObjectId> {
        self.rewritten.get(&id).copied().unwrap_or(Some(id))
    }
}

#[tracing::instrument(skip_all, fields(signature = ?signature, tree = ?tree_mode))]
pub(crate) fn perform(
    mut repo: gix::Repository,
    graph: &HistoryGraph,
    edit: Edit,
    signature: Signature,
    mut tree_mode: Tree,
) -> Result<Outcome> {
    let (root, replacement, inserted, removed, repeat, mut split_upper) = match edit {
        Edit::Replace { target, commit } => (Some(target), Some(commit), false, false, false, None),
        Edit::Insert { anchor, commit } => (anchor, Some(commit), true, false, false, None),
        Edit::Remove { target } => (Some(target), None, false, true, false, None),
        Edit::Split { target, source, upper } => (Some(target), Some(source), false, false, false, Some(upper)),
        Edit::Repeat { base } => (Some(base), None, false, false, true, None),
    };
    if repeat {
        tree_mode = Tree::CherryPick;
    }

    let affected = match root {
        Some(root) => graph
            .descendants_in_parent_order(root)
            .context("the edited commit is not in the loaded history")?,
        None => Vec::new(),
    };
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
    if inserted {
        let mut commit = replacement.clone().context("an inserted commit is required")?;
        commit.parents = root.into_iter().collect();
        marker(&mut commit, tree_mode == Tree::LeaveAsIsAndMark, root);
        let id = write_commit(&repo, commit, signature, signing.clone())?;
        selected = Some(id);
        if let Some(root) = root {
            rewritten.insert(root, Some(id));
        } else {
            rewritten.insert(id, Some(id));
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
        if Some(old_id) != root || repeat {
            commit.committer = committer.clone();
        }
        let original_parent = repeat.then(|| marked_parent(&commit)).transpose()?.flatten();
        let original_parents = original_parent.into_iter().collect::<Vec<_>>();
        commit.tree = rewritten_tree(
            &repo,
            &commit,
            if original_parents.is_empty() {
                &old_parents
            } else {
                &original_parents
            },
            &new_parents,
            tree_mode,
        )?;
        commit.parents = new_parents.into_iter().collect();
        marker(
            &mut commit,
            tree_mode == Tree::LeaveAsIsAndMark,
            old_parents.first().copied(),
        );
        let new_id = write_commit(&repo, commit, signature, signing.clone())?;
        rewritten.insert(old_id, Some(new_id));
        if Some(old_id) == root {
            if let Some(mut upper) = split_upper.take() {
                upper.parents = [new_id].into_iter().collect();
                marker(&mut upper, true, Some(new_id));
                let upper_id = write_commit(&repo, upper, Signature::RedoIfNeeded, signing.clone())?;
                rewritten.insert(old_id, Some(upper_id));
                selected = Some(upper_id);
            } else {
                selected = Some(new_id);
            }
        }
    }

    let objects = repo
        .objects
        .take_object_memory()
        .context("candidate object memory was unavailable")?;
    for (id, (kind, data)) in objects.iter() {
        repo.write_buf_with_known_id(*kind, data, *id)
            .map_err(|err| anyhow::anyhow!("could not persist a prepared rebase object: {err}"))?;
    }

    let transitions = worktree_transitions(&repo, &rewritten, inserted || tree_mode == Tree::LeaveAsIsAndMark)?;
    let index_reset_from = if inserted || tree_mode == Tree::LeaveAsIsAndMark {
        root
    } else {
        None
    };
    let index_resets = index_reset_from
        .map(|old| index_resets(&repo, &rewritten, old))
        .transpose()?;
    for transition in &transitions {
        super::forget::preflight_tree_transition(
            &transition.repo,
            &transition.workdir,
            transition.old,
            transition.new,
        )?;
    }
    let rollback_refs = update_refs(&repo, &rewritten, root.is_none(), selected, &committer)?;
    for (transitioned, transition) in transitions.iter().enumerate() {
        if let Err(err) = super::forget::apply_tree_transition(&transition.workdir, transition.old, transition.new) {
            return rollback(&repo, &committer, &transitions[..transitioned], &rollback_refs, err);
        }
    }
    let index_resets = index_resets.unwrap_or_default();
    for (index, reset) in index_resets.iter().enumerate() {
        if let Err(mut err) = reset_index(&reset.workdir, reset.new) {
            for applied in index_resets[..=index].iter().rev() {
                if let Err(restore) = std::fs::write(&applied.index, &applied.before) {
                    err = err.context(format!("index rollback failed: {restore}"));
                }
            }
            return rollback(&repo, &committer, &transitions, &rollback_refs, err);
        }
    }
    Ok(Outcome { selected, rewritten })
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
        if let Some(workdir) = worktree_repo.workdir().filter(|path| path.is_dir()) {
            let index = worktree_repo.index_path();
            out.push(IndexReset {
                workdir: workdir.to_owned(),
                index: index.to_owned(),
                before: std::fs::read(index).context("could not preserve an affected index")?,
                new,
            });
        }
    }
    Ok(out)
}

struct IndexReset {
    workdir: PathBuf,
    index: PathBuf,
    before: Vec<u8>,
    new: ObjectId,
}

fn reset_index(workdir: &std::path::Path, id: ObjectId) -> Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(workdir)
        .args(["reset", "--mixed", "--quiet"])
        .arg(id.to_string())
        .output()
        .context("could not update the index after inserting a commit")?;
    if output.status.success() {
        Ok(())
    } else {
        anyhow::bail!("git reset failed: {}", String::from_utf8_lossy(&output.stderr).trim())
    }
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
        if repeat {
            let commit = repo.find_commit(*id)?.decode()?.into_owned()?;
            if !has_marker(&commit) {
                anyhow::bail!("all repeated rebase commits must carry the pending marker");
            }
        }
    }
    if repeat
        && let Some(base) = affected.first()
        && let Some(parent) = graph.parents_of(*base).and_then(|parents| parents.first().copied())
        && has_marker(&repo.find_commit(parent)?.decode()?.into_owned()?)
    {
        anyhow::bail!("the parent of a repeated rebase must not carry the pending marker");
    }
    Ok(())
}

fn rewritten_tree(
    repo: &gix::Repository,
    commit: &gix::objs::Commit,
    old_parents: &[ObjectId],
    new_parents: &[ObjectId],
    mode: Tree,
) -> Result<ObjectId> {
    if mode != Tree::CherryPick || old_parents == new_parents {
        return Ok(commit.tree);
    }
    let old_base = parent_tree(repo, old_parents.first().copied())?;
    let new_base = parent_tree(repo, new_parents.first().copied())?;
    if commit.tree == old_base {
        return Ok(new_base);
    }
    if old_base == new_base {
        return Ok(commit.tree);
    }
    cherry_pick_tree(repo, old_base, new_base, commit.tree)
}

pub(super) fn cherry_pick_tree(
    repo: &gix::Repository,
    old_base: ObjectId,
    new_base: ObjectId,
    tree: ObjectId,
) -> Result<ObjectId> {
    if tree == old_base {
        return Ok(new_base);
    }
    if old_base == new_base {
        return Ok(tree);
    }
    let labels = gix::merge::blob::builtin_driver::text::Labels {
        ancestor: Some(BStr::new(b"parent")),
        current: Some(BStr::new(b"rebased parent")),
        other: Some(BStr::new(b"commit")),
    };
    let mut outcome = repo
        .merge_trees(old_base, new_base, tree, labels, repo.tree_merge_options()?)
        .context("could not cherry-pick a descendant tree")?;
    if outcome.has_unresolved_conflicts(gix::merge::tree::TreatAsUnresolved::git()) {
        anyhow::bail!("rebasing would cause a merge conflict");
    }
    Ok(outcome
        .tree
        .write()
        .context("could not prepare a rebased tree")?
        .detach())
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
        .retain(|(name, _)| name.as_slice() != MARKER && name.as_slice() != ORIGINAL_PARENT);
    if add {
        commit.extra_headers.push((MARKER.into(), PENDING.into()));
        if let Some(parent) = original_parent {
            commit
                .extra_headers
                .push((ORIGINAL_PARENT.into(), parent.to_hex().to_string().into()));
        }
    }
}

fn marked_parent(commit: &gix::objs::Commit) -> Result<Option<ObjectId>> {
    commit
        .extra_headers
        .iter()
        .find(|(name, _)| name.as_slice() == ORIGINAL_PARENT)
        .map(|(_, value)| ObjectId::from_hex(value).context("pending rebase has an invalid original parent"))
        .transpose()
}

pub(super) fn has_marker(commit: &gix::objs::Commit) -> bool {
    commit
        .extra_headers
        .iter()
        .any(|(name, value)| name.as_slice() == MARKER && value.as_slice() == PENDING)
}

fn write_commit(
    repo: &gix::Repository,
    mut commit: gix::objs::Commit,
    signature: Signature,
    signing: Option<gix::objs::signature::sign::Options>,
) -> Result<ObjectId> {
    let had_signature = commit.extra_headers.iter().any(|(name, _)| is_signature(name));
    commit.extra_headers.retain(|(name, _)| !is_signature(name));
    commit = match (signature, signing) {
        (Signature::RedoIfNeeded, Some(options)) => commit.sign(options).context("could not sign rebased commit")?,
        (Signature::InvalidateExisting, Some(_)) if had_signature => {
            let field = gix::objs::commit::signature_field_name(commit.tree.kind());
            commit.extra_headers.push((field.into(), BString::default()));
            commit
        }
        _ => commit,
    };
    Ok(repo
        .write_object(&commit)
        .context("could not prepare rebased commit")?
        .detach())
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
        let Some(old) = worktree_repo
            .head()
            .ok()
            .and_then(|head| head.id().map(gix::Id::detach))
        else {
            continue;
        };
        let Some(new) = rewritten.get(&old).copied() else {
            continue;
        };
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

fn update_refs(
    repo: &gix::Repository,
    rewritten: &HashMap<ObjectId, Option<ObjectId>>,
    unborn: bool,
    inserted: Option<ObjectId>,
    committer: &gix::actor::Signature,
) -> Result<Vec<RefEdit>> {
    let mut edits = Vec::new();
    let mut rollback = Vec::new();
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
        edits.push(ref_edit(name.clone(), old, *new));
        rollback.push(reverse_ref_edit(name, old, *new));
    }
    if let Some(head) = repo.try_find_reference("HEAD")?
        && let Some(old) = head.try_id().map(gix::Id::detach)
        && let Some(new) = rewritten.get(&old)
    {
        let name = head.name().to_owned();
        edits.push(ref_edit(name.clone(), old, *new));
        rollback.push(reverse_ref_edit(name, old, *new));
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
    if edits.is_empty() {
        anyhow::bail!("no mutable reference points to an affected commit");
    }
    let mut time = gix::date::parse::TimeBuf::default();
    repo.edit_references_as(edits, Some(committer.to_ref(&mut time)))
        .context("could not update references after rebasing")?;
    Ok(rollback)
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

fn reverse_ref_edit(name: gix::refs::FullName, old: ObjectId, new: Option<ObjectId>) -> RefEdit {
    RefEdit {
        name,
        deref: false,
        change: match new {
            Some(new) => Change::Update {
                log: log_change(),
                expected: PreviousValue::MustExistAndMatch(Target::Object(new)),
                new: Target::Object(old),
            },
            None => Change::Update {
                log: log_change(),
                expected: PreviousValue::MustNotExist,
                new: Target::Object(old),
            },
        },
    }
}

fn ref_edit(name: gix::refs::FullName, old: ObjectId, new: Option<ObjectId>) -> RefEdit {
    RefEdit {
        name,
        deref: false,
        change: match new {
            Some(new) => Change::Update {
                log: log_change(),
                expected: PreviousValue::MustExistAndMatch(Target::Object(old)),
                new: Target::Object(new),
            },
            None => Change::Delete {
                expected: PreviousValue::MustExistAndMatch(Target::Object(old)),
                log: RefLog::AndReference,
            },
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
        Ok(gix::open_opts(
            path,
            gix::open::Options::isolated().config_overrides([
                "user.name=rebasing committer".to_owned(),
                "user.email=rebasing@example.com".to_owned(),
                "gitoxide.commit.committerDate=2001-01-01T00:00:00 +0000".to_owned(),
                "commit.gpgSign=false".to_owned(),
            ]),
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
            repo.clone(),
            &graph,
            Edit::Replace { target: middle, commit },
            Signature::RedoIfNeeded,
            Tree::LeaveAsIs,
        )?;
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
        insta::assert_snapshot!(
            "reworded-middle-stack",
            gix_testtools::repository::snapshot(fixture.path())?.to_string()
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
            repo.clone(),
            &graph,
            Edit::Remove { target: middle },
            Signature::RedoIfNeeded,
            Tree::CherryPick,
        )?;
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
        let marked = perform(
            repo.clone(),
            &graph,
            Edit::Replace { target: middle, commit },
            Signature::InvalidateExisting,
            Tree::LeaveAsIsAndMark,
        )?
        .selected
        .expect("marking rewrites the selected commit");

        let graph = super::super::loaded_graph(&repo)?;
        perform(
            repo.clone(),
            &graph,
            Edit::Repeat { base: marked },
            Signature::RedoIfNeeded,
            Tree::CherryPick,
        )?;
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
            repo.clone(),
            &graph,
            Edit::Replace { target: middle, commit },
            Signature::RedoIfNeeded,
            Tree::LeaveAsIs,
        )?
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

        assert!(
            perform(
                repo,
                &graph,
                Edit::Remove { target: middle },
                Signature::RedoIfNeeded,
                Tree::CherryPick,
            )
            .is_err(),
            "an unresolved cherry-pick aborts the complete rebase"
        );
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
}
