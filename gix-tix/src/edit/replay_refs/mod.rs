use std::collections::{BTreeMap, HashSet};

use anyhow::{Context, Result, ensure};
use gix::{
    ObjectId,
    bstr::{BStr, ByteSlice},
    refs::{
        Category, FullName, FullNameRef, Target,
        transaction::{Change, PreviousValue, RefEdit, RefLog},
    },
};

use super::{rebase, stash, undo};

#[cfg(test)]
mod tests;

const PREFIX: &[u8] = b"refs/tix/replay/";
const CONTINUATION_PREFIX: &[u8] = b"refs/tix/replay/todo-";

pub(crate) fn is_ref(name: &BStr) -> bool {
    unqualified_name(name).starts_with(PREFIX)
}

fn is_continuation_ref(name: &BStr) -> bool {
    unqualified_name(name).starts_with(CONTINUATION_PREFIX)
}

fn unqualified_name(name: &BStr) -> &BStr {
    name.strip_prefix(b"main-worktree/")
        .or_else(|| {
            name.strip_prefix(b"worktrees/")
                .and_then(|name| name.splitn(2, |byte| *byte == b'/').nth(1))
        })
        .unwrap_or(name)
        .as_bstr()
}

pub(crate) fn ref_chain_reaches(repo: &gix::Repository, name: &FullNameRef) -> Result<bool> {
    let mut name = name.to_owned();
    let mut seen = HashSet::new();
    loop {
        if is_ref(name.as_bstr()) {
            return Ok(true);
        }
        ensure!(seen.insert(name.clone()), "a symbolic reference chain contains a cycle");
        let Some(reference) = repo.try_find_reference(name.as_ref())? else {
            return Ok(false);
        };
        let Some(next) = reference.target().try_name().map(ToOwned::to_owned) else {
            return Ok(false);
        };
        name = next;
    }
}

pub(crate) fn is_checkpoint(repo: &gix::Repository, commit_id: ObjectId) -> Result<bool> {
    Ok(repo
        .find_commit(commit_id)?
        .decode()?
        .extra_headers()
        .find("tix-replay-checkpoint")
        .is_some())
}

fn reference(owner_commit_id: ObjectId) -> Result<FullName> {
    format!("refs/tix/replay/{owner_commit_id}")
        .try_into()
        .context("generated an invalid merge replay reference")
}

/// A saved continuation owns its commit IDs independently of active checkouts or undo history.
/// Only consuming that continuation releases these refs; amending its HEAD keeps them intact.
pub(super) fn continuation_edits(
    repo: &gix::Repository,
    retain: Option<(ObjectId, &[ObjectId])>,
    release: Option<(ObjectId, &[ObjectId])>,
) -> Result<stash::RewriteEdits> {
    let mut edits = stash::RewriteEdits {
        forward: Vec::new(),
        rollback: Vec::new(),
    };
    let mut seen = HashSet::new();
    if let Some((owner_commit_id, commit_ids)) = retain {
        for commit_id in commit_ids {
            let name = continuation_reference(owner_commit_id, *commit_id)?;
            if seen.insert(name.clone()) {
                retain_resource(repo, name, *commit_id, &mut edits)?;
            }
        }
    }
    if let Some((owner_commit_id, commit_ids)) = release {
        for commit_id in commit_ids {
            let name = continuation_reference(owner_commit_id, *commit_id)?;
            if !seen.insert(name.clone()) {
                continue;
            }
            let Some(existing) = repo.try_find_reference(name.as_ref())? else {
                continue;
            };
            ensure!(
                existing.target().try_id() == Some(commit_id.as_ref()),
                "a merge replay continuation reference points to a different commit"
            );
            retire_resource(name, *commit_id, &mut edits);
        }
    }
    Ok(edits)
}

fn continuation_reference(owner_commit_id: ObjectId, commit_id: ObjectId) -> Result<FullName> {
    format!("refs/tix/replay/todo-{owner_commit_id}/{commit_id}")
        .try_into()
        .context("generated an invalid merge replay continuation reference")
}

fn retain_resource(
    repo: &gix::Repository,
    name: FullName,
    commit_id: ObjectId,
    edits: &mut stash::RewriteEdits,
) -> Result<()> {
    repo.find_commit(commit_id)
        .context("a merge replay resource commit is unavailable")?;
    if let Some(existing) = repo.try_find_reference(name.as_ref())? {
        ensure!(
            existing.target().try_id() == Some(commit_id.as_ref()),
            "a merge replay resource reference points to a different commit"
        );
    } else {
        edits.forward.push(RefEdit::update(
            name.clone(),
            commit_id,
            PreviousValue::MustNotExist,
            "tix retain merge replay",
        ));
        edits.rollback.push(RefEdit::delete(
            name,
            PreviousValue::MustExistAndMatch(Target::Object(commit_id)),
        ));
    }
    Ok(())
}

