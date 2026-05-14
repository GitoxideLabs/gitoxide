use std::borrow::Cow;

use gix_merge::blob::builtin_driver::text;
use gix_object::Write;

use crate::{
    Repository,
    config::{cache::util::ApplyLeniencyDefault, tree},
    prelude::ObjectIdExt,
    repository::{
        blob_merge_options, merge_commits, merge_resource_cache, merge_trees, tree_merge_options, virtual_merge_base,
        virtual_merge_base_with_graph,
    },
};

/// Merge-utilities
impl Repository {
    /// Create a resource cache that can hold the three resources needed for a three-way merge. `worktree_roots`
    /// determines which side of the merge is read from the worktree, or from which worktree.
    ///
    /// The platform can be used to set up resources and finally perform a merge among blobs.
    ///
    /// Note that the current index is used for attribute queries.
    pub fn merge_resource_cache(
        &self,
        worktree_roots: gix_merge::blob::pipeline::WorktreeRoots,
    ) -> Result<gix_merge::blob::Platform, merge_resource_cache::Error> {
        let index = self.index_or_load_from_head_or_empty()?;
        let mode = {
            let renormalize = self
                .config
                .resolved
                .boolean(tree::Merge::RENORMALIZE)
                .map(|res| {
                    tree::Merge::RENORMALIZE
                        .enrich_error(res)
                        .with_lenient_default(self.config.lenient_config)
                })
                .transpose()?
                .unwrap_or_default();
            if renormalize {
                gix_merge::blob::pipeline::Mode::Renormalize
            } else {
                gix_merge::blob::pipeline::Mode::ToGit
            }
        };
        let attrs = self
            .attributes_only(
                &index,
                if worktree_roots.is_unset() {
                    gix_worktree::stack::state::attributes::Source::IdMapping
                } else {
                    gix_worktree::stack::state::attributes::Source::WorktreeThenIdMapping
                },
            )?
            .inner;
        let filter = gix_filter::Pipeline::new(self.command_context()?, crate::filter::Pipeline::options(self)?);
        let filter = gix_merge::blob::Pipeline::new(worktree_roots, filter, self.config.merge_pipeline_options()?);
        let options = gix_merge::blob::platform::Options {
            default_driver: self.config.resolved.string(tree::Merge::DEFAULT).map(Cow::into_owned),
        };
        let drivers = self.config.merge_drivers()?;
        Ok(gix_merge::blob::Platform::new(filter, mode, attrs, drivers, options))
    }

    /// Return options for use with [`gix_merge::blob::PlatformRef::merge()`], accessible through
    /// [merge_resource_cache()](Self::merge_resource_cache).
    pub fn blob_merge_options(&self) -> Result<gix_merge::blob::platform::merge::Options, blob_merge_options::Error> {
        Ok(gix_merge::blob::platform::merge::Options {
            is_virtual_ancestor: false,
            resolve_binary_with: None,
            text: gix_merge::blob::builtin_driver::text::Options {
                diff_algorithm: self.diff_algorithm()?,
                conflict: text::Conflict::Keep {
                    style: self
                        .config
                        .resolved
                        .string(tree::Merge::CONFLICT_STYLE)
                        .map(|value| {
                            tree::Merge::CONFLICT_STYLE
                                .try_into_conflict_style(value)
                                .with_lenient_default(self.config.lenient_config)
                        })
                        .transpose()?
                        .unwrap_or_default(),
                    marker_size: text::Conflict::DEFAULT_MARKER_SIZE.try_into().unwrap(),
                },
            },
        })
    }

    /// Read all relevant configuration options to instantiate options for use in [`merge_trees()`](Self::merge_trees).
    pub fn tree_merge_options(&self) -> Result<crate::merge::tree::Options, tree_merge_options::Error> {
        let (mut rewrites, mut is_configured) = crate::diff::utils::new_rewrites_inner(
            &self.config.resolved,
            self.config.lenient_config,
            &tree::Merge::RENAMES,
            &tree::Merge::RENAME_LIMIT,
        )?;
        if !is_configured {
            (rewrites, is_configured) =
                crate::diff::utils::new_rewrites(&self.config.resolved, self.config.lenient_config)?;
        }
        if !is_configured {
            rewrites = Some(Default::default());
        }
        Ok(gix_merge::tree::Options {
            rewrites,
            blob_merge: self.blob_merge_options()?,
            blob_merge_command_ctx: self.command_context()?,
            fail_on_conflict: None,
            marker_size_multiplier: 0,
            symlink_conflicts: None,
            tree_conflicts: None,
        }
        .into())
    }

