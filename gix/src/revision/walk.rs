use gix_hash::ObjectId;
use gix_object::{Find, WriteTo};
pub use gix_traverse::commit::simple::CommitTimeOrder;

use crate::{Repository, Result, error::ResultExt, ext::ObjectIdExt, revision};

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
    /// Show commits before their parents, keeping commits on a branch together where possible.
    ///
    /// This is equivalent to `git log --topo-order`. Without a commit graph, the visible history
    /// must be explored before the first commit can be returned.
    TopoOrder,
    /// Show commits before their parents, otherwise prioritizing newer commit times.
    ///
    /// This is equivalent to `git log --date-order` and requires the same preparation as [`Self::TopoOrder`].
    DateOrder,
}

impl Sorting {
    fn into_simple(self) -> Option<gix_traverse::commit::simple::Sorting> {
        Some(match self {
            Sorting::BreadthFirst => gix_traverse::commit::simple::Sorting::BreadthFirst,
            Sorting::ByCommitTime(order) => gix_traverse::commit::simple::Sorting::ByCommitTime(order),
            Sorting::ByCommitTimeCutoff { seconds, order } => {
                gix_traverse::commit::simple::Sorting::ByCommitTimeCutoff { order, seconds }
            }
            Sorting::TopoOrder | Sorting::DateOrder => return None,
        })
    }
}

impl From<gix_traverse::commit::topo::Sorting> for Sorting {
    fn from(sorting: gix_traverse::commit::topo::Sorting) -> Self {
        match sorting {
            gix_traverse::commit::topo::Sorting::TopoOrder => Sorting::TopoOrder,
            gix_traverse::commit::topo::Sorting::DateOrder => Sorting::DateOrder,
        }
    }
}

/// Information about a commit that we obtained naturally as part of the iteration.
#[derive(Debug, Clone)]
pub struct Info<'repo> {
    /// The detached id of the commit.
    pub id: gix_hash::ObjectId,
    /// All parent ids we have encountered. Note that these will be at most one if [`Parents::First`][gix_traverse::commit::Parents::First] is enabled.
    /// Shallow boundary commits have no traversal parents, even though [`Self::object()`] retains their stored parents.
    pub parent_ids: gix_traverse::commit::ParentIds,
    /// The generation number if this commit was read from a commit-graph.
    pub generation: Option<gix_revwalk::graph::Generation>,
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
    pub fn object(&self) -> Result<crate::Commit<'repo>> {
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
    /// If the iteration didn't obtain commit times, as with breadth-first traversal.
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
            generation: info.generation,
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
            generation: self.generation,
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
    pub(crate) hidden: Vec<ObjectId>,
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
            hidden: Vec::new(),
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
    /// Set the sort mode for commits. [`Repository::rev_walk()`] defaults to [`Sorting::BreadthFirst`].
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
    /// Active object replacements and shallow boundaries always disable the commit-graph,
    /// which may contain different parents.
    pub fn use_commit_graph(mut self, toggle: impl Into<Option<bool>>) -> Self {
        self.use_commit_graph = toggle.into();
        self
    }

    /// Set or unset the commit-graph to use when [`Self::use_commit_graph()`] or `core.commitGraph` enables it.
    ///
    /// If `graph` is `None`, an enabled graph is still loaded from the repository.
    /// [`use_commit_graph(false)`][Self::use_commit_graph()] disables even an explicitly supplied graph.
    /// Active object replacements and shallow boundaries disable even an explicitly supplied graph.
    pub fn with_commit_graph(mut self, graph: Option<gix_commitgraph::Graph>) -> Self {
        self.commit_graph = graph;
        self
    }

    /// Don't cross the given `ids` (commits) during traversal.
    ///
    /// Most callers should prefer [`Self::with_hidden()`] instead, which implements Git-style exclusions like
    /// `^branch-to-not-list` by excluding the given commits and all their ancestors, even when reachable along another path,
    /// without changing the sorting.
    ///
    /// Note that this forces the [sorting](Self::sorting()) to [`ByCommitTimeCutoff`](Sorting::ByCommitTimeCutoff)
    /// configured with the oldest available commit time, ensuring that no commits older than the oldest of `ids` will be returned either.
    /// Also note that commits that can't be accessed or are missing are simply ignored for the purpose of obtaining the cutoff date.
    ///
    /// If this is not desired, [set the sorting](Self::sorting()) to something else right after this call.
    pub fn with_boundary(mut self, ids: impl IntoIterator<Item = impl Into<ObjectId>>) -> Self {
        let (mut cutoff, order) = match self.sorting {
            Sorting::ByCommitTimeCutoff { seconds, order } => (Some(seconds), order),
            Sorting::ByCommitTime(order) => (None, order),
            Sorting::BreadthFirst | Sorting::TopoOrder | Sorting::DateOrder => (None, CommitTimeOrder::default()),
        };
        for id in ids.into_iter() {
            let id = id.into();
            if !self.boundary.contains(&id) {
                if let Some(time) = self.repo.find_commit(id).ok().and_then(|c| c.time().ok())
                    && (cutoff.is_none() || cutoff > Some(time.seconds))
                {
                    cutoff = time.seconds.into();
                }
                self.boundary.push(id);
            }
        }

        if let Some(cutoff) = cutoff {
            self.sorting = Sorting::ByCommitTimeCutoff { seconds: cutoff, order }
        }
        self
    }

    /// Don't cross the given `tips` (commits) during traversal or return them, and also don't return any of their ancestors.
    ///
    /// This allows achieving revspecs like `^branch-to-not-list`, where the commit behind that name would be passed as `ids`.
    ///
    /// In other words, each of the `tips` acts like a starting point for an iteration that will paint commits as unwanted, and
    /// wanted commits cannot cross it.
    ///
    /// The side effect of this is that commits can't be returned immediately as one still has to wait and see if they may be unwanted later.
    /// This makes traversals with hidden commits more costly, with a chance to traverse all commits if the hidden and non-hidden commits are disjoint.
    /// Without usable commit-graph generation numbers, Git's date-based stopping heuristic limits
    /// hidden traversal; extreme clock skew can leave hidden ancestors beyond the stopping point.
    pub fn with_hidden(mut self, tips: impl IntoIterator<Item = impl Into<ObjectId>>) -> Self {
        self.hidden = tips.into_iter().map(Into::into).collect();
        self
    }
}

