use crate::commit::topo::{Error, Sorting, WalkFlags};
use crate::commit::{find, Either, Info, Parents, Topo};
use gix_hash::{oid, ObjectId};
use gix_revwalk::PriorityQueue;
use smallvec::SmallVec;

pub(in crate::commit) type GenAndCommitTime = (u32, i64);

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
        if let Self::Topo(ref mut inner_vec) = self {
            inner_vec.sort_by(|a, b| a.0.cmp(&b.0));
        }
    }
}

impl<Find, Predicate> Topo<Find, Predicate>
where
    Find: gix_object::Find,
{
    pub(super) fn compute_indegrees_to_depth(&mut self, gen_cutoff: u32) -> Result<(), Error> {
        while let Some(((gen, _), _)) = self.indegree_queue.peek() {
            if *gen >= gen_cutoff {
                self.indegree_walk_step()?;
            } else {
                break;
            }
        }

        Ok(())
    }

    fn indegree_walk_step(&mut self) -> Result<(), Error> {
        if let Some(((gen, _), id)) = self.indegree_queue.pop() {
            self.explore_to_depth(gen)?;

            let parents = self.collect_parents(&id)?;
            for (id, gen_time) in parents {
                self.indegrees.entry(id).and_modify(|e| *e += 1).or_insert(2);

                let state = self.states.get_mut(&id).ok_or(Error::MissingStateUnexpected)?;
                if !state.contains(WalkFlags::InDegree) {
                    *state |= WalkFlags::InDegree;
                    self.indegree_queue.insert(gen_time, id);
                }
            }
        }
        Ok(())
    }

    fn explore_to_depth(&mut self, gen_cutoff: u32) -> Result<(), Error> {
        while let Some(((gen, _), _)) = self.explore_queue.peek() {
            if *gen >= gen_cutoff {
                self.explore_walk_step()?;
            } else {
                break;
            }
        }
        Ok(())
    }

    fn explore_walk_step(&mut self) -> Result<(), Error> {
        if let Some((_, id)) = self.explore_queue.pop() {
            let parents = self.collect_parents(&id)?;
            self.process_parents(&id, &parents)?;

            for (id, gen_time) in parents {
                let state = self.states.get_mut(&id).ok_or(Error::MissingStateUnexpected)?;

                if !state.contains(WalkFlags::Explored) {
                    *state |= WalkFlags::Explored;
                    self.explore_queue.insert(gen_time, id);
                }
            }
        }
        Ok(())
    }

    fn expand_topo_walk(&mut self, id: &oid) -> Result<(), Error> {
        let parents = self.collect_parents(id)?;
        self.process_parents(id, &parents)?;

        for (pid, (parent_gen, parent_commit_time)) in parents {
            let parent_state = self.states.get(&pid).ok_or(Error::MissingStateUnexpected)?;
            if parent_state.contains(WalkFlags::Uninteresting) {
                continue;
            }

            if parent_gen < self.min_gen {
                self.min_gen = parent_gen;
                self.compute_indegrees_to_depth(self.min_gen)?;
            }

            let i = self.indegrees.get_mut(&pid).ok_or(Error::MissingIndegreeUnexpected)?;
            *i -= 1;
            if *i != 1 {
                continue;
            }

            let parent_ids = self.collect_all_parents(&pid)?.into_iter().map(|e| e.0).collect();
            self.topo_queue.push(
                parent_commit_time,
                Info {
                    id: pid,
                    parent_ids,
                    commit_time: Some(parent_commit_time),
                },
            );
        }

        Ok(())
    }

    fn process_parents(&mut self, id: &oid, parents: &[(ObjectId, GenAndCommitTime)]) -> Result<(), Error> {
        let state = self.states.get_mut(id).ok_or(Error::MissingStateUnexpected)?;
        if state.contains(WalkFlags::Added) {
            return Ok(());
        }

        *state |= WalkFlags::Added;

        // If the current commit is uninteresting we pass that on to ALL
        // parents, otherwise we set the Seen flag.
        let (pass, insert) = if state.contains(WalkFlags::Uninteresting) {
            let flags = WalkFlags::Uninteresting;
            for (id, _) in parents {
                let grand_parents = self.collect_all_parents(id)?;

                for (id, _) in &grand_parents {
                    self.states
                        .entry(*id)
                        .and_modify(|s| *s |= WalkFlags::Uninteresting)
                        .or_insert(WalkFlags::Uninteresting | WalkFlags::Seen);
                }
            }
            (flags, flags)
        } else {
            // NOTE: git sets SEEN like we do but keeps the SYMMETRIC_LEFT and
            // ANCENSTRY_PATH if they are set, but they have no purpose here.
            let flags = WalkFlags::empty();
            (flags, WalkFlags::Seen)
        };

        for (id, _) in parents {
            self.states.entry(*id).and_modify(|s| *s |= pass).or_insert(insert);
        }
        Ok(())
    }

    fn collect_parents(&mut self, id: &oid) -> Result<SmallVec<[(ObjectId, GenAndCommitTime); 1]>, Error> {
        collect_parents(
            &mut self.commit_graph,
            &self.find,
            id,
            matches!(self.parents, Parents::First),
            &mut self.buf,
        )
    }

    // Same as collect_parents but disregards the first_parent flag
    pub(super) fn collect_all_parents(
        &mut self,
        id: &oid,
    ) -> Result<SmallVec<[(ObjectId, GenAndCommitTime); 1]>, Error> {
        collect_parents(&mut self.commit_graph, &self.find, id, false, &mut self.buf)
    }

    fn pop_commit(&mut self) -> Option<Result<Info, Error>> {
        let commit = self.topo_queue.pop()?;
        let i = match self.indegrees.get_mut(&commit.id) {
            Some(i) => i,
            None => {
                return Some(Err(Error::MissingIndegreeUnexpected));
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
    type Item = Result<Info, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.pop_commit()? {
                Ok(id) => {
                    if (self.predicate)(&id.id) {
                        return Some(Ok(id));
                    }
                }
                Err(e) => return Some(Err(e)),
            }
        }
    }
}

fn collect_parents<Find>(
    cache: &mut Option<gix_commitgraph::Graph>,
    f: Find,
    id: &oid,
    first_only: bool,
    buf: &mut Vec<u8>,
) -> Result<SmallVec<[(ObjectId, GenAndCommitTime); 1]>, Error>
where
    Find: gix_object::Find,
{
    let mut parents = SmallVec::<[(ObjectId, GenAndCommitTime); 1]>::new();
    match find(cache.as_ref(), &f, id, buf)? {
        Either::CommitRefIter(c) => {
            for token in c {
                use gix_object::commit::ref_iter::Token as T;
                match token {
                    Ok(T::Tree { .. }) => continue,
                    Ok(T::Parent { id }) => {
                        parents.push((id, (0, 0))); // Dummy numbers to be filled in
                        if first_only {
                            break;
                        }
                    }
                    Ok(_past_parents) => break,
                    Err(err) => return Err(err.into()),
                }
            }
            // Need to check the cache again. That a commit is not in the cache
            // doesn't mean a parent is not.
            for (id, gen_time) in parents.iter_mut() {
                let commit = find(cache.as_ref(), &f, id, buf)?;
                *gen_time = gen_and_commit_time(commit)?;
            }
        }
        Either::CachedCommit(c) => {
            for pos in c.iter_parents() {
                let Ok(pos) = pos else {
                    // drop corrupt cache and use ODB from now on.
                    *cache = None;
                    return collect_parents(cache, f, id, first_only, buf);
                };
                let parent_commit = cache
                    .as_ref()
                    .expect("cache exists if CachedCommit was returned")
                    .commit_at(pos);
                parents.push((
                    parent_commit.id().into(),
                    (parent_commit.generation(), parent_commit.committer_timestamp() as i64),
                ));
                if first_only {
                    break;
                }
            }
        }
    }
    Ok(parents)
}

pub(super) fn gen_and_commit_time(c: Either<'_, '_>) -> Result<GenAndCommitTime, Error> {
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
                    Err(err) => return Err(err.into()),
                }
            }
            Ok((gix_commitgraph::GENERATION_NUMBER_INFINITY, commit_time))
        }
        Either::CachedCommit(c) => Ok((c.generation(), c.committer_timestamp() as i64)),
    }
}
