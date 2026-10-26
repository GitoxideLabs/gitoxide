use gix_error::Result;
use std::collections::HashSet;

use gix_error::OptionExt;
use gix_hash::{ObjectId, oid};
use gix_revwalk::{PriorityQueue, graph::IdMap};

use crate::commit::{
    Info, Parents, Topo, find,
    topo::{
        Sorting, WalkFlags,
        iter::{gen_and_commit_time, is_selected},
    },
};

/// Builder for [`Topo`].
pub struct Builder<Find, Predicate> {
    commit_graph: Option<gix_commitgraph::Graph>,
    find: Find,
    predicate: Predicate,
    sorting: Sorting,
    parents: Parents,
    tips: Vec<ObjectId>,
    tips_seen: HashSet<ObjectId>,
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
            tips_seen: Default::default(),
            ends: Default::default(),
            predicate: |_| true,
        }
    }

    /// Include a commit and traverse its parents only if `predicate` returns `true`.
    /// The predicate runs once per visible candidate, before looking it up. Rejected paths
    /// stop, but their ancestors can still be reached through other accepted paths.
    /// Hidden ancestry always follows all parents, independently of this predicate.
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
            tips_seen: self.tips_seen,
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
    /// The behavior is similar to specifying additional `tips` in `git rev-list --topo-order ^ends tips`.
    pub fn with_tips(mut self, tips: impl IntoIterator<Item = impl Into<ObjectId>>) -> Self {
        for tip in tips.into_iter().map(Into::into) {
            if self.tips_seen.insert(tip) {
                self.tips.push(tip);
            }
        }
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
    /// Without a commit graph, this visits the visible history before yielding commits.
    /// Hidden ancestry is limited with Git's date-based heuristic, as in [`crate::commit::Simple::hide()`];
    /// extreme clock skew can leave hidden ancestors beyond the stopping point.
    pub fn build(self) -> Result<Topo<Find, Predicate>> {
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
        let mut tips = self.tips;
        tips.retain(|id| is_selected(&mut w.states, &mut w.predicate, id));

        // Initial flags for the states of the tips and ends. All of them are
        // seen and added to the explore and indegree queues. The ends are by
        // definition (?) uninteresting and bottom.
        let tip_flags = WalkFlags::Seen | WalkFlags::Explored | WalkFlags::InDegree;
        let end_flags = tip_flags | WalkFlags::Uninteresting | WalkFlags::Bottom;

        for (id, flags) in tips
            .iter()
            .map(|id| (id, tip_flags))
            .chain(self.ends.iter().map(|id| (id, end_flags)))
        {
            *w.indegrees.entry(*id).or_default() = 1;
            let commit = find(w.commit_graph.as_ref(), &w.find, id, &mut w.buf)?;
            let (generation, time) = gen_and_commit_time(commit)?;

            if generation < w.min_gen {
                w.min_gen = generation;
            }

            w.states.entry(*id).or_insert(WalkFlags::empty()).insert(flags);
            w.explore_queue.insert((generation, time), *id);
            w.indegree_queue.insert((generation, time), *id);
        }

        if w.commit_graph.is_none() && !self.ends.is_empty() {
            let mut sorted_tips = tips.clone();
            sorted_tips.sort_unstable();
            let hidden = crate::commit::simple::compute_hidden_frontier(
                &sorted_tips,
                &self.ends,
                &w.find,
                None,
                w.parents,
                |id| is_selected(&mut w.states, &mut w.predicate, id),
            )?;
            // Limit the range before computing indegrees, as Git does before sorting a limited walk.
            for id in hidden.keys().chain(&self.ends) {
                w.states
                    .entry(*id)
                    .or_insert(WalkFlags::Seen)
                    .insert(WalkFlags::Uninteresting | WalkFlags::Pruned);
            }
        } else {
            // NOTE: Parents of the ends must also be marked uninteresting for some
            // reason. See handle_commit()
            for id in &self.ends {
                let parents = w.collect_all_parents(id)?;
                for parent in parents {
                    w.states
                        .entry(parent.commit_id)
                        .and_modify(|s| *s |= WalkFlags::Uninteresting)
                        .or_insert(WalkFlags::Uninteresting | WalkFlags::Seen);
                }
            }
        }

        w.compute_indegrees_to_depth(w.min_gen)?;

        // NOTE: in Git the ends are also added to the topo_queue in addition to
        // the tips, but then in simplify_commit() Git is told to ignore it. For
        // now the tests pass.
        for id in tips.iter() {
            let i = w
                .indegrees
                .get(id)
                .ok_or_raise(|| gix_error::corruption("Indegree information is missing"))?;

            if *i != 1
                || w.states
                    .get(id)
                    .is_some_and(|state| state.contains(WalkFlags::Uninteresting))
            {
                continue;
            }

            let commit = find(w.commit_graph.as_ref(), &w.find, id, &mut w.buf)?;
            let (generation, time) = gen_and_commit_time(commit)?;
            let parent_ids = w.collect_parent_ids(id)?;

            w.topo_queue.push(
                time,
                Info {
                    id: *id,
                    parent_ids,
                    generation: (generation != gix_commitgraph::GENERATION_NUMBER_INFINITY).then_some(generation),
                    commit_time: Some(time),
                },
            );
        }

        w.topo_queue.initial_sort();
        Ok(w)
    }
}
