use crate::Result;
impl crate::Repository {
    /// Create a graph data-structure capable of accelerating graph traversals and storing state of type `T` with each commit
    /// it encountered.
    ///
    /// Shallow commits are treated as roots when reading from the object database. Reading the repository's
    /// shallow boundaries can fail.
    ///
    /// A supplied `cache` takes precedence for the commits it contains. The caller is responsible for its
    /// parent links and generation numbers matching the intended history. Use [`Self::revision_graph_with_cache()`]
    /// to select and own a cache according to repository configuration and compatibility.
    ///
    /// ### Performance
    ///
    /// Note that the [Graph][gix_revwalk::Graph] can be sensitive to various object database settings that may affect the performance
    /// of the commit walk.
    pub fn revision_graph<'cache, T>(
        &self,
        cache: Option<&'cache gix_commitgraph::Graph>,
    ) -> Result<gix_revwalk::Graph<'_, 'cache, T>> {
        Ok(match self.shallow_commits()? {
            Some(shallow) => gix_revwalk::Graph::new(
                crate::revision::walk::Objects {
                    repo: self,
                    shallow: Some(shallow),
                },
                cache,
            ),
            None => gix_revwalk::Graph::new(&self.objects, cache),
        })
    }

    /// Create a [`Self::revision_graph()`] that owns the cache selected by [`Self::commit_graph_if_enabled()`].
    /// Recover the cache for reuse with [`gix_revwalk::Graph::into_cache()`] and transfer it to another graph with
    /// [`gix_revwalk::Graph::with_owned_cache()`].
    pub fn revision_graph_with_cache<T>(&self) -> Result<gix_revwalk::Graph<'_, 'static, T>> {
        let cache = self.commit_graph_if_enabled()?;
        Ok(self.revision_graph(None)?.with_owned_cache(cache))
    }

    /// Return a cache for commits and their graph structure, as managed by `git commit-graph`, for accelerating commit walks on
    /// a low level.
    ///
    /// Parent-buffer allocations respect `gitoxide.objects.allocLimit`.
    pub fn commit_graph(&self) -> Result<gix_commitgraph::Graph> {
        gix_commitgraph::Graph::at(
            &self.objects.store_ref().path().join("info"),
            self.object_hash(),
            self.config.alloc_limit_bytes,
        )
    }

    /// Return a newly opened commit-graph if it is available, enabled in the Git configuration, and compatible
    /// with the repository's history. Shallow boundaries and active replacements make it incompatible.
    pub fn commit_graph_if_enabled(&self) -> Result<Option<gix_commitgraph::Graph>> {
        if self.is_shallow()? || self.has_object_replacements() {
            return Ok(None);
        }
        self.config
            .may_use_commit_graph()?
            .then(|| self.commit_graph())
            .transpose()
            .or_else(|err| if err.is_not_found() { Ok(None) } else { Err(err) })
    }

    pub(crate) fn has_object_replacements(&self) -> bool {
        !self.objects.ignore_replacements && self.objects.store_ref().replacements().next().is_some()
    }
}
