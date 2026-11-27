use gix_error::{OptionExt, Result, ResultExt, bail, message};
use gix_hash::ObjectId;

use crate::{
    Graph,
    graph::{Commit, Generation, IdMap},
};

/// A containment query for one target commit and any number of candidate tips.
///
/// Completed answers are shared across tips, so their common history is traversed only once.
/// The graph can retain decoded commits across queries with different targets.
pub struct Query<'query, 'find, 'commit_graph, T = ()> {
    /// Caches decoded commits for reuse across queries with different targets.
    graph: &'query mut Graph<'find, 'commit_graph, Commit<T>>,
    target_generation: Option<Generation>,
    target_outside_graph: bool,
    /// Traversal state for this target, keyed by commits already in `graph`.
    /// Keeping it separate from `Commit<T>::data` preserves the caller's payload and avoids
    /// scanning the graph to clear stale state when starting a query for another target.
    answers: IdMap<State>,
    stack: Vec<Frame>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum State {
    /// The commit is on the active traversal stack and its answer is not yet known.
    /// Encountering it again while it is on the stack indicates an ancestry cycle.
    Visiting,
    /// The commit is the target or reaches it through at least one parent.
    Yes,
    /// The commit cannot reach the target: either none of its parents reaches it or
    /// commit-graph pruning ruled it out without walking its parents.
    No,
}

#[derive(Clone, Copy)]
struct Frame {
    commit_id: ObjectId,
    /// Zero-based index of the parent of `commit_id` to check. It stays unchanged while that parent is explored,
    /// so the walk can read its cached answer on return. It advances only after ruling out the parent.
    next_parent: usize,
}

impl<'query, 'find, 'commit_graph, T: Default> Query<'query, 'find, 'commit_graph, T> {
    /// Start a query for an existing `target_commit_id`. A supplied commit-graph can prune paths using generations.
    ///
    /// All parent links in `graph` must describe the intended history. Represent shallow boundaries by omitting
    /// those commits' parents and use a commit-graph only if it represents the same history.
    pub fn new(
        target_commit_id: impl Into<ObjectId>,
        graph: &'query mut Graph<'find, 'commit_graph, Commit<T>>,
    ) -> Result<Self> {
        let target_commit_id = target_commit_id.into();
        let generation = load_generation(graph, target_commit_id)?;
        let target_outside_graph = generation.is_none()
            && graph
                .commit_graph()
                .is_some_and(|disk| disk.lookup(target_commit_id).is_none());
        let mut answers = IdMap::default();
        answers.insert(target_commit_id, State::Yes);
        Ok(Query {
            graph,
            target_generation: generation.filter(Generation::is_reliable),
            target_outside_graph,
            answers,
            stack: Vec::new(),
        })
    }

    /// Return whether `tip_commit_id` is the target or can reach it through any parent.
    /// A missing or invalid commit encountered during traversal is an error. Completed answers remain reusable after an error.
    pub fn contains(&mut self, tip_commit_id: impl Into<ObjectId>) -> Result<bool> {
        let answer = self.walk(tip_commit_id.into());
        if answer.is_err() {
            for frame in self.stack.drain(..) {
                self.answers.remove(&frame.commit_id);
            }
        }
        answer
    }

    fn walk(&mut self, tip_commit_id: ObjectId) -> Result<bool> {
        if let Some(contains) = self.answer_or_push_frame(tip_commit_id)? {
            return Ok(contains);
        }
        while let Some(frame) = self.stack.last().copied() {
            let parent_id = self.graph[frame.commit_id.as_ref()]
                .parents
                .get(frame.next_parent)
                .copied();
            let answer = match parent_id {
                None => State::No,
                Some(parent_id) => match self.answer_or_push_frame(parent_id)? {
                    Some(true) => State::Yes,
                    Some(false) => {
                        self.stack
                            .last_mut()
                            .expect("the current frame is still on the stack")
                            .next_parent += 1;
                        continue;
                    }
                    None => continue,
                },
            };
            self.answers.insert(frame.commit_id, answer);
            self.stack.pop();
        }
        Ok(self.answers[&tip_commit_id] == State::Yes)
    }

    /// Return a known answer, or push a frame for this commit and return `Ok(None)`.
    fn answer_or_push_frame(&mut self, commit_id: ObjectId) -> Result<Option<bool>> {
        if let Some(&state) = self.answers.get(&commit_id) {
            return match state {
                State::Yes => Ok(Some(true)),
                State::No => Ok(Some(false)),
                State::Visiting => bail!(message!("Commit ancestry contains a cycle at {commit_id}").corrupted()),
            };
        }

        let generation = load_generation(self.graph, commit_id)?;
        // A commit-graph holds all ancestors of its commits. Reliable generations strictly decrease along parent links.
        if generation.is_some_and(|generation| {
            self.target_outside_graph
                || (generation.is_reliable() && self.target_generation.is_some_and(|target| generation <= target))
        }) {
            self.answers.insert(commit_id, State::No);
            return Ok(Some(false));
        }

        self.answers.insert(commit_id, State::Visiting);
        self.stack.push(Frame {
            commit_id,
            next_parent: 0,
        });
        Ok(None)
    }
}

fn load_generation<T: Default>(
    graph: &mut Graph<'_, '_, Commit<T>>,
    commit_id: ObjectId,
) -> Result<Option<Generation>> {
    Ok(graph
        .get_or_insert_commit(commit_id, |_| {})
        .or_raise(|| message!("Could not load commit {commit_id} for containment"))?
        .ok_or_raise(|| message!("Commit {commit_id} could not be found for containment").not_found())?
        .generation)
}