    /// Merge `our_tree` and `their_tree` together, assuming they have the same `ancestor_tree`, to yield a new tree
    /// which is provided as [tree editor](crate::object::tree::Editor) to inspect and finalize results at will.
    /// No change to the worktree or index is made, but objects may be written to the object database as merge results
    /// are stored.
    /// If these changes should not be observable outside of this instance, consider [enabling object memory](Self::with_object_memory).
    ///
    /// Note that `ancestor_tree` can be the [empty tree hash](gix_hash::ObjectId::empty_tree) to indicate no common ancestry.
    ///
    /// `labels` are typically chosen to identify the refs or names for `our_tree` and `their_tree` and `ancestor_tree` respectively.
    ///
    /// `options` should be initialized with [`tree_merge_options()`](Self::tree_merge_options()).
    ///
    /// ### Performance
    ///
    /// It's highly recommended to [set an object cache](Repository::compute_object_cache_size_for_tree_diffs)
    /// to avoid extracting the same object multiple times.
    pub fn merge_trees(
        &self,
        ancestor_tree: impl AsRef<gix_hash::oid>,
        our_tree: impl AsRef<gix_hash::oid>,
        their_tree: impl AsRef<gix_hash::oid>,
        labels: gix_merge::blob::builtin_driver::text::Labels<'_>,
        options: crate::merge::tree::Options,
    ) -> Result<crate::merge::tree::Outcome<'_>, merge_trees::Error> {
        let mut diff_cache = self.diff_resource_cache_for_tree_diff()?;
        let mut blob_merge = self.merge_resource_cache(Default::default())?;
        let gix_merge::tree::Outcome {
            tree,
            conflicts,
            failed_on_first_unresolved_conflict,
        } = gix_merge::tree(
            ancestor_tree.as_ref(),
            our_tree.as_ref(),
            their_tree.as_ref(),
            labels,
            self,
            |buf| self.write_buf(gix_object::Kind::Blob, buf),
            &mut Default::default(),
            &mut diff_cache,
            &mut blob_merge,
            options.into(),
        )?;

