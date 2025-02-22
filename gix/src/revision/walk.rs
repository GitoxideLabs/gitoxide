use gix_hash::ObjectId;
use gix_object::FindExt;
use gix_traverse::commit::simple::CommitTimeOrder;

use crate::{ext::ObjectIdExt, revision, Repository};

/// The error returned by [`Platform::all()`] and [`Platform::selected()`].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    SimpleTraversal(#[from] gix_traverse::commit::simple::Error),
    #[error(transparent)]
    ShallowCommits(#[from] crate::shallow::read::Error),
    #[error(transparent)]
    ConfigBoolean(#[from] crate::config::boolean::Error),
}

/// Specify how to sort commits during a [revision::Walk] traversal.
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
    /// In the *sample history* the order would be `8, 6, 7, 5, 4, 3, 2, 1`
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
    /// This mode benefits greatly from having an [object cache](crate::Repository::object_cache_size) configured
    /// to avoid having to look up each commit twice.
    ByCommitTime(CommitTimeOrder),
    /// This sorting is similar to [`ByCommitTime`](Sorting::ByCommitTimeCutoff), but adds a cutoff to not return commits older than
    /// a given time, stopping the iteration once no younger commits is queued to be traversed.
    ///
    /// As the query is usually repeated with different cutoff dates, this search mode benefits greatly from an object cache.
    ///
    /// In the *sample history* and a cut-off date of 4, the returned list of commits would be `8, 7, 6, 4`
    ByCommitTimeCutoff {
        /// The order in wich to prioritize lookups
        order: CommitTimeOrder,
        /// The amount of seconds since unix epoch to use as cut-off time.
        seconds: gix_date::SecondsSinceUnixEpoch,
    },
}

impl Sorting {
    fn into_simple(self) -> Option<gix_traverse::commit::simple::Sorting> {
        Some(match self {
            Sorting::BreadthFirst => gix_traverse::commit::simple::Sorting::BreadthFirst,
            Sorting::ByCommitTime(order) => gix_traverse::commit::simple::Sorting::ByCommitTime(order),
            Sorting::ByCommitTimeCutoff { seconds, order } => {
                gix_traverse::commit::simple::Sorting::ByCommitTimeCutoff { order, seconds }
            }
        })
    }
}

/// Information about a commit that we obtained naturally as part of the iteration.
#[derive(Debug, Clone)]
pub struct Info<'repo> {
    /// The detached id of the commit.
    pub id: gix_hash::ObjectId,
    /// All parent ids we have encountered. Note that these will be at most one if [`Parents::First`][gix_traverse::commit::Parents::First] is enabled.
    pub parent_ids: gix_traverse::commit::ParentIds,
    /// The time at which the commit was created. It will only be `Some(_)` if the chosen traversal was
    /// taking dates into consideration.
    pub commit_time: Option<gix_date::SecondsSinceUnixEpoch>,

    repo: &'repo Repository,
}

/// Access
impl<'repo> Info<'repo> {
    /// Provide an attached version of our [`id`][Info::id] field.
    pub fn id(&self) -> crate::Id<'repo> {
        self.id.attach(self.repo)
    }

    /// Read the whole object from the object database.
    ///
    /// Note that this is an expensive operation which shouldn't be performed unless one needs more than parent ids
    /// and commit time.
    pub fn object(&self) -> Result<crate::Commit<'repo>, crate::object::find::existing::Error> {
        Ok(self.id().object()?.into_commit())
    }

    /// Provide an iterator yielding attached versions of our [`parent_ids`][Info::parent_ids] field.
    pub fn parent_ids(&self) -> impl Iterator<Item = crate::Id<'repo>> + '_ {
        self.parent_ids.iter().map(|id| id.attach(self.repo))
    }

    /// Returns the commit-time of this commit.
    ///
    /// ### Panics
    ///
    /// If the iteration wasn't ordered by date.
    pub fn commit_time(&self) -> gix_date::SecondsSinceUnixEpoch {
        self.commit_time.expect("traversal involving date caused it to be set")
    }
}

