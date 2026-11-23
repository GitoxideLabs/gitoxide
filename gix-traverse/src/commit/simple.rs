use std::{
    cmp::{Ordering, Reverse},
    collections::VecDeque,
};

use gix_date::SecondsSinceUnixEpoch;
use gix_error::Result;
use gix_hash::{ObjectId, oid};
use smallvec::SmallVec;

use crate::commit::Parents;

#[derive(Default, Debug, Copy, Clone)]
/// The order with which to prioritize the search.
pub enum CommitTimeOrder {
    #[default]
    /// Sort commits by newest first.
    NewestFirst,
    /// Sort commits by oldest first.
    #[doc(alias = "Sort::REVERSE", alias = "git2")]
    OldestFirst,
}

/// Specify how to sort commits during a [simple](super::Simple) traversal.
///
/// ### Sample History
///
/// The following history will be referred to for explaining how the sort order works, with the number denoting the commit timestamp
/// (*their X-alignment doesn't matter*).
///
/// ```text
/// ---1----2----4----7 <- second parent of 8
///     \              \
///      3----5----6----8---
/// ```
#[derive(Default, Debug, Copy, Clone)]
pub enum Sorting {
    /// Commits are sorted as they are mentioned in the commit graph.
    ///
    /// In the *sample history* the order would be `8, 6, 7, 5, 4, 3, 2, 1`.
    ///
    /// ### Note
    ///
    /// This is not to be confused with `git log/rev-list --topo-order`, which is notably different from
    /// as it avoids overlapping branches.
    #[default]
    BreadthFirst,
    /// Commits are sorted by their commit time in the order specified, either newest or oldest first.
    ///
    /// The sorting applies to all currently queued commit ids and thus is full.
    ///
    /// In the *sample history* the order would be `8, 7, 6, 5, 4, 3, 2, 1` for [`NewestFirst`](CommitTimeOrder::NewestFirst),
    /// or `1, 2, 3, 4, 5, 6, 7, 8` for [`OldestFirst`](CommitTimeOrder::OldestFirst).
    ///
    /// # Performance
    ///
    /// This mode benefits greatly from having an object_cache in `find()`
    /// to avoid having to lookup each commit twice.
    ByCommitTime(CommitTimeOrder),
    /// This sorting is similar to [`ByCommitTime`](Sorting::ByCommitTime), but adds a cutoff to not return commits older than
    /// a given time, stopping the iteration once no younger commits is queued to be traversed.
    ///
    /// As the query is usually repeated with different cutoff dates, this search mode benefits greatly from an object cache.
    ///
    /// In the *sample history* and a cut-off date of 4, the returned list of commits would be `8, 7, 6, 4`.
    ByCommitTimeCutoff {
        /// The order in which to prioritize lookups.
        order: CommitTimeOrder,
        /// The number of seconds since unix epoch, the same value obtained by any `gix_date::Time` structure and the way git counts time.
        seconds: gix_date::SecondsSinceUnixEpoch,
    },
}

use std::result::Result as Either;

type QueueKey<T> = Either<T, Reverse<T>>;
type CommitDateQueue = gix_revwalk::PriorityQueue<QueueKey<SecondsSinceUnixEpoch>, ObjectId>;

bitflags::bitflags! {
    #[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
    struct PaintFlags: u8 {
        const VISIBLE = 1 << 0;
        const HIDDEN = 1 << 1;
        const STALE = 1 << 2;
    }
}

/// Priority for hidden-frontier painting that prefers newer commits, using generation numbers
/// when available and falling back to commit time as a tie-breaker.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct GenThenTime {
    generation: gix_revwalk::graph::Generation,
    time: SecondsSinceUnixEpoch,
}

impl<T> From<&gix_revwalk::graph::Commit<T>> for GenThenTime {
    fn from(commit: &gix_revwalk::graph::Commit<T>) -> Self {
        GenThenTime {
            generation: commit.generation.unwrap_or(gix_commitgraph::GENERATION_NUMBER_INFINITY),
            time: commit.commit_time,
        }
    }
}

impl Ord for GenThenTime {
    fn cmp(&self, other: &Self) -> Ordering {
        self.generation.cmp(&other.generation).then(self.time.cmp(&other.time))
    }
}

