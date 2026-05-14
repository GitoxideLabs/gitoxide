use crate::{Id, Repository, filter, worktree::IndexPersistedOrInMemory};

///
pub mod pipeline {
    /// The error returned by [Repository::filter_pipeline()](super::Repository::filter_pipeline()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Could not obtain head commit of bare repository")]
        HeadCommit(#[from] crate::reference::head_commit::Error),
        #[error(transparent)]
        DecodeCommit(#[from] gix_object::decode::Error),
        #[error("Could not create index from tree at HEAD^{{tree}}")]
        TreeTraverse(#[from] crate::repository::index_from_tree::Error),
        #[error(transparent)]
        BareAttributes(#[from] crate::config::attribute_stack::Error),
        #[error(transparent)]
        WorktreeIndex(#[from] crate::worktree::open_index::Error),
        #[error(transparent)]
        Init(#[from] crate::filter::pipeline::options::Error),
    }
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
            let index = self.index_from_tree(&tree_if_bare.map_or_else(
                || {
                    self.head_commit()
                        .map_err(pipeline::Error::from)
                        .and_then(|c| c.tree_id().map(Id::detach).map_err(Into::into))
                },
                Ok,
            )?)?;
            let cache = self.attributes_only(&index, gix_worktree::stack::state::attributes::Source::IdMapping)?;
            (cache, IndexPersistedOrInMemory::InMemory(index))
        } else {
            let index = self.index_or_empty()?;
            let cache = self.attributes_only(
                &index,
                gix_worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
            )?;
            (cache, IndexPersistedOrInMemory::Persisted(index))
        };
        Ok((filter::Pipeline::new(self, cache.detach())?, index))
    }
}
