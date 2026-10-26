use gix_commitgraph::Parent;
use gix_error::{Error, ErrorExt, Result, corruption, message};
use gix_hash::oid;
use gix_revwalk::{PriorityQueue, graph::IdMap};
use smallvec::SmallVec;

use crate::commit::{
    Either, Info, ParentIds, Parents, Topo, find,
    topo::{Sorting, WalkFlags},
};

pub(in crate::commit) type GenAndCommitTime = (u32, i64);

fn missing_indegree() -> Error {
    message("Indegree information is missing").corrupted_error()
}

fn missing_state() -> Error {
    message("Internal state (bitflags) not found").corrupted_error()
}

// Git's priority queue works as a LIFO stack if no compare function is set,
// which is the case for `--topo-order.` However, even in that case the initial
// items of the queue are sorted according to the commit time before beginning
// the walk.
#[derive(Debug)]
pub(in crate::commit) enum Queue {
    Date(PriorityQueue<i64, Info>),
    Topo(Vec<(i64, Info)>),
}

impl Queue {
    pub(super) fn new(s: Sorting) -> Self {
        match s {
            Sorting::DateOrder => Self::Date(PriorityQueue::new()),
            Sorting::TopoOrder => Self::Topo(vec![]),
        }
    }

    pub(super) fn push(&mut self, commit_time: i64, info: Info) {
        match self {
            Self::Date(q) => q.insert(commit_time, info),
            Self::Topo(q) => q.push((commit_time, info)),
        }
    }

    fn pop(&mut self) -> Option<Info> {
        match self {
            Self::Date(q) => q.pop().map(|(_, info)| info),
            Self::Topo(q) => q.pop().map(|(_, info)| info),
        }
    }

    pub(super) fn initial_sort(&mut self) {
        if let Self::Topo(inner_vec) = self {
            inner_vec.sort_by_key(|a| a.0);
        }
    }
}