        let validate = self.config.protect_options()?;
        Ok(crate::merge::tree::Outcome {
            tree: crate::object::tree::Editor {
                inner: tree,
                validate,
                repo: self,
            },
            conflicts,
            failed_on_first_unresolved_conflict,
        })
    }

    /// Merge `our_commit` and `their_commit` together to yield a new tree which is provided as [tree editor](crate::object::tree::Editor)
    /// to inspect and finalize results at will. The merge-base will be determined automatically between both commits, along with special
    /// handling in case there are multiple merge-bases.
    /// No change to the worktree or index is made, but objects may be written to the object database as merge results
    /// are stored.
    /// If these changes should not be observable outside of this instance, consider [enabling object memory](Self::with_object_memory).
    ///
    /// `labels` are typically chosen to identify the refs or names for `our_commit` and `their_commit`, with the ancestor being set
    /// automatically as part of the merge-base handling.
    ///
    /// `options` should be initialized with [`Repository::tree_merge_options().into()`](Self::tree_merge_options()).
    ///
    /// ### Performance
    ///
    /// It's highly recommended to [set an object cache](Repository::compute_object_cache_size_for_tree_diffs)
    /// to avoid extracting the same object multiple times.
    pub fn merge_commits(
        &self,
        our_commit: impl Into<gix_hash::ObjectId>,
        their_commit: impl Into<gix_hash::ObjectId>,
        labels: gix_merge::blob::builtin_driver::text::Labels<'_>,
        options: crate::merge::commit::Options,
    ) -> Result<crate::merge::commit::Outcome<'_>, merge_commits::Error> {
        let mut diff_cache = self.diff_resource_cache_for_tree_diff()?;
        let mut blob_merge = self.merge_resource_cache(Default::default())?;
        let commit_graph = self.commit_graph_if_enabled()?;
        let mut graph = self.revision_graph(commit_graph.as_ref());
        let gix_merge::commit::Outcome {
            tree_merge:
                gix_merge::tree::Outcome {
                    tree,
                    conflicts,
                    failed_on_first_unresolved_conflict,
                },
            merge_base_tree_id,
            merge_bases,
            virtual_merge_bases,
        } = gix_merge::commit(
            our_commit.into(),
            their_commit.into(),
            labels,
            &mut graph,
            &mut diff_cache,
            &mut blob_merge,
            self,
            &mut |id| id.to_owned().attach(self).shorten_or_id().to_string(),
            options.into(),
        )?;

        let validate = self.config.protect_options()?;
        let tree_merge = crate::merge::tree::Outcome {
            tree: crate::object::tree::Editor {
                inner: tree,
                validate,
                repo: self,
            },
            conflicts,
            failed_on_first_unresolved_conflict,
        };
        Ok(crate::merge::commit::Outcome {
            tree_merge,
            merge_base_tree_id,
            merge_bases,
            virtual_merge_bases,
        })
    }

    /// Create a single virtual merge-base by merging all `merge_bases` into one.
    /// If the list is empty, an error will be returned as the histories are then unrelated.
    /// If there is only one commit in the list, it is returned directly with this case clearly marked in the outcome.
    ///
    /// Note that most of `options` are overwritten to match the requirements of a merge-base merge, but they can be useful
    /// to control the diff algorithm or rewrite tracking, for example.
    ///
    /// This method is useful in conjunction with [`Self::merge_trees()`], as the ancestor tree can be produced here.
    // TODO: test
    pub fn virtual_merge_base(
        &self,
        merge_bases: impl IntoIterator<Item = impl Into<gix_hash::ObjectId>>,
        options: crate::merge::tree::Options,
    ) -> Result<crate::merge::virtual_merge_base::Outcome<'_>, virtual_merge_base::Error> {
        let commit_graph = self.commit_graph_if_enabled()?;
        let mut graph = self.revision_graph(commit_graph.as_ref());
        Ok(self.virtual_merge_base_with_graph(merge_bases, &mut graph, options)?)
    }

    /// Like [`Self::virtual_merge_base()`], but also allows to reuse a `graph` for faster merge-base calculation,
    /// particularly if `graph` was used to find the `merge_bases`.
    pub fn virtual_merge_base_with_graph(
        &self,
        merge_bases: impl IntoIterator<Item = impl Into<gix_hash::ObjectId>>,
        graph: &mut gix_revwalk::Graph<'_, '_, gix_revwalk::graph::Commit<gix_revision::merge_base::Flags>>,
        options: crate::merge::tree::Options,
    ) -> Result<crate::merge::virtual_merge_base::Outcome<'_>, virtual_merge_base_with_graph::Error> {
        let mut merge_bases: Vec<_> = merge_bases.into_iter().map(Into::into).collect();
        let first = merge_bases
            .pop()
            .ok_or(virtual_merge_base_with_graph::Error::MissingCommit)?;
        let Some(second) = merge_bases.pop() else {
            let tree_id = self.find_commit(first)?.tree_id()?;
            let commit_id = first.attach(self);
            return Ok(crate::merge::virtual_merge_base::Outcome {
                virtual_merge_bases: Vec::new(),
                commit_id,
                tree_id,
            });
        };

        let mut diff_cache = self.diff_resource_cache_for_tree_diff()?;
        let mut blob_merge = self.merge_resource_cache(Default::default())?;

        let gix_merge::commit::virtual_merge_base::Outcome {
            virtual_merge_bases,
            commit_id,
            tree_id,
        } = gix_merge::commit::virtual_merge_base(
            first,
            second,
            merge_bases,
            graph,
            &mut diff_cache,
            &mut blob_merge,
            self,
            &mut |id| id.to_owned().attach(self).shorten_or_id().to_string(),
            options.into(),
        )?;

        Ok(crate::merge::virtual_merge_base::Outcome {
            virtual_merge_bases: virtual_merge_bases.into_iter().map(|id| id.attach(self)).collect(),
            commit_id: commit_id.attach(self),
            tree_id: tree_id.attach(self),
        })
    }
}
