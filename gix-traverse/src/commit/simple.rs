use std::{
    cmp::{Ordering, Reverse},
    collections::VecDeque,
};

use gix_date::SecondsSinceUnixEpoch;
use gix_hash::ObjectId;
use smallvec::SmallVec;

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

/// The error is part of the item returned by the [Ancestors](super::Simple) iterator.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    Find(#[from] gix_object::find::existing_iter::Error),
    #[error(transparent)]
    ObjectDecode(#[from] gix_object::decode::Error),
    #[error(transparent)]
    HiddenGraph(#[from] gix_revwalk::graph::get_or_insert_default::Error),
}

use Result as Either;

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

impl From<&gix_revwalk::graph::Commit<PaintFlags>> for GenThenTime {
    fn from(commit: &gix_revwalk::graph::Commit<PaintFlags>) -> Self {
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

/// Compute the boundary at which the visible walk must stop because commits become reachable from
/// both the visible tips and the hidden tips.
///
/// The algorithm performs a merge-base-style paint in a temporary `gix_revwalk::Graph`:
/// visible tips are marked with `VISIBLE`, hidden tips with `HIDDEN`, and these flags are
/// propagated to parents in generation/time order. Once a commit carries both flags, it is part
/// of the overlap between the two histories and is marked `STALE` so older ancestors no longer
/// need to keep re-propagating the same combined state.
///
/// The returned set is not all commits reachable from hidden tips. It is only the overlap frontier
/// where the visible traversal must stop. The actual `Simple` walk then skips these commits and
/// refuses to enqueue parents across them, which avoids traversing hidden-only history.
fn compute_hidden_frontier(
    visible_tips: &[ObjectId],
    hidden_tips: &[ObjectId],
    objects: &impl gix_object::Find,
    cache: Option<&gix_commitgraph::Graph>,
) -> Result<gix_revwalk::graph::IdMap<()>, Error> {
    let mut graph = gix_revwalk::Graph::<gix_revwalk::graph::Commit<PaintFlags>>::new(objects, cache);
    let mut queue = gix_revwalk::PriorityQueue::<GenThenTime, ObjectId>::new();

    for &visible in visible_tips {
        graph.get_or_insert_full_commit(visible, |commit| {
            commit.data |= PaintFlags::VISIBLE;
            queue.insert(GenThenTime::from(&*commit), visible);
        })?;
    }
    for &hidden in hidden_tips {
        graph.get_or_insert_full_commit(hidden, |commit| {
            commit.data |= PaintFlags::HIDDEN;
            queue.insert(GenThenTime::from(&*commit), hidden);
        })?;
    }

    while queue.iter_unordered().any(|id| {
        graph
            .get(id)
            .is_some_and(|commit| !commit.data.contains(PaintFlags::STALE))
    }) {
        let (_info, commit_id) = match queue.pop() {
            Some(v) => v,
            None => break,
        };
        let commit = graph.get_mut(&commit_id).expect("queued commits are in the graph");
        let mut flags = commit.data;
        if flags == (PaintFlags::VISIBLE | PaintFlags::HIDDEN) {
            flags |= PaintFlags::STALE;
        }

        for parent_id in commit.parents.clone() {
            graph.get_or_insert_full_commit(parent_id, |parent| {
                if (parent.data & flags) != flags {
                    parent.data |= flags;
                    queue.insert(GenThenTime::from(&*parent), parent_id);
                }
            })?;
        }
    }

    Ok(graph
        .detach()
        .into_iter()
        .filter_map(|(id, commit)| {
            commit
                .data
                .contains(PaintFlags::VISIBLE | PaintFlags::HIDDEN)
                .then_some((id, ()))
        })
        .collect())
}

///
mod init {
    use super::{
        CommitDateQueue, CommitTimeOrder, Error, Sorting, State, collect_parents, compute_hidden_frontier, to_queue_key,
    };
    use crate::commit::{Either, Info, ParentIds, Parents, Simple};
    use gix_date::SecondsSinceUnixEpoch;
    use gix_hash::{ObjectId, oid};
    use gix_object::{CommitRefIter, FindExt};
    use std::{cmp::Reverse, collections::VecDeque};

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
    impl<Find, Predicate> Simple<Find, Predicate>
    where
        Find: gix_object::Find,
    {
        /// Set the `sorting` method.
        pub fn sorting(mut self, sorting: Sorting) -> Result<Self, Error> {
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
        pub fn hide(mut self, tips: impl IntoIterator<Item = ObjectId>) -> Result<Self, Error> {
            self.state.hidden_tips = tips.into_iter().collect();
            Ok(self)
        }

        /// Set the commitgraph as `cache` to greatly accelerate any traversal.
        ///
        /// The cache will be used if possible, but we will fall back without error to using the object
        /// database for commit lookup. If the cache is corrupt, we will fall back to the object database as well.
        pub fn commit_graph(mut self, cache: Option<gix_commitgraph::Graph>) -> Self {
            self.cache = cache;
            self
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

        fn compute_hidden_frontier(&mut self, hidden_tips: Vec<ObjectId>) -> Result<(), Error> {
            self.state.hidden.clear();
            if hidden_tips.is_empty() {
                return Ok(());
            }
            let visible_tips = self.visible_inputs_sorted();
            if visible_tips.is_empty() {
                return Ok(());
            }
            self.state.hidden =
                compute_hidden_frontier(&visible_tips, &hidden_tips, &self.objects, self.cache.as_ref())?;
            self.state.next.retain(|id| !self.state.hidden.contains_key(id));
            self.state.queue = std::mem::replace(&mut self.state.queue, gix_revwalk::PriorityQueue::new())
                .into_iter_unordered()
                .filter(|(_, id)| !self.state.hidden.contains_key(id))
                .collect();
            Ok(())
        }
    }

    fn add_to_queue(
        commit_id: ObjectId,
        order: CommitTimeOrder,
        cutoff_time: Option<SecondsSinceUnixEpoch>,
        queue: &mut CommitDateQueue,
        objects: &impl gix_object::Find,
        buf: &mut Vec<u8>,
    ) -> Result<(), Error> {
        let commit_iter = objects.find_commit_iter(&commit_id, buf)?;
        let time = commit_iter.committer()?.seconds();
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
    impl<Find, Predicate> Simple<Find, Predicate> {
        /// Return an iterator for accessing data of the current commit, parsed lazily.
        pub fn commit_iter(&self) -> CommitRefIter<'_> {
            CommitRefIter::from_bytes(self.commit_data(), self.state.object_hash)
        }

        /// Return the current commits' raw data, which can be parsed using [`gix_object::CommitRef::from_bytes()`].
        pub fn commit_data(&self) -> &[u8] {
            &self.state.buf
        }
    }

    impl<Find, Predicate> Iterator for Simple<Find, Predicate>
    where
        Find: gix_object::Find,
        Predicate: FnMut(&oid) -> bool,
    {
        type Item = Result<Info, Error>;

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
    impl<Find, Predicate> Simple<Find, Predicate>
    where
        Find: gix_object::Find,
        Predicate: FnMut(&oid) -> bool,
    {
        fn next_by_commit_date(
            &mut self,
            order: CommitTimeOrder,
            cutoff: Option<SecondsSinceUnixEpoch>,
        ) -> Option<Result<Info, Error>> {
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

                match super::super::find(self.cache.as_ref(), &self.objects, &oid, &mut state.buf) {
                    Ok(Either::CachedCommit(commit)) => {
                        if !collect_parents(&mut state.parent_ids, self.cache.as_ref(), commit.iter_parents()) {
                            // drop corrupt caches and try again with ODB
                            self.cache = None;
                            return self.next_by_commit_date(order, cutoff);
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
                                Err(err) => return Some(Err(err.into())),
                            }
                        }
                    }
                    Err(err) => return Some(Err(err.into())),
                }

                return Some(Ok(Info {
                    id: oid,
                    parent_ids: parents,
                    commit_time: Some(commit_time),
                }));
            }
        }

        fn next_by_topology(&mut self) -> Option<Result<Info, Error>> {
            let state = &mut self.state;
            let next = &mut state.next;

            loop {
                let oid = next.pop_front()?;
                state.object_hash = oid.kind();
                if state.hidden.contains_key(&oid) {
                    continue;
                }
                let mut parents: ParentIds = Default::default();

                match super::super::find(self.cache.as_ref(), &self.objects, &oid, &mut state.buf) {
                    Ok(Either::CachedCommit(commit)) => {
                        if !collect_parents(&mut state.parent_ids, self.cache.as_ref(), commit.iter_parents()) {
                            // drop corrupt caches and try again with ODB
                            self.cache = None;
                            return self.next_by_topology();
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
                                Err(err) => return Some(Err(err.into())),
                            }
                        }
                    }
                    Err(err) => return Some(Err(err.into())),
                }

                return Some(Ok(Info {
                    id: oid,
                    parent_ids: parents,
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

    #[allow(clippy::too_many_arguments)]
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