impl PartialOrd<Self> for GenThenTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The state used and potentially shared by multiple graph traversals.
#[derive(Clone)]
pub(super) struct State {
    /// Pending visible commits when traversal is driven in insertion/topological order.
    ///
    /// This queue is consumed by `next_by_topology()`, and also becomes the active frontier for
    /// first-parent traversal after any time-ordered queue is flattened back into FIFO order.
    next: VecDeque<ObjectId>,
    /// Pending visible commits when traversal is driven by commit date.
    ///
    /// This queue is consumed by `next_by_commit_date()`. It holds the same logical frontier as
    /// `next`, but keeps it ordered by commit time instead of insertion order.
    queue: CommitDateQueue,
    /// Backing storage for the currently yielded commit.
    buf: Vec<u8>,
    /// The object hash kind of the currently yielded commit data in `buf`.
    /// It's used to know the kind of hash to expect when a new iterator is returned from `buf`
    /// via `Simple::commit_iter()`.
    object_hash: gix_hash::Kind,
    /// Set of commits that were already enqueued for the visible traversal, for cycle-checking.
    seen: gix_hashtable::HashSet<ObjectId>,
    /// Hidden frontier commits that must not be yielded or crossed during traversal.
    hidden: gix_revwalk::graph::IdMap<()>,
    /// Hidden input tips from which the hidden frontier is derived.
    ///
    /// These are consumed on the first call ot `next` to compute the hidden frontier once.
    hidden_tips: Vec<ObjectId>,
    /// Scratch buffer for parent commit lookups when commit times are loaded from the object database.
    parents_buf: Vec<u8>,
    /// Reusable parent id/time storage populated from the commit-graph cache.
    parent_ids: SmallVec<[(ObjectId, SecondsSinceUnixEpoch); 2]>,
}

fn to_queue_key(i: i64, order: CommitTimeOrder) -> QueueKey<i64> {
    match order {
        CommitTimeOrder::NewestFirst => Ok(i),
        CommitTimeOrder::OldestFirst => Err(Reverse(i)),
    }
}

/// Commits marked both visible and hidden during painting. The visible walk neither yields these
/// commits nor follows their parents, so the frontier need not contain the full hidden ancestry.
///
/// For example, with parents to the left:
///
/// ```text
///              A --- V (visible tip)
///             /
/// R --- P --- X
///             \
///              B --- H (hidden tip)
/// ```
///
/// Both paths reach `X`. With usable generation numbers, painting can stop there: the frontier
/// contains just `X`, and the visible walk yields `V` and `A` without visiting `P` or `R`.
type Frontier = gix_revwalk::graph::IdMap<()>;
type CommitMetadata = gix_revwalk::graph::Commit<()>;

struct PaintedCommit<'a> {
    parents: &'a crate::commit::ParentIds,
    priority: GenThenTime,
    flags: PaintFlags,
}

impl<'a> PaintedCommit<'a> {
    fn new<T>(commit: &'a gix_revwalk::graph::Commit<T>, flags: PaintFlags) -> Self {
        PaintedCommit {
            parents: &commit.parents,
            priority: GenThenTime::from(commit),
            flags,
        }
    }
}

trait Paint {
    fn get(&self, id: &oid) -> Option<PaintedCommit<'_>>;
    fn update(&mut self, id: ObjectId, update: impl FnOnce(GenThenTime, &mut PaintFlags)) -> Result<()>;
    fn mark_hidden(&mut self, id: &oid) -> Option<&crate::commit::ParentIds>;
    fn into_frontier(self) -> Frontier;
}

/// Paint a single walk without a shared cache. The temporary graph keeps metadata and flags together;
/// only the frontier IDs survive after painting.
impl Paint for gix_revwalk::Graph<'_, '_, gix_revwalk::graph::Commit<PaintFlags>> {
    fn get(&self, id: &oid) -> Option<PaintedCommit<'_>> {
        self.get(id).map(|commit| PaintedCommit::new(commit, commit.data))
    }

    fn update(&mut self, id: ObjectId, update: impl FnOnce(GenThenTime, &mut PaintFlags)) -> Result<()> {
        self.get_or_insert_full_commit(id, |commit| update(GenThenTime::from(&*commit), &mut commit.data))?;
        Ok(())
    }

    fn mark_hidden(&mut self, id: &oid) -> Option<&crate::commit::ParentIds> {
        let commit = self.get_mut(id)?;
        commit.data |= PaintFlags::HIDDEN;
        Some(&commit.parents)
    }

    fn into_frontier(self) -> Frontier {
        self.detach()
            .into_iter()
            .filter_map(|(id, commit)| {
                commit
                    .data
                    .contains(PaintFlags::VISIBLE | PaintFlags::HIDDEN)
                    .then_some((id, ()))
            })
            .collect()
    }
}

/// Paint a single walk using metadata that can be reused across walks with the same object lookup
/// and commit-graph snapshot.
///
/// Only commits in `flags` count as visited by this walk. Metadata cached by earlier walks must not
/// change which commits the date cutoff can mark as hidden.
struct SharedPaint<'a> {
    metadata: &'a mut gix_revwalk::graph::IdMap<CommitMetadata>,
    lookup: gix_revwalk::Graph<'a, 'a, ()>,
    flags: gix_revwalk::graph::IdMap<PaintFlags>,
}

