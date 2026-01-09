use std::cmp::Ordering;

use gix_hash::ObjectId;
use gix_revwalk::graph;

use super::{Error, Simple};
use crate::{merge_base::Flags, Graph, PriorityQueue};

/// Given a commit at `first` id, traverse the commit `graph` and return all possible merge-base between it and `others`,
/// sorted from best to worst. Returns `None` if there is no merge-base as `first` and `others` don't share history.
/// If `others` is empty, `Some(first)` is returned.
///
/// Note that this function doesn't do any work if `first` is contained in `others`, which is when `first` will be returned
/// as only merge-base right away. This is even the case if some commits of `others` are disjoint.
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
pub fn merge_base(
    first: ObjectId,
    others: &[ObjectId],
    graph: &mut Graph<'_, '_, graph::Commit<Flags>>,
) -> Result<Option<Vec<ObjectId>>, Error> {
    let _span = gix_trace::coarse!("gix_revision::merge_base()", ?first, ?others);
    if others.is_empty() || others.contains(&first) {
        return Ok(Some(vec![first]));
    }

    graph.clear_commit_data(|f| *f = Flags::empty());
    let bases = paint_down_to_common(first, others, graph)?;

    let bases = remove_redundant(&bases, graph)?;
    Ok((!bases.is_empty()).then_some(bases))
}

/// Remove all those commits from `commits` if they are in the history of another commit in `commits`.
/// That way, we return only the topologically most recent commits in `commits`.
fn remove_redundant(
    commits: &[(ObjectId, GenThenTime)],
    graph: &mut Graph<'_, '_, graph::Commit<Flags>>,
) -> Result<Vec<ObjectId>, Error> {
    if commits.is_empty() {
        return Ok(Vec::new());
    }
    graph.clear_commit_data(|f| *f = Flags::empty());
    let _span = gix_trace::detail!("gix_revision::remove_redundant()", num_commits = %commits.len());
    let sorted_commits = {
        let mut v = commits.to_vec();
        v.sort_by(|a, b| a.1.cmp(&b.1));
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
                .map_err(|_| Simple("could not insert parent commit into graph"))?;
        }
    }
    walk_start.sort_by(|a, b| a.0.cmp(&b.0));
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
                    .map_err(|_| Simple("could not insert parent commit into graph"))?
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

fn paint_down_to_common(
    first: ObjectId,
    others: &[ObjectId],
    graph: &mut Graph<'_, '_, graph::Commit<Flags>>,
) -> Result<Vec<(ObjectId, GenThenTime)>, Error> {
    let mut queue = PriorityQueue::<GenThenTime, ObjectId>::new();
    graph.get_or_insert_full_commit(first, |commit| {
        commit.data |= Flags::COMMIT1;
        queue.insert(GenThenTime::from(&*commit), first);
    })
    .map_err(|_| Simple("could not insert commit into graph"))?;

    for other in others {
        graph
            .get_or_insert_full_commit(*other, |commit| {
                commit.data |= Flags::COMMIT2;
                queue.insert(GenThenTime::from(&*commit), *other);
            })
            .map_err(|_| Simple("could not insert commit into graph"))?;
    }

    let mut out = Vec::new();
    while queue
        .iter_unordered()
        .any(|id| graph.get(id).is_some_and(|commit| !commit.data.contains(Flags::STALE)))
    {
        let (info, commit_id) = queue.pop().expect("we have non-stale");
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
            graph
                .get_or_insert_full_commit(parent_id, |parent| {
                    if (parent.data & flags_without_result) != flags_without_result {
                        parent.data |= flags_without_result;
                        queue.insert(GenThenTime::from(&*parent), parent_id);
                    }
                })
                .map_err(|_| Simple("could not insert parent commit into graph"))?;
        }
    }

    Ok(out)
}

// TODO(ST): Should this type be used for `describe` as well?
#[derive(Debug, Clone, Copy)]
struct GenThenTime {
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
