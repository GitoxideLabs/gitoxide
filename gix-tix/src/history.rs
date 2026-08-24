use std::{
    cmp::Ordering as CmpOrdering,
    collections::{BTreeSet, HashMap, HashSet},
    ffi::OsString,
    sync::atomic::{AtomicBool, Ordering},
};

use anyhow::{Context, Result};
use gix::{
    ObjectId,
    bstr::{BStr, BString, ByteSlice, ByteVec},
    objs::commit::ref_iter::Token,
};

use crate::app::{Attribution, AttributionKind, Author, Commit, LoadedCommits, Metadata, SignatureState};

pub(crate) type SharedAuthors = gix::features::threading::OwnShared<gix::features::threading::Mutable<Authors>>;
static EMPTY_AUTHOR: std::sync::LazyLock<Author> = std::sync::LazyLock::new(|| Author {
    name: BStr::new(b""),
    email: BStr::new(b""),
});

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Decoration {
    pub name: BString,
    pub kind: DecorationKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DecorationKind {
    Head,
    Pin,
    HeadPinBranch,
    Stash,
    Review,
    CurrentWorktreeBranch,
    CurrentWorktreeDetached,
    WorktreeBranch,
    WorktreeDetached,
    Local,
    Remote,
    Tag,
    AnnotatedTag,
    Special,
}

pub(crate) type Decorations = HashMap<ObjectId, Vec<Decoration>>;

pub(crate) const PIN_PREFIX: &[u8] = b"refs/worktree/tix/pins/";
pub(crate) const HEAD_PIN_NAME: &[u8] = b"refs/worktree/tix/pins/HEAD";
pub(crate) const REVIEW_PIN_PREFIX: &[u8] = b"refs/worktree/tix/pins/review/";
pub(crate) const STASH_PREFIX: &[u8] = b"refs/tix/stash/";
pub(crate) const REVIEW_PREFIX: &[u8] = b"refs/worktree/tix/review/";
pub(crate) const REVIEW_STASH_PREFIX: &[u8] = b"refs/worktree/tix/review/stashes/";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Pin {
    pub name: gix::refs::FullName,
    pub target: gix::refs::Target,
    pub id: ObjectId,
}

impl Pin {
    pub(crate) fn is_head(&self) -> bool {
        self.name.as_bstr() == HEAD_PIN_NAME
    }

    pub(crate) fn is_review_return(&self) -> bool {
        self.name.as_bstr().starts_with(REVIEW_PIN_PREFIX)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorktreeCheckout {
    pub id: ObjectId,
    pub label_id: ObjectId,
    pub checkout_name: BString,
    pub reference: Option<gix::refs::FullName>,
    pub is_current: bool,
    pub is_detached: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SelectionRef {
    pub name: BString,
    pub upstream: Option<Option<ObjectId>>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct CommitIndex(u32);

impl CommitIndex {
    fn new(index: usize) -> Result<Self> {
        Ok(CommitIndex(
            index
                .try_into()
                .context("tix cannot index more than u32::MAX commits")?,
        ))
    }

    pub(crate) fn as_usize(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Debug)]
struct GraphCommit {
    id: ObjectId,
    parents: std::ops::Range<u32>,
    commit_time: gix::date::SecondsSinceUnixEpoch,
    generation: u32,
    state: u8,
}

impl GraphCommit {
    fn generation(&self) -> Option<gix::revwalk::graph::Generation> {
        (self.generation != 0).then_some(self.generation)
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Node {
    flags: u8,
    expanded: u8,
    emitted: bool,
}

#[derive(Debug, Default)]
pub(crate) struct HistoryGraph {
    commits: Vec<GraphCommit>,
    parents: Vec<CommitIndex>,
    by_id: HashMap<ObjectId, CommitIndex>,
    stored_order: Vec<CommitIndex>,
    tracking: HashMap<CommitIndex, Vec<SelectionRef>>,
    relations: HashMap<(CommitIndex, CommitIndex), (usize, usize)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GenThenTime {
    generation: gix::revwalk::graph::Generation,
    time: gix::date::SecondsSinceUnixEpoch,
}

impl From<&GraphCommit> for GenThenTime {
    fn from(commit: &GraphCommit) -> Self {
        GenThenTime {
            generation: commit
                .generation()
                .unwrap_or(gix::commitgraph::GENERATION_NUMBER_INFINITY),
            time: commit.commit_time,
        }
    }
}

impl Ord for GenThenTime {
    fn cmp(&self, other: &Self) -> CmpOrdering {
        self.generation.cmp(&other.generation).then(self.time.cmp(&other.time))
    }
}

impl PartialOrd for GenThenTime {
    fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> {
        Some(self.cmp(other))
    }
}

impl HistoryGraph {
    #[cfg(test)]
    pub(crate) fn from_test_commits(commits: &[(ObjectId, Vec<ObjectId>)]) -> Self {
        let mut graph = HistoryGraph::default();
        for (id, _) in commits {
            graph.intern(*id).expect("the test graph fits into u32");
        }
        for (id, parents) in commits {
            let index = graph.index(*id).expect("the test commit was interned");
            let start = graph.parents.len() as u32;
            let parents: Vec<_> = parents
                .iter()
                .map(|parent| graph.index(*parent).expect("the test parent was interned"))
                .collect();
            graph.parents.extend(parents);
            let end = graph.parents.len() as u32;
            graph.commits[index.as_usize()] = GraphCommit {
                id: *id,
                parents: start..end,
                commit_time: index.as_usize() as i64,
                generation: index.as_usize() as u32 + 1,
                state: NODE_LOADED,
            };
        }
        graph.set_current_view(&commits.iter().map(|(id, _)| *id).collect::<Vec<_>>());
        graph
    }

    pub(crate) fn for_commits(repo: &gix::Repository, ids: &[ObjectId]) -> Result<Self> {
        let mut graph = HistoryGraph::default();
        let shallow: HashSet<_> = repo
            .shallow_commits()
            .context("could not read shallow commits")?
            .into_iter()
            .flat_map(|commits| commits.iter().copied().collect::<Vec<_>>())
            .collect();
        let commit_graph = repo
            .commit_graph_if_enabled()
            .context("could not open commit-graph for rebase")?;
        let mut buf = Vec::new();
        for id in ids {
            graph.ensure_commit(repo, commit_graph.as_ref(), &shallow, *id, &mut buf)?;
        }
        graph.set_current_view(ids);
        Ok(graph)
    }

    fn intern(&mut self, id: ObjectId) -> Result<CommitIndex> {
        if let Some(index) = self.by_id.get(&id) {
            return Ok(*index);
        }
        let index = CommitIndex::new(self.commits.len())?;
        self.commits.push(GraphCommit {
            id,
            parents: 0..0,
            commit_time: 0,
            generation: 0,
            state: 0,
        });
        self.by_id.insert(id, index);
        Ok(index)
    }

    pub(crate) fn index(&self, id: ObjectId) -> Option<CommitIndex> {
        self.by_id.get(&id).copied()
    }

    pub(crate) fn id(&self, index: CommitIndex) -> ObjectId {
        self.commits[index.as_usize()].id
    }

    pub(crate) fn parents(&self, index: CommitIndex) -> &[CommitIndex] {
        let range = self.commits[index.as_usize()].parents.clone();
        &self.parents[range.start as usize..range.end as usize]
    }

    pub(crate) fn commit_count(&self) -> usize {
        self.commits.len()
    }

    pub(crate) fn stored_commit_ids(&self) -> impl Iterator<Item = ObjectId> + '_ {
        self.stored_order.iter().map(|index| self.id(*index))
    }

    pub(crate) fn is_stored(&self, id: ObjectId) -> bool {
        self.index(id)
            .is_some_and(|index| self.commits[index.as_usize()].state & NODE_STORED != 0)
    }

    pub(crate) fn is_ancestor(&self, ancestor: ObjectId, descendant: ObjectId) -> bool {
        let (Some(ancestor), Some(descendant)) = (self.index(ancestor), self.index(descendant)) else {
            return false;
        };
        let mut seen = vec![false; self.commits.len()];
        let mut pending = vec![descendant];
        while let Some(index) = pending.pop() {
            if index == ancestor {
                return true;
            }
            if std::mem::replace(&mut seen[index.as_usize()], true) {
                continue;
            }
            pending.extend_from_slice(self.parents(index));
        }
        false
    }

    pub(crate) fn commits_with_descendants(&self) -> HashSet<ObjectId> {
        self.commits
            .iter()
            .filter(|commit| commit.state & NODE_IN_VIEW != 0)
            .flat_map(|commit| {
                self.parents[commit.parents.start as usize..commit.parents.end as usize]
                    .iter()
                    .copied()
            })
            .map(|parent| self.id(parent))
            .collect()
    }

    pub(crate) fn parents_of(&self, id: ObjectId) -> Option<Vec<ObjectId>> {
        let index = self.index(id)?;
        Some(self.parents(index).iter().map(|parent| self.id(*parent)).collect())
    }

    pub(crate) fn commits_with_merge_descendants(&self) -> HashSet<ObjectId> {
        let mut pending: Vec<_> = self
            .commits
            .iter()
            .filter(|commit| commit.state & NODE_IN_VIEW != 0 && commit.parents.len() > 1)
            .flat_map(|commit| {
                let range = commit.parents.clone();
                self.parents[range.start as usize..range.end as usize].iter().copied()
            })
            .collect();
        let mut ancestors = HashSet::new();
        while let Some(index) = pending.pop() {
            if ancestors.insert(index) {
                pending.extend(
                    self.parents(index)
                        .iter()
                        .copied()
                        .filter(|parent| self.commits[parent.as_usize()].state & NODE_IN_VIEW != 0),
                );
            }
        }
        ancestors.into_iter().map(|index| self.id(index)).collect()
    }

    pub(crate) fn descendants_in_parent_order(&self, root: ObjectId) -> Option<Vec<ObjectId>> {
        let root = self.index(root)?;
        if self.commits[root.as_usize()].state & NODE_IN_VIEW == 0 {
            return None;
        }
        let mut included = HashSet::from([root]);
        loop {
            let mut changed = false;
            for index in 0..self.commits.len() {
                let index = CommitIndex::new(index).expect("an existing graph index fits into u32");
                if self.commits[index.as_usize()].state & NODE_IN_VIEW == 0
                    || included.contains(&index)
                    || !self.parents(index).iter().any(|parent| included.contains(parent))
                {
                    continue;
                }
                included.insert(index);
                changed = true;
            }
            if !changed {
                break;
            }
        }
        let mut out = Vec::with_capacity(included.len());
        while out.len() < included.len() {
            let before = out.len();
            for index in &included {
                if out.contains(index)
                    || self
                        .parents(*index)
                        .iter()
                        .any(|parent| included.contains(parent) && !out.contains(parent))
                {
                    continue;
                }
                out.push(*index);
            }
            if out.len() == before {
                return None;
            }
        }
        Some(out.into_iter().map(|index| self.id(index)).collect())
    }

    pub(crate) fn set_current_view(&mut self, tips: &[ObjectId]) {
        for commit in &mut self.commits {
            commit.state &= !NODE_IN_VIEW;
        }
        let mut pending: Vec<_> = tips.iter().filter_map(|id| self.index(*id)).collect();
        while let Some(index) = pending.pop() {
            if self.commits[index.as_usize()].state & NODE_IN_VIEW != 0 {
                continue;
            }
            self.commits[index.as_usize()].state |= NODE_IN_VIEW;
            pending.extend_from_slice(self.parents(index));
        }
    }

    fn parent_ids(&self, index: CommitIndex) -> gix::traverse::commit::ParentIds {
        self.parents(index).iter().map(|parent| self.id(*parent)).collect()
    }

    fn ensure_commit(
        &mut self,
        repo: &gix::Repository,
        cache: Option<&gix::commitgraph::Graph>,
        shallow: &HashSet<ObjectId>,
        id: ObjectId,
        buf: &mut Vec<u8>,
    ) -> Result<CommitIndex> {
        let index = self.intern(id)?;
        if self.commits[index.as_usize()].state & NODE_LOADED != 0 {
            return Ok(index);
        }
        let commit = gix::traverse::commit::find(cache, &repo.objects, &id, buf)
            .context("could not load commit for cached history traversal")?;
        let (mut parents, commit_time, generation) = match commit {
            gix::traverse::commit::Either::CommitRefIter(iter) => {
                let mut parents = gix::traverse::commit::ParentIds::new();
                let mut commit_time = 0;
                for token in iter {
                    match token.context("could not decode cached history commit")? {
                        Token::Tree { .. } => {}
                        Token::Parent { id } => parents.push(id),
                        Token::Committer { signature } => {
                            commit_time = signature.seconds();
                            break;
                        }
                        _ => {}
                    }
                }
                (parents, commit_time, None)
            }
            gix::traverse::commit::Either::CachedCommit(commit) => {
                let cache = cache.expect("cached commits originate from the provided commit-graph");
                let mut parents = gix::traverse::commit::ParentIds::new();
                for parent in commit.iter_parents() {
                    let parent =
                        parent.map_err(|err| anyhow::anyhow!("could not decode commit-graph parent: {err}"))?;
                    parents.push(cache.id_at(parent).to_owned());
                }
                (
                    parents,
                    commit.committer_timestamp() as gix::date::SecondsSinceUnixEpoch,
                    Some(commit.generation()),
                )
            }
        };
        if shallow.contains(&id) {
            parents.clear();
        }
        let parents: Vec<_> = parents
            .into_iter()
            .map(|parent| self.intern(parent))
            .collect::<Result<_>>()?;
        let start: u32 = self
            .parents
            .len()
            .try_into()
            .context("tix cannot index more than u32::MAX parent edges")?;
        self.parents.extend(parents);
        let end: u32 = self
            .parents
            .len()
            .try_into()
            .context("tix cannot index more than u32::MAX parent edges")?;
        let node = &mut self.commits[index.as_usize()];
        node.parents = start..end;
        node.commit_time = commit_time;
        node.generation = generation.unwrap_or_default();
        node.state |= NODE_LOADED;
        Ok(index)
    }

    #[expect(clippy::too_many_arguments)]
    fn schedule_cached(
        &mut self,
        repo: &gix::Repository,
        cache: Option<&gix::commitgraph::Graph>,
        shallow: &HashSet<ObjectId>,
        states: &mut Vec<WalkState>,
        queue: &mut gix::revwalk::PriorityQueue<gix::date::SecondsSinceUnixEpoch, CommitIndex>,
        buf: &mut Vec<u8>,
        id: ObjectId,
        flags: u8,
    ) -> Result<()> {
        let index = self.ensure_commit(repo, cache, shallow, id, buf)?;
        states.resize(self.commits.len(), WalkState::default());
        let state = &mut states[index.as_usize()];
        if state.flags & flags != flags {
            state.flags |= flags;
            queue.insert(self.commits[index.as_usize()].commit_time, index);
        }
        Ok(())
    }

    pub(crate) fn selection_refs(&self, id: ObjectId, decorations: &Decorations) -> Vec<SelectionRef> {
        let tracked = self.index(id).and_then(|index| self.tracking.get(&index));
        let mut refs: Vec<_> = decorations
            .get(&id)
            .into_iter()
            .flatten()
            .map(|decoration| {
                let upstream = if matches!(
                    decoration.kind,
                    DecorationKind::Local
                        | DecorationKind::HeadPinBranch
                        | DecorationKind::CurrentWorktreeBranch
                        | DecorationKind::WorktreeBranch
                ) {
                    tracked
                        .into_iter()
                        .flatten()
                        .find(|reference| reference.name == decoration.name)
                        .and_then(|reference| reference.upstream)
                } else {
                    None
                };
                SelectionRef {
                    name: decoration.name.clone(),
                    upstream,
                }
            })
            .collect();
        refs.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.upstream.cmp(&b.upstream)));
        refs
    }

    pub(crate) fn selection_relation(
        &mut self,
        id: ObjectId,
        refs: &[SelectionRef],
        hidden: &[ObjectId],
    ) -> Option<crate::app::SelectionRelation> {
        let has_upstream = refs.iter().any(|reference| reference.upstream.is_some());
        for upstream in refs.iter().filter_map(|reference| reference.upstream.flatten()) {
            let pair = self.index(id).zip(self.index(upstream))?;
            let relation = if let Some(relation) = self.relations.get(&pair).copied() {
                Some(relation)
            } else {
                let relation = self.paint(id, std::slice::from_ref(&upstream))?;
                self.relations.insert(pair, relation);
                Some(relation)
            };
            if let Some((ahead, behind)) = relation {
                return Some(crate::app::SelectionRelation::Tracking { ahead, behind });
            }
        }
        if has_upstream || refs.is_empty() || hidden.is_empty() {
            return None;
        }
        self.paint(id, hidden)
            .map(|(visible, _)| crate::app::SelectionRelation::Visible(visible))
    }

    pub(crate) fn hidden_branch_updates(
        &self,
        view_tips: &[ObjectId],
        hidden_tips: impl IntoIterator<Item = ObjectId>,
    ) -> HashMap<ObjectId, (usize, ObjectId)> {
        let hidden_tips: HashSet<_> = hidden_tips.into_iter().collect();
        let mut out: HashMap<ObjectId, (usize, ObjectId)> = HashMap::new();
        for tip in hidden_tips {
            let Some((ahead, _, bases)) = self.paint_with_bases(tip, view_tips) else {
                continue;
            };
            if ahead == 0 {
                continue;
            }
            for base in bases {
                out.entry(base)
                    .and_modify(|previous| *previous = (*previous).max((ahead, tip)))
                    .or_insert((ahead, tip));
            }
        }
        out
    }

    fn paint(&self, first: ObjectId, others: &[ObjectId]) -> Option<(usize, usize)> {
        self.paint_inner(first, others, false)
            .map(|(ahead, behind, _)| (ahead, behind))
    }

    fn paint_with_bases(&self, first: ObjectId, others: &[ObjectId]) -> Option<(usize, usize, Vec<ObjectId>)> {
        self.paint_inner(first, others, true)
    }

    fn paint_inner(
        &self,
        first: ObjectId,
        others: &[ObjectId],
        collect_bases: bool,
    ) -> Option<(usize, usize, Vec<ObjectId>)> {
        let first = self.index(first)?;
        let others: Vec<_> = others.iter().map(|id| self.index(*id)).collect::<Option<_>>()?;
        let mut flags = vec![0u8; self.commits.len()];
        let mut queue = gix::revwalk::PriorityQueue::<GenThenTime, CommitIndex>::new();
        let mut queued = vec![false; self.commits.len()];
        let mut pending = 0usize;
        let mut bases = Vec::new();
        for (index, flag) in std::iter::once((first, VISIBLE)).chain(others.into_iter().map(|index| (index, HIDDEN))) {
            flags[index.as_usize()] |= flag;
            if !queued[index.as_usize()] {
                queued[index.as_usize()] = true;
                queue.insert(GenThenTime::from(&self.commits[index.as_usize()]), index);
                pending += 1;
            }
        }
        while pending != 0 {
            let Some((_priority, index)) = queue.pop() else { break };
            queued[index.as_usize()] = false;
            let mut propagated = flags[index.as_usize()];
            if propagated & STALE == 0 {
                pending -= 1;
            }
            if propagated & (VISIBLE | HIDDEN) == VISIBLE | HIDDEN {
                if collect_bases && propagated & STALE == 0 {
                    bases.push(self.id(index));
                }
                propagated |= STALE;
                flags[index.as_usize()] = propagated;
            }
            for &parent in self.parents(index) {
                let parent_flags = &mut flags[parent.as_usize()];
                let previous = *parent_flags;
                if previous & propagated != propagated {
                    *parent_flags = previous | propagated;
                    if queued[parent.as_usize()] {
                        if previous & STALE == 0 && *parent_flags & STALE != 0 {
                            pending -= 1;
                        }
                    } else {
                        queued[parent.as_usize()] = true;
                        if *parent_flags & STALE == 0 {
                            pending += 1;
                        }
                        queue.insert(GenThenTime::from(&self.commits[parent.as_usize()]), parent);
                    }
                }
            }
        }
        let mut ahead = 0;
        let mut behind = 0;
        for flags in flags {
            match flags & (VISIBLE | HIDDEN) {
                VISIBLE => ahead += 1,
                HIDDEN => behind += 1,
                _ => {}
            }
        }
        if collect_bases && bases.len() > 1 {
            let candidates = bases.clone();
            bases.retain(|candidate| {
                !candidates
                    .iter()
                    .any(|other| candidate != other && self.is_ancestor(*candidate, *other))
            });
        }
        Some((ahead, behind, bases))
    }

    pub(crate) fn refresh(
        &mut self,
        repo: &gix::Repository,
        revisions: &[OsString],
        hidden_revisions: &[OsString],
        include_worktrees: bool,
        expand: &HashSet<ObjectId>,
        authors: &SharedAuthors,
    ) -> Result<Refresh> {
        let refs = snapshot(repo, revisions, hidden_revisions, include_worktrees)?;
        let hidden_only = refs.view_tips.is_empty() && !refs.hidden_tips.is_empty();
        let shallow: HashSet<_> = repo
            .shallow_commits()
            .context("could not read shallow commits")?
            .into_iter()
            .flat_map(|commits| commits.iter().copied().collect::<Vec<_>>())
            .collect();
        let cache = repo
            .commit_graph_if_enabled()
            .context("could not open commit-graph for history refresh")?;
        let local_refs = local_refs_by_target(repo)?;
        let mut tracking = HashMap::new();
        let mut states = vec![WalkState::default(); self.commits.len()];
        let mut queue = gix::revwalk::PriorityQueue::new();
        let mut buf = Vec::new();
        for id in refs.view_tips.iter().chain(&refs.hidden_tips).copied() {
            self.schedule_cached(
                repo,
                cache.as_ref(),
                &shallow,
                &mut states,
                &mut queue,
                &mut buf,
                id,
                VISIBLE,
            )?;
        }
        for &id in expand {
            self.schedule_cached(
                repo,
                cache.as_ref(),
                &shallow,
                &mut states,
                &mut queue,
                &mut buf,
                id,
                EXPAND,
            )?;
        }
        for (&id, names) in &local_refs {
            let Some(index) = self.index(id) else { continue };
            if self.commits[index.as_usize()].state & NODE_STORED == 0 {
                continue;
            }
            let tracked = resolve_tracking(repo, names)?;
            if tracked.iter().any(|reference| reference.upstream.flatten().is_some()) {
                self.schedule_cached(
                    repo,
                    cache.as_ref(),
                    &shallow,
                    &mut states,
                    &mut queue,
                    &mut buf,
                    id,
                    INTERNAL,
                )?;
            }
            for upstream in tracked.iter().filter_map(|reference| reference.upstream.flatten()) {
                self.schedule_cached(
                    repo,
                    cache.as_ref(),
                    &shallow,
                    &mut states,
                    &mut queue,
                    &mut buf,
                    upstream,
                    INTERNAL,
                )?;
            }
            tracking.insert(index, tracked);
        }

        let mut rows = Vec::new();
        let mut attributions = Vec::new();
        while let Some((_time, index)) = queue.pop() {
            let state = &mut states[index.as_usize()];
            let delta = state.flags & !state.expanded;
            if delta == 0 {
                continue;
            }
            state.expanded |= delta;
            let id = self.id(index);
            let commit = &self.commits[index.as_usize()];
            let was_stored = commit.state & NODE_STORED != 0;
            let should_store = delta & (VISIBLE | EXPAND) != 0 && !was_stored;
            let stop = !should_store
                && commit.state & NODE_COMPLETE != 0
                && (delta & EXPAND == 0 || was_stored && !expand.contains(&id));
            let parent_indices = self.parents(index).to_vec();
            let parent_ids = self.parent_ids(index);
            let generation = commit.generation();
            if should_store {
                if let Some(names) = local_refs.get(&id) {
                    let tracked = resolve_tracking(repo, names)?;
                    if tracked.iter().any(|reference| reference.upstream.flatten().is_some()) {
                        self.schedule_cached(
                            repo,
                            cache.as_ref(),
                            &shallow,
                            &mut states,
                            &mut queue,
                            &mut buf,
                            id,
                            INTERNAL,
                        )?;
                    }
                    for upstream in tracked.iter().filter_map(|reference| reference.upstream.flatten()) {
                        self.schedule_cached(
                            repo,
                            cache.as_ref(),
                            &shallow,
                            &mut states,
                            &mut queue,
                            &mut buf,
                            upstream,
                            INTERNAL,
                        )?;
                    }
                    tracking.insert(index, tracked);
                }
                let metadata = if generation.is_some() {
                    None
                } else {
                    let object = repo.find_commit(id).context("could not read refreshed commit")?;
                    let mut authors = gix::features::threading::lock(authors);
                    Some(decode_metadata(object.iter(), &mut authors, &mut attributions)?)
                };
                let metadata_loaded = metadata.is_some();
                let Metadata {
                    committer_time,
                    author_time,
                    author,
                    attributions: row_attributions,
                    title,
                    has_agent_marker,
                    is_review,
                    signature,
                } = metadata.unwrap_or_else(|| Metadata {
                    committer_time: Default::default(),
                    author_time: Default::default(),
                    author: &EMPTY_AUTHOR,
                    attributions: 0..0,
                    title: BString::default(),
                    has_agent_marker: false,
                    is_review: false,
                    signature: SignatureState::Unsigned,
                });
                rows.push(Commit {
                    id,
                    parent_ids: parent_ids.clone(),
                    committer_time,
                    author_time,
                    author,
                    attributions: row_attributions,
                    title,
                    metadata_loaded,
                    has_agent_marker,
                    is_review,
                    signature,
                });
                self.commits[index.as_usize()].state |= NODE_STORED;
                self.stored_order.push(index);
            }
            if stop {
                continue;
            }
            if hidden_only {
                continue;
            }
            for parent in parent_indices {
                self.schedule_cached(
                    repo,
                    cache.as_ref(),
                    &shallow,
                    &mut states,
                    &mut queue,
                    &mut buf,
                    self.id(parent),
                    delta & (VISIBLE | INTERNAL | EXPAND),
                )?;
            }
        }
        for (index, state) in states.into_iter().enumerate() {
            if state.expanded & (VISIBLE | INTERNAL | EXPAND) != 0 {
                self.commits[index].state |= NODE_COMPLETE;
            }
        }
        self.tracking = tracking;
        self.set_current_view(if hidden_only {
            &refs.hidden_tips
        } else {
            &refs.view_tips
        });
        let decorations = decorations(repo, &refs.pins, &refs.worktrees)?;
        Ok(Refresh {
            refs,
            decorations,
            commits: LoadedCommits { rows, attributions },
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RefSnapshot {
    pub view: HashMap<BString, gix::refs::Target>,
    pub hidden: HashMap<BString, gix::refs::Target>,
    pub view_tips: Vec<ObjectId>,
    pub hidden_tips: Vec<ObjectId>,
    pub pins: Vec<Pin>,
    pub worktrees: Vec<WorktreeCheckout>,
}

#[derive(Debug)]
pub(crate) struct Refresh {
    pub refs: RefSnapshot,
    pub decorations: Decorations,
    pub commits: LoadedCommits,
}
#[derive(Default)]
pub(crate) struct Authors {
    strings: HashSet<&'static [u8]>,
    authors: HashMap<(&'static BStr, &'static BStr), &'static Author>,
}
const COMMIT_BATCH_SIZE: usize = 1024;
const VISIBLE: u8 = 1 << 0;
const INTERNAL: u8 = 1 << 1;
const HIDDEN: u8 = 1 << 2;
const STALE: u8 = 1 << 3;
const EXPAND: u8 = 1 << 4;
const NODE_LOADED: u8 = 1 << 0;
const NODE_COMPLETE: u8 = 1 << 1;
const NODE_STORED: u8 = 1 << 2;
const NODE_IN_VIEW: u8 = 1 << 3;

#[derive(Clone, Copy, Default)]
struct WalkState {
    flags: u8,
    expanded: u8,
}

#[expect(clippy::too_many_arguments)]
fn schedule(
    graph: &mut HistoryGraph,
    repo: &gix::Repository,
    cache: Option<&gix::commitgraph::Graph>,
    states: &mut Vec<Node>,
    queue: &mut gix::revwalk::PriorityQueue<gix::date::SecondsSinceUnixEpoch, CommitIndex>,
    shallow: &HashSet<ObjectId>,
    buf: &mut Vec<u8>,
    id: ObjectId,
    flags: u8,
) -> Result<()> {
    let index = graph.ensure_commit(repo, cache, shallow, id, buf)?;
    states.resize(graph.commits.len(), Node::default());
    let state = &mut states[index.as_usize()];
    if state.flags & flags != flags {
        state.flags |= flags;
        queue.insert(graph.commits[index.as_usize()].commit_time, index);
    }
    Ok(())
}

fn hidden_frontier(
    graph: &mut HistoryGraph,
    repo: &gix::Repository,
    cache: Option<&gix::commitgraph::Graph>,
    visible_tips: &[ObjectId],
    hidden_tips: &[ObjectId],
    shallow: &HashSet<ObjectId>,
) -> Result<HashSet<ObjectId>> {
    if hidden_tips.is_empty() {
        return Ok(HashSet::new());
    }
    let mut flags = Vec::<u8>::new();
    let mut queue = gix::revwalk::PriorityQueue::<GenThenTime, CommitIndex>::new();
    let mut buf = Vec::new();
    for (tips, flag) in [(visible_tips, VISIBLE), (hidden_tips, HIDDEN)] {
        for &id in tips {
            let index = graph.ensure_commit(repo, cache, shallow, id, &mut buf)?;
            flags.resize(graph.commits.len(), 0);
            flags[index.as_usize()] |= flag;
            queue.insert(GenThenTime::from(&graph.commits[index.as_usize()]), index);
        }
    }
    while queue.iter_unordered().any(|index| flags[index.as_usize()] & STALE == 0) {
        let Some((_priority, index)) = queue.pop() else { break };
        let mut propagated = flags[index.as_usize()];
        if propagated & (VISIBLE | HIDDEN) == VISIBLE | HIDDEN {
            propagated |= STALE;
            flags[index.as_usize()] = propagated;
        }
        let parents = graph.parents(index).to_vec();
        for parent in parents {
            let parent_id = graph.id(parent);
            let parent = graph.ensure_commit(repo, cache, shallow, parent_id, &mut buf)?;
            flags.resize(graph.commits.len(), 0);
            let parent_flags = &mut flags[parent.as_usize()];
            if *parent_flags & propagated != propagated {
                *parent_flags |= propagated;
                queue.insert(GenThenTime::from(&graph.commits[parent.as_usize()]), parent);
            }
        }
    }
    Ok(flags
        .into_iter()
        .enumerate()
        .filter(|(_, flags)| flags & (VISIBLE | HIDDEN) == VISIBLE | HIDDEN)
        .map(|(index, _)| graph.id(CommitIndex(index as u32)))
        .collect())
}

fn local_refs_by_target(repo: &gix::Repository) -> Result<HashMap<ObjectId, Vec<BString>>> {
    let mut out = HashMap::<ObjectId, Vec<BString>>::new();
    let platform = repo.references().context("could not open references")?;
    let refs = platform
        .local_branches()
        .context("could not iterate local branches")?
        .peeled()
        .context("could not prepare local branches for peeling")?;
    for reference in refs {
        let reference = match reference {
            Ok(reference) => reference,
            Err(err) if is_missing_ref(&*err) => continue,
            Err(err) => return Err(anyhow::anyhow!("could not read local branch: {err}")),
        };
        out.entry(reference.id().detach())
            .or_default()
            .push(reference.name().as_bstr().to_owned());
    }
    Ok(out)
}

fn resolve_tracking(repo: &gix::Repository, names: &[BString]) -> Result<Vec<SelectionRef>> {
    let mut out = Vec::with_capacity(names.len());
    for full_name in names {
        let Some(reference) = repo
            .try_find_reference(full_name.as_bstr())
            .with_context(|| format!("could not read local branch {full_name}"))?
        else {
            continue;
        };
        let upstream = reference
            .remote_tracking_ref_name(gix::remote::Direction::Fetch)
            .map(|name| {
                let name = name.context("could not resolve remote-tracking branch name")?;
                Ok::<_, anyhow::Error>(
                    repo.try_find_reference(name.as_bstr())
                        .with_context(|| format!("could not read remote-tracking branch {name}"))?
                        .and_then(|mut reference| reference.peel_to_id().ok().map(gix::Id::detach)),
                )
            })
            .transpose()?;
        out.push(SelectionRef {
            name: full_name
                .strip_prefix(b"refs/heads/")
                .unwrap_or(full_name.as_slice())
                .into(),
            upstream,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.upstream.cmp(&b.upstream)));
    out.dedup();
    Ok(out)
}

#[derive(Debug)]
pub(crate) enum Event {
    Decorations(Decorations),
    Commits(LoadedCommits),
    HiddenCommits(LoadedCommits),
    VisibleComplete,
    Complete(HistoryGraph),
    Cancelled,
}

pub(crate) fn load(
    repo: &gix::Repository,
    revisions: &[OsString],
    hidden_revisions: &[OsString],
    include_worktrees: bool,
    authors: &SharedAuthors,
    cancelled: &AtomicBool,
    mut emit: impl FnMut(Event) -> bool,
) -> Result<()> {
    let refs = snapshot(repo, revisions, hidden_revisions, include_worktrees)?;
    let tips = refs.view_tips;
    let hidden_tips = refs.hidden_tips;

    if !emit(Event::Decorations(decorations(repo, &refs.pins, &refs.worktrees)?)) {
        return Ok(());
    }
    if tips.is_empty() && hidden_tips.is_empty() {
        emit(Event::VisibleComplete);
        emit(Event::Complete(HistoryGraph::default()));
        return Ok(());
    }
    let shallow: HashSet<_> = repo
        .shallow_commits()
        .context("could not read shallow commits")?
        .into_iter()
        .flat_map(|commits| commits.iter().copied().collect::<Vec<_>>())
        .collect();
    let commit_graph = repo
        .commit_graph_if_enabled()
        .context("could not open commit-graph for history traversal")?;
    let mut graph = HistoryGraph::default();
    if tips.is_empty() {
        let mut rows = Vec::with_capacity(hidden_tips.len());
        let mut attributions = Vec::new();
        let mut authors = gix::features::threading::lock(authors);
        let mut buf = Vec::new();
        for &id in &hidden_tips {
            if cancelled.load(Ordering::Relaxed) {
                emit(Event::Cancelled);
                return Ok(());
            }
            let index = graph.ensure_commit(repo, commit_graph.as_ref(), &shallow, id, &mut buf)?;
            if graph.commits[index.as_usize()].state & NODE_STORED != 0 {
                continue;
            }
            rows.push(decode_commit(repo, id, &mut authors, &mut attributions)?);
            graph.commits[index.as_usize()].state |= NODE_STORED;
            graph.stored_order.push(index);
        }
        drop(authors);
        if !rows.is_empty() && !emit(Event::HiddenCommits(LoadedCommits { rows, attributions })) {
            return Ok(());
        }
        emit(Event::VisibleComplete);
        graph.set_current_view(&hidden_tips);
        emit(Event::Complete(graph));
        return Ok(());
    }
    let hidden = hidden_frontier(&mut graph, repo, commit_graph.as_ref(), &tips, &hidden_tips, &shallow)?;
    let local_refs = local_refs_by_target(repo)?;
    let mut tracking = HashMap::new();
    let mut states = vec![Node::default(); graph.commits.len()];
    let mut queue = gix::revwalk::PriorityQueue::new();
    let mut buf = Vec::new();
    for &tip in &tips {
        schedule(
            &mut graph,
            repo,
            commit_graph.as_ref(),
            &mut states,
            &mut queue,
            &shallow,
            &mut buf,
            tip,
            VISIBLE,
        )?;
    }
    let mut rows = Vec::with_capacity(COMMIT_BATCH_SIZE);
    let mut attributions = Vec::with_capacity(COMMIT_BATCH_SIZE);
    let mut connected = Vec::new();
    let mut connected_seen = HashSet::new();
    while let Some((_time, index)) = queue.pop() {
        if cancelled.load(Ordering::Relaxed) {
            emit(Event::Cancelled);
            return Ok(());
        }
        let id = graph.id(index);
        let (delta, should_emit) = {
            let state = &mut states[index.as_usize()];
            let delta = state.flags & !state.expanded;
            if delta == 0 {
                continue;
            }
            state.expanded |= delta;
            let should_emit = delta & VISIBLE != 0 && !state.emitted && !hidden.contains(&id);
            state.emitted |= should_emit;
            (delta, should_emit)
        };
        graph.commits[index.as_usize()].state |= NODE_COMPLETE;
        let parent_indices = graph.parents(index).to_vec();
        let parent_ids = graph.parent_ids(index);
        let generation = graph.commits[index.as_usize()].generation();
        if should_emit && let Some(names) = local_refs.get(&id) {
            let refs = resolve_tracking(repo, names)?;
            if refs.iter().any(|reference| reference.upstream.flatten().is_some()) {
                schedule(
                    &mut graph,
                    repo,
                    commit_graph.as_ref(),
                    &mut states,
                    &mut queue,
                    &shallow,
                    &mut buf,
                    id,
                    INTERNAL,
                )?;
            }
            for upstream in refs.iter().filter_map(|reference| reference.upstream.flatten()) {
                schedule(
                    &mut graph,
                    repo,
                    commit_graph.as_ref(),
                    &mut states,
                    &mut queue,
                    &shallow,
                    &mut buf,
                    upstream,
                    INTERNAL,
                )?;
            }
            tracking.insert(index, refs);
        }
        let metadata = if !should_emit || generation.is_some() {
            None
        } else {
            let object = repo.find_commit(id).context("could not read commit")?;
            let mut authors = gix::features::threading::lock(authors);
            Some(decode_metadata(object.iter(), &mut authors, &mut attributions)?)
        };
        if should_emit {
            let metadata_loaded = metadata.is_some();
            let Metadata {
                committer_time,
                author_time,
                author,
                attributions: row_attributions,
                title,
                has_agent_marker,
                is_review,
                signature,
            } = metadata.unwrap_or_else(|| Metadata {
                committer_time: Default::default(),
                author_time: Default::default(),
                author: &EMPTY_AUTHOR,
                attributions: 0..0,
                title: BString::default(),
                has_agent_marker: false,
                is_review: false,
                signature: SignatureState::Unsigned,
            });
            if !hidden_revisions.is_empty() {
                connected.extend(parent_ids.iter().copied().filter(|id| connected_seen.insert(*id)));
            }
            rows.push(Commit {
                id,
                parent_ids: parent_ids.clone(),
                committer_time,
                author_time,
                author,
                attributions: row_attributions,
                title,
                metadata_loaded,
                has_agent_marker,
                is_review,
                signature,
            });
            graph.commits[index.as_usize()].state |= NODE_STORED;
            graph.stored_order.push(index);
            if rows.len() == COMMIT_BATCH_SIZE
                && !emit(Event::Commits(LoadedCommits {
                    rows: std::mem::replace(&mut rows, Vec::with_capacity(COMMIT_BATCH_SIZE)),
                    attributions: std::mem::replace(&mut attributions, Vec::with_capacity(COMMIT_BATCH_SIZE)),
                }))
            {
                return Ok(());
            }
        }
        let propagated = if hidden.contains(&id) {
            delta & INTERNAL
        } else {
            delta & (VISIBLE | INTERNAL)
        };
        for parent in parent_indices {
            let parent_id = graph.id(parent);
            let parent_flags = if hidden.contains(&parent_id) {
                propagated & !VISIBLE
            } else {
                propagated
            };
            if parent_flags != 0 {
                schedule(
                    &mut graph,
                    repo,
                    commit_graph.as_ref(),
                    &mut states,
                    &mut queue,
                    &shallow,
                    &mut buf,
                    parent_id,
                    parent_flags,
                )?;
            }
        }
    }
    if !rows.is_empty() && !emit(Event::Commits(LoadedCommits { rows, attributions })) {
        return Ok(());
    }
    if graph.stored_order.is_empty() {
        connected.extend(
            tips.iter()
                .copied()
                .filter(|commit_id| connected_seen.insert(*commit_id)),
        );
    }
    if !hidden_revisions.is_empty() {
        connected.retain(|id| graph.index(*id).is_none_or(|index| !states[index.as_usize()].emitted));
        let mut rows = Vec::with_capacity(connected.len());
        let mut attributions = Vec::new();
        let mut authors = gix::features::threading::lock(authors);
        for id in connected {
            if cancelled.load(Ordering::Relaxed) {
                emit(Event::Cancelled);
                return Ok(());
            }
            rows.push(decode_commit(repo, id, &mut authors, &mut attributions)?);
            let index = graph.ensure_commit(repo, commit_graph.as_ref(), &shallow, id, &mut buf)?;
            if graph.commits[index.as_usize()].state & NODE_STORED == 0 {
                graph.commits[index.as_usize()].state |= NODE_STORED;
                graph.stored_order.push(index);
            }
        }
        if !rows.is_empty() && !emit(Event::HiddenCommits(LoadedCommits { rows, attributions })) {
            return Ok(());
        }
    }
    emit(Event::VisibleComplete);
    graph.tracking = tracking;
    graph.set_current_view(&tips);
    emit(Event::Complete(graph));
    Ok(())
}

pub(crate) fn snapshot(
    repo: &gix::Repository,
    revisions: &[OsString],
    hidden: &[OsString],
    include_worktrees: bool,
) -> Result<RefSnapshot> {
    snapshot_ignoring_pin(repo, revisions, hidden, include_worktrees, None)
}

pub(crate) fn ref_tree_revisions(repo: &gix::Repository, include_tags: bool) -> Result<Vec<OsString>> {
    let mut out = Vec::new();
    for reference in repo
        .references()
        .context("could not open references")?
        .all()
        .context("could not iterate references")?
    {
        let mut reference = match reference {
            Ok(reference) => reference,
            Err(err) if is_missing_ref(&*err) => continue,
            Err(err) => return Err(anyhow::anyhow!("could not read reference: {err}")),
        };
        let name = reference.name().as_bstr().to_owned();
        let kind = decoration_kind(&name);
        if matches!(kind, DecorationKind::Special | DecorationKind::Pin)
            || matches!(kind, DecorationKind::Tag) && !include_tags
            || name.starts_with(STASH_PREFIX)
            || name.starts_with(REVIEW_STASH_PREFIX)
        {
            continue;
        }
        let Ok(id) = reference.peel_to_id() else { continue };
        if repo.find_header(id)?.kind() != gix::object::Kind::Commit {
            continue;
        }
        out.push(gix::path::from_bstr(&name).into_owned().into_os_string());
    }
    if repo.head().is_ok_and(|head| head.referent_name().is_none()) {
        out.push("HEAD".into());
    }
    Ok(out)
}

pub(crate) fn snapshot_ignoring_pin(
    repo: &gix::Repository,
    revisions: &[OsString],
    hidden: &[OsString],
    include_worktrees: bool,
    ignored_pin: Option<&BStr>,
) -> Result<RefSnapshot> {
    let pins = applicable_pins(repo)?
        .into_iter()
        .filter(|pin| ignored_pin != Some(pin.name.as_bstr()))
        .collect::<Vec<_>>();
    let worktrees = worktree_checkouts(repo);
    let mut view = referenced_refs(repo, revisions)?;
    for pin in &pins {
        insert_ref_chain(repo, pin.name.as_bstr(), &mut view)?;
    }
    let mut view_tips = resolve_tips(repo, revisions)?.unwrap_or_default();
    view_tips.extend(pins.iter().map(|pin| pin.id));
    if include_worktrees {
        for worktree in &worktrees {
            view_tips.extend([worktree.id, worktree.label_id]);
            if let Some(reference) = &worktree.reference {
                insert_ref_chain(repo, reference.as_bstr(), &mut view)?;
            }
        }
    }
    let mut seen = HashSet::new();
    view_tips.retain(|id| seen.insert(*id));
    Ok(RefSnapshot {
        view,
        hidden: referenced_refs(repo, hidden)?,
        view_tips,
        hidden_tips: resolve_revisions(repo, hidden, "hidden ")?,
        pins,
        worktrees,
    })
}

pub(crate) fn worktree_checkouts(repo: &gix::Repository) -> Vec<WorktreeCheckout> {
    let mut out = Vec::new();
    let current_worktree = repo.worktree().map(|worktree| worktree.id().map(ToOwned::to_owned));
    match repo.main_repo() {
        Ok(main) if !main.is_bare() => {
            let name = main.workdir().and_then(worktree_basename);
            add_worktree_checkout(&main, name, b"main".as_bstr(), current_worktree == Some(None), &mut out);
        }
        Ok(_) => {}
        Err(err) => tracing::warn!(error = %err, "ignoring inaccessible main worktree"),
    }
    match repo.worktrees() {
        Ok(worktrees) => {
            for proxy in worktrees {
                let worktree = proxy.id().to_owned();
                let name = proxy.base().ok().as_deref().and_then(worktree_basename);
                let is_current = current_worktree
                    .as_ref()
                    .and_then(Option::as_ref)
                    .is_some_and(|current| current == &worktree);
                match proxy.into_repo_with_possibly_inaccessible_worktree() {
                    Ok(repository) => {
                        add_worktree_checkout(&repository, name, worktree.as_bstr(), is_current, &mut out);
                    }
                    Err(err) => {
                        tracing::warn!(worktree = %worktree, error = %err, "ignoring inaccessible linked worktree");
                    }
                }
            }
        }
        Err(err) => tracing::warn!(error = %err, "ignoring unreadable linked worktree list"),
    }
    out.sort_by(|a, b| {
        a.id.cmp(&b.id)
            .then_with(|| a.label_id.cmp(&b.label_id))
            .then_with(|| a.checkout_name.cmp(&b.checkout_name))
            .then_with(|| a.reference.cmp(&b.reference))
    });
    out.dedup();
    out
}

fn add_worktree_checkout(
    repo: &gix::Repository,
    checkout_name: Option<BString>,
    worktree: &BStr,
    is_current: bool,
    out: &mut Vec<WorktreeCheckout>,
) {
    let mut head = match repo.head() {
        Ok(head) => head,
        Err(err) => {
            tracing::warn!(%worktree, error = %err, "ignoring worktree with unreadable HEAD");
            return;
        }
    };
    let is_detached = head.is_detached();
    let head_reference = head.referent_name().map(ToOwned::to_owned);
    let id = match head.try_peel_to_id() {
        Ok(Some(id)) => id.detach(),
        Ok(None) => return,
        Err(err) => {
            tracing::warn!(%worktree, error = %err, "ignoring worktree with unresolved HEAD");
            return;
        }
    };
    let remembered = is_detached
        .then(|| remembered_worktree_branch(repo, worktree))
        .flatten();
    let (reference, label_id) = remembered
        .or_else(|| head_reference.map(|reference| (reference, id)))
        .map_or((None, id), |(reference, id)| (Some(reference), id));
    let checkout_name = checkout_name.unwrap_or_else(|| worktree.to_owned());
    out.push(WorktreeCheckout {
        id,
        label_id,
        checkout_name,
        reference,
        is_current,
        is_detached,
    });
}

fn remembered_worktree_branch(repo: &gix::Repository, worktree: &BStr) -> Option<(gix::refs::FullName, ObjectId)> {
    let mut pin = match repo.try_find_reference(HEAD_PIN_NAME.as_bstr()) {
        Ok(Some(pin)) => pin,
        Ok(None) => return None,
        Err(err) => {
            tracing::warn!(%worktree, error = %err, "ignoring unreadable worktree HEAD pin");
            return None;
        }
    };
    let Some(reference) = pin.target().try_name().map(ToOwned::to_owned) else {
        tracing::warn!(%worktree, "ignoring direct worktree HEAD pin");
        return None;
    };
    if !reference.as_bstr().starts_with(b"refs/heads/") {
        tracing::warn!(%worktree, name = %reference, "ignoring non-branch worktree HEAD pin");
        return None;
    }
    let id = match pin.peel_to_id() {
        Ok(id) => id.detach(),
        Err(err) => {
            tracing::warn!(%worktree, name = %reference, error = %err, "ignoring unresolved worktree HEAD pin");
            return None;
        }
    };
    match repo.find_header(id) {
        Ok(header) if header.kind() == gix::object::Kind::Commit => Some((reference, id)),
        Ok(_) => {
            tracing::warn!(%worktree, name = %reference, "ignoring worktree HEAD pin with a non-commit target");
            None
        }
        Err(err) => {
            tracing::warn!(%worktree, name = %reference, error = %err, "ignoring unreadable worktree HEAD pin target");
            None
        }
    }
}

fn worktree_basename(path: &std::path::Path) -> Option<BString> {
    path.file_name()
        .and_then(|name| gix::path::os_str_into_bstr(name).ok())
        .map(ToOwned::to_owned)
}

pub(crate) fn all_pins(repo: &gix::Repository) -> Result<Vec<Pin>> {
    refs_with_commit_targets(repo, PIN_PREFIX, "pin")
}

pub(crate) fn all_reviews(repo: &gix::Repository) -> Result<Vec<Pin>> {
    refs_with_commit_targets(repo, REVIEW_PREFIX, "review")
}

pub(crate) fn review_number(name: &BStr) -> Option<&BStr> {
    let suffix = name.strip_prefix(REVIEW_PREFIX)?;
    (suffix.first().is_some_and(|digit| matches!(digit, b'1'..=b'9')) && suffix.iter().all(u8::is_ascii_digit))
        .then_some(suffix.as_bstr())
}

pub(crate) fn review_pin_number(name: &BStr) -> Option<&BStr> {
    let suffix = name.strip_prefix(REVIEW_PIN_PREFIX)?;
    (suffix.first().is_some_and(|digit| matches!(digit, b'1'..=b'9')) && suffix.iter().all(u8::is_ascii_digit))
        .then_some(suffix.as_bstr())
}

fn refs_with_commit_targets(repo: &gix::Repository, prefix: &[u8], label: &str) -> Result<Vec<Pin>> {
    let mut out = Vec::new();
    let references = repo.references().context("could not open references")?;
    for reference in references
        .prefixed(prefix.as_bstr())
        .with_context(|| format!("could not iterate tix {label}s"))?
    {
        let mut reference = match reference {
            Ok(reference) => reference,
            Err(err) if is_missing_ref(&*err) => continue,
            Err(err) => return Err(anyhow::anyhow!("could not read tix {label}: {err}")),
        };
        let suffix = reference.name().as_bstr().strip_prefix(prefix).unwrap_or_default();
        let valid_suffix = if prefix == REVIEW_PREFIX {
            review_number(reference.name().as_bstr()).is_some()
        } else {
            review_pin_number(reference.name().as_bstr()).is_some()
                || suffix.len() >= 4 && suffix.iter().all(u8::is_ascii_alphanumeric)
        };
        if !valid_suffix {
            tracing::warn!(name = %reference.name(), %label, "ignoring malformed tix reference");
            continue;
        }
        let name = reference.name().to_owned();
        let target = reference.target().into_owned();
        if let Some(target_name) = target.try_name()
            && crate::edit::undo::ref_chain_reaches_queue(repo, target_name)?
        {
            tracing::warn!(name = %name, %label, "ignoring tix reference into the undo queue");
            continue;
        }
        if name.as_bstr() == HEAD_PIN_NAME
            && !target
                .try_name()
                .is_some_and(|name| name.as_bstr().starts_with(b"refs/heads/"))
        {
            tracing::warn!(name = %name, "ignoring malformed HEAD pin");
            continue;
        }
        let id = match reference.peel_to_id() {
            Ok(id) => id.detach(),
            Err(err) => {
                tracing::warn!(name = %name, error = %err, %label, "ignoring unresolved tix reference");
                continue;
            }
        };
        if crate::edit::undo::is_queue_commit(repo, id)? {
            tracing::warn!(name = %name, %label, "ignoring tix reference to an undo queue commit");
            continue;
        }
        match repo.find_header(id) {
            Ok(header) if header.kind() == gix::object::Kind::Commit => {}
            Ok(_) => {
                tracing::warn!(name = %name, %label, "ignoring tix reference that does not resolve to a commit");
                continue;
            }
            Err(err) => {
                tracing::warn!(name = %name, error = %err, %label, "ignoring unreadable tix reference target");
                continue;
            }
        }
        out.push(Pin { name, target, id });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

pub(crate) fn applicable_pins(repo: &gix::Repository) -> Result<Vec<Pin>> {
    let head = repo.head().context("could not read HEAD while resolving tix pins")?;
    let detached = head.is_detached();
    if head.id().is_none() {
        return Ok(Vec::new());
    }
    drop(head);
    let pins = all_pins(repo)?;
    if detached {
        return Ok(pins);
    }
    Ok(pins.into_iter().filter(|pin| !pin.is_head()).collect())
}

pub(crate) fn referenced_refs(
    repo: &gix::Repository,
    revisions: &[OsString],
) -> Result<HashMap<BString, gix::refs::Target>> {
    if revisions.is_empty() && repo.head()?.is_unborn() {
        return Ok(HashMap::new());
    }
    let implicit_head = OsString::from("HEAD");
    let revisions = if revisions.is_empty() {
        std::slice::from_ref(&implicit_head)
    } else {
        revisions
    };
    let mut out = HashMap::new();
    for revision in revisions {
        let revision = gix::path::os_str_into_bstr(revision)
            .with_context(|| format!("revision {} is not valid UTF-8", revision.to_string_lossy()))?;
        let spec = repo
            .rev_parse(revision)
            .with_context(|| format!("could not parse revision {revision}"))?;
        for reference in [spec.first_reference(), spec.second_reference()].into_iter().flatten() {
            anyhow::ensure!(
                !crate::edit::undo::ref_chain_reaches_queue(repo, reference.name.as_ref())?,
                "the undo queue is not a selectable revision"
            );
            insert_ref_chain(repo, reference.name.as_bstr(), &mut out)?;
        }
    }
    Ok(out)
}

fn insert_ref_chain(repo: &gix::Repository, name: &BStr, out: &mut HashMap<BString, gix::refs::Target>) -> Result<()> {
    let mut name = name.to_owned();
    loop {
        if out.contains_key(&name) {
            return Ok(());
        }
        let reference = match repo.try_find_reference(name.as_bstr()) {
            Ok(reference) => reference,
            Err(err) if is_missing_ref(&err) => return Ok(()),
            Err(err) => return Err(err).with_context(|| format!("could not read reference {name}")),
        };
        let Some(reference) = reference else {
            return Ok(());
        };
        let target = reference.target().into_owned();
        let next = target.try_name().map(|name| name.as_bstr().to_owned());
        out.insert(name, target);
        let Some(next) = next else { return Ok(()) };
        name = next;
    }
}

pub(crate) fn load_metadata(
    repo: &gix::Repository,
    id: ObjectId,
    authors: &SharedAuthors,
) -> Result<(Metadata<BString>, Vec<Attribution>)> {
    let object = repo.find_commit(id).context("could not read commit")?;
    let mut attributions = Vec::new();
    let mut authors = gix::features::threading::lock(authors);
    let metadata = decode_metadata(object.iter(), &mut authors, &mut attributions)?;
    Ok((metadata, attributions))
}

fn decode_commit(
    repo: &gix::Repository,
    id: ObjectId,
    authors: &mut Authors,
    attributions: &mut Vec<Attribution>,
) -> Result<Commit<BString>> {
    let object = repo.find_commit(id).context("could not read commit")?;
    let parent_ids = object.parent_ids().map(gix::Id::detach).collect();
    let Metadata {
        committer_time,
        author_time,
        author,
        attributions: row_attributions,
        title,
        has_agent_marker,
        is_review,
        signature,
    } = decode_metadata(object.iter(), authors, attributions)?;
    Ok(Commit {
        id,
        parent_ids,
        committer_time,
        author_time,
        author,
        attributions: row_attributions,
        title,
        metadata_loaded: true,
        has_agent_marker,
        is_review,
        signature,
    })
}

fn decode_metadata<'a>(
    tokens: impl Iterator<Item = Result<Token<'a>, gix::objs::decode::Error>>,
    authors: &mut Authors,
    attributions: &mut Vec<Attribution>,
) -> Result<Metadata<BString>> {
    let mut committer_time = None;
    let mut author_time = None;
    let mut author = None;
    let attribution_start = attributions.len();
    let mut title = None;
    let mut has_agent_marker = false;
    let mut is_review = false;
    let mut signature = SignatureState::Unsigned;
    for token in tokens {
        match token.context("could not decode commit")? {
            Token::Author { signature } => {
                author_time = Some(signature.time().context("could not decode author time")?);
                let signature = signature.trim();
                author = Some(authors.intern_author(signature.name, signature.email));
            }
            Token::Committer { signature } => {
                committer_time = Some(signature.time().context("could not decode committer time")?);
            }
            Token::Message(message) => {
                has_agent_marker = contains_agent_marker(message);
                let message = gix::objs::commit::MessageRef::from_bytes(message);
                title = Some(message.summary().into_owned());
                if let Some(body) = message.body() {
                    for trailer in body.trailers() {
                        let Some(kind) = attribution_kind(&trailer) else {
                            continue;
                        };
                        let mut value: &[u8] = trailer.value.as_ref();
                        let identity = match gix::actor::IdentityRef::from_bytes_consuming(&mut value) {
                            Ok(identity) if value.trim().is_empty() => identity.trim(),
                            _ if kind == AttributionKind::Assisted && !trailer.value.trim().is_empty() => {
                                gix::actor::IdentityRef {
                                    name: trailer.value.trim().as_bstr(),
                                    email: b"".as_bstr(),
                                }
                            }
                            _ => continue,
                        };
                        attributions.push(Attribution {
                            kind,
                            author: authors.intern_author(identity.name, identity.email),
                        });
                    }
                }
            }
            Token::ExtraHeader((name, _)) if name == "tix-rebase-parent" => {
                signature = SignatureState::PendingRebase;
            }
            Token::ExtraHeader((name, value))
                if name == "tix-rebase" && value.as_ref().starts_with(b"onto refs/worktree/tix/review/") =>
            {
                is_review = true;
            }
            Token::ExtraHeader((name, value)) if name == "gpgsig" || name == "gpgsig-sha256" => {
                if value.is_empty() {
                    signature = SignatureState::PendingRebase;
                } else if signature != SignatureState::PendingRebase {
                    signature = SignatureState::Unverified;
                }
            }
            _ => {}
        }
    }
    Ok(Metadata {
        committer_time: committer_time.context("commit has no committer time")?,
        author_time: author_time.context("commit has no author time")?,
        author: author.context("commit has no author")?,
        attributions: attribution_start..attributions.len(),
        title: title.context("commit has no message")?,
        has_agent_marker,
        is_review,
        signature,
    })
}

pub(crate) fn contains_agent_marker(message: &[u8]) -> bool {
    [b"--- agent".as_slice(), b"<!-- agent -->".as_slice()]
        .iter()
        .any(|marker| message.windows(marker.len()).any(|window| window == *marker))
}

fn resolve_tips(repo: &gix::Repository, revisions: &[OsString]) -> Result<Option<Vec<ObjectId>>> {
    if revisions.is_empty() {
        repo.head()
            .context("could not read HEAD")?
            .try_peel_to_id()
            .context("could not resolve HEAD")
            .map(|id| id.map(|id| vec![id.detach()]))
    } else {
        resolve_revisions(repo, revisions, "").map(Some)
    }
}

#[expect(
    clippy::type_complexity,
    reason = "both successful revisions and warning text are returned"
)]
pub(crate) fn available_hidden_revisions(
    repo: &gix::Repository,
    revisions: &[OsString],
    auto_hide: bool,
) -> Result<(Vec<OsString>, Vec<(OsString, String)>)> {
    let mut revisions = revisions.to_vec();
    if auto_hide {
        revisions.extend(auto_hidden_revisions(repo)?);
    }
    let mut available = Vec::new();
    let mut unavailable = Vec::new();
    let mut first_error = None;
    for revision in &revisions {
        match resolve_revisions(repo, std::slice::from_ref(revision), "hidden ") {
            Ok(_) => available.push(revision.clone()),
            Err(err) => {
                unavailable.push((revision.clone(), format!("{err:#}")));
                first_error.get_or_insert(err);
            }
        }
    }
    if available.is_empty()
        && let Some(err) = first_error
    {
        return Err(err);
    }
    Ok((available, unavailable))
}

fn auto_hidden_revisions(repo: &gix::Repository) -> Result<Vec<OsString>> {
    let mut branches = BTreeSet::new();
    for remote_name in repo.remote_names() {
        let mut head = BString::from("refs/remotes/");
        head.extend_from_slice(&remote_name);
        head.extend_from_slice(b"/HEAD");
        let Ok(Some(reference)) = repo.try_find_reference(head.as_bstr()) else {
            continue;
        };
        let target = reference.target();
        let Some(tracking) = target.try_name() else { continue };
        let Ok(Some((upstream, remote))) = repo.upstream_branch_and_remote_for_tracking_branch(tracking) else {
            continue;
        };
        if remote.name().map(gix::remote::Name::as_bstr) != Some(remote_name.as_bstr()) {
            continue;
        }
        let Ok(Some(mut local)) = repo.try_find_reference(upstream.as_ref()) else {
            continue;
        };
        let Ok(id) = local.peel_to_id() else { continue };
        if repo
            .find_header(id)
            .is_ok_and(|header| header.kind() == gix::object::Kind::Commit)
        {
            branches.insert(upstream);
        }
    }
    Ok(branches
        .into_iter()
        .map(|name| gix::path::from_bstr(name.as_bstr()).into_owned().into_os_string())
        .collect())
}

fn attribution_kind(trailer: &gix::objs::commit::message::body::TrailerRef<'_>) -> Option<AttributionKind> {
    if trailer.is_co_authored_by() {
        Some(AttributionKind::CoAuthor)
    } else if trailer.is_assisted_by() {
        Some(AttributionKind::Assisted)
    } else if trailer.is_reviewed_by() {
        Some(AttributionKind::Reviewed)
    } else if trailer.is_acked_by() {
        Some(AttributionKind::Acked)
    } else if trailer.is_tested_by() {
        Some(AttributionKind::Tested)
    } else if trailer.is_signed_off_by() {
        Some(AttributionKind::SignedOff)
    } else {
        None
    }
}

fn resolve_revisions(repo: &gix::Repository, revisions: &[OsString], kind: &str) -> Result<Vec<ObjectId>> {
    revisions
        .iter()
        .map(|revision| {
            let revision = gix::path::os_str_into_bstr(revision)
                .with_context(|| format!("{kind}revision {} is not valid UTF-8", revision.to_string_lossy()))?;
            resolve_revision(repo, revision)
                .with_context(|| format!("could not resolve {kind}revision {revision}"))
                .map(|(id, _reference)| id)
        })
        .collect()
}

pub(crate) fn resolve_revision(
    repo: &gix::Repository,
    revision: &BStr,
) -> Result<(ObjectId, Option<gix::refs::FullName>)> {
    let spec = repo.rev_parse(revision)?;
    let first_reference = spec.first_reference().map(|reference| reference.name.clone());
    for reference in [spec.first_reference(), spec.second_reference()].into_iter().flatten() {
        anyhow::ensure!(
            !crate::edit::undo::ref_chain_reaches_queue(repo, reference.name.as_ref())?,
            "the undo queue is not a selectable revision"
        );
    }
    let id = spec
        .single()
        .context("revision does not name a single object")?
        .object()
        .context("could not read revision")?
        .peel_to_kind(gix::object::Kind::Commit)
        .context("revision does not resolve to a commit")?
        .id;
    anyhow::ensure!(
        !crate::edit::undo::is_queue_commit(repo, id)?,
        "the undo queue is not a selectable revision"
    );
    Ok((id, first_reference))
}

impl Authors {
    fn intern_author(&mut self, name: &[u8], email: &[u8]) -> &'static Author {
        let name = self.intern_string(name);
        let email = self.intern_string(email);
        self.authors.entry((name, email)).or_insert_with(|| {
            let author: &'static Author = Box::leak(Box::new(Author { name, email }));
            author
        })
    }

    fn intern_string(&mut self, value: &[u8]) -> &'static BStr {
        match self.strings.get(value) {
            Some(value) => value.as_bstr(),
            None => {
                let value: &'static [u8] = Box::leak(value.to_vec().into_boxed_slice());
                self.strings.insert(value);
                value.as_bstr()
            }
        }
    }
}

pub(crate) fn decorations(repo: &gix::Repository, pins: &[Pin], worktrees: &[WorktreeCheckout]) -> Result<Decorations> {
    decorations_excluding(repo, pins, worktrees, &HashSet::new())
}

pub(crate) fn decorations_excluding(
    repo: &gix::Repository,
    pins: &[Pin],
    worktrees: &[WorktreeCheckout],
    excluded: &HashSet<BString>,
) -> Result<Decorations> {
    let mut out = Decorations::new();
    let head_pin_branch = pins
        .iter()
        .find(|pin| pin.is_head())
        .and_then(|pin| pin.target.try_name())
        .map(ToOwned::to_owned);
    let pins: HashSet<_> = pins.iter().map(|pin| pin.name.as_bstr()).collect();
    let review_count = all_reviews(repo)?.len();
    for reference in repo
        .references()
        .context("could not open references")?
        .all()
        .context("could not iterate references")?
    {
        let mut reference = match reference {
            Ok(reference) => reference,
            Err(err) if is_missing_ref(&*err) => continue,
            Err(err) => return Err(anyhow::anyhow!("could not read reference: {err}")),
        };
        let full_name = reference.name().to_owned();
        if excluded.contains(full_name.as_bstr()) || crate::edit::undo::is_queue_ref(full_name.as_bstr()) {
            continue;
        }
        if full_name.as_bstr() == HEAD_PIN_NAME {
            continue;
        }
        if full_name.as_bstr().starts_with(REVIEW_PIN_PREFIX) {
            continue;
        }
        if full_name.as_bstr().starts_with(REVIEW_STASH_PREFIX) {
            let leaf: Result<Option<ObjectId>> = (|| {
                let stash = repo.find_commit(reference.peel_to_id()?)?;
                Ok(stash.parent_ids().next().map(gix::Id::detach))
            })();
            let leaf = match leaf {
                Ok(leaf) => leaf,
                Err(err) => {
                    tracing::warn!(name = %full_name, error = %err, "ignored malformed review stash reference");
                    None
                }
            };
            if let Some(id) = leaf {
                out.entry(id).or_default().push(Decoration {
                    name: "stash".into(),
                    kind: DecorationKind::Stash,
                });
            }
            continue;
        }
        if full_name.as_bstr().starts_with(STASH_PREFIX) {
            let id = match crate::edit::stash::associated_commit(full_name.as_bstr()) {
                Ok(Some(id)) => id,
                Ok(None) => unreachable!("the stash prefix was checked"),
                Err(err) => {
                    tracing::warn!(name = %full_name, error = %err, "ignored malformed tix stash reference");
                    continue;
                }
            };
            out.entry(id).or_default().push(Decoration {
                name: "stash".into(),
                kind: DecorationKind::Stash,
            });
            continue;
        }
        let pin_suffix = full_name.as_bstr().strip_prefix(PIN_PREFIX).map(BString::from);
        let review_suffix = review_number(full_name.as_bstr()).map(BString::from);
        if pin_suffix.is_some() && !pins.contains(full_name.as_bstr()) {
            continue;
        }
        let mut kind = decoration_kind(full_name.as_bstr());
        if kind == DecorationKind::Tag {
            let annotated = match reference.try_id() {
                Some(id) => id.header().context("could not inspect tag")?.kind() == gix::objs::Kind::Tag,
                None => false,
            };
            if annotated {
                kind = DecorationKind::AnnotatedTag;
            }
        }
        let Ok(id) = reference.peel_to_id() else {
            continue;
        };
        let id = id.detach();
        if worktrees.iter().any(|worktree| {
            worktree.is_detached && worktree.label_id == id && worktree.reference.as_ref() == Some(&full_name)
        }) {
            kind = DecorationKind::HeadPinBranch;
        } else if worktrees.iter().any(|worktree| {
            worktree.is_current && worktree.label_id == id && worktree.reference.as_ref() == Some(&full_name)
        }) {
            kind = DecorationKind::CurrentWorktreeBranch;
        } else if worktrees.iter().any(|worktree| {
            !worktree.is_current && worktree.label_id == id && worktree.reference.as_ref() == Some(&full_name)
        }) {
            kind = DecorationKind::WorktreeBranch;
        }
        if head_pin_branch.as_ref() == Some(&full_name) {
            kind = DecorationKind::HeadPinBranch;
        }
        let mut name = pin_suffix.map_or_else(
            || {
                review_suffix.as_ref().map_or_else(
                    || full_name.shorten().to_owned(),
                    |suffix| {
                        if review_count == 1 {
                            "review".into()
                        } else {
                            format!("review:{}", suffix.to_str_lossy()).into()
                        }
                    },
                )
            },
            |suffix| format!("pin:{}", suffix.to_str_lossy()).into(),
        );
        if matches!(kind, DecorationKind::Tag | DecorationKind::AnnotatedTag) {
            name.insert_str(0, "tag: ");
        }
        out.entry(id).or_default().push(Decoration { name, kind });
    }
    for worktree in worktrees
        .iter()
        .filter(|worktree| !worktree.is_current && !worktree.is_detached)
    {
        let Some(reference) = &worktree.reference else { continue };
        if excluded.contains(reference.as_bstr()) {
            continue;
        }
        let name = reference.shorten().to_owned();
        let decorations = out.entry(worktree.id).or_default();
        if !decorations
            .iter()
            .any(|decoration| decoration.kind == DecorationKind::WorktreeBranch && decoration.name == name)
        {
            decorations.push(Decoration {
                name,
                kind: DecorationKind::WorktreeBranch,
            });
        }
    }
    for worktree in worktrees
        .iter()
        .filter(|worktree| worktree.reference.is_none() || worktree.is_detached && !worktree.is_current)
    {
        let kind = if worktree.is_current {
            DecorationKind::CurrentWorktreeDetached
        } else {
            DecorationKind::WorktreeDetached
        };
        let decorations = out.entry(worktree.id).or_default();
        if !decorations
            .iter()
            .any(|decoration| decoration.kind == kind && decoration.name == worktree.checkout_name)
        {
            decorations.push(Decoration {
                name: worktree.checkout_name.clone(),
                kind,
            });
        }
    }
    if !excluded.contains(b"HEAD".as_bstr())
        && let Some(id) = repo
            .head()
            .context("could not read HEAD")?
            .try_peel_to_id()
            .context("could not peel HEAD")?
    {
        out.entry(id.detach()).or_default().push(Decoration {
            name: "HEAD".into(),
            kind: DecorationKind::Head,
        });
    }
    Ok(out)
}

pub(crate) fn is_missing_ref(mut err: &(dyn std::error::Error + 'static)) -> bool {
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

pub(crate) fn decoration_kind(name: &[u8]) -> DecorationKind {
    if name.starts_with(PIN_PREFIX) {
        DecorationKind::Pin
    } else if name.starts_with(STASH_PREFIX) {
        DecorationKind::Stash
    } else if name.starts_with(REVIEW_PREFIX) {
        DecorationKind::Review
    } else if name.starts_with(b"refs/heads/") {
        DecorationKind::Local
    } else if name.starts_with(b"refs/tags/") {
        DecorationKind::Tag
    } else if name.starts_with(b"refs/remotes/") {
        DecorationKind::Remote
    } else {
        DecorationKind::Special
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, process::Command};

    use super::*;
    use crate::app::AttributionKind;

    fn fixture() -> gix_testtools::Result<std::path::PathBuf> {
        gix_testtools::scripted_fixture_read_only_needs_archive("history.sh")
    }

    fn id(n: u8) -> ObjectId {
        let mut bytes = [0; 20];
        bytes[19] = n;
        ObjectId::Sha1(bytes)
    }

    fn insert_commit(graph: &mut HistoryGraph, n: u8, parents: &[u8], generation: u32) {
        let index = graph.intern(id(n)).expect("the small test graph fits in u32");
        let parents: Vec<_> = parents
            .iter()
            .map(|parent| graph.intern(id(*parent)).expect("the small test graph fits in u32"))
            .collect();
        let start = graph.parents.len() as u32;
        graph.parents.extend(parents);
        let end = graph.parents.len() as u32;
        graph.commits[index.as_usize()] = GraphCommit {
            id: id(n),
            parents: start..end,
            commit_time: generation.into(),
            generation,
            state: NODE_LOADED,
        };
    }

    fn loaded(path: &std::path::Path, revisions: &[&str], hidden_revisions: &[&str]) -> Result<Vec<Event>> {
        let mut events = Vec::new();
        let authors =
            gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(Authors::default()));
        let repo = crate::test_repository::open(path)?;
        load(
            &repo,
            &revisions.iter().map(OsString::from).collect::<Vec<_>>(),
            &hidden_revisions.iter().map(OsString::from).collect::<Vec<_>>(),
            false,
            &authors,
            &AtomicBool::new(false),
            |event| {
                events.push(event);
                true
            },
        )?;
        Ok(events)
    }

    #[test]
    fn only_missing_ref_reads_are_ignored() {
        let ref_error = |kind| gix::refs::file::iter::loose_then_packed::Error::ReadFileContents {
            source: std::io::Error::from(kind),
            path: "refs/heads/racing".into(),
        };
        assert!(
            is_missing_ref(&ref_error(std::io::ErrorKind::NotFound)),
            "a ref removed after iteration began is transient"
        );
        assert!(
            !is_missing_ref(&ref_error(std::io::ErrorKind::PermissionDenied)),
            "unrelated ref read errors remain actionable"
        );
    }

    #[test]
    fn paints_criss_cross_relations_from_cached_parents() {
        let mut graph = HistoryGraph::default();
        for (n, parents, generation) in [
            (1, vec![], 1),
            (2, vec![1], 2),
            (3, vec![1], 2),
            (4, vec![2, 3], 3),
            (5, vec![3, 2], 3),
            (6, vec![4], 4),
            (7, vec![5], 4),
        ] {
            insert_commit(&mut graph, n, &parents, generation);
        }

        assert_eq!(
            graph.paint(id(6), &[id(7)]),
            Some((2, 2)),
            "both merge tips stop at the shared criss-cross ancestry"
        );
    }

    #[test]
    fn marks_visible_views_behind_hidden_branches_at_their_base() {
        let mut graph = HistoryGraph::default();
        for (n, parents, generation) in [
            (1, vec![], 1),
            (2, vec![1], 2),
            (3, vec![1], 2),
            (4, vec![3], 3),
            (5, vec![], 1),
        ] {
            insert_commit(&mut graph, n, &parents, generation);
        }

        assert_eq!(
            graph.hidden_branch_updates(&[id(2), id(5)], [id(4)]),
            HashMap::from([(id(1), (2, id(4)))]),
            "only the fork point in the hidden branch's past reports its missing commits"
        );

        insert_commit(&mut graph, 7, &[1], 2);
        insert_commit(&mut graph, 6, &[7], 3);
        assert_eq!(
            graph.hidden_branch_updates(&[id(2), id(5)], [id(4), id(6)]),
            HashMap::from([(id(1), (2, id(6)))]),
            "equal distances choose a deterministic hidden tip"
        );
    }

    #[test]
    fn ignores_missing_hidden_revisions_only_if_another_one_resolves() -> gix_testtools::Result {
        let fixture = fixture()?;
        let repo = crate::test_repository::open(&fixture)?;
        let revisions = ["unknown".into(), "main".into()];

        let (available, unavailable) = available_hidden_revisions(&repo, &revisions, false)?;
        assert_eq!(available, [OsString::from("main")]);
        assert_eq!(unavailable.len(), 1);
        assert!(
            available_hidden_revisions(&repo, &[OsString::from("unknown")], false).is_err(),
            "all missing hidden revisions retain the previous fatal behavior"
        );
        Ok(())
    }

    #[test]
    fn remote_heads_infer_existing_local_hidden_branches() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let path = fixture.path();
        let git = |args: &[&str]| -> gix_testtools::Result {
            let output = Command::new("git").current_dir(path).args(args).output()?;
            assert!(
                output.status.success(),
                "git {args:?} prepares remote HEADs: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            Ok(())
        };
        git(&["branch", "trunk", "main"])?;
        for (remote, fetch, tracking) in [
            (
                "origin",
                "+refs/heads/*:refs/remotes/origin/*",
                "refs/remotes/origin/main",
            ),
            (
                "backup",
                "+refs/heads/*:refs/remotes/backup/*",
                "refs/remotes/backup/main",
            ),
            (
                "team",
                "+refs/heads/trunk:refs/remotes/team/default",
                "refs/remotes/team/default",
            ),
            (
                "stale",
                "+refs/heads/missing:refs/remotes/stale/default",
                "refs/remotes/stale/default",
            ),
        ] {
            git(&["config", &format!("remote.{remote}.url"), "https://example.com/repo"])?;
            git(&["config", &format!("remote.{remote}.fetch"), fetch])?;
            git(&["update-ref", tracking, "main"])?;
            git(&["symbolic-ref", &format!("refs/remotes/{remote}/HEAD"), tracking])?;
        }
        git(&["config", "remote.direct.url", "https://example.com/repo"])?;
        git(&["config", "remote.direct.fetch", "+refs/heads/*:refs/remotes/direct/*"])?;
        git(&["update-ref", "refs/remotes/direct/HEAD", "main"])?;

        let repo = crate::test_repository::open(path)?;
        assert_eq!(
            auto_hidden_revisions(&repo)?,
            [OsString::from("refs/heads/main"), OsString::from("refs/heads/trunk")],
            "remote defaults are reverse-mapped, deduplicated, and limited to existing local branches"
        );
        assert_eq!(
            available_hidden_revisions(&repo, &[OsString::from("topic")], true)?.0,
            [
                OsString::from("topic"),
                OsString::from("refs/heads/main"),
                OsString::from("refs/heads/trunk")
            ],
            "explicit and inferred hidden revisions are combined"
        );
        Ok(())
    }

    #[test]
    fn walks_the_same_reachable_set_as_git_for_multiple_tips() -> gix_testtools::Result {
        let fixture = fixture()?;
        let events = loaded(&fixture, &["main", "topic"], &[])?;
        let actual: HashSet<_> = events
            .iter()
            .flat_map(|event| match event {
                Event::Commits(batch) => batch.rows.iter().map(|row| row.id.to_hex().to_string()).collect(),
                _ => Vec::new(),
            })
            .collect();
        let output = Command::new("git")
            .current_dir(&fixture)
            .args(["rev-list", "main", "topic", "--"])
            .output()?;
        assert!(
            output.status.success(),
            "git rev-list provides the reference result: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let expected = String::from_utf8(output.stdout)?.lines().map(str::to_owned).collect();
        assert_eq!(actual, expected, "all commits reachable from either tip are shown once");
        assert!(matches!(events.last(), Some(Event::Complete(_))), "the walk completes");
        let (topic, attributions) = events
            .iter()
            .filter_map(|event| match event {
                Event::Commits(batch) => batch
                    .rows
                    .iter()
                    .find(|row| row.title == "topic")
                    .map(|row| (row, &batch.attributions)),
                _ => None,
            })
            .next()
            .expect("the topic commit is reachable");
        assert_eq!(
            topic.author.name, "Codex",
            "history loading retains the raw name despite the configured mailmap"
        );
        assert_eq!(topic.author.email, "Codex@OpenAI.com", "the author email is retained");
        assert!(
            topic.author.is_bot(),
            "well-known bot email addresses identify bot authors"
        );
        assert!(topic.has_agent_marker, "history loading recognizes the agent marker");
        assert_eq!(
            attributions[topic.attributions.clone()]
                .iter()
                .map(|attribution| { (attribution.kind, attribution.author.name, attribution.is_agent(),) })
                .collect::<Vec<_>>(),
            [
                (AttributionKind::CoAuthor, b"Human Coauthor".as_bstr(), false),
                (AttributionKind::CoAuthor, b"Claude".as_bstr(), true),
                (AttributionKind::Assisted, b"Opus 4.7".as_bstr(), true),
                (AttributionKind::Reviewed, b"Reviewer".as_bstr(), false),
                (AttributionKind::Acked, b"Acknowledger".as_bstr(), false),
                (AttributionKind::Tested, b"Tester".as_bstr(), false),
                (AttributionKind::SignedOff, b"Signer".as_bstr(), false),
            ],
            "known attribution trailers retain their order and malformed identities are omitted"
        );
        assert_eq!(
            topic.committer_time.format_or_unix(gix::date::time::format::SHORT),
            "2000-01-04",
            "the committer date is retained"
        );
        Ok(())
    }

    #[test]
    fn recognizes_supported_agent_markers() {
        assert!(contains_agent_marker(b"subject\n\n--- agent\n"));
        assert!(contains_agent_marker(b"subject\n\n<!-- agent -->\n"));
        assert!(!contains_agent_marker(b"subject\n\nagent"));
    }

    #[test]
    fn snapshots_references_and_symbolic_targets_from_revisions() -> gix_testtools::Result {
        let fixture = fixture()?;
        let repo = crate::test_repository::open(fixture)?;
        let implicit = snapshot(&repo, &[], &[], false)?;
        assert!(
            implicit.view.contains_key(b"HEAD".as_bstr()),
            "an implicit revision watches HEAD"
        );
        assert!(
            implicit.view.contains_key(b"refs/heads/main".as_bstr()),
            "the symbolic target of HEAD is watched as well"
        );

        let explicit = snapshot(&repo, &[OsString::from("main")], &[OsString::from("topic")], false)?;
        assert!(explicit.view.contains_key(b"refs/heads/main".as_bstr()));
        assert!(explicit.hidden.contains_key(b"refs/heads/topic".as_bstr()));
        Ok(())
    }

    #[test]
    fn undo_queue_revisions_are_private_but_retained_commits_remain_selectable() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let retained = repo.head_id()?.detach();
        let retained_ref: gix::refs::FullName = "refs/heads/undo-retained".try_into()?;
        repo.reference(
            retained_ref.clone(),
            retained,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "prepare retained undo commit",
        )?;
        let entry = crate::edit::undo::record(
            &repo,
            "retain commit",
            &[crate::edit::undo::RefChange {
                name: retained_ref,
                before: crate::edit::undo::State::Missing,
                after: crate::edit::undo::State::Object(retained),
            }],
        )?
        .expect("the reference change creates an undo entry");
        let sentinel = repo
            .find_commit(entry)?
            .parent_ids()
            .next()
            .context("the undo entry has its queue predecessor")?
            .detach();

        for revision in [
            crate::edit::undo::TIP_REF.to_owned(),
            crate::edit::undo::CURSOR_REF.to_owned(),
            entry.to_string(),
            sentinel.to_string(),
        ] {
            let revision = BString::from(revision);
            assert!(
                resolve_revision(&repo, revision.as_bstr()).is_err(),
                "queue revision {revision} is not selectable"
            );
            assert!(
                snapshot(&repo, &[revision.to_os_str()?.to_owned()], &[], false).is_err(),
                "queue revision {revision} cannot enter a history view"
            );
        }

        let retained_revision = BString::from(retained.to_string());
        assert_eq!(
            resolve_revision(&repo, retained_revision.as_bstr())?.0,
            retained,
            "a commit retained as a non-first queue parent stays selectable"
        );
        assert_eq!(
            snapshot(&repo, &[retained_revision.to_os_str()?.to_owned()], &[], false)?.view_tips,
            [retained],
            "retention does not make an ordinary commit private"
        );
        Ok(())
    }

    #[test]
    fn discovers_worktree_decorations_and_optionally_adds_their_tips() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        for args in [
            ["worktree", "add", "-q", "topic-wt", "topic"].as_slice(),
            ["worktree", "add", "-q", "--detach", "detached-wt", "main~2"].as_slice(),
            ["worktree", "add", "-q", "--detach", "broken-wt", "main~2"].as_slice(),
        ] {
            let status = Command::new("git").current_dir(fixture.path()).args(args).status()?;
            assert!(status.success(), "git creates the worktree fixture");
        }
        let remembered_branch = Command::new("git")
            .current_dir(fixture.path())
            .args(["branch", "remembered", "main~1"])
            .status()?;
        assert!(remembered_branch.success(), "git creates the remembered branch");
        let remembered_worktree = Command::new("git")
            .current_dir(fixture.path())
            .args(["worktree", "add", "-q", "remembered-wt", "remembered"])
            .status()?;
        assert!(
            remembered_worktree.success(),
            "git checks out the branch remembered by another worktree"
        );
        let remembered_pin = Command::new("git")
            .current_dir(fixture.path().join("detached-wt"))
            .args(["symbolic-ref", "refs/worktree/tix/pins/HEAD", "refs/heads/remembered"])
            .status()?;
        assert!(remembered_pin.success(), "the foreign worktree remembers its branch");
        std::fs::remove_dir_all(fixture.path().join("detached-wt"))?;
        std::fs::write(
            fixture.path().join(".git/worktrees/broken-wt/HEAD"),
            "not a ref or object id\n",
        )?;

        let repo = crate::test_repository::open(fixture.path())?;
        let main = repo.rev_parse_single("main")?.detach();
        let topic = repo.rev_parse_single("topic")?.detach();
        let root = repo.rev_parse_single("main~2")?.detach();
        let remembered = repo.rev_parse_single("remembered")?.detach();
        let worktrees = worktree_checkouts(&repo);
        assert!(worktrees.iter().any(|worktree| {
            worktree.id == main
                && worktree.label_id == main
                && worktree.is_current
                && !worktree.is_detached
                && worktree
                    .reference
                    .as_ref()
                    .is_some_and(|name| name == "refs/heads/main")
        }));
        assert!(worktrees.iter().any(|worktree| {
            worktree.id == topic
                && worktree.label_id == topic
                && worktree.checkout_name == "topic-wt"
                && !worktree.is_current
                && !worktree.is_detached
                && worktree
                    .reference
                    .as_ref()
                    .is_some_and(|name| name == "refs/heads/topic")
        }));
        assert!(worktrees.iter().any(|worktree| {
            worktree.id == root
                && worktree.label_id == remembered
                && worktree.checkout_name == "detached-wt"
                && worktree
                    .reference
                    .as_ref()
                    .is_some_and(|name| name == "refs/heads/remembered")
                && !worktree.is_current
                && worktree.is_detached
        }));
        assert_eq!(worktrees.len(), 4, "the malformed worktree is ignored");

        let main_repo_decorations = decorations(&repo, &[], &worktrees)?;
        let main_decorations = main_repo_decorations.get(&main).expect("main is decorated");
        assert!(main_decorations.iter().any(|decoration| {
            decoration.kind == DecorationKind::CurrentWorktreeBranch && decoration.name == "main"
        }));
        assert!(
            !main_decorations
                .iter()
                .any(|decoration| { decoration.kind == DecorationKind::WorktreeBranch && decoration.name == "main" })
        );
        assert!(main_repo_decorations.get(&topic).is_some_and(|decorations| {
            decorations
                .iter()
                .any(|decoration| decoration.kind == DecorationKind::WorktreeBranch && decoration.name == "topic")
        }));
        assert!(main_repo_decorations.get(&root).is_some_and(|decorations| {
            decorations.iter().any(|decoration| {
                decoration.kind == DecorationKind::WorktreeDetached && decoration.name == "detached-wt"
            })
        }));
        assert!(
            main_repo_decorations.get(&remembered).is_some_and(|decorations| {
                decorations.iter().any(|decoration| {
                    decoration.kind == DecorationKind::HeadPinBranch && decoration.name == "remembered"
                }) && decorations.iter().any(|decoration| {
                    decoration.kind == DecorationKind::WorktreeBranch && decoration.name == "remembered"
                })
            }),
            "a remembered branch can simultaneously identify an attached foreign worktree"
        );

        let explicit = [OsString::from("main")];
        let without = snapshot(&repo, &explicit, &[], false)?;
        assert_eq!(without.view_tips, [main], "explicit revisions are unchanged by default");
        assert!(!without.view.contains_key(b"refs/heads/remembered".as_bstr()));
        let with = snapshot(&repo, &explicit, &[], true)?;
        assert!(with.view_tips.contains(&main));
        assert!(with.view_tips.contains(&topic));
        assert!(with.view_tips.contains(&root));
        assert!(with.view_tips.contains(&remembered));
        assert!(with.view.contains_key(b"refs/heads/remembered".as_bstr()));

        let linked_path = fixture.path().join("topic-wt");
        let linked_repo = crate::test_repository::open(&linked_path)?;
        let linked_worktrees = worktree_checkouts(&linked_repo);
        assert!(
            linked_worktrees
                .iter()
                .any(|worktree| worktree.id == topic && worktree.is_current)
        );
        let linked_decorations = decorations(&linked_repo, &[], &linked_worktrees)?;
        assert!(linked_decorations.get(&topic).is_some_and(|decorations| {
            decorations.iter().any(|decoration| {
                decoration.kind == DecorationKind::CurrentWorktreeBranch && decoration.name == "topic"
            }) && !decorations
                .iter()
                .any(|decoration| decoration.kind == DecorationKind::WorktreeBranch && decoration.name == "topic")
        }));
        assert!(linked_decorations.get(&main).is_some_and(|decorations| {
            decorations
                .iter()
                .any(|decoration| decoration.kind == DecorationKind::WorktreeBranch && decoration.name == "main")
        }));

        let status = Command::new("git")
            .current_dir(&linked_path)
            .args(["checkout", "-q", "--detach", "main~1"])
            .status()?;
        assert!(status.success(), "git detaches the current linked worktree");
        let detached_repo = crate::test_repository::open(&linked_path)?;
        let detached_worktrees = worktree_checkouts(&detached_repo);
        let current = detached_worktrees
            .iter()
            .find(|worktree| worktree.is_current)
            .expect("the current detached worktree is discovered");
        assert!(current.reference.is_none());
        assert!(current.is_detached);
        assert_eq!(current.label_id, current.id);
        let detached_decorations = decorations(&detached_repo, &[], &detached_worktrees)?;
        assert!(detached_decorations.get(&current.id).is_some_and(|decorations| {
            decorations.iter().any(|decoration| {
                decoration.kind == DecorationKind::CurrentWorktreeDetached && decoration.name == current.checkout_name
            })
        }));

        let symbolic = Command::new("git")
            .current_dir(&linked_path)
            .args(["symbolic-ref", "refs/worktree/tix/pins/HEAD", "refs/heads/topic"])
            .status()?;
        assert!(symbolic.success(), "git remembers the detached worktree's branch");
        let remembered_repo = crate::test_repository::open(&linked_path)?;
        let remembered_worktrees = worktree_checkouts(&remembered_repo);
        let current = remembered_worktrees
            .iter()
            .find(|worktree| worktree.is_current)
            .expect("the remembered detached worktree is discovered");
        assert_ne!(current.id, topic, "the worktree remains detached away from its branch");
        assert_eq!(current.label_id, topic, "the label follows the remembered branch tip");
        assert_eq!(current.checkout_name, "topic-wt");
        assert!(current.is_detached);
        assert!(
            current
                .reference
                .as_ref()
                .is_some_and(|name| name == "refs/heads/topic")
        );
        let remembered_pins = applicable_pins(&remembered_repo)?;
        let remembered_decorations = decorations(&remembered_repo, &remembered_pins, &remembered_worktrees)?;
        assert!(remembered_decorations.get(&topic).is_some_and(|decorations| {
            decorations
                .iter()
                .any(|decoration| decoration.kind == DecorationKind::HeadPinBranch && decoration.name == "topic")
        }));
        Ok(())
    }

    #[test]
    fn decodes_commits_missing_from_a_stale_graph_and_defers_graph_commits() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let fixture_path = fixture.path();
        let graph = Command::new("git")
            .current_dir(fixture_path)
            .args(["commit-graph", "write", "--reachable"])
            .status()?;
        assert!(graph.success(), "git writes the initial commit-graph");

        std::fs::write(fixture_path.join("new"), "new\n")?;
        let add = Command::new("git")
            .current_dir(fixture_path)
            .args(["add", "new"])
            .status()?;
        assert!(add.success(), "the new file is staged");
        let commit = Command::new("git")
            .current_dir(fixture_path)
            .env("GIT_AUTHOR_DATE", "2000-01-05T00:00:00 +0000")
            .env("GIT_COMMITTER_DATE", "2000-01-06T00:00:00 +0000")
            .args(["-c", "commit.gpgSign=false", "commit", "-q", "-m", "new"])
            .status()?;
        assert!(commit.success(), "a commit newer than the graph is created");

        let events = loaded(fixture_path, &["main"], &[])?;
        let rows: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                Event::Commits(batch) => Some(batch.rows.as_slice()),
                _ => None,
            })
            .flatten()
            .collect();
        let newest = rows.first().expect("the new commit is walked first");
        assert!(newest.metadata_loaded, "ODB commits are decoded during the walk");
        assert_eq!(newest.title, "new");
        assert_eq!(
            newest.author_time.format_or_unix(gix::date::time::format::SHORT),
            "2000-01-05",
            "author dates are retained independently"
        );
        assert_eq!(
            newest.committer_time.format_or_unix(gix::date::time::format::SHORT),
            "2000-01-06",
            "committer dates remain available"
        );
        let deferred = rows
            .iter()
            .find(|row| !row.metadata_loaded)
            .expect("older graph commits defer metadata");

        let repo = crate::test_repository::open(fixture_path)?;
        let authors =
            gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(Authors::default()));
        let (metadata, _) = load_metadata(&repo, deferred.id, &authors)?;
        assert!(
            !metadata.title.is_empty(),
            "deferred metadata can be loaded for the view"
        );
        Ok(())
    }

    #[test]
    fn views_without_visible_commits_emit_only_their_boundary_tips() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let main = repo.rev_parse_single("main")?.detach();
        let behind = repo.rev_parse_single("main~1")?.detach();
        let topic = repo.rev_parse_single("topic")?.detach();
        drop(repo);
        assert!(
            Command::new("git")
                .current_dir(fixture.path())
                .args(["symbolic-ref", "HEAD", "refs/heads/unborn"])
                .status()?
                .success(),
            "the fixture enters an unborn branch"
        );

        for (visible, hidden, expected) in [
            (&[][..], &["main"][..], HashSet::from([main])),
            (&[][..], &["main", "topic"][..], HashSet::from([main, topic])),
            (&["main"][..], &["main"][..], HashSet::from([main])),
            (&["main~1"][..], &["main"][..], HashSet::from([behind])),
        ] {
            let events = loaded(fixture.path(), visible, hidden)?;
            let visible: Vec<_> = events
                .iter()
                .filter_map(|event| match event {
                    Event::Commits(commits) => Some(commits.rows.iter().map(|row| row.id)),
                    _ => None,
                })
                .flatten()
                .collect();
            let boundaries: HashSet<_> = events
                .iter()
                .filter_map(|event| match event {
                    Event::HiddenCommits(commits) => Some(commits.rows.iter().map(|row| row.id)),
                    _ => None,
                })
                .flatten()
                .collect();
            let graph = events
                .iter()
                .find_map(|event| match event {
                    Event::Complete(graph) => Some(graph),
                    _ => None,
                })
                .expect("the hidden-only history completes");

            assert!(visible.is_empty(), "hidden ancestry is not exposed as visible history");
            assert_eq!(boundaries, expected, "only the applicable fallback tips are emitted");
            assert_eq!(
                graph.stored_commit_ids().collect::<HashSet<_>>(),
                expected,
                "only hidden tips are retained as rows"
            );
        }
        Ok(())
    }

    #[test]
    fn hidden_only_refresh_stops_after_the_new_tip() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let path = fixture.path();
        assert!(
            Command::new("git")
                .current_dir(path)
                .args(["symbolic-ref", "HEAD", "refs/heads/unborn"])
                .status()?
                .success()
        );
        let mut graph = loaded(path, &[], &["main"])?
            .into_iter()
            .find_map(|event| match event {
                Event::Complete(graph) => Some(graph),
                _ => None,
            })
            .expect("the hidden-only history completes");

        for args in [
            &["symbolic-ref", "HEAD", "refs/heads/main"][..],
            &[
                "-c",
                "commit.gpgSign=false",
                "commit",
                "--allow-empty",
                "-q",
                "-m",
                "new",
            ][..],
            &["symbolic-ref", "HEAD", "refs/heads/unborn"][..],
        ] {
            assert!(Command::new("git").current_dir(path).args(args).status()?.success());
        }
        let repo = crate::test_repository::open(path)?;
        let new_tip = repo.rev_parse_single("main")?.detach();
        let authors =
            gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(Authors::default()));
        let refresh = graph.refresh(&repo, &[], &["main".into()], false, &HashSet::new(), &authors)?;

        assert_eq!(
            refresh.commits.rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            [new_tip],
            "refresh emits the advanced hidden tip without walking its ancestry"
        );
        Ok(())
    }

    #[test]
    fn refresh_stops_at_the_persistent_graph() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let events = loaded(fixture.path(), &["main"], &[])?;
        let mut graph = events
            .into_iter()
            .find_map(|event| match event {
                Event::Complete(graph) => Some(graph),
                _ => None,
            })
            .expect("history loading returns the persistent graph");

        std::fs::write(fixture.path().join("new"), "new\n")?;
        for args in [
            &["add", "new"][..],
            &["-c", "commit.gpgSign=false", "commit", "-q", "-m", "new"],
        ] {
            let status = Command::new("git").current_dir(fixture.path()).args(args).status()?;
            assert!(status.success(), "git prepares one new commit");
        }
        let repo = crate::test_repository::open(fixture.path())?;
        let authors =
            gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(Authors::default()));
        let first = graph.refresh(&repo, &["main".into()], &[], false, &HashSet::new(), &authors)?;
        assert_eq!(first.commits.rows.len(), 1, "only the new descendant is loaded");
        let second = graph.refresh(&repo, &["main".into()], &[], false, &HashSet::new(), &authors)?;
        assert!(
            second.commits.rows.is_empty(),
            "an unchanged tip stops immediately at complete cached ancestry"
        );
        Ok(())
    }

    #[test]
    fn refresh_excludes_replaced_commits_from_descendant_queries() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let events = loaded(fixture.path(), &["main"], &[])?;
        let mut graph = events
            .into_iter()
            .find_map(|event| match event {
                Event::Complete(graph) => Some(graph),
                _ => None,
            })
            .expect("history loading returns the persistent graph");
        let repo = crate::test_repository::open(fixture.path())?;
        let old_tip = repo.rev_parse_single("main")?.detach();
        let parent = repo
            .find_commit(old_tip)?
            .parent_ids()
            .next()
            .expect("the main tip has a parent")
            .detach();
        drop(repo);

        let amend = Command::new("git")
            .current_dir(fixture.path())
            .args([
                "-c",
                "commit.gpgSign=false",
                "commit",
                "--amend",
                "-q",
                "-m",
                "replacement",
            ])
            .status()?;
        assert!(amend.success(), "git replaces the visible tip");

        let repo = crate::test_repository::open(fixture.path())?;
        let replacement = repo.rev_parse_single("main")?.detach();
        let authors =
            gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(Authors::default()));
        graph.refresh(&repo, &["main".into()], &[], false, &HashSet::new(), &authors)?;
        let descendants = graph
            .descendants_in_parent_order(parent)
            .expect("the shared parent remains in the current view");
        assert!(descendants.contains(&replacement), "the replacement tip is editable");
        assert!(
            !descendants.contains(&old_tip),
            "the obsolete tip is excluded from edits"
        );
        Ok(())
    }

    #[test]
    fn refresh_stops_at_cached_tracking_ancestry() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let main = crate::test_repository::open(fixture.path())?
            .rev_parse_single("main")?
            .detach();
        for args in [
            &["config", "remote.origin.url", "https://example.com/repo"][..],
            &["config", "remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*"][..],
            &["config", "branch.topic.remote", "origin"][..],
            &["config", "branch.topic.merge", "refs/heads/main"][..],
            &["update-ref", "refs/remotes/origin/main", &main.to_hex().to_string()][..],
        ] {
            let status = Command::new("git").current_dir(fixture.path()).args(args).status()?;
            assert!(status.success(), "git configures a tracking branch");
        }
        let events = loaded(fixture.path(), &["topic"], &[])?;
        let mut graph = events
            .into_iter()
            .find_map(|event| match event {
                Event::Complete(graph) => Some(graph),
                _ => None,
            })
            .expect("history loading returns the persistent graph");
        let repo = crate::test_repository::open(fixture.path())?;
        let index = graph.index(main).expect("the tracking tip was scheduled");
        let fake_parent = graph.intern(id(255)).expect("the small test graph fits in u32");
        let mut parents = graph.parents(index).to_vec();
        parents.push(fake_parent);
        let start = graph.parents.len() as u32;
        graph.parents.extend(parents);
        let end = graph.parents.len() as u32;
        let cached = &mut graph.commits[index.as_usize()];
        assert!(cached.state & NODE_COMPLETE != 0 && cached.state & NODE_STORED == 0);
        cached.parents = start..end;

        let authors =
            gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(Authors::default()));
        let refresh = graph.refresh(&repo, &["topic".into()], &[], false, &HashSet::new(), &authors)?;
        assert!(
            refresh.commits.rows.is_empty(),
            "an unchanged tracking tip stops before revisiting its cached parents"
        );
        Ok(())
    }

    #[test]
    fn refresh_walks_cached_tracking_ancestry_when_a_symbolic_pin_makes_it_visible() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let main = crate::test_repository::open(fixture.path())?
            .rev_parse_single("main")?
            .detach();
        let merged = crate::test_repository::open(fixture.path())?
            .rev_parse_single("merged")?
            .detach();
        for args in [
            &["config", "remote.origin.url", "https://example.com/repo"][..],
            &["config", "remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*"][..],
            &["config", "branch.topic.remote", "origin"][..],
            &["config", "branch.topic.merge", "refs/heads/main"][..],
            &["update-ref", "refs/remotes/origin/main", &main.to_hex().to_string()][..],
        ] {
            let status = Command::new("git").current_dir(fixture.path()).args(args).status()?;
            assert!(status.success(), "git configures a tracking branch");
        }
        let events = loaded(fixture.path(), &["topic"], &[])?;
        let mut graph = events
            .into_iter()
            .find_map(|event| match event {
                Event::Complete(graph) => Some(graph),
                _ => None,
            })
            .expect("history loading returns the persistent graph");
        let cached = graph.index(main).expect("the tracking tip was scheduled");
        assert!(
            graph.commits[cached.as_usize()].state & NODE_COMPLETE != 0
                && graph.commits[cached.as_usize()].state & NODE_STORED == 0,
            "the tracking tip is cached without being visible"
        );

        assert!(
            Command::new("git")
                .current_dir(fixture.path())
                .args(["switch", "-q", "topic"])
                .status()?
                .success(),
            "git moves HEAD away from the pin target"
        );
        let repo = crate::test_repository::open(fixture.path())?;
        crate::edit::time_travel::create_or_reuse_pin(
            &repo,
            gix::refs::Target::Symbolic("refs/remotes/origin/main".try_into()?),
            main,
            "test symbolic ref-tree pin",
        )?;
        let authors =
            gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(Authors::default()));
        let refresh = graph.refresh(&repo, &["topic".into()], &[], false, &HashSet::new(), &authors)?;
        let refreshed: HashSet<_> = refresh.commits.rows.iter().map(|row| row.id).collect();
        assert!(
            refreshed.contains(&main) && refreshed.contains(&merged),
            "a newly visible cached tip is loaded together with its missing ancestry: {refreshed:?}"
        );
        Ok(())
    }

    #[test]
    fn hidden_history_keeps_tracking_relations_complete_and_can_be_expanded() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let path = fixture.path();
        let git = |args: &[&str]| -> gix_testtools::Result {
            let output = Command::new("git").current_dir(path).args(args).output()?;
            assert!(
                output.status.success(),
                "git {args:?} prepares the hidden tracking fixture: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            Ok(())
        };
        let commit = |name: &str| -> gix_testtools::Result {
            std::fs::write(path.join(name), format!("{name}\n"))?;
            git(&["add", name])?;
            git(&["commit", "-q", "-m", name])
        };

        git(&["config", "commit.gpgsign", "false"])?;
        git(&["switch", "-q", "-c", "relation-base", "main"])?;
        commit("base-0")?;
        let base = crate::test_repository::open(path)?.rev_parse_single("HEAD")?.detach();
        for name in ["base-1", "base-2", "base-3"] {
            commit(name)?;
        }
        git(&["switch", "-q", "-c", "hidden"])?;
        commit("hidden-only")?;
        let hidden_only = crate::test_repository::open(path)?.rev_parse_single("HEAD")?.detach();
        git(&["switch", "-q", "-c", "local", "relation-base"])?;
        commit("local-only")?;
        let local = crate::test_repository::open(path)?.rev_parse_single("HEAD")?.detach();
        git(&["switch", "-q", "--detach", &base.to_hex().to_string()])?;
        commit("upstream-only")?;
        let upstream = crate::test_repository::open(path)?.rev_parse_single("HEAD")?.detach();
        git(&["switch", "-q", "local"])?;
        for args in [
            &["config", "remote.origin.url", "https://example.com/repo"][..],
            &["config", "remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*"][..],
            &["config", "branch.local.remote", "origin"][..],
            &["config", "branch.local.merge", "refs/heads/local"][..],
            &[
                "update-ref",
                "refs/remotes/origin/local",
                &upstream.to_hex().to_string(),
            ][..],
        ] {
            git(args)?;
        }

        let mut decorations = Decorations::new();
        let mut visible = HashSet::new();
        let mut boundary = HashSet::new();
        let mut graph = None;
        for event in loaded(path, &["local"], &["hidden"])? {
            match event {
                Event::Decorations(value) => decorations = value,
                Event::Commits(batch) => visible.extend(batch.rows.into_iter().map(|row| row.id)),
                Event::HiddenCommits(batch) => {
                    boundary.extend(batch.rows.into_iter().map(|row| row.id));
                    visible.extend(boundary.iter().copied());
                }
                Event::Complete(value) => graph = Some(value),
                Event::VisibleComplete | Event::Cancelled => {}
            }
        }
        let mut graph = graph.expect("history loading returns the persistent graph");
        let refs = graph.selection_refs(local, &decorations);
        let counts = Command::new("git")
            .current_dir(path)
            .args([
                "rev-list",
                "--left-right",
                "--count",
                "local...refs/remotes/origin/local",
            ])
            .output()?;
        assert!(counts.status.success(), "git computes the expected tracking relation");
        let expected: Vec<_> = String::from_utf8(counts.stdout)?
            .split_whitespace()
            .map(str::parse::<usize>)
            .collect::<Result<_, _>>()?;
        assert_eq!(
            graph.selection_relation(local, &refs, &[]),
            Some(crate::app::SelectionRelation::Tracking {
                ahead: expected[0],
                behind: expected[1],
            }),
            "hidden tips do not truncate either side of the tracking relation"
        );

        let repo = crate::test_repository::open(path)?;
        let authors =
            gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(Authors::default()));
        let refresh = graph.refresh(&repo, &["local".into()], &[], false, &boundary, &authors)?;
        visible.extend(refresh.commits.rows.into_iter().map(|row| row.id));
        let expected: HashSet<_> = repo
            .rev_walk([local])
            .all()?
            .map(|info| info.map(|info| info.id))
            .collect::<Result<_, _>>()?;
        assert_eq!(
            visible, expected,
            "showing hidden materializes the original view ancestry"
        );
        assert!(
            !visible.contains(&hidden_only),
            "showing hidden does not add commits reachable only from a hidden tip"
        );
        Ok(())
    }

    #[test]
    fn hides_tips_and_every_commit_reachable_from_them() -> gix_testtools::Result {
        let fixture = fixture()?;
        let events = loaded(&fixture, &["topic"], &["main"])?;
        let actual: HashSet<_> = events
            .iter()
            .flat_map(|event| match event {
                Event::Commits(batch) => batch.rows.iter().map(|row| row.id.to_hex().to_string()).collect(),
                _ => Vec::new(),
            })
            .collect();
        let output = Command::new("git")
            .current_dir(&fixture)
            .args(["rev-list", "topic", "--not", "main", "--"])
            .output()?;
        assert!(
            output.status.success(),
            "git rev-list provides the reference result: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let expected = String::from_utf8(output.stdout)?.lines().map(str::to_owned).collect();
        assert_eq!(actual, expected, "hidden tips use Git's exclusion semantics");
        let repo = crate::test_repository::open(&fixture)?;
        let connected: Vec<_> = events
            .iter()
            .flat_map(|event| match event {
                Event::HiddenCommits(batch) => batch.rows.iter().map(|row| row.id).collect(),
                _ => Vec::new(),
            })
            .collect();
        assert_eq!(
            connected,
            [repo.rev_parse_single("topic^")?],
            "only the excluded parent directly connected to visible history is retained"
        );
        assert!(
            matches!(events.last(), Some(Event::Complete(_))),
            "the filtered walk completes"
        );
        Ok(())
    }

    #[test]
    fn reports_decorations_and_honours_cancellation() -> gix_testtools::Result {
        let fixture = fixture()?;
        let events = loaded(&fixture, &["main"], &[])?;
        let Event::Decorations(decorations) = &events[0] else {
            panic!("decorations are sent first")
        };
        assert!(
            decorations
                .values()
                .flatten()
                .any(|decoration| { decoration.name == "tag: v1" && decoration.kind == DecorationKind::AnnotatedTag }),
            "annotated tags decorate their commit"
        );
        assert!(
            decorations
                .values()
                .flatten()
                .all(|decoration| decoration.name != "origin/HEAD"),
            "dangling symbolic references are omitted"
        );

        let mut cancelled = Vec::new();
        let authors =
            gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(Authors::default()));
        let repo = crate::test_repository::open(&fixture)?;
        load(&repo, &[], &[], false, &authors, &AtomicBool::new(true), |event| {
            cancelled.push(event);
            true
        })?;
        assert!(
            matches!(cancelled.as_slice(), [Event::Decorations(_), Event::Cancelled]),
            "cancellation preserves decorations and stops before commits"
        );
        Ok(())
    }

    #[test]
    fn classifies_reference_kinds() {
        assert_eq!(decoration_kind(b"refs/worktree/tix/pins/abcd"), DecorationKind::Pin);
        assert_eq!(
            decoration_kind(b"refs/tix/stash/0123456789012345678901234567890123456789"),
            DecorationKind::Stash
        );
        assert_eq!(decoration_kind(b"refs/worktree/tix/review/1"), DecorationKind::Review);
        assert_eq!(decoration_kind(b"refs/heads/main"), DecorationKind::Local);
        assert_eq!(decoration_kind(b"refs/tags/v1"), DecorationKind::Tag);
        assert_eq!(decoration_kind(b"refs/remotes/origin/main"), DecorationKind::Remote);
        assert_eq!(decoration_kind(b"refs/patches/main/patch"), DecorationKind::Special);
        assert_eq!(decoration_kind(b"refs/stash"), DecorationKind::Special);
    }

    #[test]
    fn ref_tree_revisions_cover_normal_refs_and_optionally_tags() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        repo.reference(
            "refs/worktree/tix/pins/test",
            repo.head_id()?,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "test tree revisions",
        )?;
        for name in [crate::edit::undo::TIP_REF, crate::edit::undo::CURSOR_REF] {
            repo.reference(
                name,
                repo.head_id()?,
                gix::refs::transaction::PreviousValue::MustNotExist,
                "test undo reference filtering",
            )?;
        }
        let names = |include_tags| {
            ref_tree_revisions(&repo, include_tags).map(|revisions| {
                revisions
                    .into_iter()
                    .map(|revision| revision.to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
            })
        };

        let with_tags = names(true)?;
        let without_tags = names(false)?;
        assert!(
            with_tags.iter().any(|name| name.starts_with("refs/heads/")),
            "branches are default traversal tips"
        );
        assert!(
            with_tags.iter().any(|name| name.starts_with("refs/tags/")),
            "tags are included by default"
        );
        assert!(
            without_tags.iter().all(|name| !name.starts_with("refs/tags/")),
            "tagless rendering does not traverse tag-only history"
        );
        assert!(
            with_tags
                .iter()
                .all(|name| !name.starts_with("refs/worktree/tix/pins/")),
            "ordinary pins never become tree traversal tips"
        );
        assert!(
            with_tags
                .iter()
                .all(|name| !crate::edit::undo::is_queue_ref(name.as_bytes().into())),
            "undo queue refs never become tree traversal tips"
        );
        Ok(())
    }

    #[test]
    fn ignores_malformed_and_non_commit_pins() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let head = repo.head_id()?.detach();
        let blob = repo.write_blob(b"not a commit")?.detach();
        repo.reference(
            "refs/worktree/tix/pins/a",
            head,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "test malformed pin",
        )?;
        repo.reference(
            "refs/worktree/tix/pins/abcd",
            blob,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "test non-commit pin",
        )?;
        repo.reference(
            HEAD_PIN_NAME.as_bstr(),
            head,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "test non-symbolic HEAD pin",
        )?;

        assert!(all_pins(&repo)?.is_empty(), "invalid pins never enter history");
        Ok(())
    }

    #[test]
    fn head_pin_marks_its_branch_without_an_ordinary_pin_decoration() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let symbolic = Command::new("git")
            .current_dir(fixture.path())
            .args(["symbolic-ref", "refs/worktree/tix/pins/HEAD", "refs/heads/main"])
            .status()?;
        assert!(symbolic.success(), "git creates the symbolic HEAD pin");
        let detached = Command::new("git")
            .current_dir(fixture.path())
            .args(["checkout", "-q", "--detach", "main~2"])
            .status()?;
        assert!(detached.success(), "git detaches HEAD below the remembered branch");

        let repo = crate::test_repository::open(fixture.path())?;
        let main = repo.rev_parse_single("main")?.detach();
        let snapshot = snapshot(&repo, &[], &[], false)?;
        assert!(snapshot.view_tips.contains(&main), "the HEAD pin retains its branch");
        let decorations = decorations(&repo, &snapshot.pins, &snapshot.worktrees)?;
        let main = decorations.get(&main).expect("the remembered branch is decorated");
        assert!(
            main.iter()
                .any(|decoration| { decoration.kind == DecorationKind::HeadPinBranch && decoration.name == "main" })
        );
        assert!(
            main.iter().all(|decoration| decoration.kind != DecorationKind::Pin),
            "the HEAD pin has no ordinary pin marker"
        );
        Ok(())
    }

    #[test]
    fn attached_hidden_base_keeps_its_ordinary_pin_decoration() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let head_id = repo.head_id()?.detach();
        let (_, created) = crate::edit::time_travel::create_or_reuse_pin(
            &repo,
            gix::refs::Target::Object(head_id),
            head_id,
            "test hidden-base pin",
        )?;
        assert!(created, "the hidden base receives an ordinary pin");
        drop(repo);

        let events = loaded(fixture.path(), &[], &["main"])?;
        assert!(
            events.iter().any(|event| matches!(
                event,
                Event::HiddenCommits(commits) if commits.rows.iter().any(|row| row.id == head_id)
            )),
            "the attached HEAD is displayed as the hidden boundary"
        );
        let decorations = events
            .iter()
            .find_map(|event| match event {
                Event::Decorations(decorations) => Some(decorations),
                _ => None,
            })
            .expect("history loading emits decorations first");
        assert!(
            decorations.get(&head_id).is_some_and(|decorations| decorations
                .iter()
                .any(|decoration| decoration.kind == DecorationKind::Pin)),
            "the hidden base pin remains visible so the action is offered as unpin"
        );
        Ok(())
    }

    #[test]
    fn review_resources_are_not_history_tips_and_stashes_decorate_the_saved_leaf() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let review = repo.rev_parse_single("topic")?.detach();
        let mut stash = repo.find_commit(review)?.decode()?.into_owned()?;
        stash.parents = [review].into_iter().collect();
        stash.message = "review stash".into();
        let stash = repo.write_object(&stash)?.detach();
        repo.reference(
            "refs/worktree/tix/review/1",
            review,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "test review",
        )?;
        repo.reference(
            "refs/worktree/tix/review/stashes/1",
            stash,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "test review stash",
        )?;

        assert_eq!(all_reviews(&repo)?.len(), 1);
        assert!(
            !snapshot(&repo, &[], &[], false)?.view_tips.contains(&review),
            "review resources do not retain history"
        );
        let decorations = decorations(&repo, &[], &worktree_checkouts(&repo))?;
        assert!(
            decorations.get(&review).is_some_and(|decorations| decorations
                .iter()
                .any(|decoration| decoration.kind == DecorationKind::Stash)),
            "saved review state decorates the review leaf"
        );
        assert!(
            decorations.get(&stash).is_none_or(|decorations| decorations
                .iter()
                .all(|decoration| decoration.kind != DecorationKind::Stash)),
            "the internal stash commit is not decorated"
        );
        Ok(())
    }

    #[test]
    fn commit_stashes_decorate_the_associated_commit_without_becoming_tips() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let head = repo.head_id()?.detach();
        let stash = repo.rev_parse_single("topic")?.detach();
        let name = super::super::edit::stash::reference(head)?;
        repo.reference(
            name,
            stash,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "test commit stash",
        )?;

        assert!(!snapshot(&repo, &[], &[], false)?.view_tips.contains(&stash));
        let decorations = decorations(&repo, &[], &worktree_checkouts(&repo))?;
        assert!(
            decorations.get(&head).is_some_and(|decorations| decorations
                .iter()
                .any(|decoration| decoration.kind == DecorationKind::Stash)),
            "the ref-name suffix associates the stash with HEAD"
        );
        assert!(
            decorations.get(&stash).is_none_or(|decorations| decorations
                .iter()
                .all(|decoration| decoration.kind != DecorationKind::Stash)),
            "the stash object itself is not decorated"
        );
        Ok(())
    }

    #[test]
    fn pins_are_private_to_the_current_worktree() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let linked = gix_testtools::tempfile::tempdir()?;
        let linked_path = linked.path().join("linked");
        assert!(
            std::process::Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["worktree", "add", "-q", "--detach"])
                .arg(&linked_path)
                .arg("topic")
                .status()?
                .success(),
            "the linked-worktree fixture is created"
        );
        let main = crate::test_repository::open(fixture.path())?;
        let linked = crate::test_repository::open(&linked_path)?;
        let main_id = main.head_id()?.detach();
        let linked_id = linked.head_id()?.detach();
        for (repo, id) in [(&main, main_id), (&linked, linked_id)] {
            repo.reference(
                "refs/worktree/tix/pins/abcd",
                id,
                gix::refs::transaction::PreviousValue::MustNotExist,
                "test private pin",
            )?;
        }
        assert_eq!(
            all_pins(&main)?.into_iter().map(|pin| pin.id).collect::<Vec<_>>(),
            [main_id],
            "the main worktree sees only its private pin"
        );
        assert_eq!(
            all_pins(&linked)?.into_iter().map(|pin| pin.id).collect::<Vec<_>>(),
            [linked_id],
            "the linked worktree sees only its private pin"
        );
        Ok(())
    }

    #[test]
    fn interns_raw_author_identities() {
        let mut authors = Authors::default();

        let first = authors.intern_author(b"author\xff", b"one@example.com");
        let second = authors.intern_author(b"author\xff", b"one@example.com");
        let other = authors.intern_author(b"author\xff", b"two@example.com");

        assert!(std::ptr::eq(first, second), "equal identities share one allocation");
        assert!(!std::ptr::eq(first, other), "different emails remain distinct");
        assert_eq!(authors.authors.len(), 2);
        assert_eq!(first.name, b"author\xff".as_bstr(), "Git names remain byte strings");
    }
}
