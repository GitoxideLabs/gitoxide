use crate::revision;
#[cfg(feature = "revision")]
use crate::{Error, Id, Result, bstr::BStr, error::message};

/// Methods for resolving revisions by spec or working with the commit graph.
impl crate::Repository {
    /// Parse a revision specification and turn it into the object(s) it describes, similar to `git rev-parse`.
    ///
    /// # Deviation
    ///
    /// - `@` actually stands for `HEAD`, whereas `git` resolves it to the object pointed to by `HEAD` without making the
    ///   `HEAD` ref available for lookups.
    #[doc(alias = "revparse", alias = "git2")]
    #[cfg(feature = "revision")]
    pub fn rev_parse<'a>(&self, spec: impl Into<&'a BStr>) -> Result<revision::Spec<'_>> {
        revision::Spec::from_bstr(
            spec,
            self,
            revision::spec::parse::Options {
                object_kind_hint: self.config.object_kind_hint,
                ..Default::default()
            },
        )
    }

    /// Parse a revision specification and return single object id as represented by this instance.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
    /// # let repo = doctest::open_repo(doctest::basic_repo_dir()?)?;
    /// let parent = repo.rev_parse_single("HEAD^")?.object()?.into_commit();
    ///
    /// assert_eq!(parent.message_raw()?, "c1\n");
    /// assert_ne!(parent.id, repo.rev_parse_single("HEAD")?);
    /// # Ok(()) }
    /// ```
    #[doc(alias = "revparse_single", alias = "git2")]
    #[cfg(feature = "revision")]
    pub fn rev_parse_single<'repo, 'a>(&'repo self, spec: impl Into<&'a BStr>) -> Result<Id<'repo>> {
        let spec = spec.into();
        self.rev_parse(spec)?.single().ok_or_else(|| {
            let spec: crate::bstr::BString = spec.into();
            Error::from_error(message!("revspec {spec:?} did not resolve to a single object"))
        })
    }

    /// Obtain the best merge-base between commit `one` and `two`, or `None` if there is none.
    /// Graph setup and traversal failures are returned as errors.
    ///
    /// # Performance
    /// For repeated calls, prefer [`merge_base_with_graph()`](Self::merge_base_with_graph()).
    /// Also be sure to [set an object cache](crate::Repository::object_cache_size_if_unset) to accelerate repeated commit lookups.
    #[cfg(feature = "revision")]
    pub fn merge_base(
        &self,
        one: impl Into<gix_hash::ObjectId>,
        two: impl Into<gix_hash::ObjectId>,
    ) -> Result<Option<Id<'_>>> {
        let cache = self.commit_graph_if_enabled()?;
        let mut graph = self.revision_graph(cache.as_ref());
        self.merge_base_with_graph(one, two, &mut graph)
    }

    /// Obtain the best merge-base between commit `one` and `two`, or `None` if there is none, providing a
    /// commit-graph `graph` to potentially greatly accelerate the operation by reusing graphs from previous runs.
    /// Traversal failures are returned as errors.
    ///
    /// # Performance
    /// Be sure to [set an object cache](crate::Repository::object_cache_size_if_unset) to accelerate repeated commit lookups.
    #[cfg(feature = "revision")]
    pub fn merge_base_with_graph(
        &self,
        one: impl Into<gix_hash::ObjectId>,
        two: impl Into<gix_hash::ObjectId>,
        graph: &mut gix_revwalk::Graph<'_, '_, gix_revwalk::graph::Commit<gix_revision::merge_base::Flags>>,
    ) -> Result<Option<Id<'_>>> {
        use crate::prelude::ObjectIdExt;
        let one = one.into();
        let two = two.into();
        let bases = gix_revision::merge_base(one, &[two], graph)?;
        Ok(bases.map(|bases| bases.first().attach(self)))
    }

    /// Get all merge-bases between commit `one` and `others`, or an empty list if there is none, providing a
    /// commit-graph `graph` to potentially greatly speed up the operation.
    /// Traversal failures are returned as errors.
    ///
    /// # Performance
    /// Be sure to [set an object cache](crate::Repository::object_cache_size_if_unset) to speed up repeated commit lookups.
    #[doc(alias = "merge_bases_many", alias = "git2")]
    #[cfg(feature = "revision")]
    pub fn merge_bases_many_with_graph(
        &self,
        one: impl Into<gix_hash::ObjectId>,
        others: &[gix_hash::ObjectId],
        graph: &mut gix_revwalk::Graph<'_, '_, gix_revwalk::graph::Commit<gix_revision::merge_base::Flags>>,
    ) -> Result<Vec<Id<'_>>> {
        use crate::prelude::ObjectIdExt;
        let one = one.into();
        Ok(match gix_revision::merge_base(one, others, graph)? {
            Some(bases) => bases.into_iter().map(|id| id.attach(self)).collect(),
            None => Vec::new(),
        })
    }

    /// Like [`merge_bases_many_with_graph()`](Self::merge_bases_many_with_graph), but without the ability to speed up consecutive calls with a [graph](gix_revwalk::Graph).
    ///
    /// # Performance
    ///
    /// Be sure to [set an object cache](crate::Repository::object_cache_size_if_unset) to speed up repeated commit lookups, and consider
    /// using [`merge_bases_many_with_graph()`](Self::merge_bases_many_with_graph) for consecutive calls.
    #[doc(alias = "git2")]
    #[cfg(feature = "revision")]
    pub fn merge_bases_many(
        &self,
        one: impl Into<gix_hash::ObjectId>,
        others: &[gix_hash::ObjectId],
    ) -> Result<Vec<Id<'_>>> {
        let cache = self.commit_graph_if_enabled()?;
        let mut graph = self.revision_graph(cache.as_ref());
        self.merge_bases_many_with_graph(one, others, &mut graph)
    }

    /// Return the best merge-base among all `commits`, or `None` if `commits` yields no commit or no merge-base was found.
    ///
    /// Use `graph` to speed up repeated calls. Traversal failures are returned as errors.
    #[cfg(feature = "revision")]
    pub fn merge_base_octopus_with_graph(
        &self,
        commits: impl IntoIterator<Item = impl Into<gix_hash::ObjectId>>,
        graph: &mut gix_revwalk::Graph<'_, '_, gix_revwalk::graph::Commit<gix_revision::merge_base::Flags>>,
    ) -> Result<Option<Id<'_>>> {
        use crate::prelude::ObjectIdExt;
        let commit_ids: Vec<_> = commits.into_iter().map(Into::into).collect();
        let Some(first) = commit_ids.first().copied() else {
            return Ok(None);
        };
        let merge_base_id = gix_revision::merge_base::octopus(first, &commit_ids[1..], graph)?;
        Ok(merge_base_id.map(|id| id.attach(self)))
    }

    /// Return the best merge-base among all `commits`, or `None` if `commits` yields no commit or no merge-base was found.
    /// Graph setup and traversal failures are returned as errors.
    ///
    /// For repeated calls, prefer [`Self::merge_base_octopus_with_graph()`] for cache-reuse.
    #[cfg(feature = "revision")]
    pub fn merge_base_octopus(
        &self,
        commits: impl IntoIterator<Item = impl Into<gix_hash::ObjectId>>,
    ) -> Result<Option<Id<'_>>> {
        let cache = self.commit_graph_if_enabled()?;
        let mut graph = self.revision_graph(cache.as_ref());
        self.merge_base_octopus_with_graph(commits, &mut graph)
    }

    /// Return `true` if `ancestor_commit_id` can be reached from `descendant_commit_id` by following parents, similar to
    /// `git merge-base --is-ancestor`. A commit is its own ancestor, and commits of unrelated histories aren't ancestors
    /// of each other. Graph setup and traversal failures are returned as errors.
    /// Shallow boundaries and active object replacements are respected.
    ///
    /// Note that `git2`'s `graph_descendant_of()` takes the descendant first, and doesn't consider a commit its own descendant.
    ///
    /// # Performance
    /// For repeated calls, prefer [`is_ancestor_with_graph()`](Self::is_ancestor_with_graph()).
    /// Also be sure to [set an object cache](crate::Repository::object_cache_size_if_unset) to accelerate repeated commit lookups.
    #[doc(alias = "graph_descendant_of", alias = "git2")]
    #[cfg(feature = "revision")]
    pub fn is_ancestor(
        &self,
        ancestor_commit_id: impl Into<gix_hash::ObjectId>,
        descendant_commit_id: impl Into<gix_hash::ObjectId>,
    ) -> Result<bool> {
        let cache = self.commit_graph_if_enabled()?;
        let mut graph = self.revision_graph(cache.as_ref());
        self.is_ancestor_with_graph(ancestor_commit_id, descendant_commit_id, &mut graph)
    }

    /// Like [`is_ancestor()`](Self::is_ancestor()), but with a commit-graph `graph` to potentially greatly accelerate
    /// the operation by reusing graphs from previous runs. Traversal failures are returned as errors.
    /// Shallow boundaries or active replacements require a temporary graph instead.
    ///
    /// # Performance
    /// Be sure to [set an object cache](crate::Repository::object_cache_size_if_unset) to accelerate repeated commit lookups.
    #[cfg(feature = "revision")]
    pub fn is_ancestor_with_graph(
        &self,
        ancestor_commit_id: impl Into<gix_hash::ObjectId>,
        descendant_commit_id: impl Into<gix_hash::ObjectId>,
        graph: &mut gix_revwalk::Graph<'_, '_, gix_revwalk::graph::Commit<gix_revision::merge_base::Flags>>,
    ) -> Result<bool> {
        let ancestor_commit_id = ancestor_commit_id.into();
        let descendant_commit_id = descendant_commit_id.into();
        match self.revision_graph_for_modified_history()? {
            Some(mut graph) => {
                gix_revision::merge_base::is_ancestor(ancestor_commit_id, descendant_commit_id, &mut graph)
            }
            None => gix_revision::merge_base::is_ancestor(ancestor_commit_id, descendant_commit_id, graph),
        }
    }

    /// Count commits reachable exclusively from `tip_commit_id` or `base_commit_id`, matching
    /// `git rev-list --left-right --count tip...base`. The result's `ahead` count belongs to the tip,
    /// and its `behind` count belongs to the base. Equal commits return zero for both counts.
    /// Shallow boundaries and missing parent objects end the available history; both input commits must exist.
    /// Active object replacements are respected.
    /// Graph setup and traversal failures are returned as errors.
    ///
    /// # Performance
    /// Exact counts can require substantially more traversal than [`is_ancestor()`](Self::is_ancestor()).
    /// For repeated calls, prefer [`ahead_behind_with_graph()`](Self::ahead_behind_with_graph()).
    #[cfg(feature = "revision")]
    pub fn ahead_behind(
        &self,
        tip_commit_id: impl Into<gix_hash::ObjectId>,
        base_commit_id: impl Into<gix_hash::ObjectId>,
    ) -> Result<gix_revision::AheadBehind> {
        let cache = self.commit_graph_if_enabled()?;
        let mut graph = self.revision_graph(cache.as_ref());
        self.ahead_behind_with_graph(tip_commit_id, base_commit_id, &mut graph)
    }

    /// Like [`ahead_behind()`](Self::ahead_behind()), but reuses cached commits in `graph`.
    /// Its flags are automatically cleared. Traversal failures are returned as errors.
    /// Shallow boundaries or active replacements require a temporary graph instead.
    ///
    /// # Performance
    /// With nonzero, unsaturated commit-graph generations, traversal stops once only shared history remains.
    /// Otherwise, the walk may visit all available ancestors.
    #[cfg(feature = "revision")]
    pub fn ahead_behind_with_graph(
        &self,
        tip_commit_id: impl Into<gix_hash::ObjectId>,
        base_commit_id: impl Into<gix_hash::ObjectId>,
        graph: &mut gix_revwalk::Graph<'_, '_, gix_revwalk::graph::Commit<gix_revision::merge_base::Flags>>,
    ) -> Result<gix_revision::AheadBehind> {
        let tip_commit_id = tip_commit_id.into();
        let base_commit_id = base_commit_id.into();
        match self.revision_graph_for_modified_history()? {
            Some(mut graph) => gix_revision::ahead_behind(tip_commit_id, base_commit_id, &mut graph),
            None => gix_revision::ahead_behind(tip_commit_id, base_commit_id, graph),
        }
    }

    #[cfg(feature = "revision")]
    fn revision_graph_for_modified_history(
        &self,
    ) -> Result<Option<gix_revwalk::Graph<'_, '_, gix_revwalk::graph::Commit<gix_revision::merge_base::Flags>>>> {
        let shallow_commits = self.shallow_commits()?;
        if shallow_commits.is_none()
            && (self.objects.ignore_replacements || self.objects.store_ref().replacements().next().is_none())
        {
            return Ok(None);
        }
        let mut graph = self.revision_graph(None);
        if let Some(commits) = shallow_commits {
            for commit_id in commits.iter() {
                graph.get_or_insert_full_commit(*commit_id, |commit| commit.parents.clear())?;
            }
        }
        Ok(Some(graph))
    }

    /// Create the baseline for a revision walk by initializing it with the `tips` to start iterating on.
    ///
    /// It can be configured further before starting the actual walk.
    #[doc(alias = "revwalk", alias = "git2")]
    pub fn rev_walk(
        &self,
        tips: impl IntoIterator<Item = impl Into<gix_hash::ObjectId>>,
    ) -> revision::walk::Platform<'_> {
        revision::walk::Platform::new(tips, self)
    }
}
