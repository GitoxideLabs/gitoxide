use anyhow::{Context, Result};
use gix::{ObjectId, bstr::ByteSlice};

use super::{TreeRewrite, is_pending, parent_tree};

#[cfg(test)]
mod tests;

pub(super) const HEADER: &str = "tix-rebase-merge";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Parent,
    Combine,
}

/// The Git parents always describe the destination topology. These slots also
/// retain correspondence when multiple destination parents become identical.
struct State {
    source_commit_id: ObjectId,
    parent_index: usize,
    phase: Phase,
    checkpoint_commit_id: ObjectId,
    parents: Vec<ObjectId>,
}

impl State {
    fn read(commit: &gix::objs::Commit) -> Result<Option<Self>> {
        let mut headers = commit.extra_headers.iter().filter(|(name, _)| name == HEADER);
        let Some((_, value)) = headers.next() else {
            return Ok(None);
        };
        anyhow::ensure!(headers.next().is_none(), "a merge has duplicate replay metadata");
        let mut words = value.split(|byte| *byte == b' ');
        let source_commit_id = ObjectId::from_hex(words.next().context("missing merge replay source")?)?;
        let parent_index = words.next().context("missing merge replay parent")?.to_str()?.parse()?;
        let phase = match words.next() {
            Some(b"parent") => Phase::Parent,
            Some(b"combine") => Phase::Combine,
            _ => anyhow::bail!("invalid merge replay phase"),
        };
        let checkpoint_commit_id = ObjectId::from_hex(words.next().context("missing merge replay checkpoint")?)?;
        let parents = words.map(ObjectId::from_hex).collect::<Result<Vec<_>, _>>()?;
        anyhow::ensure!(
            parents.len() > 1 && parent_index < parents.len(),
            "invalid merge replay parent slots"
        );
        anyhow::ensure!(
            std::iter::once(&source_commit_id)
                .chain(std::iter::once(&checkpoint_commit_id))
                .chain(&parents)
                .all(|commit_id| !commit_id.is_null() && commit_id.kind() == commit.tree.kind()),
            "invalid merge replay object ID"
        );
        Ok(Some(Self {
            source_commit_id,
            parent_index,
            phase,
            checkpoint_commit_id,
            parents,
        }))
    }

    fn store(&self, commit: &mut gix::objs::Commit) {
        clear(commit);
        let phase = match self.phase {
            Phase::Parent => "parent",
            Phase::Combine => "combine",
        };
        let mut value = format!(
            "{} {} {phase} {}",
            self.source_commit_id, self.parent_index, self.checkpoint_commit_id
        );
        for parent_commit_id in &self.parents {
            use std::fmt::Write;
            write!(&mut value, " {parent_commit_id}").expect("writing into a string cannot fail");
        }
        commit.extra_headers.push((HEADER.into(), value.into()));
    }

    fn mapped_parents(&self, commit: &gix::objs::Commit, new_parents: &[ObjectId]) -> Result<Vec<ObjectId>> {
        if new_parents.len() == self.parents.len() {
            return Ok(new_parents.to_vec());
        }
        anyhow::ensure!(
            new_parents.len() == commit.parents.len(),
            "merge replay changed the number of parents"
        );
        self.parents
            .iter()
            .map(|parent| {
                commit
                    .parents
                    .iter()
                    .position(|actual| actual == parent)
                    .map(|index| new_parents[index])
                    .context("a merge replay parent is missing")
            })
            .collect()
    }
}

pub(super) fn checkpoint(commit: &gix::objs::Commit) -> Result<Option<ObjectId>> {
    Ok(State::read(commit)?.map(|state| state.checkpoint_commit_id))
}

pub(super) fn parents(commit: &gix::objs::Commit) -> Result<Option<Vec<ObjectId>>> {
    Ok(State::read(commit)?.map(|state| state.parents))
}

pub(super) fn clear(commit: &mut gix::objs::Commit) {
    commit.extra_headers.retain(|(name, _)| name != HEADER);
}