fn retire_resource(name: FullName, commit_id: ObjectId, edits: &mut stash::RewriteEdits) {
    edits.forward.push(RefEdit::delete(
        name.clone(),
        PreviousValue::MustExistAndMatch(Target::Object(commit_id)),
    ));
    edits.rollback.push(RefEdit::update(
        name,
        commit_id,
        PreviousValue::MustNotExist,
        "tix restore merge replay",
    ));
}

/// Retain published replay checkpoints and retire resources only after proving their owner unreachable.
/// `final_ref_edits` are the operation's non-replay edits, before publication. `checkout` overrides only
/// the current worktree's final HEAD; attached HEADs otherwise follow the projected reference edits.
pub(super) fn prepare(
    repo: &gix::Repository,
    published: impl IntoIterator<Item = ObjectId>,
    retired: impl IntoIterator<Item = ObjectId>,
    final_ref_edits: &[RefEdit],
    checkout: Option<ObjectId>,
) -> Result<stash::RewriteEdits> {
    let mut edits = stash::RewriteEdits {
        forward: Vec::new(),
        rollback: Vec::new(),
    };
    let mut published: Vec<_> = published.into_iter().collect();
    published.sort_unstable();
    published.dedup();
    let mut retained = HashSet::new();
    for owner_commit_id in published {
        let commit = repo.find_commit(owner_commit_id)?.decode()?.into_owned()?;
        let Some(checkpoint_commit_id) = rebase::replay_checkpoint(&commit)? else {
            continue;
        };
        retained.insert(owner_commit_id);
        retain_resource(repo, reference(owner_commit_id)?, checkpoint_commit_id, &mut edits)?;
    }

    let mut candidates = BTreeMap::new();
    for owner_commit_id in retired {
        if retained.contains(&owner_commit_id) || candidates.contains_key(&owner_commit_id) {
            continue;
        }
        let name = reference(owner_commit_id)?;
        match repo.try_find_reference(name.as_ref()) {
            Ok(Some(reference)) => {
                if let Some(checkpoint_commit_id) = reference.try_id().map(gix::Id::detach) {
                    candidates.insert(owner_commit_id, (name, checkpoint_commit_id));
                }
            }
            Ok(None) => {}
            Err(err) => {
                tracing::warn!(commit_id = %owner_commit_id, error = %err, "retained unreadable merge replay reference");
            }
        }
    }
    if candidates.is_empty() {
        return Ok(edits);
    }
    let unreachable = match unreachable_owners(repo, candidates.keys().copied().collect(), final_ref_edits, checkout) {
        Ok(unreachable) => unreachable,
        Err(err) => {
            tracing::warn!(error = %err, "retained merge replay resources because reachability is incomplete");
            return Ok(edits);
        }
    };
    for (owner_commit_id, (name, checkpoint_commit_id)) in candidates {
        if !unreachable.contains(&owner_commit_id) {
            continue;
        }
        retire_resource(name, checkpoint_commit_id, &mut edits);
    }
    Ok(edits)
}

