use std::cmp::{Ordering, Reverse};

use gix_error::{OptionExt, Result, ResultExt, message};
use gix_hash::ObjectId;
use gix_revwalk::graph;

use crate::{Graph, PriorityQueue, merge_base::Flags};

/// Given a commit at `first` id, traverse the commit `graph` and return all possible merge-base between it and `others`,
/// sorted by descending commit time. Equal timestamps retain traversal order.
/// Returns `None` if there is no merge-base as `first` and `others` don't share history.
/// If `others` is empty, `Some(first)` is returned.
///
/// All input commits must exist. After validating them, if `first` is contained in `others`, it is returned as the
/// only merge-base without traversing their parents. This is even the case if some commits of `others` are disjoint.
///
/// Additionally, this function isn't stable and results may differ dependeing on the order in which `first` and `others` are
/// provided due to its special rules.
///
/// If a stable result is needed, use [`merge_base::octopus()`](crate::merge_base::octopus()).
///
/// # Performance
///
/// For repeated calls, be sure to re-use `graph` as its content will be kept and reused for a great speed-up. The contained flags
/// will automatically be cleared.
/// With a commit-graph providing reliable generations, the walk stops once either side's non-stale
/// frontier is exhausted and no merge-base candidates remain queued.
pub fn merge_base(
    first: ObjectId,
    others: &[ObjectId],
    graph: &mut Graph<'_, '_, graph::Commit<Flags>>,
) -> Result<Option<nonempty::NonEmpty<ObjectId>>> {
    let _span = gix_trace::coarse!("gix_revision::merge_base()", ?first, ?others);
    insert_input_commits(first, others, graph)?;
    if others.is_empty() || others.contains(&first) {
        return Ok(Some(nonempty::NonEmpty::new(first)));
    }

    graph.clear_commit_data(|f| *f = Flags::empty());
    let bases = paint_down_to_common(first, others, None, false, graph)?;

    let mut bases = match bases.as_slice() {
        [] => return Ok(None),
        [(base_commit_id, _)] => return Ok(Some(nonempty::NonEmpty::new(*base_commit_id))),
        _ => remove_redundant(&bases, graph)?,
    };
    bases.sort_by_key(|commit_id| Reverse(graph.get(commit_id).expect("merge bases were traversed").commit_time));
    Ok(nonempty::NonEmpty::from_vec(bases))
}

/// Return `true` if `ancestor_commit_id` can be reached from `descendant_commit_id` by following parents, similar to
/// `git merge-base --is-ancestor`. Equal IDs return `true` without consulting the graph.
/// Queries across unrelated histories return `false`.
///
/// For distinct IDs, both input commits and all traversed parents must exist. Represent intentional shallow
/// boundaries by clearing those roots' parents in the graph.
///
/// # Performance
///
/// For repeated calls, be sure to re-use `graph` as its content will be kept and reused for a great speed-up. The contained flags
/// will automatically be cleared.
/// With a commit-graph, there is no traversal if `descendant_commit_id` is in it but `ancestor_commit_id` isn't, or if
/// both have nonzero, unsaturated generations and the one of `ancestor_commit_id` isn't lower. Otherwise, such a
/// generation of `ancestor_commit_id` stops the walk once only commits with a lower generation remain.
pub fn is_ancestor(
    ancestor_commit_id: ObjectId,
    descendant_commit_id: ObjectId,
    graph: &mut Graph<'_, '_, graph::Commit<Flags>>,
) -> Result<bool> {
    let _span = gix_trace::coarse!(
        "gix_revision::merge_base::is_ancestor()",
        ?ancestor_commit_id,
        ?descendant_commit_id
    );
    if ancestor_commit_id == descendant_commit_id {
        return Ok(true);
    }
    insert_input_commits(ancestor_commit_id, &[descendant_commit_id], graph)?;

    let ancestor = GenThenTime::from(graph.get(&ancestor_commit_id).expect("input commits were inserted"));
    let descendant = GenThenTime::from(graph.get(&descendant_commit_id).expect("input commits were inserted"));
    // A commit-graph contains all ancestors of its commits, and parents have lower generations than their children.
    if (ancestor.is_outside_commit_graph() && !descendant.is_outside_commit_graph())
        || (ancestor.has_reliable_generation()
            && descendant.has_reliable_generation()
            && ancestor.generation >= descendant.generation)
    {
        return Ok(false);
    }

    graph.clear_commit_data(|f| *f = Flags::empty());
    let min_generation = ancestor.has_reliable_generation().then_some(ancestor.generation);
    paint_down_to_common(ancestor_commit_id, &[descendant_commit_id], min_generation, true, graph)?;
    Ok(graph
        .get(&ancestor_commit_id)
        .expect("input commits were inserted")
        .data
        .contains(Flags::COMMIT2))
}