/// Initialization and detachment
impl<'repo> Info<'repo> {
    /// Create a new instance that represents `info`, but is attached to `repo` as well.
    pub fn new(info: gix_traverse::commit::Info, repo: &'repo Repository) -> Self {
        Info {
            id: info.id,
            parent_ids: info.parent_ids,
            commit_time: info.commit_time,
            repo,
        }
    }
    /// Consume this instance and remove the reference to the underlying repository.
    ///
    /// This is useful for sending instances across threads, for example.
    pub fn detach(self) -> gix_traverse::commit::Info {
        gix_traverse::commit::Info {
            id: self.id,
            parent_ids: self.parent_ids,
            commit_time: self.commit_time,
        }
    }
}

/// A platform to traverse the revision graph by adding starting points as well as points which shouldn't be crossed,
/// returned by [`Repository::rev_walk()`].
///
/// **Note that we automatically leverage the commitgraph data structure**, but if you know that additional information like
/// author or commit messages will be required of *all* commits traversed here, it should be better to avoid trying to load it
/// by [turning commit-graph support off][Platform::use_commit_graph()]. This certainly is a micro-optimization though.
pub struct Platform<'repo> {
    /// The owning repository.
    pub repo: &'repo Repository,
    pub(crate) tips: Vec<ObjectId>,
    pub(crate) boundary: Vec<ObjectId>,
    pub(crate) sorting: Sorting,
    pub(crate) parents: gix_traverse::commit::Parents,
    pub(crate) use_commit_graph: Option<bool>,
    pub(crate) commit_graph: Option<gix_commitgraph::Graph>,
}

impl<'repo> Platform<'repo> {
    pub(crate) fn new(tips: impl IntoIterator<Item = impl Into<ObjectId>>, repo: &'repo Repository) -> Self {
        revision::walk::Platform {
            repo,
            tips: tips.into_iter().map(Into::into).collect(),
            sorting: Default::default(),
            parents: Default::default(),
            use_commit_graph: None,
            commit_graph: None,
            boundary: Vec::new(),
        }
    }
}

/// Create-time builder methods
impl Platform<'_> {
    /// Set the sort mode for commits to the given value. The default is to order topologically breadth-first.
    pub fn sorting(mut self, sorting: Sorting) -> Self {
        self.sorting = sorting;
        self
    }

    /// Only traverse the first parent of the commit graph.
    pub fn first_parent_only(mut self) -> Self {
        self.parents = gix_traverse::commit::Parents::First;
        self
    }

    /// Allow using the commitgraph, if present, if `toggle` is `true`, or disallow it with `false`. Set it to `None` to leave
    /// control over this to the configuration of `core.commitGraph` (the default).
    ///
    /// Errors when loading the graph lead to falling back to the object database, it's treated as optional cache.
    pub fn use_commit_graph(mut self, toggle: impl Into<Option<bool>>) -> Self {
        self.use_commit_graph = toggle.into();
        self
    }

    /// Set or unset the commit-graph to use for the iteration. This is useful if the caller wants to check if a commit-graph exists
    /// and refer different implementations depending on the outcome.
    ///
    /// It interacts with [`use_commit_graph`][Platform::use_commit_graph()] as one would expect, but it's worth noting that if `None`,
    /// with [`use_commit_graph`][Platform::use_commit_graph()] being `true`, a graph will still be used for iteration.
    /// To turn the commit-graph off, call [`use_commit_graph(false)`][Platform::use_commit_graph()] instead.
    pub fn with_commit_graph(mut self, graph: Option<gix_commitgraph::Graph>) -> Self {
        self.commit_graph = graph;
        self
    }

    /// Don't cross the given `ids` during traversal.
    ///
    /// Note that this forces the [sorting](Self::sorting()) to [`ByCommitTimeCutoff`](Sorting::ByCommitTimeCutoff)
    /// configured with the oldest available commit time, ensuring that no commits older than the oldest of `ids` will be returned either.
    /// Also note that commits that can't be accessed or are missing are simply ignored for the purpose of obtaining the cutoff date.
    ///
    /// A boundary is distinctly different from exclusive refsepcs `^branch-to-not-list` in Git log.
    ///
    /// If this is not desired, [set the sorting](Self::sorting()) to something else right after this call.
    pub fn with_boundary(mut self, ids: impl IntoIterator<Item = impl Into<ObjectId>>) -> Self {
        let (mut cutoff, order) = match self.sorting {
            Sorting::ByCommitTimeCutoff { seconds, order } => (Some(seconds), order),
            Sorting::ByCommitTime(order) => (None, order),
            Sorting::BreadthFirst => (None, CommitTimeOrder::default()),
        };
        for id in ids.into_iter() {
            let id = id.into();
            if !self.boundary.contains(&id) {
                if let Some(time) = self.repo.find_commit(id).ok().and_then(|c| c.time().ok()) {
                    if cutoff.is_none() || cutoff > Some(time.seconds) {
                        cutoff = time.seconds.into();
                    }
                }
                self.boundary.push(id);
            }
        }

        if let Some(cutoff) = cutoff {
            self.sorting = Sorting::ByCommitTimeCutoff { seconds: cutoff, order }
        }
        self
    }
}

