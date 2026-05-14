use crate::{Repository, config::tree};

/// The error returned by [Repository::tree_index_status()].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    IndexFromMTree(#[from] crate::repository::index_from_tree::Error),
    #[error(transparent)]
    RewritesConfiguration(#[from] crate::diff::new_rewrites::Error),
    #[error("Could not create diff-cache for similarity checks")]
    DiffResourceCache(#[from] crate::repository::diff_resource_cache::Error),
    #[error(transparent)]
    TreeIndexDiff(#[from] gix_diff::index::Error),
}

/// Specify how to perform rewrite tracking [Repository::tree_index_status()].
#[derive(Default, Debug, Copy, Clone)]
pub enum TrackRenames {
    /// Check `status.renames` and then `diff.renames` if the former isn't set. Otherwise, default to performing rewrites if nothing
    /// is set.
    #[default]
    AsConfigured,
    /// Track renames according ot the given configuration.
    Given(gix_diff::Rewrites),
    /// Do not track renames.
    Disabled,
}

/// The outcome of [Repository::tree_index_status()].
#[derive(Clone)]
pub struct Outcome {
    /// Additional information produced by the rename tracker.
    ///
    /// It may be `None` if rename tracking was disabled.
    pub rewrite: Option<gix_diff::rewrites::Outcome>,
    /// The index produced from the input `tree` for the purpose of diffing.
    ///
    /// At some point this might go away once it's possible to diff an index from a tree directly.
    pub tree_index: gix_index::State,
}

impl Repository {
    /// Produce the `git status` portion that shows the difference between `tree_id` (usually `HEAD^{tree}`) and the `worktree_index`
    /// (typically the current `.git/index`), and pass all changes to `cb(change, tree_index, worktree_index)` with
    /// full access to both indices that contributed to the change.
    ///
    /// *(It's notable that internally, the `tree_id` is converted into an index before diffing these)*.
    /// Set `pathspec` to `Some(_)` to further reduce the set of files to check.
    ///
    /// ### Notes
    ///
    /// * This is a low-level method - prefer the [`Repository::status()`] platform instead for access to various iterators
    ///   over the same information.
    pub fn tree_index_status<'repo, E>(
        &'repo self,
        tree_id: &gix_hash::oid,
        worktree_index: &gix_index::State,
        pathspec: Option<&mut crate::Pathspec<'repo>>,
        renames: TrackRenames,
        mut cb: impl FnMut(
            gix_diff::index::ChangeRef<'_, '_>,
            &gix_index::State,
            &gix_index::State,
        ) -> Result<gix_diff::index::Action, E>,
    ) -> Result<Outcome, Error>
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        let _span = gix_trace::coarse!("gix::tree_index_status");
        let tree_index: gix_index::State = self.index_from_tree(tree_id)?.into();
        let rewrites = match renames {
            TrackRenames::AsConfigured => {
                let (mut rewrites, mut is_configured) = crate::diff::utils::new_rewrites_inner(
                    &self.config.resolved,
                    self.config.lenient_config,
                    &tree::Status::RENAMES,
                    &tree::Status::RENAME_LIMIT,
                )?;
                if !is_configured {
                    (rewrites, is_configured) =
                        crate::diff::utils::new_rewrites(&self.config.resolved, self.config.lenient_config)?;
                }
                if !is_configured {
                    rewrites = Some(Default::default());
                }
                rewrites
            }
            TrackRenames::Given(rewrites) => Some(rewrites),
            TrackRenames::Disabled => None,
        };
        let mut resource_cache = None;
        if rewrites.is_some() {
            resource_cache = Some(self.diff_resource_cache_for_tree_diff()?);
        }
        let mut pathspec_storage = None;
        if pathspec.is_none() {
            pathspec_storage = self
                .pathspec(
                    true,
                    None::<&str>,
                    false,
                    &gix_index::State::new(self.object_hash()),
                    gix_worktree::stack::state::attributes::Source::IdMapping,
                )
                .expect("Impossible for this to fail without patterns")
                .into();
        }

        let pathspec =
            pathspec.unwrap_or_else(|| pathspec_storage.as_mut().expect("set if pathspec isn't set by user"));
        let rewrite = gix_diff::index(
            &tree_index,
            worktree_index,
            |change| cb(change, &tree_index, worktree_index),
            rewrites
                .zip(resource_cache.as_mut())
                .map(|(rewrites, resource_cache)| gix_diff::index::RewriteOptions {
                    resource_cache,
                    find: self,
                    rewrites,
                }),
            &mut pathspec.search,
            &mut |relative_path, case, is_dir, out| {
                let stack = pathspec.stack.as_mut().expect("initialized in advance");
                stack
                    .set_case(case)
                    .at_entry(
                        relative_path,
                        Some(crate::pathspec::is_dir_to_mode(is_dir)),
                        &pathspec.repo.objects,
                    )
                    .is_ok_and(|platform| platform.matching_attributes(out))
            },
        )?;

        Ok(Outcome { rewrite, tree_index })
    }
}