/// Validate input tips even when a shortcut or unrelated histories would otherwise stop traversal early.
pub(crate) fn insert_input_commits(
    first_commit_id: ObjectId,
    others: &[ObjectId],
    graph: &mut Graph<'_, '_, graph::Commit<Flags>>,
) -> Result {
    for commit_id in std::iter::once(&first_commit_id).chain(others) {
        graph
            .get_or_insert_full_commit(*commit_id, |_| {})
            .or_raise(|| message("could not insert commit into graph"))?
            .ok_or_raise(|| message!("Commit {commit_id} could not be found for revision traversal").not_found())?;
    }
    Ok(())
}

/// Remove all those commits from `commits` if they are in the history of another commit in `commits`.
/// That way, we return only the topologically most recent commits in `commits`.
/// Called with multiple candidate commits.
fn remove_redundant(
    commits: &[(ObjectId, GenThenTime)],
    graph: &mut Graph<'_, '_, graph::Commit<Flags>>,
) -> Result<Vec<ObjectId>> {
    graph.clear_commit_data(|f| *f = Flags::empty());
    let _span = gix_trace::detail!("gix_revision::remove_redundant()", num_commits = %commits.len());
    let sorted_commits = {
        let mut v = commits.to_vec();
        v.sort_by_key(|a| a.1);
        v
    };
    let mut min_gen_pos = 0;
    let mut min_gen = sorted_commits[min_gen_pos].1.generation;

    let mut walk_start = Vec::with_capacity(commits.len());
    for (id, _) in commits {
        let commit = graph.get_mut(id).expect("previously added");
        commit.data |= Flags::RESULT;
        for parent_id in commit.parents.clone() {
            graph
                .get_or_insert_full_commit(parent_id, |parent| {
                    // prevent double-addition
                    if !parent.data.contains(Flags::STALE) {
                        parent.data |= Flags::STALE;
                        walk_start.push((parent_id, GenThenTime::from(&*parent)));
                    }
                })
                .or_raise(|| message("could not insert parent commit into graph"))?;
        }
    }
    walk_start.sort_by_key(|a| a.0);
    // allow walking everything at first.
    walk_start
        .iter_mut()
        .for_each(|(id, _)| graph.get_mut(id).expect("added previously").data.remove(Flags::STALE));
    let mut count_still_independent = commits.len();

    let mut stack = Vec::new();
    while let Some((commit_id, commit_info)) = walk_start.pop().filter(|_| count_still_independent > 1) {
        stack.clear();
        graph.get_mut(&commit_id).expect("added").data |= Flags::STALE;
        stack.push((commit_id, commit_info));

        while let Some((commit_id, commit_info)) = stack.last().copied() {
            let commit = graph.get_mut(&commit_id).expect("all commits have been added");
            let commit_parents = commit.parents.clone();
            if commit.data.contains(Flags::RESULT) {
                commit.data.remove(Flags::RESULT);
                count_still_independent -= 1;
                if count_still_independent <= 1 {
                    break;
                }
                if *commit_id == *sorted_commits[min_gen_pos].0 {
                    while min_gen_pos < commits.len() - 1
                        && graph
                            .get(&sorted_commits[min_gen_pos].0)
                            .expect("already added")
                            .data
                            .contains(Flags::STALE)
                    {
                        min_gen_pos += 1;
                    }
                    min_gen = sorted_commits[min_gen_pos].1.generation;
                }
            }

            if commit_info.generation < min_gen {
                stack.pop();
                continue;
            }

            let previous_len = stack.len();
            for parent_id in &commit_parents {
                if graph
                    .get_or_insert_full_commit(*parent_id, |parent| {
                        if !parent.data.contains(Flags::STALE) {
                            parent.data |= Flags::STALE;
                            stack.push((*parent_id, GenThenTime::from(&*parent)));
                        }
                    })
                    .or_raise(|| message("could not insert parent commit into graph"))?
                    .is_some()
                {
                    break;
                }
            }

            if previous_len == stack.len() {
                stack.pop();
            }
        }
    }

    Ok(commits
        .iter()
        .filter_map(|(id, _info)| {
            graph
                .get(id)
                .filter(|commit| !commit.data.contains(Flags::STALE))
                .map(|_| *id)
        })
        .collect())
}

