use gix_date::SecondsSinceUnixEpoch;
use gix_hash::ObjectId;

use crate::{Flags, Metadata, Negotiator};

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
    fn add_to_queue(
        &mut self,
        id: ObjectId,
        mark: Flags,
        graph: &mut crate::Graph<'_, '_>,
    ) -> Result<(), gix_error::Exn> {
        let commit = graph.get_or_insert_commit(id, |entry| {
            entry.flags |= mark | Flags::SEEN;
        })?;
        if let Some(timestamp) = commit.map(|c| c.commit_time) {
            self.revs.insert(timestamp, id);
            if !mark.contains(Flags::COMMON) {
                self.non_common_revs += 1;
            }
        }
        Ok(())
    }

    fn mark_common(&mut self, id: ObjectId, graph: &mut crate::Graph<'_, '_>) -> Result<(), gix_error::Exn> {
        let mut is_common = false;
        if let Some(commit) = graph
            .get_or_insert_commit(id, |entry| {
                is_common = entry.flags.contains(Flags::COMMON);
                entry.flags |= Flags::COMMON;
            })?
            .filter(|_| !is_common)
        {
            let mut queue = gix_revwalk::PriorityQueue::from_iter(Some((commit.commit_time, id)));
            while let Some(id) = queue.pop_value() {
                if let Some(commit) = graph.get_or_insert_commit(id, |entry| {
                    if !entry.flags.contains(Flags::POPPED) {
                        self.non_common_revs -= 1;
                    }
                })? {
                    for parent_id in commit.parents.clone() {
                        // This is a bit of a problem as there is no representation of the `parsed` based skip. However,
                        // We assume that parents that aren't in the graph yet haven't been seen, and that's all we need.
                        if !graph.contains(&parent_id) {
                            continue;
                        }
                        let mut was_unseen_or_common = false;
                        if let Some(parent) = graph
                            .get_or_insert_commit(parent_id, |entry| {
                                was_unseen_or_common =
                                    !entry.flags.contains(Flags::SEEN) || entry.flags.contains(Flags::COMMON);
                                entry.flags |= Flags::COMMON;
                            })?
                            .filter(|_| !was_unseen_or_common)
                        {
                            queue.insert(parent.commit_time, parent_id);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn push_parent(
        &mut self,
        entry: Metadata,
        parent_id: ObjectId,
        graph: &mut crate::Graph<'_, '_>,
    ) -> Result<bool, gix_error::Exn> {
        let mut was_seen = false;
        if let Some(parent) = graph
            .get(&parent_id)
            .inspect(|parent| {
                was_seen = parent.data.flags.contains(Flags::SEEN);
            })
            .filter(|_| was_seen)
        {
            if parent.data.flags.contains(Flags::POPPED) {
                return Ok(false);
            }
        } else {
            self.add_to_queue(parent_id, Flags::default(), graph)?;
        }
        if entry.flags.intersects(Flags::COMMON | Flags::ADVERTISED) {
            self.mark_common(parent_id, graph)?;
        } else {
            let new_original_ttl = if entry.ttl > 0 {
                entry.original_ttl
            } else {
                entry.original_ttl * 3 / 2 + 1
            };
            let new_ttl = if entry.ttl > 0 { entry.ttl - 1 } else { new_original_ttl };
            let parent = graph.get_mut(&parent_id).expect("present or inserted");
            if parent.data.original_ttl < new_original_ttl {
                parent.data.original_ttl = new_original_ttl;
                parent.data.ttl = new_ttl;
            }
        }
        Ok(true)
    }
}

impl Negotiator for Algorithm {
    fn known_common(&mut self, id: ObjectId, graph: &mut crate::Graph<'_, '_>) -> Result<(), gix_error::Exn> {
        if graph
            .get(&id)
            .is_some_and(|commit| commit.data.flags.contains(Flags::SEEN))
        {
            return Ok(());
        }
        self.add_to_queue(id, Flags::ADVERTISED, graph)
    }

    fn add_tip(&mut self, id: ObjectId, graph: &mut crate::Graph<'_, '_>) -> Result<(), gix_error::Exn> {
        if graph
            .get(&id)
            .is_some_and(|commit| commit.data.flags.contains(Flags::SEEN))
        {
            return Ok(());
        }
        self.add_to_queue(id, Flags::default(), graph)
    }

    fn next_have(&mut self, graph: &mut crate::Graph<'_, '_>) -> Option<Result<ObjectId, gix_error::Exn>> {
        loop {
            let id = self.revs.pop_value().filter(|_| self.non_common_revs != 0)?;
            let commit = graph.get_mut(&id).expect("it was added to the graph by now");
            commit.data.flags |= Flags::POPPED;

            if !commit.data.flags.contains(Flags::COMMON) {
                self.non_common_revs -= 1;
            }
            let mut to_send = None;
            if !commit.data.flags.contains(Flags::COMMON) && commit.data.ttl == 0 {
                to_send = Some(id);
            }

            let data = commit.data;
            let mut parent_pushed = false;
            for parent_id in commit.parents.clone() {
                parent_pushed |= match self.push_parent(data, parent_id, graph) {
                    Ok(r) => r,
                    Err(err) => return Some(Err(err)),
                }
            }

            if !data.flags.contains(Flags::COMMON) && !parent_pushed {
                to_send = Some(id);
            }

            if let Some(to_send) = to_send {
                return Some(Ok(to_send));
            }
        }
    }

    fn in_common_with_remote(
        &mut self,
        id: ObjectId,
        graph: &mut crate::Graph<'_, '_>,
    ) -> Result<bool, gix_error::Exn> {
        let mut was_seen = false;
        let known_to_be_common = graph.get(&id).is_some_and(|commit| {
            was_seen = commit.data.flags.contains(Flags::SEEN);
            commit.data.flags.contains(Flags::COMMON)
        });
        assert!(
            was_seen,
            "Cannot receive ACK for commit we didn't send a HAVE for: {id}"
        );
        self.mark_common(id, graph)?;
        Ok(known_to_be_common)
    }
}
