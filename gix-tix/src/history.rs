use std::{
    cmp::Ordering as CmpOrdering,
    collections::{HashMap, HashSet},
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
    Local,
    Remote,
    Tag,
    AnnotatedTag,
    Special,
}

pub(crate) type Decorations = HashMap<ObjectId, Vec<Decoration>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SelectionRef {
    pub name: BString,
    pub upstream: Option<Option<ObjectId>>,
}

#[derive(Clone, Copy, Debug, Default)]
struct Node {
    complete: bool,
    stored: bool,
    flags: u8,
    expanded: u8,
    emitted: bool,
}

#[derive(Debug, Default)]
pub(crate) struct HistoryGraph {
    commits: gix::revwalk::graph::IdMap<gix::revwalk::graph::Commit<Node>>,
    tracking: HashMap<ObjectId, Vec<SelectionRef>>,
    relations: HashMap<(ObjectId, ObjectId), (usize, usize)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GenThenTime {
    generation: gix::revwalk::graph::Generation,
    time: gix::date::SecondsSinceUnixEpoch,
}

impl From<&gix::revwalk::graph::Commit<Node>> for GenThenTime {
    fn from(commit: &gix::revwalk::graph::Commit<Node>) -> Self {
        GenThenTime {
            generation: commit
                .generation
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
    fn ensure_commit(
        &mut self,
        repo: &gix::Repository,
        cache: Option<&gix::commitgraph::Graph>,
        shallow: &HashSet<ObjectId>,
        id: ObjectId,
        buf: &mut Vec<u8>,
    ) -> Result<()> {
        if self.commits.contains_key(&id) {
            return Ok(());
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
        self.commits.insert(
            id,
            gix::revwalk::graph::Commit {
                parents,
                commit_time,
                generation,
                data: Node::default(),
            },
        );
        Ok(())
    }

    #[expect(clippy::too_many_arguments)]
    fn schedule_cached(
        &mut self,
        repo: &gix::Repository,
        cache: Option<&gix::commitgraph::Graph>,
        shallow: &HashSet<ObjectId>,
        states: &mut HashMap<ObjectId, WalkState>,
        queue: &mut gix::revwalk::PriorityQueue<gix::date::SecondsSinceUnixEpoch, ObjectId>,
        buf: &mut Vec<u8>,
        id: ObjectId,
        flags: u8,
    ) -> Result<()> {
        self.ensure_commit(repo, cache, shallow, id, buf)?;
        let state = states.entry(id).or_default();
        if state.flags & flags != flags {
            state.flags |= flags;
            queue.insert(self.commits[&id].commit_time, id);
        }
        Ok(())
    }

    pub(crate) fn selection_refs(&self, id: ObjectId, decorations: &Decorations) -> Vec<SelectionRef> {
        let tracked = self.tracking.get(&id);
        let mut refs: Vec<_> = decorations
            .get(&id)
            .into_iter()
            .flatten()
            .map(|decoration| {
                let upstream = if decoration.kind == DecorationKind::Local {
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
            let relation = if let Some(relation) = self.relations.get(&(id, upstream)).copied() {
                Some(relation)
            } else {
                let relation = self.paint(id, std::slice::from_ref(&upstream))?;
                self.relations.insert((id, upstream), relation);
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

    fn paint(&self, first: ObjectId, others: &[ObjectId]) -> Option<(usize, usize)> {
        if !self.commits.contains_key(&first) || others.iter().any(|id| !self.commits.contains_key(id)) {
            return None;
        }
        let mut flags = HashMap::<ObjectId, u8>::new();
        let mut queue = gix::revwalk::PriorityQueue::<GenThenTime, ObjectId>::new();
        let mut queued = HashSet::new();
        let mut pending = 0usize;
        for (id, flag) in std::iter::once((first, VISIBLE)).chain(others.iter().copied().map(|id| (id, HIDDEN))) {
            *flags.entry(id).or_default() |= flag;
            if queued.insert(id) {
                queue.insert(GenThenTime::from(&self.commits[&id]), id);
                pending += 1;
            }
        }
        while pending != 0 {
            let Some((_priority, id)) = queue.pop() else { break };
            queued.remove(&id);
            let mut propagated = flags[&id];
            if propagated & STALE == 0 {
                pending -= 1;
            }
            if propagated & (VISIBLE | HIDDEN) == VISIBLE | HIDDEN {
                propagated |= STALE;
                *flags.get_mut(&id).expect("queued commits have flags") = propagated;
            }
            for &parent in &self.commits[&id].parents {
                let Some(commit) = self.commits.get(&parent) else {
                    continue;
                };
                let parent_flags = flags.entry(parent).or_default();
                let previous = *parent_flags;
                if previous & propagated != propagated {
                    *parent_flags = previous | propagated;
                    if queued.contains(&parent) {
                        if previous & STALE == 0 && *parent_flags & STALE != 0 {
                            pending -= 1;
                        }
                    } else {
                        queued.insert(parent);
                        if *parent_flags & STALE == 0 {
                            pending += 1;
                        }
                        queue.insert(GenThenTime::from(commit), parent);
                    }
                }
            }
        }
        let mut ahead = 0;
        let mut behind = 0;
        for flags in flags.into_values() {
            match flags & (VISIBLE | HIDDEN) {
                VISIBLE => ahead += 1,
                HIDDEN => behind += 1,
                _ => {}
            }
        }
        Some((ahead, behind))
    }

    pub(crate) fn refresh(
        &mut self,
        repo: &gix::Repository,
        revisions: &[OsString],
        hidden_revisions: &[OsString],
        expand: &HashSet<ObjectId>,
        authors: &SharedAuthors,
    ) -> Result<Refresh> {
        let refs = snapshot(repo, revisions, hidden_revisions)?;
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
        let mut states = HashMap::<ObjectId, WalkState>::new();
        let mut queue = gix::revwalk::PriorityQueue::new();
        let mut buf = Vec::new();
        for id in refs.view_tips.iter().chain(&refs.hidden_tips).chain(expand).copied() {
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
        for (&id, names) in &local_refs {
            if self.commits.get(&id).is_none_or(|commit| !commit.data.stored) {
                continue;
            }
            let tracked = resolve_tracking(repo, names)?;
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
            tracking.insert(id, tracked);
        }

        let mut rows = Vec::new();
        let mut attributions = Vec::new();
        while let Some((_time, id)) = queue.pop() {
            let Some(state) = states.get_mut(&id) else { continue };
            let delta = state.flags & !state.expanded;
            if delta == 0 {
                continue;
            }
            state.expanded |= delta;
            let commit = &self.commits[&id];
            let was_stored = commit.data.stored;
            let stop = commit.data.complete && !expand.contains(&id);
            let should_store = delta & VISIBLE != 0 && !was_stored;
            let parent_ids = commit.parents.clone();
            let generation = commit.generation;
            if should_store {
                if let Some(names) = local_refs.get(&id) {
                    let tracked = resolve_tracking(repo, names)?;
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
                    tracking.insert(id, tracked);
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
                    author,
                    attributions: row_attributions,
                    title,
                    has_agent_marker,
                    signature,
                } = metadata.unwrap_or_else(|| Metadata {
                    committer_time: Default::default(),
                    author: &EMPTY_AUTHOR,
                    attributions: 0..0,
                    title: BString::default(),
                    has_agent_marker: false,
                    signature: SignatureState::Unsigned,
                });
                rows.push(Commit {
                    id,
                    parent_ids: parent_ids.clone(),
                    committer_time,
                    author,
                    attributions: row_attributions,
                    title,
                    metadata_loaded,
                    has_agent_marker,
                    signature,
                });
                self.commits
                    .get_mut(&id)
                    .expect("loaded commit remains present")
                    .data
                    .stored = true;
            }
            if stop {
                continue;
            }
            for parent in parent_ids {
                self.schedule_cached(
                    repo,
                    cache.as_ref(),
                    &shallow,
                    &mut states,
                    &mut queue,
                    &mut buf,
                    parent,
                    delta & (VISIBLE | INTERNAL),
                )?;
            }
        }
        for (id, state) in states {
            if state.expanded & (VISIBLE | INTERNAL) != 0 {
                self.commits
                    .get_mut(&id)
                    .expect("walked commits remain cached")
                    .data
                    .complete = true;
            }
        }
        self.tracking = tracking;
        Ok(Refresh {
            refs,
            decorations: decorations(repo)?,
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

#[derive(Default)]
struct WalkState {
    flags: u8,
    expanded: u8,
}

fn schedule(
    graph: &mut gix::revwalk::Graph<'_, '_, gix::revwalk::graph::Commit<Node>>,
    queue: &mut gix::revwalk::PriorityQueue<gix::date::SecondsSinceUnixEpoch, ObjectId>,
    shallow: &HashSet<ObjectId>,
    id: ObjectId,
    flags: u8,
) -> Result<()> {
    let Some(commit) = graph
        .get_or_insert_full_commit(id, |commit| {
            if shallow.contains(&id) {
                commit.parents.clear();
            }
        })
        .context("could not load commit for history traversal")?
    else {
        return Ok(());
    };
    if commit.data.flags & flags != flags {
        commit.data.flags |= flags;
        queue.insert(commit.commit_time, id);
    }
    Ok(())
}

fn hidden_frontier(
    graph: &mut gix::revwalk::Graph<'_, '_, gix::revwalk::graph::Commit<Node>>,
    visible_tips: &[ObjectId],
    hidden_tips: &[ObjectId],
    shallow: &HashSet<ObjectId>,
) -> Result<HashSet<ObjectId>> {
    if hidden_tips.is_empty() {
        return Ok(HashSet::new());
    }
    let mut flags = HashMap::<ObjectId, u8>::new();
    let mut queue = gix::revwalk::PriorityQueue::<GenThenTime, ObjectId>::new();
    for (tips, flag) in [(visible_tips, VISIBLE), (hidden_tips, HIDDEN)] {
        for &id in tips {
            let Some(commit) = graph
                .get_or_insert_full_commit(id, |commit| {
                    if shallow.contains(&id) {
                        commit.parents.clear();
                    }
                })
                .context("could not load commit while preparing hidden history")?
            else {
                continue;
            };
            *flags.entry(id).or_default() |= flag;
            queue.insert(GenThenTime::from(&*commit), id);
        }
    }
    while queue
        .iter_unordered()
        .any(|id| flags.get(id).is_some_and(|flags| flags & STALE == 0))
    {
        let Some((_priority, id)) = queue.pop() else { break };
        let mut propagated = flags[&id];
        if propagated & (VISIBLE | HIDDEN) == VISIBLE | HIDDEN {
            propagated |= STALE;
            *flags.get_mut(&id).expect("queued commits have flags") = propagated;
        }
        let parents = graph.get(&id).expect("queued commits are loaded").parents.clone();
        for parent in parents {
            let Some(commit) = graph
                .get_or_insert_full_commit(parent, |commit| {
                    if shallow.contains(&parent) {
                        commit.parents.clear();
                    }
                })
                .context("could not load hidden commit parent")?
            else {
                continue;
            };
            let parent_flags = flags.entry(parent).or_default();
            if *parent_flags & propagated != propagated {
                *parent_flags |= propagated;
                queue.insert(GenThenTime::from(&*commit), parent);
            }
        }
    }
    Ok(flags
        .into_iter()
        .filter_map(|(id, flags)| (flags & (VISIBLE | HIDDEN) == VISIBLE | HIDDEN).then_some(id))
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
    authors: &SharedAuthors,
    cancelled: &AtomicBool,
    mut emit: impl FnMut(Event) -> bool,
) -> Result<()> {
    let Some(tips) = resolve_tips(repo, revisions)? else {
        emit(Event::Decorations(decorations(repo)?));
        emit(Event::VisibleComplete);
        emit(Event::Complete(HistoryGraph::default()));
        return Ok(());
    };
    let hidden_tips = resolve_revisions(repo, hidden_revisions, "hidden ")?;

    if !emit(Event::Decorations(decorations(repo)?)) {
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
    let mut graph = repo.revision_graph::<gix::revwalk::graph::Commit<Node>>(commit_graph.as_ref());
    let hidden = hidden_frontier(&mut graph, &tips, &hidden_tips, &shallow)?;
    let local_refs = local_refs_by_target(repo)?;
    let mut tracking = HashMap::new();
    let mut queue = gix::revwalk::PriorityQueue::new();
    for &tip in &tips {
        schedule(&mut graph, &mut queue, &shallow, tip, VISIBLE)?;
    }
    let mut rows = Vec::with_capacity(COMMIT_BATCH_SIZE);
    let mut attributions = Vec::with_capacity(COMMIT_BATCH_SIZE);
    let mut connected = Vec::new();
    let mut connected_seen = HashSet::new();
    while let Some((_time, id)) = queue.pop() {
        if cancelled.load(Ordering::Relaxed) {
            emit(Event::Cancelled);
            return Ok(());
        }
        let commit = graph.get_mut(&id).expect("queued commits are loaded");
        let delta = commit.data.flags & !commit.data.expanded;
        if delta == 0 {
            continue;
        }
        commit.data.expanded |= delta;
        commit.data.complete = true;
        let should_emit = delta & VISIBLE != 0 && !commit.data.emitted && !hidden.contains(&id);
        commit.data.emitted |= should_emit;
        commit.data.stored |= should_emit;
        let parent_ids = commit.parents.clone();
        let generation = commit.generation;
        if should_emit && let Some(names) = local_refs.get(&id) {
            let refs = resolve_tracking(repo, names)?;
            for upstream in refs.iter().filter_map(|reference| reference.upstream.flatten()) {
                schedule(&mut graph, &mut queue, &shallow, upstream, INTERNAL)?;
            }
            tracking.insert(id, refs);
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
                author,
                attributions: row_attributions,
                title,
                has_agent_marker,
                signature,
            } = metadata.unwrap_or_else(|| Metadata {
                committer_time: Default::default(),
                author: &EMPTY_AUTHOR,
                attributions: 0..0,
                title: BString::default(),
                has_agent_marker: false,
                signature: SignatureState::Unsigned,
            });
            if !hidden_revisions.is_empty() {
                connected.extend(parent_ids.iter().copied().filter(|id| connected_seen.insert(*id)));
            }
            rows.push(Commit {
                id,
                parent_ids: parent_ids.clone(),
                committer_time,
                author,
                attributions: row_attributions,
                title,
                metadata_loaded,
                has_agent_marker,
                signature,
            });
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
        for parent in parent_ids {
            let parent_flags = if hidden.contains(&parent) {
                propagated & !VISIBLE
            } else {
                propagated
            };
            if parent_flags != 0 {
                schedule(&mut graph, &mut queue, &shallow, parent, parent_flags)?;
            }
        }
    }
    if !rows.is_empty() && !emit(Event::Commits(LoadedCommits { rows, attributions })) {
        return Ok(());
    }
    if !hidden_revisions.is_empty() {
        connected.retain(|id| graph.get(id).is_none_or(|commit| !commit.data.emitted));
        let mut rows = Vec::with_capacity(connected.len());
        let mut attributions = Vec::new();
        let mut authors = gix::features::threading::lock(authors);
        for id in connected {
            if cancelled.load(Ordering::Relaxed) {
                emit(Event::Cancelled);
                return Ok(());
            }
            let object = repo.find_commit(id).context("could not read connected hidden commit")?;
            let parent_ids = object.parent_ids().map(gix::Id::detach).collect();
            let Metadata {
                committer_time,
                author,
                attributions: row_attributions,
                title,
                has_agent_marker,
                signature,
            } = decode_metadata(object.iter(), &mut authors, &mut attributions)?;
            rows.push(Commit {
                id,
                parent_ids,
                committer_time,
                author,
                attributions: row_attributions,
                title,
                metadata_loaded: true,
                has_agent_marker,
                signature,
            });
            if let Some(commit) = graph
                .get_or_insert_full_commit(id, |commit| {
                    if shallow.contains(&id) {
                        commit.parents.clear();
                    }
                })
                .context("could not retain connected hidden commit")?
            {
                commit.data.stored = true;
            }
        }
        if !rows.is_empty() && !emit(Event::HiddenCommits(LoadedCommits { rows, attributions })) {
            return Ok(());
        }
    }
    emit(Event::VisibleComplete);
    emit(Event::Complete(HistoryGraph {
        commits: graph.detach(),
        tracking,
        relations: HashMap::new(),
    }));
    Ok(())
}

pub(crate) fn snapshot(repo: &gix::Repository, revisions: &[OsString], hidden: &[OsString]) -> Result<RefSnapshot> {
    Ok(RefSnapshot {
        view: referenced_refs(repo, revisions)?,
        hidden: referenced_refs(repo, hidden)?,
        view_tips: resolve_tips(repo, revisions)?.unwrap_or_default(),
        hidden_tips: resolve_revisions(repo, hidden, "hidden ")?,
    })
}

fn referenced_refs(repo: &gix::Repository, revisions: &[OsString]) -> Result<HashMap<BString, gix::refs::Target>> {
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

fn decode_metadata<'a>(
    tokens: impl Iterator<Item = Result<Token<'a>, gix::objs::decode::Error>>,
    authors: &mut Authors,
    attributions: &mut Vec<Attribution>,
) -> Result<Metadata<BString>> {
    let mut committer_time = None;
    let mut author = None;
    let attribution_start = attributions.len();
    let mut title = None;
    let mut has_agent_marker = false;
    let mut signature = SignatureState::Unsigned;
    for token in tokens {
        match token.context("could not decode commit")? {
            Token::Author { signature } => {
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
            Token::ExtraHeader((name, _)) if name == "gpgsig" || name == "gpgsig-sha256" => {
                signature = SignatureState::Unverified;
            }
            _ => {}
        }
    }
    Ok(Metadata {
        committer_time: committer_time.context("commit has no committer time")?,
        author: author.context("commit has no author")?,
        attributions: attribution_start..attributions.len(),
        title: title.context("commit has no message")?,
        has_agent_marker,
        signature,
    })
}

fn contains_agent_marker(message: &[u8]) -> bool {
    [b"--- agent".as_slice(), b"<!-- agent -->".as_slice()]
        .iter()
        .any(|marker| message.windows(marker.len()).any(|window| window == *marker))
}

pub(crate) fn count_up_to(
    repo: &gix::Repository,
    revisions: &[OsString],
    hidden_revisions: &[OsString],
    limit: usize,
) -> Result<usize> {
    let Some(tips) = resolve_tips(repo, revisions)? else {
        return Ok(0);
    };
    let hidden_tips = resolve_revisions(repo, hidden_revisions, "hidden ")?;
    let walk = repo
        .rev_walk(tips)
        .with_hidden(hidden_tips)
        .sorting(gix::revision::walk::Sorting::ByCommitTime(Default::default()))
        .all()
        .context("could not start revision walk")?;
    let mut count = 0;
    for info in walk.take(limit) {
        info.context("could not traverse revision history")?;
        count += 1;
    }
    Ok(count)
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
            repo.rev_parse_single(revision)
                .with_context(|| format!("could not resolve {kind}revision {revision}"))?
                .object()
                .with_context(|| format!("could not read {kind}revision"))?
                .peel_to_kind(gix::object::Kind::Commit)
                .with_context(|| format!("{kind}revision does not resolve to a commit"))
                .map(|object| object.id)
        })
        .collect()
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

pub(crate) fn decorations(repo: &gix::Repository) -> Result<Decorations> {
    let mut out = Decorations::new();
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
        let mut kind = decoration_kind(reference.name().as_bstr());
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
        let mut name = reference.name().shorten().to_owned();
        if matches!(kind, DecorationKind::Tag | DecorationKind::AnnotatedTag) {
            name.insert_str(0, "tag: ");
        }
        out.entry(id).or_default().push(Decoration { name, kind });
    }
    if let Some(id) = repo
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

fn decoration_kind(name: &[u8]) -> DecorationKind {
    if name.starts_with(b"refs/heads/") {
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

    fn loaded(path: &std::path::Path, revisions: &[&str], hidden_revisions: &[&str]) -> Result<Vec<Event>> {
        let mut events = Vec::new();
        let authors =
            gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(Authors::default()));
        let repo = gix::open(path)?;
        load(
            &repo,
            &revisions.iter().map(OsString::from).collect::<Vec<_>>(),
            &hidden_revisions.iter().map(OsString::from).collect::<Vec<_>>(),
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
            graph.commits.insert(
                id(n),
                gix::revwalk::graph::Commit {
                    parents: parents.into_iter().map(id).collect(),
                    commit_time: generation.into(),
                    generation: Some(generation),
                    data: Node::default(),
                },
            );
        }

        assert_eq!(
            graph.paint(id(6), &[id(7)]),
            Some((2, 2)),
            "both merge tips stop at the shared criss-cross ancestry"
        );
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
        let repo = gix::open(fixture)?;
        let implicit = snapshot(&repo, &[], &[])?;
        assert!(
            implicit.view.contains_key(b"HEAD".as_bstr()),
            "an implicit revision watches HEAD"
        );
        assert!(
            implicit.view.contains_key(b"refs/heads/main".as_bstr()),
            "the symbolic target of HEAD is watched as well"
        );

        let explicit = snapshot(&repo, &[OsString::from("main")], &[OsString::from("topic")])?;
        assert!(explicit.view.contains_key(b"refs/heads/main".as_bstr()));
        assert!(explicit.hidden.contains_key(b"refs/heads/topic".as_bstr()));
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
            .env("GIT_COMMITTER_DATE", "2000-01-05T00:00:00 +0000")
            .args(["commit", "-q", "-m", "new"])
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
        let deferred = rows
            .iter()
            .find(|row| !row.metadata_loaded)
            .expect("older graph commits defer metadata");

        let repo = gix::open(fixture_path)?;
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
        for args in [&["add", "new"][..], &["commit", "-q", "-m", "new"]] {
            let status = Command::new("git").current_dir(fixture.path()).args(args).status()?;
            assert!(status.success(), "git prepares one new commit");
        }
        let repo = gix::open(fixture.path())?;
        let authors =
            gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(Authors::default()));
        let first = graph.refresh(&repo, &["main".into()], &[], &HashSet::new(), &authors)?;
        assert_eq!(first.commits.rows.len(), 1, "only the new descendant is loaded");
        let second = graph.refresh(&repo, &["main".into()], &[], &HashSet::new(), &authors)?;
        assert!(
            second.commits.rows.is_empty(),
            "an unchanged tip stops immediately at complete cached ancestry"
        );
        Ok(())
    }

    #[test]
    fn refresh_stops_at_cached_tracking_ancestry() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let main = gix::open(fixture.path())?.rev_parse_single("main")?.detach();
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
        let repo = gix::open(fixture.path())?;
        let cached = graph.commits.get_mut(&main).expect("the tracking tip was scheduled");
        assert!(cached.data.complete && !cached.data.stored);
        cached.parents.push(id(255));

        let authors =
            gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(Authors::default()));
        let refresh = graph.refresh(&repo, &["topic".into()], &[], &HashSet::new(), &authors)?;
        assert!(
            refresh.commits.rows.is_empty(),
            "an unchanged tracking tip stops before revisiting its cached parents"
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
        let repo = gix::open(&fixture)?;
        let connected: Vec<_> = events
            .iter()
            .flat_map(|event| match event {
                Event::HiddenCommits(batch) => batch.rows.iter().map(|row| row.id).collect(),
                _ => Vec::new(),
            })
            .collect();
        assert_eq!(
            connected,
            [repo.rev_parse_single("topic^")?.detach()],
            "only the excluded parent directly connected to visible history is retained"
        );
        let revisions = [OsString::from("topic")];
        let hidden = [OsString::from("main")];
        assert_eq!(
            count_up_to(&repo, &revisions, &hidden, 1)?,
            actual.len().min(1),
            "the screen-size probe stops at its limit"
        );
        assert_eq!(
            count_up_to(&repo, &revisions, &hidden, usize::MAX)?,
            actual.len(),
            "the screen-size probe uses the same hidden history"
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
        let repo = gix::open(&fixture)?;
        load(&repo, &[], &[], &authors, &AtomicBool::new(true), |event| {
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
        assert_eq!(decoration_kind(b"refs/heads/main"), DecorationKind::Local);
        assert_eq!(decoration_kind(b"refs/tags/v1"), DecorationKind::Tag);
        assert_eq!(decoration_kind(b"refs/remotes/origin/main"), DecorationKind::Remote);
        assert_eq!(decoration_kind(b"refs/patches/main/patch"), DecorationKind::Special);
        assert_eq!(decoration_kind(b"refs/stash"), DecorationKind::Special);
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