struct PaintQueue {
    queue: PriorityQueue<(u64, gix_date::SecondsSinceUnixEpoch, Reverse<usize>), ObjectId>,
    order: PaintOrder,
    /// Git visits equal-priority commits in insertion order.
    next_sequence: usize,
    /// Non-stale queued commits carrying each color. Candidates count toward both sides, keeping the walk alive
    /// until they have been recorded, even if no exclusively colored commits remain on one side.
    non_stale: [usize; 2],
}

enum PaintOrder {
    CorrectedCommitDate,
    /// Ancestor queries can stop below a topological generation, so must visit that generation first.
    TopologicalGeneration,
}

impl PaintQueue {
    fn update_counts(&mut self, flags: Flags, add: bool) {
        if flags.contains(Flags::STALE) {
            return;
        }
        for (side, count) in [Flags::COMMIT1, Flags::COMMIT2].into_iter().zip(&mut self.non_stale) {
            if flags.contains(side) {
                if add {
                    *count += 1;
                } else {
                    *count -= 1;
                }
            }
        }
    }

    fn insert(&mut self, commit_id: ObjectId, commit: &mut graph::Commit<Flags>, flags: Flags) {
        if commit.data.contains(Flags::ENQUEUED) {
            self.update_counts(commit.data, false);
        } else {
            let generation = match self.order {
                PaintOrder::CorrectedCommitDate => {
                    commit.corrected_commit_date.map_or(u64::MAX, std::num::NonZeroU64::get)
                }
                PaintOrder::TopologicalGeneration => {
                    u64::from(commit.generation.unwrap_or(gix_commitgraph::GENERATION_NUMBER_INFINITY))
                }
            };
            self.queue
                .insert((generation, commit.commit_time, Reverse(self.next_sequence)), commit_id);
            self.next_sequence += 1;
            commit.data |= Flags::ENQUEUED;
        }
        commit.data |= flags;
        self.update_counts(commit.data, true);
    }

    fn pop(&mut self, graph: &mut Graph<'_, '_, graph::Commit<Flags>>) -> Option<(GenThenTime, ObjectId)> {
        let (_, commit_id) = self.queue.pop()?;
        let commit = graph.get_mut(&commit_id).expect("everything queued is in graph");
        let info = GenThenTime::from(&*commit);
        // Keep this commit counted until after the exit check so the last pending candidate is processed.
        // Side exhaustion is only final when children are visited before parents.
        let topologically_ordered = match self.order {
            PaintOrder::CorrectedCommitDate => commit.corrected_commit_date.is_some_and(|date| date.get() < u64::MAX),
            PaintOrder::TopologicalGeneration => info.has_reliable_generation(),
        };
        if self.non_stale == [0, 0] || (topologically_ordered && self.non_stale.contains(&0)) {
            return None;
        }
        commit.data.remove(Flags::ENQUEUED);
        self.update_counts(commit.data, false);
        Some((info, commit_id))
    }
}

