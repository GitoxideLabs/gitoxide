#[cfg(feature = "revision")]
use gix_error::{ResultExt, message};

use crate::revision;
#[cfg(feature = "revision")]
use crate::{Error, Id, Result, bstr::BStr};

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
            Error::from_error(gix_error::message!(
                "revspec {spec:?} did not resolve to a single object"
            ))
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
        let cache = self
            .commit_graph_if_enabled()
            .or_raise(|| message("Could not set up the commit graph for merge-base traversal"))?;
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
        let bases = gix_revision::merge_base(one, &[two], graph)
            .or_raise(|| message("Could not traverse commits to find a merge base"))?;
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
        Ok(
            match gix_revision::merge_base(one, others, graph)
                .or_raise(|| message("Could not traverse commits to find a merge base"))?
            {
                Some(bases) => bases.into_iter().map(|id| id.attach(self)).collect(),
                None => Vec::new(),
            },
        )
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
        let cache = self
            .commit_graph_if_enabled()
            .or_raise(|| message("Could not set up the commit graph for merge-base traversal"))?;
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
        let merge_base_id = gix_revision::merge_base::octopus(first, &commit_ids[1..], graph)
            .or_raise(|| message("Could not traverse commits to find a merge base"))?;
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
        let cache = self
            .commit_graph_if_enabled()
            .or_raise(|| message("Could not set up the commit graph for merge-base traversal"))?;
        let mut graph = self.revision_graph(cache.as_ref());
        self.merge_base_octopus_with_graph(commits, &mut graph)
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