impl Paint for SharedPaint<'_> {
    fn get(&self, id: &oid) -> Option<PaintedCommit<'_>> {
        let flags = *self.flags.get(id)?;
        Some(PaintedCommit::new(
            self.metadata.get(id).expect("painted commits have metadata"),
            flags,
        ))
    }

    fn update(&mut self, id: ObjectId, update: impl FnOnce(GenThenTime, &mut PaintFlags)) -> Result<()> {
        use gix_hashtable::hash_map::Entry;
        let commit = match self.metadata.entry(id) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let Some(commit) = self.lookup.try_lookup(&id)? else {
                    return Ok(());
                };
                entry.insert(commit.to_owned(|| ())?)
            }
        };
        update(GenThenTime::from(&*commit), self.flags.entry(id).or_default());
        Ok(())
    }

    fn mark_hidden(&mut self, id: &oid) -> Option<&crate::commit::ParentIds> {
        // The date cutoff only paints commits loaded by this walk, even if other metadata is cached.
        *self.flags.get_mut(id)? |= PaintFlags::HIDDEN;
        Some(&self.metadata.get(id).expect("painted commits have metadata").parents)
    }

    fn into_frontier(self) -> Frontier {
        self.flags
            .into_iter()
            .filter_map(|(id, flags)| {
                flags
                    .contains(PaintFlags::VISIBLE | PaintFlags::HIDDEN)
                    .then_some((id, ()))
            })
            .collect()
    }
}

/// Compute the boundary at which the visible walk must stop because commits become reachable from
/// both the visible tips and the hidden tips.
///
/// The algorithm performs a merge-base-style paint:
/// visible tips are marked with `VISIBLE`, hidden tips with `HIDDEN`, and these flags are
/// propagated to parents in generation/time order. Once a commit carries both flags, it is part
/// of the overlap between the two histories and is marked `STALE`. With generation numbers,
/// older ancestors no longer need to keep re-propagating the same combined state.
/// Without them, Git's date-based heuristic stops after five hidden commits when only hidden
/// commits older than the last visible commit remain queued. Extreme clock skew can put hidden
/// ancestors beyond this stopping point.
/// With usable generation numbers, painting also stops once every queued commit carries `HIDDEN`,
/// like Git's `everybody_uninteresting()` in `limit_list()`. Generation order makes this exact:
/// no popped commit is reachable from a queued commit, so popped commits keep their flags.
/// Every commit the visible walk asks about, a tip or a parent of a commit it yields,
/// was either popped already or is still queued with both flags.
///
/// The returned set is not all commits reachable from hidden tips. It covers the overlap frontier
/// where the visible traversal must stop. The actual `Simple` walk then skips these commits and
/// refuses to enqueue parents across them, which avoids traversing hidden-only history.
/// `visible_tips` must be sorted and contain no duplicates.
/// `parents` and `predicate` restrict visible paths only; hidden ancestry always follows all parents.
pub(super) fn compute_hidden_frontier(
    visible_tips: &[ObjectId],
    hidden_tips: &[ObjectId],
    objects: &impl gix_object::Find,
    cache: Option<&gix_commitgraph::Graph>,
    parents: Parents,
    predicate: impl FnMut(&oid) -> bool,
) -> Result<gix_revwalk::graph::IdMap<()>> {
    paint_frontier(
        visible_tips,
        hidden_tips,
        gix_revwalk::Graph::new(objects, cache),
        cache.is_some_and(|cache| !cache.generation_numbers_available()),
        parents,
        predicate,
    )
}

