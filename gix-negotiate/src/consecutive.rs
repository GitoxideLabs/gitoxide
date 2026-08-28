use gix_date::SecondsSinceUnixEpoch;
use gix_hash::ObjectId;

use crate::{Error, Flags, Negotiator};

pub(crate) struct Algorithm {
    revs: gix_revwalk::PriorityQueue<SecondsSinceUnixEpoch, ObjectId>,
    non_common_revs: usize,
}

impl Default for Algorithm {
    fn default() -> Self {
        Self {
            revs: gix_revwalk::PriorityQueue::new(),
            non_common_revs: 0,
        }
    }
}

impl Algorithm {
    /// Add `id` to our priority queue and *add* `flags` to it.
    fn add_to_queue(&mut self, id: ObjectId, mark: Flags, graph: &mut crate::Graph<'_, '_>) -> Result<(), Error> {
        let mut is_common = false;
        let mut has_mark = false;
        if let Some(commit) = graph
            .get_or_insert_commit(id, |data| {
                has_mark = data.flags.intersects(mark);
                data.flags |= mark;
                is_common = data.flags.contains(Flags::COMMON);
            })?
            .filter(|_| !has_mark)
        {
            self.revs.insert(commit.commit_time, id);
            if !is_common {
                self.non_common_revs += 1;
            }
        }
        Ok(())
    }

    fn mark_common(
        &mut self,
        id: ObjectId,
        mode: Mark,
        ancestors: Ancestors,
        graph: &mut crate::Graph<'_, '_>,
    ) -> Result<(), Error> {
        let mut is_common = false;
        if let Some(commit) = graph
            .get_or_insert_commit(id, |data| is_common = data.flags.contains(Flags::COMMON))?
            .filter(|_| !is_common)
        {
            let mut queue = gix_revwalk::PriorityQueue::from_iter(Some((commit.commit_time, (id, 0_usize))));
            if let Mark::ThisCommitAndAncestors = mode {
                commit.data.flags |= Flags::COMMON;
                if commit.data.flags.contains(Flags::SEEN) && !commit.data.flags.contains(Flags::POPPED) {
                    self.non_common_revs -= 1;
                }
            }
            while let Some((id, generation)) = queue.pop_value() {
                if graph
                    .get(&id)
                    .is_none_or(|commit| !commit.data.flags.contains(Flags::SEEN))
                {
                    self.add_to_queue(id, Flags::SEEN, graph)?;
                } else if (matches!(ancestors, Ancestors::AllUnseen) || generation < 2)
                    && let Some(commit) = graph.get_or_insert_commit(id, |_| {})?
                {
                    for parent_id in commit.parents.clone() {
                        let mut prev_flags = Flags::default();
                        if let Some(parent) = graph
                            .get_or_insert_commit(parent_id, |data| {
                                prev_flags = data.flags;
                                data.flags |= Flags::COMMON;
                            })?
                            .filter(|_| !prev_flags.contains(Flags::COMMON))
                        {
                            if prev_flags.contains(Flags::SEEN) && !prev_flags.contains(Flags::POPPED) {
                                self.non_common_revs -= 1;
                            }
                            queue.insert(parent.commit_time, (parent_id, generation + 1));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl Negotiator for Algorithm {
    fn known_common(&mut self, id: ObjectId, graph: &mut crate::Graph<'_, '_>) -> Result<(), Error> {
        if graph
            .get(&id)
            .is_none_or(|commit| !commit.data.flags.contains(Flags::SEEN))
        {
            self.add_to_queue(id, Flags::COMMON_REF | Flags::SEEN, graph)?;
            self.mark_common(id, Mark::AncestorsOnly, Ancestors::DirectUnseen, graph)?;
        }
        Ok(())
    }

    fn add_tip(&mut self, id: ObjectId, graph: &mut crate::Graph<'_, '_>) -> Result<(), Error> {
        self.add_to_queue(id, Flags::SEEN, graph)
    }

    fn next_have(&mut self, graph: &mut crate::Graph<'_, '_>) -> Option<Result<ObjectId, Error>> {
        loop {
            let id = self.revs.pop_value().filter(|_| self.non_common_revs != 0)?;
            let commit = graph.get_mut(&id).expect("it was added to the graph by now");
            let flags = &mut commit.data.flags;
            *flags |= Flags::POPPED;

            if !flags.contains(Flags::COMMON) {
                self.non_common_revs -= 1;
            }

            let (res, mark) = if flags.contains(Flags::COMMON) {
                (None, Flags::COMMON | Flags::SEEN)
            } else if flags.contains(Flags::COMMON_REF) {
                (Some(id), Flags::COMMON | Flags::SEEN)
            } else {
                (Some(id), Flags::SEEN)
            };

            for parent_id in commit.parents.clone() {
                if graph
                    .get(&parent_id)
                    .is_none_or(|commit| !commit.data.flags.contains(Flags::SEEN))
                    && let Err(err) = self.add_to_queue(parent_id, mark, graph)
                {
                    return Some(Err(err));
                }
                if mark.contains(Flags::COMMON)
                    && let Err(err) = self.mark_common(parent_id, Mark::AncestorsOnly, Ancestors::AllUnseen, graph)
                {
                    return Some(Err(err));
                }
            }

            if let Some(id) = res {
                return Some(Ok(id));
            }
        }
    }

    fn in_common_with_remote(&mut self, id: ObjectId, graph: &mut crate::Graph<'_, '_>) -> Result<bool, Error> {
        let known_to_be_common = graph
            .get(&id)
            .is_some_and(|commit| commit.data.flags.contains(Flags::COMMON));
        self.mark_common(id, Mark::ThisCommitAndAncestors, Ancestors::DirectUnseen, graph)?;
        Ok(known_to_be_common)
    }
}

enum Mark {
    AncestorsOnly,
    ThisCommitAndAncestors,
}

enum Ancestors {
    /// Traverse only the parents of a commit.
    DirectUnseen,
    /// Traverse all ancestors that weren't yet seen.
    AllUnseen,
}