/// Unchanged inputs are already represented by the recorded merge tree.
pub(super) fn parents_pending(
    repo: &gix::Repository,
    commit: &gix::objs::Commit,
    new_parents: &[ObjectId],
) -> Result<bool> {
    let state = State::read(commit)?;
    let source = state
        .as_ref()
        .map(|state| -> Result<_> { Ok(repo.find_commit(state.source_commit_id)?.decode()?.into_owned()?) })
        .transpose()?;
    let mapped = state
        .as_ref()
        .map(|state| state.mapped_parents(commit, new_parents))
        .transpose()?;
    let original_parents = &source.as_ref().unwrap_or(commit).parents;
    for (index, parent_commit_id) in mapped.as_deref().unwrap_or(new_parents).iter().enumerate() {
        if original_parents.get(index) != Some(parent_commit_id)
            && is_pending(&repo.find_commit(*parent_commit_id)?.decode()?.into_owned()?)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Replay each parent against the same recorded merge, then combine independent
/// candidates. Applying parent deltas to the evolving result loses resolutions.
pub(super) fn rewrite(
    repo: &gix::Repository,
    source_commit_id: ObjectId,
    commit: &mut gix::objs::Commit,
    new_parents: &[ObjectId],
    eager: bool,
    resolution: bool,
) -> Result<TreeRewrite> {
    let previous = State::read(commit)?;
    if previous.is_none() {
        let mut seen = std::collections::HashSet::new();
        if commit
            .parents
            .iter()
            .copied()
            .eq(new_parents.iter().copied().filter(|parent| seen.insert(*parent)))
        {
            return Ok(TreeRewrite::Complete(commit.tree));
        }
    }
    let mut state = previous.unwrap_or_else(|| State {
        source_commit_id,
        parent_index: 0,
        phase: Phase::Parent,
        checkpoint_commit_id: source_commit_id,
        parents: new_parents.to_vec(),
    });
    let source = repo.find_commit(state.source_commit_id)?.decode()?.into_owned()?;
    anyhow::ensure!(
        source.parents.len() == state.parents.len(),
        "merge replay changed the number of parent slots"
    );
    // A pending commit can have deduplicated Git parents; expand the changed
    // edges back into its saved slots before replaying them.
    let mapped = state.mapped_parents(commit, new_parents)?;
    if resolution || state.parent_index > 0 || state.phase == Phase::Combine {
        for (old, new) in state.parents.iter().zip(&mapped).take(state.parent_index + 1) {
            anyhow::ensure!(
                parent_tree(repo, Some(*old))? == parent_tree(repo, Some(*new))?,
                "resolve the pending merge before changing a replayed parent"
            );
        }
    }
    state.parents = mapped;
    if !eager {
        state.store(commit);
        return Ok(TreeRewrite::Complete(commit.tree));
    }
    for (original_parent_commit_id, parent_commit_id) in source.parents.iter().zip(&state.parents) {
        anyhow::ensure!(
            original_parent_commit_id == parent_commit_id
                || !is_pending(&repo.find_commit(*parent_commit_id)?.decode()?.into_owned()?),
            "merge replay requires finalized changed parents"
        );
    }
    let recorded_tree_id = source.tree;
    let checkpoint = repo.find_commit(state.checkpoint_commit_id)?.decode()?.into_owned()?;
    anyhow::ensure!(
        state.checkpoint_commit_id == state.source_commit_id
            || checkpoint.parents.as_slice() == [state.source_commit_id],
        "merge replay checkpoint does not retain its source"
    );
    let mut accumulated_tree_id = checkpoint.tree;
    let mut resolved_tree_id = resolution.then_some(commit.tree);
    if resolution && state.phase == Phase::Combine {
        accumulated_tree_id = resolved_tree_id.take().expect("a resolution tree was supplied");
        state.parent_index += 1;
        state.phase = Phase::Parent;
    }
    while state.parent_index < source.parents.len() {
        let index = state.parent_index;
        let old_tree_id = parent_tree(repo, Some(source.parents[index]))?;
        let new_tree_id = parent_tree(repo, Some(state.parents[index]))?;
        let candidate = match resolved_tree_id.take() {
            Some(tree_id) => TreeRewrite::Complete(tree_id),
            None if old_tree_id == new_tree_id => {
                state.parent_index += 1;
                continue;
            }
            None => merge_trees(repo, old_tree_id, recorded_tree_id, new_tree_id)?,
        };
        let candidate_tree_id = match candidate {
            TreeRewrite::Complete(tree_id) => tree_id,
            conflict @ TreeRewrite::Conflict { .. } => {
                state.phase = Phase::Parent;
                save(repo, commit, &mut state, &source, accumulated_tree_id)?;
                return Ok(conflict);
            }
        };
        match merge_trees(repo, recorded_tree_id, accumulated_tree_id, candidate_tree_id)? {
            TreeRewrite::Complete(tree_id) => accumulated_tree_id = tree_id,
            conflict @ TreeRewrite::Conflict { .. } => {
                state.phase = Phase::Combine;
                save(repo, commit, &mut state, &source, accumulated_tree_id)?;
                return Ok(conflict);
            }
        }
        state.parent_index += 1;
        state.phase = Phase::Parent;
    }
    clear(commit);
    Ok(TreeRewrite::Complete(accumulated_tree_id))
}

fn save(
    repo: &gix::Repository,
    commit: &mut gix::objs::Commit,
    state: &mut State,
    source: &gix::objs::Commit,
    accumulated_tree_id: ObjectId,
) -> Result<()> {
    state.checkpoint_commit_id = if accumulated_tree_id == source.tree {
        state.source_commit_id
    } else {
        repo.write_object(&gix::objs::Commit {
            tree: accumulated_tree_id,
            parents: [state.source_commit_id].into_iter().collect(),
            author: source.author.clone(),
            committer: source.committer.clone(),
            encoding: None,
            message: "Tix merge replay checkpoint\n".into(),
            extra_headers: vec![(
                "tix-replay-checkpoint".into(),
                state.source_commit_id.to_string().into(),
            )],
        })?
        .detach()
    };
    state.store(commit);
    Ok(())
}

fn merge_trees(repo: &gix::Repository, base: ObjectId, ours: ObjectId, theirs: ObjectId) -> Result<TreeRewrite> {
    if ours == theirs || base == theirs {
        return Ok(TreeRewrite::Complete(ours));
    }
    if base == ours {
        return Ok(TreeRewrite::Complete(theirs));
    }
    let mut outcome = repo.merge_trees(base, ours, theirs, Default::default(), repo.tree_merge_options()?)?;
    let unresolved = outcome.has_unresolved_conflicts(gix::merge::tree::TreatAsUnresolved::git());
    let merged = outcome.tree.write()?.detach();
    Ok(if unresolved {
        TreeRewrite::Conflict {
            ours,
            merged,
            conflicts: outcome.conflicts,
        }
    } else {
        TreeRewrite::Complete(merged)
    })
}