fn paint_frontier(
    visible_tips: &[ObjectId],
    hidden_tips: &[ObjectId],
    mut graph: impl Paint,
    mut saw_uncomputed_generation: bool,
    parents: Parents,
    mut predicate: impl FnMut(&oid) -> bool,
) -> Result<Frontier> {
    let mut queue = gix_revwalk::PriorityQueue::<GenThenTime, ObjectId>::new();
    let mut unhidden_tips = visible_tips.len();
    let mut last_visible_time = SecondsSinceUnixEpoch::MAX;
    let mut slop = 5;
    let mut stopped_on_date = false;
    // Queued commits painted only `VISIBLE`. They were queued once, when they gained that flag.
    let mut visible_only_commit_ids = gix_hashtable::HashSet::default();

    for &visible in visible_tips {
        graph.update(visible, |priority, flags| {
            *flags |= PaintFlags::VISIBLE;
            saw_uncomputed_generation |= priority.generation == 0;
            visible_only_commit_ids.insert(visible);
            queue.insert(priority, visible);
        })?;
    }
    for &hidden in hidden_tips {
        graph.update(hidden, |priority, flags| {
            if *flags == PaintFlags::VISIBLE {
                unhidden_tips -= 1;
                visible_only_commit_ids.remove(&hidden);
            }
            saw_uncomputed_generation |= priority.generation == 0;
            *flags |= PaintFlags::HIDDEN;
            queue.insert(priority, hidden);
        })?;
    }

    while unhidden_tips != 0 {
        // A queued visible-only commit already proves there is non-stale work.
        if visible_only_commit_ids.is_empty() {
            // With zero-generation layers ruled out, a usable queue head means all queued commits have one.
            if !saw_uncomputed_generation
                && queue
                    .peek()
                    .is_some_and(|(key, _)| (1..gix_commitgraph::GENERATION_NUMBER_MAX).contains(&key.generation))
            {
                break;
            }
            if queue.iter_unordered().all(|id| {
                graph.get(id).is_none_or(|commit| {
                    commit.flags.contains(PaintFlags::STALE)
                        && (1..gix_commitgraph::GENERATION_NUMBER_MAX).contains(&commit.priority.generation)
                })
            }) {
                break;
            }
        }
        let (_info, commit_id) = match queue.pop() {
            Some(v) => v,
            None => break,
        };
        let commit = graph.get(&commit_id).expect("queued commits are in the graph");
        let mut flags = commit.flags;
        let parent_ids = commit.parents.clone();
        let use_date_cutoff = !(1..gix_commitgraph::GENERATION_NUMBER_MAX).contains(&commit.priority.generation);
        if !flags.contains(PaintFlags::HIDDEN) {
            last_visible_time = commit.priority.time;
            visible_only_commit_ids.remove(&commit_id);
        }
        if flags == (PaintFlags::VISIBLE | PaintFlags::HIDDEN) {
            flags |= PaintFlags::STALE;
        }

        for (index, parent_id) in parent_ids.into_iter().enumerate() {
            if !flags.contains(PaintFlags::HIDDEN) {
                if index > 0 && matches!(parents, Parents::First) {
                    break;
                }
                if !predicate(&parent_id) {
                    continue;
                }
            }
            graph.update(parent_id, |priority, paint| {
                saw_uncomputed_generation |= priority.generation == 0;
                if (*paint & flags) != flags {
                    if flags.contains(PaintFlags::HIDDEN) && *paint == PaintFlags::VISIBLE {
                        visible_only_commit_ids.remove(&parent_id);
                        if visible_tips.binary_search(&parent_id).is_ok() {
                            unhidden_tips -= 1;
                        }
                    }
                    *paint |= flags;
                    if *paint == PaintFlags::VISIBLE {
                        visible_only_commit_ids.insert(parent_id);
                    }
                    queue.insert(priority, parent_id);
                }
            })?;
        }

        if use_date_cutoff && flags.contains(PaintFlags::HIDDEN) {
            // Git's limit_list() allows five older hidden commits before stopping without generations.
            if queue.iter_unordered().any(|id| {
                let commit = graph.get(id).expect("queued commits are in the graph");
                !commit.flags.contains(PaintFlags::HIDDEN) || commit.priority.time >= last_visible_time
            }) {
                slop = 5;
            } else {
                slop -= 1;
                if slop == 0 {
                    stopped_on_date = true;
                    break;
                }
            }
        }
    }

    if stopped_on_date {
        // Like Git's mark_parents_uninteresting(), finish painting already-loaded ancestry without more I/O.
        let mut pending = hidden_tips.to_vec();
        let mut seen = gix_hashtable::HashSet::default();
        while let Some(commit_id) = pending.pop() {
            if seen.insert(commit_id)
                && let Some(parents) = graph.mark_hidden(&commit_id)
            {
                pending.extend(parents);
            }
        }
    }

    Ok(graph.into_frontier())
}

///
mod init {
    use super::{
        CommitDateQueue, CommitTimeOrder, SharedPaint, Sorting, State, collect_parents, compute_hidden_frontier,
        paint_frontier, to_queue_key,
    };
    use crate::commit::{Either, Info, ParentIds, Parents, Simple};
    use gix_date::SecondsSinceUnixEpoch;
    use gix_error::{ErrorExt, Result, ResultExt, bail, corruption, message};
    use gix_hash::{ObjectId, oid};
    use gix_object::{CommitRefIter, FindExt};
    use std::{borrow::Borrow, cmp::Reverse, collections::VecDeque};

    impl Default for State {
        fn default() -> Self {
            State {
                next: Default::default(),
                queue: gix_revwalk::PriorityQueue::new(),
                buf: vec![],
                object_hash: gix_hash::Kind::shortest(),
                seen: Default::default(),
                hidden: Default::default(),
                hidden_tips: Vec::new(),
                parents_buf: vec![],
                parent_ids: Default::default(),
            }
        }
    }

    impl State {
        fn clear(&mut self) {
            let Self {
                next,
                queue,
                buf,
                object_hash,
                seen,
                hidden,
                hidden_tips,
                parents_buf: _,
                parent_ids: _,
            } = self;
            next.clear();
            queue.clear();
            buf.clear();
            *object_hash = gix_hash::Kind::shortest();
            seen.clear();
            hidden.clear();
            hidden_tips.clear();
        }
    }

    impl Sorting {
        /// If not topo sort, provide the cutoff date if present.
        fn cutoff_time(&self) -> Option<SecondsSinceUnixEpoch> {
            match self {
                Sorting::ByCommitTimeCutoff { seconds, .. } => Some(*seconds),
                _ => None,
            }
        }
    }