/// Produce the iterator
impl<'repo> Platform<'repo> {
    /// For each commit, let `filter` return `true` if it and its parents should be included in the traversal, or `false`
    /// if the traversal should exclude it and its ancestry entirely.
    ///
    /// An ancestor can still be reached along another included path. The predicate is called at
    /// most once per commit, and its call order depends on the traversal. Use [`Self::all()`] for no pruning.
    pub fn selected(self, mut filter: impl FnMut(&gix_hash::oid) -> bool + 'repo) -> Result<revision::Walk<'repo>> {
        let Platform {
            repo,
            tips,
            sorting,
            parents,
            use_commit_graph,
            commit_graph,
            mut boundary,
            hidden,
        } = self;
        boundary.sort();
        let objects = Objects {
            repo,
            shallow: repo.shallow_commits()?,
        };
        let commit_graph = if repo.has_object_replacements() || objects.shallow.is_some() {
            None
        } else if use_commit_graph.map_or_else(|| repo.config.may_use_commit_graph(), Ok)? {
            commit_graph.or_else(|| repo.commit_graph().ok())
        } else {
            None
        };
        let filter = move |id: &gix_hash::oid| filter(id) && boundary.binary_search(&id.to_owned()).is_err();
        let inner: Box<dyn Iterator<Item = Result<gix_traverse::commit::Info>> + 'repo> =
            if let Some(simple_sorting) = sorting.into_simple() {
                Box::new(
                    gix_traverse::commit::Simple::filtered(tips, objects, filter)
                        .sorting(simple_sorting)?
                        .parents(parents)
                        .commit_graph(commit_graph)
                        .hide(hidden)?,
                )
            } else {
                Box::new(
                    gix_traverse::commit::topo::Builder::from_iters(objects, tips, Some(hidden))
                        .sorting(match sorting {
                            Sorting::TopoOrder => gix_traverse::commit::topo::Sorting::TopoOrder,
                            _ => gix_traverse::commit::topo::Sorting::DateOrder,
                        })
                        .parents(parents)
                        .with_commit_graph(commit_graph)
                        .with_predicate(filter)
                        .build()?,
                )
            };
        Ok(revision::Walk { repo, inner })
    }

    /// Return an iterator to traverse all commits reachable as configured by the [Platform].
    ///
    /// # Performance
    ///
    /// It's highly recommended to set an [`object cache`](Repository::object_cache_size()) on the parent repo
    /// to greatly speed up performance if the returned id is supposed to be looked up right after.
    pub fn all(self) -> Result<revision::Walk<'repo>> {
        self.selected(|_| true)
    }
}

// Make shallow commits roots for both visible traversal and hidden-ancestor painting.
struct Objects<'repo> {
    repo: &'repo Repository,
    shallow: Option<crate::shallow::Commits>,
}

impl Find for Objects<'_> {
    fn try_find<'a>(&self, id: &gix_hash::oid, buffer: &'a mut Vec<u8>) -> Result<Option<gix_object::Data<'a>>> {
        if self
            .shallow
            .as_ref()
            .is_none_or(|commits| commits.binary_search(&id.to_owned()).is_err())
        {
            return self.repo.objects.try_find(id, buffer);
        }
        let Some(data) = self.repo.objects.try_find(id, buffer)? else {
            return Ok(None);
        };
        let (kind, object_hash) = (data.kind, data.object_hash);
        if kind.is_commit() {
            let mut commit = gix_object::CommitRef::from_bytes(data.data, object_hash)?;
            commit.parents.clear();
            let mut shallow_data = Vec::new();
            commit.write_to(&mut shallow_data).or_error()?;
            drop(commit);
            *buffer = shallow_data;
        }
        Ok(Some(gix_object::Data {
            kind,
            object_hash,
            data: buffer,
        }))
    }
}

pub(crate) mod iter_impl {
    use crate::Result;

    /// The iterator returned by [`crate::revision::walk::Platform::all()`].
    pub struct Walk<'repo> {
        /// The owning repository.
        pub repo: &'repo crate::Repository,
        pub(crate) inner: Box<dyn Iterator<Item = Result<gix_traverse::commit::Info>> + 'repo>,
    }

    impl<'repo> Iterator for Walk<'repo> {
        type Item = Result<super::Info<'repo>>;

        fn next(&mut self) -> Option<Self::Item> {
            self.inner
                .next()
                .map(|res| res.map(|info| super::Info::new(info, self.repo)))
        }
    }
}
