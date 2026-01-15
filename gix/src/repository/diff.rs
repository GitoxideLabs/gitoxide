use gix_object::TreeRefIter;

use crate::{
    repository::{diff_resource_cache, diff_tree_to_tree},
    Repository, Tree,
};

/// Diff-utilities
impl Repository {
    /// Create a resource cache for diffable objects, and configured with everything it needs to know to perform diffs
    /// faithfully just like `git` would.
    /// `mode` controls what version of a resource should be diffed.
    /// `worktree_roots` determine if files can be read from the worktree, where each side of the diff operation can
    /// be represented by its own worktree root. `.gitattributes` are automatically read from the worktree if at least
    /// one worktree is present.
    ///
    /// Note that attributes will always be obtained from the current `HEAD` index even if the resources being diffed
    /// might live in another tree. Further, if one of the `worktree_roots` are set, attributes will also be read from
    /// the worktree. Otherwise, it will be skipped and attributes are read from the index tree instead.
    pub fn diff_resource_cache(
        &self,
        mode: gix_diff::blob::pipeline::Mode,
        worktree_roots: gix_diff::blob::pipeline::WorktreeRoots,
    ) -> Result<gix_diff::blob::Platform, diff_resource_cache::Error> {
        let index = self.index_or_load_from_head_or_empty()?;
        Ok(crate::diff::resource_cache(
            self,
            mode,
            self.attributes_only(
                &index,
                if worktree_roots.is_unset() {
                    gix_worktree::stack::state::attributes::Source::IdMapping
                } else {
                    gix_worktree::stack::state::attributes::Source::WorktreeThenIdMapping
                },
            )?
            .inner,
            worktree_roots,
        )?)
    }

    /// Produce the changes that would need to be applied to `old_tree` to create `new_tree`.
    /// If `options` are unset, they will be filled in according to the git configuration of this repository, and with
    /// [full paths being tracked](crate::diff::Options::track_path()) as well, which typically means that
    /// rewrite tracking might be disabled if done so explicitly by the user.
    /// If `options` are set, the user can take full control over the settings.
    ///
    /// Note that this method exists to evoke similarity to `git2`, and makes it easier to fully control diff settings.
    /// A more fluent version [may be used as well](Tree::changes()).
    pub fn diff_tree_to_tree<'a, 'old_repo: 'a, 'new_repo: 'a>(
        &self,
        old_tree: impl Into<Option<&'a Tree<'old_repo>>>,
        new_tree: impl Into<Option<&'a Tree<'new_repo>>>,
        options: impl Into<Option<crate::diff::Options>>,
    ) -> Result<Vec<crate::object::tree::diff::ChangeDetached>, diff_tree_to_tree::Error> {
        let mut cache = self.diff_resource_cache(gix_diff::blob::pipeline::Mode::ToGit, Default::default())?;
        let opts = options
            .into()
            .map_or_else(|| crate::diff::Options::from_configuration(&self.config), Ok)?
            .into();

        let empty_tree = self.empty_tree();
        let old_tree = old_tree.into().unwrap_or(&empty_tree);
        let new_tree = new_tree.into().unwrap_or(&empty_tree);
        let mut out = Vec::new();
        gix_diff::tree_with_rewrites(
            TreeRefIter::from_bytes(&old_tree.data),
            TreeRefIter::from_bytes(&new_tree.data),
            &mut cache,
            &mut Default::default(),
            &self.objects,
            |change| -> Result<_, std::convert::Infallible> {
                out.push(change.into_owned());
                Ok(std::ops::ControlFlow::Continue(()))
            },
            opts,
        )?;
        Ok(out)
    }

    /// Return a resource cache suitable for diffing blobs from trees directly, where no worktree checkout exists.
    ///
    /// For more control, see [`diff_resource_cache()`](Self::diff_resource_cache).
    pub fn diff_resource_cache_for_tree_diff(&self) -> Result<gix_diff::blob::Platform, diff_resource_cache::Error> {
        self.diff_resource_cache(
            gix_diff::blob::pipeline::Mode::ToGit,
            gix_diff::blob::pipeline::WorktreeRoots::default(),
        )
    }
}
