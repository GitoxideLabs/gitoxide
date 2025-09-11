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
    candidates: Option<Candidates>,
}

#[derive(Debug, Clone, Copy)]
enum CommitState {
    /// The commit may be returned, it hasn't been hidden yet.
    Interesting {
        /// Whether this commit is along the first-parent line from some starting tip.
        /// This is used when `Parents::First` mode is combined with hidden tips to ensure
        /// only first-parent commits are returned even though all parents are traversed.
        along_first_parent: bool,
    },
    /// The commit should not be returned.
    Hidden {
        /// Whether this commit is along the first-parent line from some starting tip.
        /// This is used when `Parents::First` mode is combined with hidden tips to ensure
        /// hidden commits can still traverse and hide first-parent commits.
        along_first_parent: bool,
    },
}

impl CommitState {
    pub fn is_hidden(&self) -> bool {
        matches!(self, CommitState::Hidden { .. })
    }
    pub fn is_interesting(&self) -> bool {
        matches!(self, CommitState::Interesting { .. })
    }
    pub fn is_along_first_parent(&self) -> bool {
        match self {
            CommitState::Interesting { along_first_parent } | CommitState::Hidden { along_first_parent } => *along_first_parent,
        }
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
        /// Note that this will force the traversal into a non-intermediate mode and queue return candidates,
        /// to be released when it's clear that they truly are not hidden.
        ///
        /// Note that hidden objects are expected to exist.
        pub fn hide(mut self, tips: impl IntoIterator<Item = ObjectId>) -> Result<Self, Error> {
            self.state.candidates = Some(VecDeque::new());
            let state = &mut self.state;
            for id_to_ignore in tips {
                let previous = state.seen.insert(id_to_ignore, CommitState::Hidden { along_first_parent: true });
                // If there was something, it will pick up whatever commit-state we have set last
                // upon iteration. Also, hidden states always override everything else.
                if previous.is_none() {
                    // Assure we *start* traversing hidden variants of a commit first, give them a head-start.
                    match self.sorting {
                        Sorting::BreadthFirst => {
                            state.next.push_front((id_to_ignore, CommitState::Hidden { along_first_parent: true }));
                        }
                        Sorting::ByCommitTime(order) | Sorting::ByCommitTimeCutoff { order, .. } => {
                            add_to_queue(
                                id_to_ignore,
                                CommitState::Hidden { along_first_parent: true },
                                order,
                                self.sorting.cutoff_time(),
                                &mut state.queue,
                                &self.objects,
                                &mut state.buf,
                            )?;
                        }
                    }
                }
            }
            if !self
                .state
                .seen
                .values()
                .any(|state| matches!(state, CommitState::Hidden { .. }))
            {
                self.state.candidates = None;
            }
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
                    let commit_state = CommitState::Interesting { along_first_parent: true };
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
                        for (idx, (id, parent_commit_time)) in state.parent_ids.drain(..).enumerate() {
                            parents.push(id);
                            let is_first_parent = idx == 0;
                            let parent_state = if commit_state.is_interesting() {
                                CommitState::Interesting {
                                    along_first_parent: commit_state.is_along_first_parent() && is_first_parent,
                                }
                            } else {
                                CommitState::Hidden {
                                    along_first_parent: commit_state.is_along_first_parent() && is_first_parent,
                                }
                            };
                            insert_into_seen_and_queue(
                                &mut state.seen,
                                id,
                                &mut state.candidates,
                                parent_state,
                                &mut self.predicate,
                                next,
                                order,
                                cutoff,
                                || parent_commit_time,
                            );
                        }
                    }
                    Ok(Either::CommitRefIter(commit_iter)) => {
                        let mut parent_index = 0;
                        for token in commit_iter {
                            match token {
                                Ok(gix_object::commit::ref_iter::Token::Tree { .. }) => continue,
                                Ok(gix_object::commit::ref_iter::Token::Parent { id }) => {
                                    parents.push(id);
                                    let is_first_parent = parent_index == 0;
                                    let parent_state = if commit_state.is_interesting() {
                                        CommitState::Interesting {
                                            along_first_parent: commit_state.is_along_first_parent() && is_first_parent,
                                        }
                                    } else {
                                        CommitState::Hidden {
                                            along_first_parent: commit_state.is_along_first_parent() && is_first_parent,
                                        }
                                    };
                                    insert_into_seen_and_queue(
                                        &mut state.seen,
                                        id,
                                        &mut state.candidates,
                                        parent_state,
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
                                    parent_index += 1;
                                }
                                Ok(_unused_token) => break,
                                Err(err) => return Some(Err(err.into())),
                            }
                        }
                    }
                    Err(err) => return Some(Err(err.into())),
                }
                match commit_state {
                    CommitState::Interesting { along_first_parent } => {
                        // If we're in first-parent mode with hidden tips, only return commits along first-parent path
                        let should_return = if matches!(self.parents, Parents::First) && state.candidates.is_some() {
                            along_first_parent
                        } else {
                            true // Normal case: return all interesting commits
                        };
                        
                        if should_return {
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
                    }
                    CommitState::Hidden { .. } => continue 'skip_hidden,
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

                        for (idx, (pid, _commit_time)) in state.parent_ids.drain(..).enumerate() {
                            parents.push(pid);
                            let is_first_parent = idx == 0;
                            let parent_state = if commit_state.is_interesting() {
                                CommitState::Interesting {
                                    along_first_parent: commit_state.is_along_first_parent() && is_first_parent,
                                }
                            } else {
                                CommitState::Hidden {
                                    along_first_parent: commit_state.is_along_first_parent() && is_first_parent,
                                }
                            };
                            insert_into_seen_and_next(
                                &mut state.seen,
                                pid,
                                &mut state.candidates,
                                parent_state,
                                &mut self.predicate,
                                next,
                            );
                            // Only break after first parent if we're in first-parent mode AND we don't have hidden tips
                            if commit_state.is_interesting() && matches!(self.parents, Parents::First) && state.candidates.is_none() {
                                break;
                            }
                        }
                    }
                    Ok(Either::CommitRefIter(commit_iter)) => {
                        let mut parent_index = 0;
                        for token in commit_iter {
                            match token {
                                Ok(gix_object::commit::ref_iter::Token::Tree { .. }) => continue,
                                Ok(gix_object::commit::ref_iter::Token::Parent { id: pid }) => {
                                    parents.push(pid);
                                    let is_first_parent = parent_index == 0;
                                    let parent_state = if commit_state.is_interesting() {
                                        CommitState::Interesting {
                                            along_first_parent: commit_state.is_along_first_parent() && is_first_parent,
                                        }
                                    } else {
                                        CommitState::Hidden {
                                            along_first_parent: commit_state.is_along_first_parent() && is_first_parent,
                                        }
                                    };
                                    insert_into_seen_and_next(
                                        &mut state.seen,
                                        pid,
                                        &mut state.candidates,
                                        parent_state,
                                        &mut self.predicate,
                                        next,
                                    );
                                    parent_index += 1;
                                    // Only break after first parent if we're in first-parent mode AND we don't have hidden tips
                                    if commit_state.is_interesting() && matches!(self.parents, Parents::First) && state.candidates.is_none() {
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
                    CommitState::Interesting { along_first_parent } => {
                        // If we're in first-parent mode with hidden tips, only return commits along first-parent path
                        let should_return = if matches!(self.parents, Parents::First) && state.candidates.is_some() {
                            along_first_parent
                        } else {
                            true // Normal case: return all interesting commits
                        };
                        
                        if should_return {
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
                    }
                    CommitState::Hidden { .. } => continue 'skip_hidden,
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
                    CommitState::Interesting { .. } => predicate(&parent_id),
                    CommitState::Hidden { .. } => true,
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
                    CommitState::Interesting { .. } => (predicate)(&parent_id),
                    CommitState::Hidden { .. } => true,
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
        match (current_state.is_hidden(), next_state.is_hidden()) {
            (true, true) => false,   // Both hidden
            (false, false) => false, // Both interesting
            (true, false) => {
                // Current is hidden, next is interesting
                // keep traversing to paint more hidden. After all, the commit_state overrides the current parent state
                true
            }
            (false, true) => {
                // Current is interesting, next is hidden
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
