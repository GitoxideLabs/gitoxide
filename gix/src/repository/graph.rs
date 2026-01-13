use crate::Error;

impl crate::Repository {
    /// Create a graph data-structure capable of accelerating graph traversals and storing state of type `T` with each commit
    /// it encountered.
    ///
    /// Note that the `cache` will be used if present, and it's best obtained with
    /// [`commit_graph_if_enabled()`](crate::Repository::commit_graph_if_enabled()).
    ///
    /// Note that a commitgraph is only allowed to be used if `core.commitGraph` is true (the default), and that configuration errors are
    /// ignored as well.
    ///
    /// ### Performance
    ///
    /// Note that the [Graph][gix_revwalk::Graph] can be sensitive to various object database settings that may affect the performance
    /// of the commit walk.
    pub fn revision_graph<'cache, T>(
        &self,
        cache: Option<&'cache gix_commitgraph::Graph>,
    ) -> gix_revwalk::Graph<'_, 'cache, T> {
        gix_revwalk::Graph::new(&self.objects, cache)
    }

    /// Return a cache for commits and their graph structure, as managed by `git commit-graph`, for accelerating commit walks on
    /// a low level.
    ///
    /// Note that [`revision_graph()`][crate::Repository::revision_graph()] should be preferred for general purpose walks that don't
    /// rely on the actual commit cache to be present, while leveraging the commit-graph if possible.
    pub fn commit_graph(&self) -> Result<gix_commitgraph::Graph, Error> {
        gix_commitgraph::at(self.objects.store_ref().path().join("info")).map_err(Into::into)
    }

    /// Return a newly opened commit-graph if it is available *and* enabled in the Git configuration.
    pub fn commit_graph_if_enabled(
        &self,
    ) -> Result<Option<gix_commitgraph::Graph>, super::commit_graph_if_enabled::Error> {
        Ok(self
            .config
            .may_use_commit_graph()?
            .then(|| gix_commitgraph::at(self.objects.store_ref().path().join("info")))
            .transpose()
            .or_else(|err| match err.downcast_any_ref::<std::io::Error>() {
                Some(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
                _ => Err(err.into_error()),
            })?)
    }
}
