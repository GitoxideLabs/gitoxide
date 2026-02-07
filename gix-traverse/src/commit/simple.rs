use std::{cmp::Reverse, collections::VecDeque};

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
}

use Result as Either;

type QueueKey<T> = Either<T, Reverse<T>>;
type CommitDateQueue = gix_revwalk::PriorityQueue<QueueKey<SecondsSinceUnixEpoch>, (ObjectId, CommitState)>;
type Candidates = VecDeque<crate::commit::Info>;

/// The state used and potentially shared by multiple graph traversals.
#[derive(Clone)]
pub(super) struct State {
    next: VecDeque<(ObjectId, CommitState)>,
    queue: CommitDateQueue,
    buf: Vec<u8>,
    seen: gix_revwalk::graph::IdMap<CommitState>,
    parents_buf: Vec<u8>,
    parent_ids: SmallVec<[(ObjectId, SecondsSinceUnixEpoch); 2]>,
    /// The list (FIFO) of thus far interesting commits.
    ///
    /// As they may turn hidden later, we have to keep them until the conditions are met to return them.
    /// If `None`, there is nothing to do with hidden commits.
    // TODO(perf): review this as we don't really need candidates anymore, given our current way of doing things.
    //             However, maybe they can see use when getting an incremental traversal done.
    candidates: Option<Candidates>,
}

#[derive(Debug, Clone, Copy)]
enum CommitState {
    /// The commit may be returned, it hasn't been hidden yet.
    Interesting,
    /// The commit should not be returned.
    Hidden,
}

impl CommitState {
    pub fn is_hidden(&self) -> bool {
        matches!(self, CommitState::Hidden)
    }
    pub fn is_interesting(&self) -> bool {
        matches!(self, CommitState::Interesting)
    }
}

///
mod init {
    use std::cmp::Reverse;

    use super::{
        super::{simple::Sorting, Either, Info, ParentIds, Parents, Simple},
        collect_parents, Candidates, CommitDateQueue, CommitState, CommitTimeOrder, Error, State,
    };
    use gix_date::SecondsSinceUnixEpoch;
    use gix_hash::{oid, ObjectId};
    use gix_hashtable::hash_map::Entry;
    use gix_object::{CommitRefIter, FindExt};
    use std::collections::VecDeque;
    use Err as Oldest;
    use Ok as Newest;

    impl Default for State {
        fn default() -> Self {
            State {
                next: Default::default(),
                queue: gix_revwalk::PriorityQueue::new(),
                buf: vec![],
                seen: Default::default(),
                parents_buf: vec![],
                parent_ids: Default::default(),
                candidates: None,
            }
        }
    }

    impl State {
        fn clear(&mut self) {
            let Self {
                next,
                queue,
                buf,
                seen,
                parents_buf: _,
                parent_ids: _,
                candidates,
            } = self;
            next.clear();
            queue.clear();
            buf.clear();
            seen.clear();
            *candidates = None;
        }
    }

    fn to_queue_key(i: i64, order: CommitTimeOrder) -> super::QueueKey<i64> {
        match order {
            CommitTimeOrder::NewestFirst => Newest(i),
            CommitTimeOrder::OldestFirst => Oldest(Reverse(i)),
        }
    }

