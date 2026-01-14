use gix_object::TreeRefIter;

use super::{Action, Change, Platform};
use crate::{diff::rewrites::tracker, Tree};

/// The error return by methods on the [diff platform][Platform].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    Diff(#[from] gix_diff::tree_with_rewrites::Error),
    #[error("The user-provided callback failed")]
    ForEach(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
    #[error(transparent)]
    ResourceCache(#[from] crate::repository::diff_resource_cache::Error),
    #[error("Failure during rename tracking")]
    RenameTracking(#[from] tracker::emit::Error),
}

/// Add the item to compare to.
impl<'old> Platform<'_, 'old> {
    /// Call `for_each` repeatedly with all changes that are needed to convert the source of the diff to the tree to `other`.
    ///
    /// `other` could also be created with the [`empty_tree()`][crate::Repository::empty_tree()] method to handle the first commit
    /// in a repository - it doesn't have a parent, equivalent to compare 'nothing' to something.
    pub fn for_each_to_obtain_tree<'new, E>(
        &mut self,
        other: &Tree<'new>,
        for_each: impl FnMut(Change<'_, 'old, 'new>) -> Result<Action, E>,
    ) -> Result<Option<gix_diff::rewrites::Outcome>, Error>
    where
        E: Into<Box<dyn std::error::Error + Sync + Send + 'static>>,
    {
        self.for_each_to_obtain_tree_inner(other, for_each, None)
    }

    /// Like [`Self::for_each_to_obtain_tree()`], but with a reusable `resource_cache` which is used to perform
    /// diffs fast.
    ///
    /// Reusing it between multiple invocations saves a lot of IOps as it avoids the creation
    /// of a temporary `resource_cache` that triggers reading or checking for multiple gitattribute files.
    /// Note that it's recommended to call [`gix_diff::blob::Platform::clear_resource_cache()`] between the calls
    /// to avoid runaway memory usage, as the cache isn't limited.
    ///
    /// Note that to do rename tracking like `git` does, one has to configure the `resource_cache` with
    /// a conversion pipeline that uses [`gix_diff::blob::pipeline::Mode::ToGit`].
    pub fn for_each_to_obtain_tree_with_cache<'new, E>(
        &mut self,
        other: &Tree<'new>,
        resource_cache: &mut gix_diff::blob::Platform,
        for_each: impl FnMut(Change<'_, 'old, 'new>) -> Result<Action, E>,
    ) -> Result<Option<gix_diff::rewrites::Outcome>, Error>
    where
        E: Into<Box<dyn std::error::Error + Sync + Send + 'static>>,
    {
        self.for_each_to_obtain_tree_inner(other, for_each, Some(resource_cache))
    }

    fn for_each_to_obtain_tree_inner<'new, E>(
        &mut self,
        other: &Tree<'new>,
        mut for_each: impl FnMut(Change<'_, 'old, 'new>) -> Result<Action, E>,
        resource_cache: Option<&mut gix_diff::blob::Platform>,
    ) -> Result<Option<gix_diff::rewrites::Outcome>, Error>
    where
        E: Into<Box<dyn std::error::Error + Sync + Send + 'static>>,
    {
        let repo = self.lhs.repo;
        let mut storage;
        let cache = match resource_cache {
            None => {
                storage = repo.diff_resource_cache(gix_diff::blob::pipeline::Mode::ToGit, Default::default())?;
                &mut storage
            }
            Some(cache) => cache,
        };
        let opts = self.options.into();
        Ok(gix_diff::tree_with_rewrites(
            TreeRefIter::from_bytes(&self.lhs.data),
            TreeRefIter::from_bytes(&other.data),
            cache,
            &mut self.state,
            &repo.objects,
            |change| {
                for_each(Change::from_change_ref(change, repo, other.repo)).map(|action| match action {
                    std::ops::ControlFlow::Continue(()) => std::ops::ControlFlow::Continue(()),
                    std::ops::ControlFlow::Break(()) => std::ops::ControlFlow::Break(()),
                })
            },
            opts,
        )?)
    }
}
