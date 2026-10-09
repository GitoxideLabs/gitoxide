use gix_error::{Result, ResultExt, message};
use gix_hash::ObjectId;
use gix_revwalk::graph;

use crate::{
    Graph, PriorityQueue,
    merge_base::{
        Flags,
        function::{GenThenTime, insert_input_commits},
    },
};

/// Commit counts for a tip relative to a base.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AheadBehind {
    /// Commits reachable from the tip, but not from the base.
    pub ahead: usize,
    /// Commits reachable from the base, but not from the tip.
    pub behind: usize,
}

/// Count commits reachable exclusively from `tip_commit_id` or `base_commit_id`, matching
/// `git rev-list --left-right --count tip...base`. Equal commits return zero for both counts.
/// All input commits must exist. Missing parents are treated as the end of the available history.
///
/// # Performance
///
/// Reuse `graph` to retain cached commits between calls. Its flags are automatically cleared.
/// With nonzero, unsaturated commit-graph generations, traversal stops once only shared history remains.
/// Otherwise, the walk may visit all available ancestors to account for colors arriving at already visited commits.
/// Exact counts can require substantially more traversal than [`is_ancestor()`](crate::merge_base::is_ancestor()).
pub fn ahead_behind(
    tip_commit_id: ObjectId,
    base_commit_id: ObjectId,
    graph: &mut Graph<'_, '_, graph::Commit<Flags>>,
) -> Result<AheadBehind> {
    let _span = gix_trace::coarse!("gix_revision::ahead_behind()", ?tip_commit_id, ?base_commit_id);
    insert_input_commits(tip_commit_id, &[base_commit_id], graph)?;
    if tip_commit_id == base_commit_id {
        return Ok(AheadBehind::default());
    }

    graph.clear_commit_data(|flags| *flags = Flags::empty());
    let both = Flags::COMMIT1 | Flags::COMMIT2;
    let mut queue = PriorityQueue::new();
    let mut pending_exclusive = 2;
    let mut unreliable_generations = false;
    for (commit_id, color) in [(tip_commit_id, Flags::COMMIT1), (base_commit_id, Flags::COMMIT2)] {
        let commit = graph.get_mut(&commit_id).expect("input commits were inserted");
        let info = GenThenTime::from(&*commit);
        unreliable_generations |= !info.has_reliable_generation();
        commit.data = color | Flags::ENQUEUED;
        queue.insert(info, commit_id);
    }

    while pending_exclusive != 0 || unreliable_generations {
        let Some((_, commit_id)) = queue.pop() else {
            break;
        };
        let commit = graph.get_mut(&commit_id).expect("everything queued is in graph");
        let colors = commit.data & both;
        pending_exclusive -= usize::from(colors != both);
        commit.data.remove(Flags::ENQUEUED);
        for parent_commit_id in commit.parents.clone() {
            graph
                .get_or_insert_full_commit(parent_commit_id, |parent| {
                    if parent.data.contains(colors) {
                        return;
                    }
                    let info = GenThenTime::from(&*parent);
                    unreliable_generations |= !info.has_reliable_generation();
                    if parent.data.contains(Flags::ENQUEUED) {
                        pending_exclusive -= usize::from((parent.data & both) != both);
                    } else {
                        queue.insert(info, parent_commit_id);
                        parent.data |= Flags::ENQUEUED;
                    }
                    parent.data |= colors;
                    pending_exclusive += usize::from((parent.data & both) != both);
                })
                .or_raise(|| message("could not insert parent commit into graph for ahead/behind traversal"))?;
        }
    }

    // Date ordering can revisit commits, so count only after their colors have settled.
    let mut counts = AheadBehind::default();
    graph.clear_commit_data(|flags| {
        let colors = *flags & both;
        counts.ahead += usize::from(colors == Flags::COMMIT1);
        counts.behind += usize::from(colors == Flags::COMMIT2);
        *flags = Flags::empty();
    });
    Ok(counts)
}
