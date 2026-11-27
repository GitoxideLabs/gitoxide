use crate::Result;
impl crate::Repository {
    /// Create a revision graph for the repository's visible history, storing state of type `T` for each commit it encounters.
    ///
    /// The graph owns the optional commit-graph selected by [`Self::commit_graph_if_enabled()`]. Shallow commits
    /// are treated as roots. Reading the shallow boundaries or loading the commit-graph can fail.
    ///
    /// Recover the owned commit-graph with [`gix_revwalk::Graph::into_commit_graph()`] and transfer it to another graph with
    /// [`gix_revwalk::Graph::with_owned_commit_graph()`].
    ///
    /// ### Performance
    ///
    /// If a walk repeatedly looks up commits not covered by the commit-graph, set an
    /// [object cache](Self::object_cache_size_if_unset) before creating the revision graph to avoid
    /// rereading and decoding those objects.
    pub fn revision_graph<T>(&self) -> Result<gix_revwalk::Graph<'_, 'static, T>> {
        let commit_graph = self.commit_graph_if_enabled()?;
        Ok(self
            .revision_graph_with_commit_graph(None)?
            .with_owned_commit_graph(commit_graph))
    }

    /// Like [`Self::revision_graph()`], but borrows the supplied `commit_graph` instead of selecting one automatically.
    /// Pass `None` to read commits only from the object database, treating shallow commits as roots.
    ///
    /// A supplied commit-graph takes precedence for the commits it contains, even when shallow boundaries or
    /// object replacements would change their ancestry. The caller is responsible for its parent links and
    /// generation numbers matching the intended history.
    ///
    /// Use [`Self::commit_graph_if_enabled()`] to respect repository configuration and history compatibility,
    /// or [`Self::commit_graph()`] to open the on-disk commit-graph without those checks.
    pub fn revision_graph_with_commit_graph<'commit_graph, T>(
        &self,
        commit_graph: Option<&'commit_graph gix_commitgraph::Graph>,
    ) -> Result<gix_revwalk::Graph<'_, 'commit_graph, T>> {
        Ok(match self.shallow_commits()? {
            Some(shallow) => gix_revwalk::Graph::new(
                crate::revision::walk::Objects {
                    repo: self,
                    shallow: Some(shallow),
                },
                commit_graph,
            ),
            None => gix_revwalk::Graph::new(&self.objects, commit_graph),
        })
    }

    /// Open the commit-graph managed by `git commit-graph` to accelerate low-level commit walks.
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