/// Produce the iterator
impl<'repo> Platform<'repo> {
    /// For each commit, let `filter` return `true` if it and its parents should be included in the traversal, or `false`
    /// if the traversal should exclude it and its ancestry entirely.
    ///
    /// If `filter` is None, no pruning of the graph will be performed which is the default.
    pub fn selected(
        self,
        mut filter: impl FnMut(&gix_hash::oid) -> bool + 'repo,
    ) -> Result<revision::Walk<'repo>, Error> {
        let Platform {
            repo,
            tips,
            sorting,
            parents,
            use_commit_graph,
            commit_graph,
            mut boundary,
        } = self;
        boundary.sort();
        Ok(revision::Walk {
            repo,
            inner: Box::new(
                gix_traverse::commit::Simple::filtered(tips, &repo.objects, {
                    // Note that specific shallow handling for commit-graphs isn't needed as these contain
                    // all information there is, and exclude shallow parents to be structurally consistent.
                    let shallow_commits = repo.shallow_commits()?;
                    let mut grafted_parents_to_skip = Vec::new();
                    let mut buf = Vec::new();
                    move |id| {
                        if !filter(id) {
                            return false;
                        }
                        let id = id.to_owned();
                        if boundary.binary_search(&id).is_ok() {
                            return false;
                        }
                        match shallow_commits.as_ref() {
                            Some(commits) => {
                                if let Ok(idx) = grafted_parents_to_skip.binary_search(&id) {
                                    grafted_parents_to_skip.remove(idx);
                                    return false;
                                }
                                if commits.binary_search(&id).is_ok() {
                                    if let Ok(commit) = repo.objects.find_commit_iter(&id, &mut buf) {
                                        grafted_parents_to_skip.extend(commit.parent_ids());
                                        grafted_parents_to_skip.sort();
                                    }
                                }
                                true
                            }
                            None => true,
                        }
                    }
                })
                .sorting(sorting.into_simple().expect("for now there is nothing else"))?
                .parents(parents)
                .commit_graph(
                    commit_graph.or(use_commit_graph
                        .map_or_else(|| self.repo.config.may_use_commit_graph(), Ok)?
                        .then(|| self.repo.commit_graph().ok())
                        .flatten()),
                )
                .map(|res| res.map_err(iter::Error::from)),
            ),
        })
    }
    /// Return an iterator to traverse all commits reachable as configured by the [Platform].
    ///
    /// # Performance
    ///
    /// It's highly recommended to set an [`object cache`](Repository::object_cache_size()) on the parent repo
    /// to greatly speed up performance if the returned id is supposed to be looked up right after.
    pub fn all(self) -> Result<revision::Walk<'repo>, Error> {
        self.selected(|_| true)
    }
}

///
pub mod iter {
    /// The error returned by the [Walk](crate::revision::Walk) iterator.
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        SimpleTraversal(#[from] gix_traverse::commit::simple::Error),
    }
}

pub(crate) mod iter_impl {
    /// The iterator returned by [`crate::revision::walk::Platform::all()`].
    pub struct Walk<'repo> {
        /// The owning repository.
        pub repo: &'repo crate::Repository,
        pub(crate) inner: Box<dyn Iterator<Item = Result<gix_traverse::commit::Info, super::iter::Error>> + 'repo>,
    }

    impl<'repo> Iterator for Walk<'repo> {
        type Item = Result<super::Info<'repo>, super::iter::Error>;

        fn next(&mut self) -> Option<Self::Item> {
            self.inner
                .next()
                .map(|res| res.map(|info| super::Info::new(info, self.repo)))
        }
    }
}