    /// Builder methods
    impl<Find, Predicate, CommitGraph> Simple<Find, Predicate, CommitGraph>
    where
        Find: gix_object::Find,
        CommitGraph: Borrow<gix_commitgraph::Graph>,
    {
        /// Set the `sorting` method.
        pub fn sorting(mut self, sorting: Sorting) -> Result<Self> {
            self.sorting = sorting;
            match self.sorting {
                Sorting::BreadthFirst => self.queue_to_vecdeque(),
                Sorting::ByCommitTime(order) | Sorting::ByCommitTimeCutoff { order, .. } => {
                    let state = &mut self.state;
                    for commit_id in state.next.drain(..) {
                        add_to_queue(
                            commit_id,
                            order,
                            sorting.cutoff_time(),
                            &mut state.queue,
                            &self.objects,
                            &mut state.buf,
                        )?;
                    }
                }
            }
            Ok(self)
        }

        /// Change our commit parent handling mode to the given one.
        pub fn parents(mut self, mode: Parents) -> Self {
            self.parents = mode;
            if matches!(self.parents, Parents::First) {
                self.queue_to_vecdeque();
            }
            self
        }

        /// Hide the given `tips`, along with all commits reachable by them so that they will not be returned
        /// by the traversal.
        ///
        /// Without usable commit-graph generation numbers, this uses Git's date-based stopping
        /// heuristic. Extreme clock skew can leave hidden ancestors beyond the stopping point.
        pub fn hide(mut self, tips: impl IntoIterator<Item = ObjectId>) -> Result<Self> {
            self.state.hidden_tips = tips.into_iter().collect();
            Ok(self)
        }

        /// Set the commit-graph cache to speed up traversal. The cache can be owned, borrowed, or shared.
        ///
        /// Commit lookups fall back to the object database if the cache is unavailable or corrupt.
        pub fn with_commit_graph<Cache>(self, cache: Option<Cache>) -> Simple<Find, Predicate, Cache>
        where
            Cache: Borrow<gix_commitgraph::Graph>,
        {
            Simple {
                objects: self.objects,
                cache,
                predicate: self.predicate,
                state: self.state,
                parents: self.parents,
                sorting: self.sorting,
            }
        }

        fn queue_to_vecdeque(&mut self) {
            let state = &mut self.state;
            state.next.extend(
                std::mem::replace(&mut state.queue, gix_revwalk::PriorityQueue::new())
                    .into_iter_unordered()
                    .map(|(_time, id)| id),
            );
        }

        fn visible_inputs_sorted(&self) -> Vec<ObjectId> {
            let mut out: Vec<_> = self
                .state
                .next
                .iter()
                .copied()
                .chain(self.state.queue.iter_unordered().copied())
                .collect();
            out.sort();
            out.dedup();
            out
        }

        /// Hide `hidden_tips` during a breadth-first walk, reusing `metadata` while painting the frontier
        /// and walking the visible commits. Both passes use the graph set by [`Self::with_commit_graph()`], if any.
        ///
        /// Start with an empty metadata map and reuse it only with the same object lookup and commit-graph
        /// snapshot. Each walk computes its own frontier, including the date cutoff when generations are
        /// unavailable. Errors while painting are yielded by the iterator.
        pub fn hide_with_cache<'a>(
            mut self,
            hidden_tips: Vec<ObjectId>,
            metadata: &'a mut gix_revwalk::graph::IdMap<gix_revwalk::graph::Commit<()>>,
        ) -> Result<impl Iterator<Item = Result<Info>> + 'a>
        where
            Find: 'a,
            Predicate: FnMut(&oid) -> bool + 'a,
            CommitGraph: 'a,
        {
            if !matches!(self.sorting, Sorting::BreadthFirst) {
                bail!(message("The hidden walk cache requires breadth-first sorting").validation());
            }
            let mut hidden_tips = Some(hidden_tips);
            Ok(std::iter::from_fn(move || {
                if let Some(hidden_tips) = hidden_tips.take() {
                    self.state.hidden.clear();
                    let visible_tips = self.visible_inputs_sorted();
                    if !hidden_tips.is_empty() && !visible_tips.is_empty() {
                        let paint_graph = self.cache.as_ref().map(Borrow::borrow);
                        let graph = SharedPaint {
                            metadata,
                            lookup: gix_revwalk::Graph::new(&self.objects, paint_graph),
                            flags: Default::default(),
                        };
                        let uncomputed = paint_graph.is_some_and(|cache| !cache.generation_numbers_available());
                        match paint_frontier(&visible_tips, &hidden_tips, graph, uncomputed, Parents::All, |_| true) {
                            Ok(hidden) => {
                                self.state.hidden = hidden;
                                self.exclude_hidden_tips();
                            }
                            Err(err) => {
                                self.state.next.clear();
                                return Some(Err(err));
                            }
                        }
                    }
                }
                self.next_by_topology_with_metadata(|id| metadata.get(id))
            }))
        }

        fn compute_hidden_frontier(&mut self, hidden_tips: Vec<ObjectId>) -> Result {
            self.state.hidden.clear();
            if hidden_tips.is_empty() {
                return Ok(());
            }
            let visible_tips = self.visible_inputs_sorted();
            if visible_tips.is_empty() {
                return Ok(());
            }
            self.state.hidden = compute_hidden_frontier(
                &visible_tips,
                &hidden_tips,
                &self.objects,
                self.cache.as_ref().map(Borrow::borrow),
                Parents::All,
                |_| true,
            )?;
            self.exclude_hidden_tips();
            Ok(())
        }