impl<Find, Predicate> Topo<Find, Predicate>
where
    Find: gix_object::Find,
    Predicate: FnMut(&oid) -> bool,
{
    pub(super) fn compute_indegrees_to_depth(&mut self, gen_cutoff: u32) -> Result {
        while let Some(((generation, _), _)) = self.indegree_queue.peek() {
            if *generation >= gen_cutoff {
                self.indegree_walk_step()?;
            } else {
                break;
            }
        }

        Ok(())
    }

    fn indegree_walk_step(&mut self) -> Result {
        if let Some(((generation, _), id)) = self.indegree_queue.pop() {
            self.explore_to_depth(generation)?;

            let parents = self.collect_parents(&id)?;
            for parent in parents {
                self.indegrees
                    .entry(parent.commit_id)
                    .and_modify(|e| *e += 1)
                    .or_insert(2);

                let state = self.states.get_mut(&parent.commit_id).ok_or_else(missing_state)?;
                if !state.contains(WalkFlags::InDegree) {
                    *state |= WalkFlags::InDegree;
                    self.indegree_queue
                        .insert((parent.generation, parent.commit_time), parent.commit_id);
                }
            }
        }
        Ok(())
    }

    fn explore_to_depth(&mut self, gen_cutoff: u32) -> Result {
        while let Some(((generation, _), _)) = self.explore_queue.peek() {
            if *generation >= gen_cutoff {
                self.explore_walk_step()?;
            } else {
                break;
            }
        }
        Ok(())
    }

    fn explore_walk_step(&mut self) -> Result {
        if let Some((_, id)) = self.explore_queue.pop() {
            let parents = self.collect_parents(&id)?;
            self.process_parents(&id, &parents)?;

            for parent in parents {
                let state = self.states.get_mut(&parent.commit_id).ok_or_else(missing_state)?;

                if !state.contains(WalkFlags::Explored) {
                    *state |= WalkFlags::Explored;
                    self.explore_queue
                        .insert((parent.generation, parent.commit_time), parent.commit_id);
                }
            }
        }
        Ok(())
    }

    fn expand_topo_walk(&mut self, id: &oid) -> Result {
        let parents = self.collect_parents(id)?;
        self.process_parents(id, &parents)?;

        for Parent {
            commit_id: pid,
            generation: parent_gen,
            commit_time: parent_commit_time,
        } in parents
        {
            let parent_state = self.states.get(&pid).ok_or_else(missing_state)?;
            if parent_state.contains(WalkFlags::Uninteresting) {
                continue;
            }

            if parent_gen < self.min_gen {
                self.min_gen = parent_gen;
                self.compute_indegrees_to_depth(self.min_gen)?;
            }

            let i = self.indegrees.get_mut(&pid).ok_or_else(missing_indegree)?;
            *i -= 1;
            if *i != 1 {
                continue;
            }

            let parent_ids = self.collect_parent_ids(&pid)?;
            self.topo_queue.push(
                parent_commit_time,
                Info {
                    id: pid,
                    parent_ids,
                    generation: (parent_gen != gix_commitgraph::GENERATION_NUMBER_INFINITY).then_some(parent_gen),
                    commit_time: Some(parent_commit_time),
                },
            );
        }

        Ok(())
    }

    fn process_parents(&mut self, id: &oid, parents: &[Parent]) -> Result {
        let state = self.states.get_mut(id).ok_or_else(missing_state)?;
        if state.contains(WalkFlags::Added) {
            return Ok(());
        }

        *state |= WalkFlags::Added;

        // If the current commit is uninteresting we pass that on to ALL
        // parents, otherwise we set the Seen flag.
        if state.contains(WalkFlags::Uninteresting) {
            for parent in parents {
                for grand_parent in self.collect_all_parents(&parent.commit_id)? {
                    self.mark_uninteresting(&grand_parent);
                }
                self.mark_uninteresting(parent);
            }
        } else {
            for parent in parents {
                self.states.entry(parent.commit_id).or_insert(WalkFlags::Seen);
            }
        }
        Ok(())
    }

    fn mark_uninteresting(&mut self, parent: &Parent) {
        let state = self.states.entry(parent.commit_id).or_insert(WalkFlags::Seen);
        if state.contains(WalkFlags::Uninteresting) {
            return;
        }
        *state |= WalkFlags::Uninteresting;
        if state.contains(WalkFlags::Added) {
            // A late hidden mark must reach every parent, including ones skipped by first-parent traversal.
            state.remove(WalkFlags::Added);
            self.explore_queue
                .insert((parent.generation, parent.commit_time), parent.commit_id);
        }
    }

    fn collect_parents(&mut self, id: &oid) -> Result<SmallVec<[Parent; 1]>> {
        let state = self.states.get(id);
        if state.is_some_and(|state| state.contains(WalkFlags::Pruned)) {
            return Ok(SmallVec::new());
        }
        let uninteresting = state.is_some_and(|state| state.contains(WalkFlags::Uninteresting));
        let states = &mut self.states;
        let predicate = &mut self.predicate;
        collect_parents(
            &mut self.commit_graph,
            &self.find,
            id,
            matches!(self.parents, Parents::First) && !uninteresting,
            &mut self.buf,
            &mut |parent_id| uninteresting || is_selected(states, predicate, parent_id),
        )
    }

    // Same as collect_parents but disregards the first_parent flag
    pub(super) fn collect_all_parents(&mut self, id: &oid) -> Result<SmallVec<[Parent; 1]>> {
        collect_parents(
            &mut self.commit_graph,
            &self.find,
            id,
            false,
            &mut self.buf,
            &mut |_| true,
        )
    }

    pub(super) fn collect_parent_ids(&mut self, id: &oid) -> Result<ParentIds> {
        collect_parent_ids(
            &mut self.commit_graph,
            &self.find,
            id,
            matches!(self.parents, Parents::First),
            &mut self.buf,
        )
    }

    fn pop_commit(&mut self) -> Option<Result<Info>> {
        let commit = self.topo_queue.pop()?;
        let i = match self.indegrees.get_mut(&commit.id) {
            Some(i) => i,
            None => {
                return Some(Err(missing_indegree()));
            }
        };

        *i = 0;
        if let Err(e) = self.expand_topo_walk(&commit.id) {
            return Some(Err(e));
        }

        Some(Ok(commit))
    }
}

impl<Find, Predicate> Iterator for Topo<Find, Predicate>
where
    Find: gix_object::Find,
    Predicate: FnMut(&oid) -> bool,
{
    type Item = Result<Info>;

    fn next(&mut self) -> Option<Self::Item> {
        self.pop_commit()
    }
}

pub(super) fn is_selected(states: &mut IdMap<WalkFlags>, predicate: &mut impl FnMut(&oid) -> bool, id: &oid) -> bool {
    let state = states.entry(id.to_owned()).or_insert(WalkFlags::empty());
    if !state.intersects(WalkFlags::Selected | WalkFlags::Rejected) {
        state.insert(if predicate(id) {
            WalkFlags::Selected
        } else {
            WalkFlags::Rejected
        });
    }
    state.contains(WalkFlags::Selected)
}

