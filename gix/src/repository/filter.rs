use gix_error::ResultExt;

use crate::{Id, Repository, filter, worktree::IndexPersistedOrInMemory};

///
pub mod pipeline {
    /// The error returned by [Repository::filter_pipeline()](super::Repository::filter_pipeline()).
    pub type Error = gix_error::Error;
}

impl Repository {
    /// Configure a pipeline for converting byte buffers to the worktree representation, and byte streams to the git-internal
    /// representation. Also return the index that was used when initializing the pipeline as it may be useful when calling
    /// [convert_to_git()](filter::Pipeline::convert_to_git()).
    /// Bare repositories will either use `HEAD^{tree}` for accessing all relevant worktree files or the given `tree_if_bare`.
    ///
    /// Note that this is considered a primitive as it operates on data directly and will not have permanent effects.
    /// We also return the index that was used to configure the attributes cache (for accessing `.gitattributes`), which can be reused
    /// after it was possibly created from a tree, an expensive operation.
    ///
    /// ### Performance
    ///
    /// Note that when in a repository with worktree, files in the worktree will be read with priority, which causes at least a stat
    /// each time the directory is changed. This can be expensive if access isn't in sorted order, which would cause more then necessary
    /// stats: one per directory.
    pub fn filter_pipeline(
        &self,
        tree_if_bare: Option<gix_hash::ObjectId>,
    ) -> Result<(filter::Pipeline<'_>, IndexPersistedOrInMemory), pipeline::Error> {
        let (cache, index) = if self.is_bare() {
            let tree = tree_if_bare.map_or_else(
                || {
                    self.head_commit()
                        .or_raise(|| gix_error::message("Could not obtain head commit of bare repository"))
                        .map_err(gix_error::Error::from)
                        .and_then(|c| c.tree_id().map(Id::detach).map_err(gix_error::Error::from_error))
                },
                Ok,
            )?;
            let index = self
                .index_from_tree(&tree)
                .or_raise(|| gix_error::message("Could not create index from tree at HEAD^{tree}"))?;
            let cache = self
                .attributes_only(&index, gix_worktree::stack::state::attributes::Source::IdMapping)
                .map_err(gix_error::Error::from_error)?;
            (cache, IndexPersistedOrInMemory::InMemory(index))
        } else {
            let index = self.index_or_empty().map_err(gix_error::Error::from_error)?;
            let cache = self
                .attributes_only(
                    &index,
                    gix_worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
                )
                .map_err(gix_error::Error::from_error)?;
            (cache, IndexPersistedOrInMemory::Persisted(index))
        };
        Ok((filter::Pipeline::new(self, cache.detach())?, index))
    }
}