fn unreachable_owners(
    repo: &gix::Repository,
    mut candidates: HashSet<ObjectId>,
    edits: &[RefEdit],
    checkout: Option<ObjectId>,
) -> Result<HashSet<ObjectId>> {
    let mut references = BTreeMap::new();
    let mut repositories = vec![repo.main_repo()?];
    for worktree in repo.worktrees()? {
        repositories.push(worktree.into_repo_with_possibly_inaccessible_worktree()?);
    }
    for repository in repositories {
        for reference in repository.references()?.all()? {
            let reference = reference.map_err(anyhow::Error::from_boxed)?;
            let name = canonical_name(&repository, reference.name())?;
            if is_ref(name.as_bstr()) && !is_continuation_ref(name.as_bstr()) || is_undo_ref(name.as_bstr()) {
                continue;
            }
            let target = canonical_target(&repository, name.as_ref(), reference.target().into_owned())?;
            references.insert(name, target);
        }
        if let Some(head) = repository.try_find_reference("HEAD")? {
            let name = canonical_name(&repository, head.name())?;
            let target = canonical_target(&repository, name.as_ref(), head.target().into_owned())?;
            references.insert(name, target);
        }
    }
    for edit in edits {
        let changes_reference = match &edit.change {
            Change::Update { log, .. } => log.mode == RefLog::AndReference,
            Change::Delete { log, .. } => *log == RefLog::AndReference,
        };
        if !changes_reference {
            continue;
        }
        ensure!(!edit.deref, "merge replay cleanup requires resolved reference edits");
        let name = canonical_name(repo, edit.name.as_ref())?;
        if is_ref(name.as_bstr()) && !is_continuation_ref(name.as_bstr()) || is_undo_ref(name.as_bstr()) {
            continue;
        }
        match edit.change.new_value() {
            Some(target) => {
                let target = canonical_target(repo, name.as_ref(), target.into_owned())?;
                references.insert(name, target);
            }
            None => {
                references.remove(&name);
            }
        }
    }
    if let Some(commit_id) = checkout {
        references.insert(canonical_name(repo, "HEAD".try_into()?)?, Target::Object(commit_id));
    }

    let mut pending = Vec::new();
    for target in references.values() {
        let mut target = target;
        let mut seen = HashSet::new();
        while let Target::Symbolic(name) = target {
            ensure!(seen.insert(name), "a projected reference contains a symbolic cycle");
            let Some(next) = references.get(name) else {
                ensure!(
                    !matches!(
                        name.category(),
                        Some(Category::MainPseudoRef | Category::LinkedPseudoRef { .. })
                    ),
                    "a symbolic reference reaches an unenumerated pseudo-reference"
                );
                break;
            };
            target = next;
        }
        if let Target::Object(commit_id) = target {
            pending.push(*commit_id);
        }
    }
    let mut seen = HashSet::new();
    while let Some(commit_id) = pending.pop() {
        candidates.remove(&commit_id);
        if candidates.is_empty() {
            break;
        }
        if !seen.insert(commit_id) {
            continue;
        }
        let object = repo.find_object(commit_id)?;
        match object.kind {
            gix::objs::Kind::Commit => pending.extend(object.into_commit().decode()?.parents()),
            gix::objs::Kind::Tag => pending.push(object.into_tag().target_id()?.detach()),
            gix::objs::Kind::Tree | gix::objs::Kind::Blob => {}
        }
    }
    Ok(candidates)
}

fn canonical_target(repo: &gix::Repository, owner: &FullNameRef, target: Target) -> Result<Target> {
    let Target::Symbolic(name) = target else {
        return Ok(target);
    };
    let name = match name.category() {
        Some(Category::PseudoRef | Category::WorktreePrivate | Category::Rewritten | Category::Bisect) => {
            match owner.category() {
                Some(Category::MainRef | Category::MainPseudoRef) => {
                    Category::MainPseudoRef.to_full_name(name.as_bstr())?
                }
                Some(Category::LinkedRef { name: worktree } | Category::LinkedPseudoRef { name: worktree }) => {
                    Category::LinkedPseudoRef { name: worktree }.to_full_name(name.as_bstr())?
                }
                _ => anyhow::bail!("a shared symbolic reference has a worktree-dependent target"),
            }
        }
        _ => canonical_name(repo, name.as_ref())?,
    };
    Ok(Target::Symbolic(name))
}

fn canonical_name(repo: &gix::Repository, name: &FullNameRef) -> Result<FullName> {
    match name.category_and_short_name() {
        Some((Category::MainRef | Category::LinkedRef { .. }, short)) => {
            let unqualified: &FullNameRef = short.try_into()?;
            if !unqualified
                .category()
                .is_some_and(|category| category.is_worktree_private())
            {
                return Ok(unqualified.to_owned());
            }
        }
        Some((Category::MainPseudoRef | Category::LinkedPseudoRef { .. }, _)) => {}
        Some((category, _)) if category.is_worktree_private() => {
            return Ok(
                match repo
                    .worktree()
                    .and_then(|worktree| worktree.id().map(ToOwned::to_owned))
                {
                    Some(worktree) => Category::LinkedPseudoRef {
                        name: worktree.as_bstr(),
                    }
                    .to_full_name(name.as_bstr())?,
                    None => Category::MainPseudoRef.to_full_name(name.as_bstr())?,
                },
            );
        }
        _ => {}
    }
    Ok(name.to_owned())
}

fn is_undo_ref(name: &BStr) -> bool {
    undo::is_queue_ref(unqualified_name(name))
}