fn collect_parents<Find>(
    cache: &mut Option<gix_commitgraph::Graph>,
    f: Find,
    id: &oid,
    first_only: bool,
    buf: &mut Vec<u8>,
    predicate: &mut impl FnMut(&oid) -> bool,
) -> Result<SmallVec<[Parent; 1]>>
where
    Find: gix_object::Find,
{
    let mut parents = SmallVec::<[Parent; 1]>::new();
    match find(cache.as_ref(), &f, id, buf)? {
        Either::CommitRefIter(c) => {
            for token in c {
                use gix_object::commit::ref_iter::Token as T;
                match token {
                    Ok(T::Tree { .. }) => continue,
                    Ok(T::Parent { id }) => {
                        parents.push(Parent {
                            commit_id: id,
                            // Dummy numbers to be filled in after parsing the parent IDs.
                            generation: 0,
                            commit_time: 0,
                        });
                        if first_only {
                            break;
                        }
                    }
                    Ok(_past_parents) => break,
                    Err(err) => {
                        return Err(err.and_raise(corruption("A commit could not be decoded during traversal")));
                    }
                }
            }
            parents.retain(|parent| predicate(&parent.commit_id));
            // Need to check the cache again. That a commit is not in the cache
            // doesn't mean a parent is not.
            for parent in parents.iter_mut() {
                let commit = find(cache.as_ref(), &f, &parent.commit_id, buf)?;
                (parent.generation, parent.commit_time) = gen_and_commit_time(commit)?;
            }
        }
        Either::CachedCommit(c) => {
            for pos in c.iter_parents() {
                let Ok(pos) = pos else {
                    // drop corrupt cache and use ODB from now on.
                    *cache = None;
                    return collect_parents(cache, f, id, first_only, buf, predicate);
                };
                let parent_commit = cache
                    .as_ref()
                    .expect("cache exists if CachedCommit was returned")
                    .commit_at(pos);
                if predicate(parent_commit.id()) {
                    parents.push(Parent {
                        commit_id: parent_commit.id().into(),
                        generation: parent_commit.generation(),
                        commit_time: parent_commit.committer_timestamp() as i64,
                    });
                }
                if first_only {
                    break;
                }
            }
        }
    }
    Ok(parents)
}

fn collect_parent_ids(
    cache: &mut Option<gix_commitgraph::Graph>,
    find_object: impl gix_object::Find,
    id: &oid,
    first_only: bool,
    buf: &mut Vec<u8>,
) -> Result<ParentIds> {
    let mut parents = ParentIds::new();
    match find(cache.as_ref(), &find_object, id, buf)? {
        Either::CommitRefIter(commit) => {
            for token in commit {
                match token {
                    Ok(gix_object::commit::ref_iter::Token::Tree { .. }) => continue,
                    Ok(gix_object::commit::ref_iter::Token::Parent { id }) => parents.push(id),
                    Ok(_) => break,
                    Err(err) => return Err(err.and_raise(corruption("A commit could not be decoded during traversal"))),
                }
                if first_only {
                    break;
                }
            }
        }
        Either::CachedCommit(commit) => {
            for pos in commit.iter_parents() {
                let Ok(pos) = pos else {
                    *cache = None;
                    return collect_parent_ids(cache, find_object, id, first_only, buf);
                };
                parents.push(
                    cache
                        .as_ref()
                        .expect("cache exists for a cached commit")
                        .commit_at(pos)
                        .id()
                        .into(),
                );
                if first_only {
                    break;
                }
            }
        }
    }
    Ok(parents)
}

pub(super) fn gen_and_commit_time(c: Either<'_, '_>) -> Result<GenAndCommitTime> {
    match c {
        Either::CommitRefIter(c) => {
            let mut commit_time = 0;
            for token in c {
                use gix_object::commit::ref_iter::Token as T;
                match token {
                    Ok(T::Tree { .. }) => continue,
                    Ok(T::Parent { .. }) => continue,
                    Ok(T::Author { .. }) => continue,
                    Ok(T::Committer { signature }) => {
                        commit_time = signature.seconds();
                        break;
                    }
                    Ok(_unused_token) => break,
                    Err(err) => {
                        return Err(err.and_raise(corruption("A commit could not be decoded during traversal")));
                    }
                }
            }
            Ok((gix_commitgraph::GENERATION_NUMBER_INFINITY, commit_time))
        }
        Either::CachedCommit(c) => Ok((c.generation(), c.committer_timestamp() as i64)),
    }
}
