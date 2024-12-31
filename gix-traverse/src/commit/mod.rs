//! Provide multiple traversal implementations with different performance envelopes.
//!
//! Use [`Simple`] for fast walks that maintain minimal state, or [`Topo`] for a more elaborate traversal.
use gix_hash::ObjectId;
use gix_object::FindExt;
use gix_revwalk::graph::IdMap;
use gix_revwalk::PriorityQueue;
use smallvec::SmallVec;

/// A fast iterator over the ancestors of one or more starting commits.
pub struct Simple<Find, Predicate> {
    objects: Find,
    cache: Option<gix_commitgraph::Graph>,
    predicate: Predicate,
    state: simple::State,
    parents: Parents,
    sorting: simple::Sorting,
}

/// Simple ancestors traversal, without the need to keep track of graph-state.
pub mod simple;

/// A commit walker that walks in topographical order, like `git rev-list
/// --topo-order` or `--date-order` depending on the chosen [`topo::Sorting`].
///
/// Instantiate with [`topo::Builder`].
pub struct Topo<Find, Predicate> {
    commit_graph: Option<gix_commitgraph::Graph>,
    find: Find,
    predicate: Predicate,
    indegrees: IdMap<i32>,
    states: IdMap<topo::WalkFlags>,
    explore_queue: PriorityQueue<topo::iter::GenAndCommitTime, ObjectId>,
    indegree_queue: PriorityQueue<topo::iter::GenAndCommitTime, ObjectId>,
    topo_queue: topo::iter::Queue,
    parents: Parents,
    min_gen: u32,
    buf: Vec<u8>,
}

pub mod topo;

/// Specify how to handle commit parents during traversal.
#[derive(Default, Copy, Clone)]
pub enum Parents {
    /// Traverse all parents, useful for traversing the entire ancestry.
    #[default]
    All,
    /// Only traverse along the first parent, which commonly ignores all branches.
    First,
}

/// The collection of parent ids we saw as part of the iteration.
///
/// Note that this list is truncated if [`Parents::First`] was used.
pub type ParentIds = SmallVec<[gix_hash::ObjectId; 1]>;

/// Information about a commit that we obtained naturally as part of the iteration.
#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct Info {
    /// The id of the commit.
    pub id: gix_hash::ObjectId,
    /// All parent ids we have encountered. Note that these will be at most one if [`Parents::First`] is enabled.
    pub parent_ids: ParentIds,
    /// The time at which the commit was created. It will only be `Some(_)` if the chosen traversal was
    /// taking dates into consideration.
    pub commit_time: Option<gix_date::SecondsSinceUnixEpoch>,
}

/// TODO
pub enum Either<'buf, 'cache> {
    /// TODO
    CommitRefIter(gix_object::CommitRefIter<'buf>),
    /// TODO
    CachedCommit(gix_commitgraph::file::Commit<'cache>),
}

/// TODO
pub fn find<'cache, 'buf, Find>(
    cache: Option<&'cache gix_commitgraph::Graph>,
    objects: Find,
    id: &gix_hash::oid,
    buf: &'buf mut Vec<u8>,
) -> Result<Either<'buf, 'cache>, gix_object::find::existing_iter::Error>
where
    Find: gix_object::Find,
{
    match cache.and_then(|cache| cache.commit_by_id(id).map(Either::CachedCommit)) {
        Some(c) => Ok(c),
        None => objects.find_commit_iter(id, buf).map(Either::CommitRefIter),
    }
}