        fn exclude_hidden_tips(&mut self) {
            self.state.next.retain(|id| !self.state.hidden.contains_key(id));
            self.state.queue = std::mem::replace(&mut self.state.queue, gix_revwalk::PriorityQueue::new())
                .into_iter_unordered()
                .filter(|(_, id)| !self.state.hidden.contains_key(id))
                .collect();
        }
    }

    fn add_to_queue(
        commit_id: ObjectId,
        order: CommitTimeOrder,
        cutoff_time: Option<SecondsSinceUnixEpoch>,
        queue: &mut CommitDateQueue,
        objects: &impl gix_object::Find,
        buf: &mut Vec<u8>,
    ) -> Result {
        let commit_iter = objects.find_commit_iter(&commit_id, buf)?;
        let time = commit_iter
            .committer()
            .or_raise(|| corruption("A commit could not be decoded during traversal"))?
            .seconds();
        let key = to_queue_key(time, order);
        match (cutoff_time, order) {
            (Some(cutoff_time), _) if time >= cutoff_time => queue.insert(key, commit_id),
            (Some(_), _) => {}
            (None, _) => queue.insert(key, commit_id),
        }
        Ok(())
    }

    /// Lifecycle methods
    impl<Find> Simple<Find, fn(&oid) -> bool>
    where
        Find: gix_object::Find,
    {
        /// Create a new instance.
        ///
        /// * `find` - a way to lookup new object data during traversal by their `ObjectId`, writing their data into buffer and returning
        ///   an iterator over commit tokens if the object is present and is a commit. Caching should be implemented within this function
        ///   as needed.
        /// * `tips`
        ///   * the starting points of the iteration, usually commits
        ///   * each commit they lead to will only be returned once, including the tip that started it
        pub fn new(tips: impl IntoIterator<Item = impl Into<ObjectId>>, find: Find) -> Self {
            Self::filtered(tips, find, |_| true)
        }
    }

    impl<Find, Predicate> Simple<Find, Predicate>
    where
        Find: gix_object::Find,
        Predicate: FnMut(&oid) -> bool,
    {
        /// Create a new instance with commit filtering enabled.
        ///
        /// * `find` - a way to lookup new object data during traversal by their `ObjectId`, writing their data into buffer and returning
        ///   an iterator over commit tokens if the object is present and is a commit. Caching should be implemented within this function
        ///   as needed.
        /// * `tips`
        ///   * the starting points of the iteration, usually commits
        ///   * each commit they lead to will only be returned once, including the tip that started it
        /// * `predicate` - indicate whether a given commit should be included in the result as well
        ///   as whether its parent commits should be traversed.
        pub fn filtered(
            tips: impl IntoIterator<Item = impl Into<ObjectId>>,
            find: Find,
            mut predicate: Predicate,
        ) -> Self {
            let tips = tips.into_iter();
            let mut state = State::default();
            {
                state.clear();
                state.next.reserve(tips.size_hint().0);
                for tip in tips.map(Into::into) {
                    if state.seen.insert(tip) && predicate(&tip) {
                        state.next.push_back(tip);
                    }
                }
            }
            Self {
                objects: find,
                cache: None,
                predicate,
                state,
                parents: Default::default(),
                sorting: Default::default(),
            }
        }
    }

    /// Access
    impl<Find, Predicate, CommitGraph> Simple<Find, Predicate, CommitGraph> {
        /// Return an iterator for accessing data of the current commit, parsed lazily.
        pub fn commit_iter(&self) -> CommitRefIter<'_> {
            CommitRefIter::from_bytes(self.commit_data(), self.state.object_hash)
        }

        /// Return the current commits' raw data, which can be parsed using [`gix_object::CommitRef::from_bytes()`].
        pub fn commit_data(&self) -> &[u8] {
            &self.state.buf
        }
    }

    impl<Find, Predicate, CommitGraph> Iterator for Simple<Find, Predicate, CommitGraph>
    where
        Find: gix_object::Find,
        Predicate: FnMut(&oid) -> bool,
        CommitGraph: Borrow<gix_commitgraph::Graph>,
    {
        type Item = Result<Info>;

        fn next(&mut self) -> Option<Self::Item> {
            if !self.state.hidden_tips.is_empty() {
                let hidden_tips = std::mem::take(&mut self.state.hidden_tips);
                if let Err(err) = self.compute_hidden_frontier(hidden_tips) {
                    self.state.queue.clear();
                    self.state.next.clear();
                    return Some(Err(err));
                }
            }
            if matches!(self.parents, Parents::First) {
                self.next_by_topology()
            } else {
                match self.sorting {
                    Sorting::BreadthFirst => self.next_by_topology(),
                    Sorting::ByCommitTime(order) => self.next_by_commit_date(order, None),
                    Sorting::ByCommitTimeCutoff { seconds, order } => self.next_by_commit_date(order, seconds.into()),
                }
            }
        }
    }

    /// Utilities
    impl<Find, Predicate, CommitGraph> Simple<Find, Predicate, CommitGraph>
    where
        Find: gix_object::Find,
        Predicate: FnMut(&oid) -> bool,
        CommitGraph: Borrow<gix_commitgraph::Graph>,
    {
        fn next_by_commit_date(
            &mut self,
            order: CommitTimeOrder,
            cutoff: Option<SecondsSinceUnixEpoch>,
        ) -> Option<Result<Info>> {
            let state = &mut self.state;
            let next = &mut state.queue;

            loop {
                let (commit_time, oid) = match next.pop()? {
                    (Ok(t) | Err(Reverse(t)), o) => (t, o),
                };
                state.object_hash = oid.kind();
                if state.hidden.contains_key(&oid) {
                    continue;
                }
                let mut parents: ParentIds = Default::default();
                let generation;

                let commit_graph = self.cache.as_ref().map(Borrow::borrow);
                match super::super::find(commit_graph, &self.objects, &oid, &mut state.buf) {
                    Ok(Either::CachedCommit(commit)) => {
                        generation = Some(commit.generation());
                        if !collect_parents(&mut state.parent_ids, commit_graph, commit.iter_parents()) {
                            // drop corrupt caches and try again with ODB
                            self.cache = None;
                            next.insert(to_queue_key(commit_time, order), oid);
                            continue;
                        }
                        for (id, parent_commit_time) in state.parent_ids.drain(..) {
                            parents.push(id);
                            insert_into_seen_and_queue(
                                &mut state.seen,
                                &state.hidden,
                                id,
                                &mut self.predicate,
                                next,
                                order,
                                cutoff,
                                || parent_commit_time,
                            );
                        }
                    }
                    Ok(Either::CommitRefIter(commit_iter)) => {
                        generation = None;
                        for token in commit_iter {
                            match token {
                                Ok(gix_object::commit::ref_iter::Token::Tree { .. }) => continue,
                                Ok(gix_object::commit::ref_iter::Token::Parent { id }) => {
                                    parents.push(id);
                                    insert_into_seen_and_queue(
                                        &mut state.seen,
                                        &state.hidden,
                                        id,
                                        &mut self.predicate,
                                        next,
                                        order,
                                        cutoff,
                                        || {
                                            let parent =
                                                self.objects.find_commit_iter(id.as_ref(), &mut state.parents_buf).ok();
                                            parent
                                                .and_then(|parent| {
                                                    parent.committer().ok().map(|committer| committer.seconds())
                                                })
                                                .unwrap_or_default()
                                        },
                                    );
                                }
                                Ok(_unused_token) => break,
                                Err(err) => {
                                    return Some(Err(
                                        err.and_raise(corruption("A commit could not be decoded during traversal"))
                                    ));
                                }
                            }
                        }
                    }
                    Err(err) => return Some(Err(err)),
                }

                return Some(Ok(Info {
                    id: oid,
                    parent_ids: parents,
                    generation,
                    commit_time: Some(commit_time),
                }));
            }
        }

        fn next_by_topology(&mut self) -> Option<Result<Info>> {
            self.next_by_topology_with_metadata(|_| None)
        }

        pub(super) fn next_by_topology_with_metadata<'a>(
            &mut self,
            mut metadata: impl FnMut(&oid) -> Option<&'a gix_revwalk::graph::Commit<()>>,
        ) -> Option<Result<Info>> {
            let state = &mut self.state;
            let next = &mut state.next;

            loop {
                let oid = next.pop_front()?;
                state.object_hash = oid.kind();
                if state.hidden.contains_key(&oid) {
                    continue;
                }
                let mut parents: ParentIds = Default::default();
                let generation;

                let commit_graph = self.cache.as_ref().map(Borrow::borrow);
                if let Some(commit) =
                    metadata(&oid).filter(|commit| commit.generation.is_none() || commit_graph.is_some())
                {
                    for &pid in &commit.parents {
                        parents.push(pid);
                        insert_into_seen_and_next(&mut state.seen, &state.hidden, pid, &mut self.predicate, next);
                        if matches!(self.parents, Parents::First) {
                            break;
                        }
                    }
                    return Some(Ok(Info {
                        id: oid,
                        parent_ids: parents,
                        generation: commit.generation,
                        commit_time: None,
                    }));
                }

                match super::super::find(commit_graph, &self.objects, &oid, &mut state.buf) {
                    Ok(Either::CachedCommit(commit)) => {
                        generation = Some(commit.generation());
                        if !collect_parents(&mut state.parent_ids, commit_graph, commit.iter_parents()) {
                            // drop corrupt caches and try again with ODB
                            self.cache = None;
                            next.push_front(oid);
                            continue;
                        }

                        for (pid, _commit_time) in state.parent_ids.drain(..) {
                            parents.push(pid);
                            insert_into_seen_and_next(&mut state.seen, &state.hidden, pid, &mut self.predicate, next);
                            if matches!(self.parents, Parents::First) {
                                break;
                            }
                        }
                    }
                    Ok(Either::CommitRefIter(commit_iter)) => {
                        generation = None;
                        for token in commit_iter {
                            match token {
                                Ok(gix_object::commit::ref_iter::Token::Tree { .. }) => continue,
                                Ok(gix_object::commit::ref_iter::Token::Parent { id: pid }) => {
                                    parents.push(pid);
                                    insert_into_seen_and_next(
                                        &mut state.seen,
                                        &state.hidden,
                                        pid,
                                        &mut self.predicate,
                                        next,
                                    );
                                    if matches!(self.parents, Parents::First) {
                                        break;
                                    }
                                }
                                Ok(_a_token_past_the_parents) => break,
                                Err(err) => {
                                    return Some(Err(
                                        err.and_raise(corruption("A commit could not be decoded during traversal"))
                                    ));
                                }
                            }
                        }
                    }
                    Err(err) => return Some(Err(err)),
                }

                return Some(Ok(Info {
                    id: oid,
                    parent_ids: parents,
                    generation,
                    commit_time: None,
                }));
            }
        }
    }

    fn insert_into_seen_and_next(
        seen: &mut gix_hashtable::HashSet<ObjectId>,
        hidden: &gix_revwalk::graph::IdMap<()>,
        parent_id: ObjectId,
        predicate: &mut impl FnMut(&oid) -> bool,
        next: &mut VecDeque<ObjectId>,
    ) {
        if hidden.contains_key(&parent_id) {
            return;
        }
        if seen.insert(parent_id) && predicate(&parent_id) {
            next.push_back(parent_id);
        }
    }

    #[expect(clippy::too_many_arguments)]
    fn insert_into_seen_and_queue(
        seen: &mut gix_hashtable::HashSet<ObjectId>,
        hidden: &gix_revwalk::graph::IdMap<()>,
        parent_id: ObjectId,
        predicate: &mut impl FnMut(&oid) -> bool,
        queue: &mut CommitDateQueue,
        order: CommitTimeOrder,
        cutoff: Option<SecondsSinceUnixEpoch>,
        get_parent_commit_time: impl FnOnce() -> gix_date::SecondsSinceUnixEpoch,
    ) {
        if hidden.contains_key(&parent_id) {
            return;
        }
        if seen.insert(parent_id) && predicate(&parent_id) {
            let parent_commit_time = get_parent_commit_time();
            let key = to_queue_key(parent_commit_time, order);
            match cutoff {
                Some(cutoff_older_than) if parent_commit_time < cutoff_older_than => {}
                Some(_) | None => queue.insert(key, parent_id),
            }
        }
    }
}