/// Paint the histories of `first` and `others` in their own color and return the merge-base candidates found on the way.
/// With `min_generation`, stop once only commits with a lower reliable generation remain, as these can't reach a commit
/// of that generation. The returned candidates are incomplete then.
fn paint_down_to_common(
    first: ObjectId,
    others: &[ObjectId],
    min_generation: Option<graph::Generation>,
    require_parents: bool,
    graph: &mut Graph<'_, '_, graph::Commit<Flags>>,
) -> Result<Vec<(ObjectId, GenThenTime)>> {
    let mut queue = PaintQueue {
        queue: PriorityQueue::new(),
        order: if min_generation.is_none()
            && graph
                .commit_graph()
                .is_some_and(gix_commitgraph::Graph::corrected_commit_dates_available)
        {
            PaintOrder::CorrectedCommitDate
        } else {
            PaintOrder::TopologicalGeneration
        },
        next_sequence: 0,
        non_stale: [0; 2],
    };
    graph
        .get_or_insert_full_commit(first, |commit| {
            queue.insert(first, commit, Flags::COMMIT1);
        })
        .or_raise(|| message("could not insert commit into graph"))?;

    for other in others {
        graph
            .get_or_insert_full_commit(*other, |commit| {
                queue.insert(*other, commit, Flags::COMMIT2);
            })
            .or_raise(|| message("could not insert commit into graph"))?;
    }

    let mut out = Vec::new();
    while let Some((info, commit_id)) = queue.pop(graph) {
        if let Some(min_generation) = min_generation
            && info.has_reliable_generation()
            && info.generation < min_generation
        {
            break;
        }
        let commit = graph.get_mut(&commit_id).expect("everything queued is in graph");
        let mut flags_without_result = commit.data & (Flags::COMMIT1 | Flags::COMMIT2 | Flags::STALE);
        if flags_without_result == (Flags::COMMIT1 | Flags::COMMIT2) {
            if !commit.data.contains(Flags::RESULT) {
                commit.data |= Flags::RESULT;
                out.push((commit_id, info));
            }
            flags_without_result |= Flags::STALE;
        }

        for parent_id in commit.parents.clone() {
            let parent = graph
                .get_or_insert_full_commit(parent_id, |parent| {
                    if (parent.data & flags_without_result) != flags_without_result {
                        queue.insert(parent_id, parent, flags_without_result);
                    }
                })
                .or_raise(|| message("could not insert parent commit into graph"))?;
            if require_parents {
                parent.ok_or_raise(|| {
                    message!("Parent commit {parent_id} could not be found for ancestry traversal").not_found()
                })?;
            }
        }
    }

    Ok(out)
}

// TODO(ST): Should this type be used for `describe` as well?
#[derive(Debug, Clone, Copy)]
pub(crate) struct GenThenTime {
    /// Note that the special [`GENERATION_NUMBER_INFINITY`](gix_commitgraph::GENERATION_NUMBER_INFINITY) is used to indicate
    /// that no commitgraph is available.
    generation: gix_revwalk::graph::Generation,
    time: gix_date::SecondsSinceUnixEpoch,
}

impl From<&graph::Commit<Flags>> for GenThenTime {
    fn from(commit: &graph::Commit<Flags>) -> Self {
        GenThenTime {
            generation: commit.generation.unwrap_or(gix_commitgraph::GENERATION_NUMBER_INFINITY),
            time: commit.commit_time,
        }
    }
}

impl GenThenTime {
    /// Return `true` if the commit has no generation as it isn't in a commit-graph.
    pub(crate) fn is_outside_commit_graph(&self) -> bool {
        self.generation == gix_commitgraph::GENERATION_NUMBER_INFINITY
    }

    /// Return `true` if the generation is from a commit-graph and neither zero nor saturated, as only then all parents
    /// of the commit are known to have a lower generation.
    pub(crate) fn has_reliable_generation(&self) -> bool {
        self.generation > 0 && self.generation < gix_commitgraph::GENERATION_NUMBER_MAX
    }
}

impl Eq for GenThenTime {}

impl PartialEq<Self> for GenThenTime {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl PartialOrd<Self> for GenThenTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GenThenTime {
    fn cmp(&self, other: &Self) -> Ordering {
        self.generation.cmp(&other.generation).then(self.time.cmp(&other.time))
    }
}