    /// Builder
    impl<Find, Predicate> Simple<Find, Predicate>
    where
        Find: gix_object::Find,
    {
        /// Set the `sorting` method.
        pub fn sorting(mut self, sorting: Sorting) -> Result<Self, Error> {
            self.sorting = sorting;
            match self.sorting {
                Sorting::BreadthFirst => {
                    self.queue_to_vecdeque();
                }
                Sorting::ByCommitTime(order) | Sorting::ByCommitTimeCutoff { order, .. } => {
                    let state = &mut self.state;
                    for (commit_id, commit_state) in state.next.drain(..) {
                        add_to_queue(
                            commit_id,
                            commit_state,
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
        /// This function fully traverses all hidden tips and their ancestors, marking them as hidden
        /// before iteration begins. This approach ensures correct behavior regardless
        /// of graph topology or traversal order, matching git's `rev-list --not` behavior,
        /// at great cost to performance, unfortunately.
        ///
        /// Note that hidden objects are expected to exist.
        // TODO(perf): make this hiding iterative to avoid traversing the entire graph, always.
        pub fn hide(mut self, tips: impl IntoIterator<Item = ObjectId>) -> Result<Self, Error> {
            // Collect hidden tips first
            let hidden_tips: Vec<ObjectId> = tips.into_iter().collect();
            if hidden_tips.is_empty() {
                return Ok(self);
            }

            // Fully traverse all hidden tips and mark all reachable commits as Hidden.
            // This is "graph painting" - we paint all hidden commits upfront rather than
            // interleaving hidden and interesting traversals, which ensures correct behavior
            // regardless of graph topology or traversal order.
            let mut queue: VecDeque<ObjectId> = VecDeque::new();

            for id_to_ignore in hidden_tips {
                if self.state.seen.insert(id_to_ignore, CommitState::Hidden).is_none() {
                    queue.push_back(id_to_ignore);
                }
            }

            // Process all hidden commits and their ancestors
            while let Some(id) = queue.pop_front() {
                match super::super::find(self.cache.as_ref(), &self.objects, &id, &mut self.state.buf) {
                    Ok(Either::CachedCommit(commit)) => {
                        if !collect_parents(&mut self.state.parent_ids, self.cache.as_ref(), commit.iter_parents()) {
                            // drop corrupt caches and retry
                            self.cache = None;
                            // Re-add to queue to retry without cache
                            if self.state.seen.get(&id).is_some_and(CommitState::is_hidden) {
                                queue.push_back(id);
                            }
                            continue;
                        }
                        for (parent_id, _commit_time) in self.state.parent_ids.drain(..) {
                            if self.state.seen.insert(parent_id, CommitState::Hidden).is_none() {
                                queue.push_back(parent_id);
                            }
                        }
                    }
                    Ok(Either::CommitRefIter(commit_iter)) => {
                        for token in commit_iter {
                            match token {
                                Ok(gix_object::commit::ref_iter::Token::Tree { .. }) => continue,
                                Ok(gix_object::commit::ref_iter::Token::Parent { id: parent_id }) => {
                                    if self.state.seen.insert(parent_id, CommitState::Hidden).is_none() {
                                        queue.push_back(parent_id);
                                    }
                                }
                                Ok(_unused_token) => break,
                                Err(err) => return Err(err.into()),
                            }
                        }
                    }
                    Err(err) => return Err(err.into()),
                }
            }

            // Now that all hidden commits are painted, we no longer need special handling
            // during the main traversal. We can remove hidden commits from the main queues
            // and simply skip them during iteration.
            //
            // Note: We don't need the candidates buffer anymore since hidden commits are
            // pre-painted. But we keep it for compatibility with existing behavior and
            // in case interesting commits were already queued before hide() was called.
            self.state.candidates = None;

            // Remove any hidden commits from the interesting queues
            self.state
                .next
                .retain(|(id, _)| !self.state.seen.get(id).is_some_and(CommitState::is_hidden));

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
    }

    fn add_to_queue(
        commit_id: ObjectId,
        commit_state: CommitState,
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
            (Some(cutoff_time), _) if time >= cutoff_time => {
                queue.insert(key, (commit_id, commit_state));
            }
            (Some(_), _) => {}
            (None, _) => {
                queue.insert(key, (commit_id, commit_state));
            }
        }
        Ok(())
    }

    /// Lifecycle
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

    /// Lifecycle
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
                    let commit_state = CommitState::Interesting;
                    let seen = state.seen.insert(tip, commit_state);
                    // We know there can only be duplicate interesting ones.
                    if seen.is_none() && predicate(&tip) {
                        state.next.push_back((tip, commit_state));
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
            CommitRefIter::from_bytes(self.commit_data())
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
            if matches!(self.parents, Parents::First) {
                self.next_by_topology()
            } else {
                match self.sorting {
                    Sorting::BreadthFirst => self.next_by_topology(),
                    Sorting::ByCommitTime(order) => self.next_by_commit_date(order, None),
                    Sorting::ByCommitTimeCutoff { seconds, order } => self.next_by_commit_date(order, seconds.into()),
                }
            }
            .or_else(|| {
                self.state
                    .candidates
                    .as_mut()
                    .and_then(|candidates| candidates.pop_front().map(Ok))
            })
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

            'skip_hidden: loop {
                let (commit_time, (oid, _queued_commit_state)) = match next.pop()? {
                    (Newest(t) | Oldest(Reverse(t)), o) => (t, o),
                };
                let mut parents: ParentIds = Default::default();

                // Always use the state that is actually stored, as we may change the type as we go.
                let commit_state = *state.seen.get(&oid).expect("every commit we traverse has state added");
                if can_deplete_candidates_early(
                    next.iter_unordered().map(|t| t.1),
                    commit_state,
                    state.candidates.as_ref(),
                ) {
                    return None;
                }
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
                                id,
                                &mut state.candidates,
                                commit_state,
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
                                        id,
                                        &mut state.candidates,
                                        commit_state,
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
                match commit_state {
                    CommitState::Interesting => {
                        let info = Info {
                            id: oid,
                            parent_ids: parents,
                            commit_time: Some(commit_time),
                        };
                        match state.candidates.as_mut() {
                            None => return Some(Ok(info)),
                            Some(candidates) => {
                                // assure candidates aren't prematurely returned - hidden commits may catch up with
                                // them later.
                                candidates.push_back(info);
                            }
                        }
                    }
                    CommitState::Hidden => continue 'skip_hidden,
                }
            }
        }
    }

    /// Returns `true` if we have only hidden cursors queued for traversal, assuming that we don't see interesting ones ever again.
    ///
    /// `unqueued_commit_state` is the state of the commit that is currently being processed.
    fn can_deplete_candidates_early(
        mut queued_states: impl Iterator<Item = CommitState>,
        unqueued_commit_state: CommitState,
        candidates: Option<&Candidates>,
    ) -> bool {
        if candidates.is_none() {
            return false;
        }
        if unqueued_commit_state.is_interesting() {
            return false;
        }

        let mut is_empty = true;
        queued_states.all(|state| {
            is_empty = false;
            state.is_hidden()
        }) && !is_empty
    }

    /// Utilities
    impl<Find, Predicate> Simple<Find, Predicate>
    where
        Find: gix_object::Find,
        Predicate: FnMut(&oid) -> bool,
    {
        fn next_by_topology(&mut self) -> Option<Result<Info, Error>> {
            let state = &mut self.state;
            let next = &mut state.next;
            'skip_hidden: loop {
                let (oid, _queued_commit_state) = next.pop_front()?;
                let mut parents: ParentIds = Default::default();

                // Always use the state that is actually stored, as we may change the type as we go.
                let commit_state = *state.seen.get(&oid).expect("every commit we traverse has state added");
                if can_deplete_candidates_early(next.iter().map(|t| t.1), commit_state, state.candidates.as_ref()) {
                    return None;
                }
                match super::super::find(self.cache.as_ref(), &self.objects, &oid, &mut state.buf) {
                    Ok(Either::CachedCommit(commit)) => {
                        if !collect_parents(&mut state.parent_ids, self.cache.as_ref(), commit.iter_parents()) {
                            // drop corrupt caches and try again with ODB
                            self.cache = None;
                            return self.next_by_topology();
                        }

                        for (pid, _commit_time) in state.parent_ids.drain(..) {
                            parents.push(pid);
                            insert_into_seen_and_next(
                                &mut state.seen,
                                pid,
                                &mut state.candidates,
                                commit_state,
                                &mut self.predicate,
                                next,
                            );
                            if commit_state.is_interesting() && matches!(self.parents, Parents::First) {
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
                                        pid,
                                        &mut state.candidates,
                                        commit_state,
                                        &mut self.predicate,
                                        next,
                                    );
                                    if commit_state.is_interesting() && matches!(self.parents, Parents::First) {
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
                match commit_state {
                    CommitState::Interesting => {
                        let info = Info {
                            id: oid,
                            parent_ids: parents,
                            commit_time: None,
                        };
                        match state.candidates.as_mut() {
                            None => return Some(Ok(info)),
                            Some(candidates) => {
                                // assure candidates aren't prematurely returned - hidden commits may catch up with
                                // them later.
                                candidates.push_back(info);
                            }
                        }
                    }
                    CommitState::Hidden => continue 'skip_hidden,
                }
            }
        }
    }

    #[inline]
    fn remove_candidate(candidates: Option<&mut Candidates>, remove: ObjectId) -> Option<()> {
        let candidates = candidates?;
        let pos = candidates
            .iter_mut()
            .enumerate()
            .find_map(|(idx, info)| (info.id == remove).then_some(idx))?;
        candidates.remove(pos);
        None
    }

    fn insert_into_seen_and_next(
        seen: &mut gix_revwalk::graph::IdMap<CommitState>,
        parent_id: ObjectId,
        candidates: &mut Option<Candidates>,
        commit_state: CommitState,
        predicate: &mut impl FnMut(&oid) -> bool,
        next: &mut VecDeque<(ObjectId, CommitState)>,
    ) {
        let enqueue = match seen.entry(parent_id) {
            Entry::Occupied(mut e) => {
                let enqueue = handle_seen(commit_state, *e.get(), parent_id, candidates);
                if commit_state.is_hidden() {
                    e.insert(commit_state);
                }
                enqueue
            }
            Entry::Vacant(e) => {
                e.insert(commit_state);
                match commit_state {
                    CommitState::Interesting => predicate(&parent_id),
                    CommitState::Hidden => true,
                }
            }
        };
        if enqueue {
            next.push_back((parent_id, commit_state));
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_into_seen_and_queue(
        seen: &mut gix_revwalk::graph::IdMap<CommitState>,
        parent_id: ObjectId,
        candidates: &mut Option<Candidates>,
        commit_state: CommitState,
        predicate: &mut impl FnMut(&oid) -> bool,
        queue: &mut CommitDateQueue,
        order: CommitTimeOrder,
        cutoff: Option<SecondsSinceUnixEpoch>,
        get_parent_commit_time: impl FnOnce() -> gix_date::SecondsSinceUnixEpoch,
    ) {
        let enqueue = match seen.entry(parent_id) {
            Entry::Occupied(mut e) => {
                let enqueue = handle_seen(commit_state, *e.get(), parent_id, candidates);
                if commit_state.is_hidden() {
                    e.insert(commit_state);
                }
                enqueue
            }
            Entry::Vacant(e) => {
                e.insert(commit_state);
                match commit_state {
                    CommitState::Interesting => (predicate)(&parent_id),
                    CommitState::Hidden => true,
                }
            }
        };

        if enqueue {
            let parent_commit_time = get_parent_commit_time();
            let key = to_queue_key(parent_commit_time, order);
            match cutoff {
                Some(cutoff_older_than) if parent_commit_time < cutoff_older_than => {}
                Some(_) | None => queue.insert(key, (parent_id, commit_state)),
            }
        }
    }

    #[inline]
    #[must_use]
    fn handle_seen(
        next_state: CommitState,
        current_state: CommitState,
        id: ObjectId,
        candidates: &mut Option<Candidates>,
    ) -> bool {
        match (current_state, next_state) {
            (CommitState::Hidden, CommitState::Hidden) => false,
            (CommitState::Interesting, CommitState::Interesting) => false,
            (CommitState::Hidden, CommitState::Interesting) => {
                // keep traversing to paint more hidden. After all, the commit_state overrides the current parent state
                true
            }
            (CommitState::Interesting, CommitState::Hidden) => {
                remove_candidate(candidates.as_mut(), id);
                true
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
                    parent.committer_timestamp() as gix_date::SecondsSinceUnixEpoch, // we can't handle errors here and trying seems overkill
                )
            }),
            Err(_err) => return false,
        }
    }
    true
}