fn collect_parents(
    dest: &mut SmallVec<[(gix_hash::ObjectId, gix_date::SecondsSinceUnixEpoch); 2]>,
    cache: Option<&gix_commitgraph::Graph>,
    parents: gix_commitgraph::file::commit::Parents<'_>,
) -> bool {
    dest.clear();
    let cache = cache.as_ref().expect("parents iter is available, backed by `cache`");
    for parent_id in parents {
        match parent_id {
            Ok(pos) => dest.push({
                let parent = cache.commit_at(pos);
                (
                    parent.id().to_owned(),
                    parent.committer_timestamp() as gix_date::SecondsSinceUnixEpoch,
                )
            }),
            Err(_err) => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::compute_hidden_frontier;
    use crate::commit::Parents;

    #[test]
    fn hidden_paint_stops_at_the_fork_point_above_unrelated_history() -> gix_testtools::TestResult {
        let repo_dir =
            gix_testtools::scripted_fixture_read_only("make_repo_for_hidden_bug.sh")?.join("unrelated_history");
        let names = std::fs::read_to_string(repo_dir.join("commit-names.baseline"))?;
        let commit_id = |name: &str| {
            names
                .lines()
                .find_map(|line| line.strip_suffix(name)?.strip_suffix(' '))
                .map(|hex| gix_hash::ObjectId::from_hex(hex.as_bytes()).expect("the baseline has valid hashes"))
                .expect("the fixture has a commit with this message")
        };
        let objects_dir = repo_dir.join(".git").join("objects");
        let odb = gix_odb::at(&objects_dir, gix_testtools::object_hash())?;
        let graph = gix_commitgraph::at(objects_dir.join("info"), gix_testtools::object_hash(), None)?;

        let paint = |visible_commit_ids: &mut [gix_hash::ObjectId], hidden_commit_ids: &[gix_hash::ObjectId]| {
            visible_commit_ids.sort();
            compute_hidden_frontier(
                visible_commit_ids,
                hidden_commit_ids,
                &odb,
                Some(&graph),
                Parents::All,
                |_| true,
            )
        };

        let frontier = paint(&mut [commit_id("topic-1")], &[commit_id("main-39")])?;
        assert_eq!(
            frontier.into_keys().collect::<Vec<_>>(),
            [commit_id("main-29")],
            "painting stops once the hidden paint reaches the fork point, \
             so neither older mainline commits nor the unrelated history are painted"
        );

        let frontier = paint(
            &mut [commit_id("topic-1"), commit_id("unrelated-0")],
            &[commit_id("main-39"), commit_id("unrelated-0")],
        )?;
        assert!(
            frontier.contains_key(&commit_id("main-29")) && !frontier.contains_key(&commit_id("main-28")),
            "a visible tip that is also hidden doesn't keep painting past the fork point"
        );
        Ok(())
    }
}
