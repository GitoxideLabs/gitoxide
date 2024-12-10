use crate::commit::topo::iter::gen_and_commit_time;
use crate::commit::topo::{Error, Sorting, WalkFlags};
use crate::commit::{find, Info, Parents, Topo};
use gix_hash::{oid, ObjectId};
use gix_revwalk::graph::IdMap;
use gix_revwalk::PriorityQueue;

/// Builder for [`Topo`].
pub struct Builder<Find, Predicate> {
    commit_graph: Option<gix_commitgraph::Graph>,
    find: Find,
    predicate: Predicate,
    sorting: Sorting,
    parents: Parents,
    tips: Vec<ObjectId>,
    ends: Vec<ObjectId>,
}

impl<Find> Builder<Find, fn(&oid) -> bool>
where
    Find: gix_object::Find,
{
    /// Create a new `Builder` for a [`Topo`] that reads commits from a repository with `find`.
    /// starting at the `tips` and ending at the `ends`. Like `git rev-list
    /// --topo-order ^ends tips`.
    pub fn from_iters(
        find: Find,
        tips: impl IntoIterator<Item = impl Into<ObjectId>>,
        ends: Option<impl IntoIterator<Item = impl Into<ObjectId>>>,
    ) -> Self {
        Self::new(find).with_tips(tips).with_ends(ends.into_iter().flatten())
    }

    /// Create a new `Builder` for a [`Topo`] that reads commits from a
    /// repository with `find`.
    pub fn new(find: Find) -> Self {
        Self {
            commit_graph: Default::default(),
            find,
            sorting: Default::default(),
            parents: Default::default(),
            tips: Default::default(),
            ends: Default::default(),
            predicate: |_| true,
        }
    }

    /// Set a `predicate` to filter out revisions from the walk. Can be used to
    /// implement e.g. filtering on paths or time. This does *not* exclude the
    /// parent(s) of a revision that is excluded. Specify a revision as an 'end'
    /// if you want that behavior.
    pub fn with_predicate<Predicate>(self, predicate: Predicate) -> Builder<Find, Predicate>
    where
        Predicate: FnMut(&oid) -> bool,
    {
        Builder {
            commit_graph: self.commit_graph,
            find: self.find,
            sorting: self.sorting,
            parents: self.parents,
            tips: self.tips,
            ends: self.ends,
            predicate,
        }
    }
}

impl<Find, Predicate> Builder<Find, Predicate>
where
    Find: gix_object::Find,
    Predicate: FnMut(&oid) -> bool,
{
    /// Add commits to start reading from.
    ///
    /// The behavior is similar to specifying additional `ends` in `git rev-list --topo-order ^ends tips`.
    pub fn with_tips(mut self, tips: impl IntoIterator<Item = impl Into<ObjectId>>) -> Self {
        self.tips.extend(tips.into_iter().map(Into::into));
        self
    }

    /// Add commits ending the traversal.
    ///
    /// These commits themselves will not be read, i.e. the behavior is similar to specifying additional
    /// `ends` in `git rev-list --topo-order ^ends tips`.
    pub fn with_ends(mut self, ends: impl IntoIterator<Item = impl Into<ObjectId>>) -> Self {
        self.ends.extend(ends.into_iter().map(Into::into));
        self
    }

    /// Set the `sorting` to use for the topological walk.
    pub fn sorting(mut self, sorting: Sorting) -> Self {
        self.sorting = sorting;
        self
    }

    /// Specify how to handle commit `parents` during traversal.
    pub fn parents(mut self, parents: Parents) -> Self {
        self.parents = parents;
        self
    }

    /// Set or unset the `commit_graph` to use for the iteration.
    pub fn with_commit_graph(mut self, commit_graph: Option<gix_commitgraph::Graph>) -> Self {
        self.commit_graph = commit_graph;
        self
    }

    /// Build a new [`Topo`] instance.
    ///
    /// Note that merely building an instance is currently expensive.
    pub fn build(self) -> Result<Topo<Find, Predicate>, Error> {
        let mut w = Topo {
            commit_graph: self.commit_graph,
            find: self.find,
            predicate: self.predicate,
            indegrees: IdMap::default(),
            states: IdMap::default(),
            explore_queue: PriorityQueue::new(),
            indegree_queue: PriorityQueue::new(),
            topo_queue: super::iter::Queue::new(self.sorting),
            parents: self.parents,
            min_gen: gix_commitgraph::GENERATION_NUMBER_INFINITY,
            buf: vec![],
        };

        // Initial flags for the states of the tips and ends. All of them are
        // seen and added to the explore and indegree queues. The ends are by
        // definition (?) uninteresting and bottom.
        let tip_flags = WalkFlags::Seen | WalkFlags::Explored | WalkFlags::InDegree;
        let end_flags = tip_flags | WalkFlags::Uninteresting | WalkFlags::Bottom;

        for (id, flags) in self
            .tips
            .iter()
            .map(|id| (id, tip_flags))
            .chain(self.ends.iter().map(|id| (id, end_flags)))
        {
            *w.indegrees.entry(*id).or_default() = 1;
            let commit = find(w.commit_graph.as_ref(), &w.find, id, &mut w.buf)?;
            let (gen, time) = gen_and_commit_time(commit)?;

            if gen < w.min_gen {
                w.min_gen = gen;
            }

            w.states.insert(*id, flags);
            w.explore_queue.insert((gen, time), *id);
            w.indegree_queue.insert((gen, time), *id);
        }

        // NOTE: Parents of the ends must also be marked uninteresting for some
        // reason. See handle_commit()
        for id in &self.ends {
            let parents = w.collect_all_parents(id)?;
            for (id, _) in parents {
                w.states
                    .entry(id)
                    .and_modify(|s| *s |= WalkFlags::Uninteresting)
                    .or_insert(WalkFlags::Uninteresting | WalkFlags::Seen);
            }
        }

        w.compute_indegrees_to_depth(w.min_gen)?;

        // NOTE: in Git the ends are also added to the topo_queue in addition to
        // the tips, but then in simplify_commit() Git is told to ignore it. For
        // now the tests pass.
        for id in self.tips.iter() {
            let i = w.indegrees.get(id).ok_or(Error::MissingIndegreeUnexpected)?;

            if *i != 1 {
                continue;
            }

            let commit = find(w.commit_graph.as_ref(), &w.find, id, &mut w.buf)?;
            let (_, time) = gen_and_commit_time(commit)?;
            let parent_ids = w.collect_all_parents(id)?.into_iter().map(|e| e.0).collect();

            w.topo_queue.push(
                time,
                Info {
                    id: *id,
                    parent_ids,
                    commit_time: Some(time),
                },
            );
        }

        w.topo_queue.initial_sort();
        Ok(w)
    }
}
