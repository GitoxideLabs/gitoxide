use gix_ref::{
    Category, FullName, Target,
    transaction::{PreviousValue, RefEdit},
};

/// Delete local branches.
pub mod delete {
    use std::path::PathBuf;

    use gix_ref::FullName;

    /// A configuration-cleanup failure after all requested references were made absent.
    #[derive(Debug, thiserror::Error)]
    pub enum CleanupError {
        /// The updated configuration could not be written or committed; the existing config file is unchanged.
        #[error("Could not update the local configuration")]
        Config(#[source] crate::config::file_mut::Error),
    }

    /// The error returned by [`Repository::delete_local_branches()`][crate::Repository::delete_local_branches()].
    #[derive(Debug, thiserror::Error)]
    #[expect(missing_docs)]
    pub enum Error {
        #[error("{name:?} is not a local branch")]
        NotLocal { name: FullName },

        #[error("The local branch {name:?} is checked out in {worktree_dirs:?}")]
        CheckedOut {
            name: FullName,
            worktree_dirs: Vec<PathBuf>,
        },
        #[error("Failed to read or iterate worktree directories")]
        WorktreeListing(#[source] std::io::Error),
        #[error("Could not open a worktree repository")]
        OpenWorktreeRepo(#[source] crate::open::Error),
        #[error("Failed to follow a symbolic reference while inspecting worktrees")]
        FollowSymref(#[source] gix_ref::file::find::existing::Error),
        #[error("Could not open the local configuration transaction")]
        ConfigFile(#[source] crate::config::file_mut::Error),
        #[error("Could not delete local branches")]
        EditReferences(#[from] crate::reference::edit::Error),
        /// Reference deletion succeeded, but configuration cleanup failed.
        #[error("References {references:?} are absent, but local branch configuration cleanup failed")]
        Cleanup {
            /// Every requested reference name, including names which were already missing before the call.
            ///
            /// All of these references and their reflogs are guaranteed to be absent. Their `branch.<name>` configuration
            /// sections may remain; inspect `source` to determine which cleanup phase failed.
            references: Vec<FullName>,
            /// The branches actually deleted, as would have been returned on success.
            ///
            /// Names are sorted and deduplicated; branches missing when locked for deletion are excluded.
            deleted: Vec<FullName>,
            /// The configuration cleanup phase that failed.
            #[source]
            source: CleanupError,
        },
    }
}

impl crate::Repository {
    /// Delete all local branches in `names` and remove their `branch.<name>` sections from the local configuration.
    ///
    /// All names must be local branch references such as `refs/heads/topic`. The operation fails before making changes if
    /// any name belongs to another reference category or is checked out or reserved by bisect or rebase in any worktree.
    /// Missing branches are accepted so any associated local configuration is still removed.
    /// **It deliberately performs no merged-state check**.
    ///
    /// On success, every requested reference and its reflog is absent, and every matching `branch.<name>` section has been
    /// removed from the local configuration. Return the sorted, deduplicated names of branches that existed when locked for
    /// deletion. Missing branches are omitted from the returned vector, but their configuration is still removed.
    ///
    /// Reference deletion and configuration cleanup cannot be one atomic transaction. Once reference deletion succeeds, a
    /// configuration write or commit failure is returned as [`delete::Error::Cleanup`]. Its `references` field contains
    /// every requested name—including names which were missing initially—and guarantees only that their references and reflogs
    /// are absent. Its `deleted` field contains the branches actually deleted, just as in the success case.
    /// See [`delete::CleanupError`] to determine whether the on-disk configuration was updated.
    pub fn delete_local_branches(
        &mut self,
        names: impl IntoIterator<Item = FullName>,
    ) -> Result<Vec<FullName>, delete::Error> {
        self.delete_local_branches_inner(names.into_iter().map(|name| (name, PreviousValue::Any)).collect())
    }

    /// Delete local branches only if they still have the observed `target`, and remove their local configuration.
    ///
    /// This performs the same reference and configuration cleanup as
    /// [`Repository::delete_local_branches()`][crate::Repository::delete_local_branches()],
    /// but protects a branch which was moved or replaced after the caller inspected it.
    pub fn delete_local_branches_if_unchanged(
        &mut self,
        branches: impl IntoIterator<Item = (FullName, Target)>,
    ) -> Result<(), delete::Error> {
        self.delete_local_branches_inner(
            branches
                .into_iter()
                .map(|(name, target)| (name, PreviousValue::MustExistAndMatch(target)))
                .collect(),
        )
        .map(|_| ())
    }

    fn delete_local_branches_inner(
        &mut self,
        mut branches: Vec<(FullName, PreviousValue)>,
    ) -> Result<Vec<FullName>, delete::Error> {
        branches.sort_by(|a, b| a.0.cmp(&b.0));
        branches.dedup_by(|a, b| a.0 == b.0);
        let names = branches.iter().map(|(name, _)| name.clone()).collect::<Vec<_>>();
        if names.is_empty() {
            return Ok(names);
        }

        for name in &names {
            if name.category_and_short_name().map(|(category, _)| category) != Some(Category::LocalBranch) {
                return Err(delete::Error::NotLocal { name: name.clone() });
            }
        }

        let checked_out = self.checked_out_branches(self.namespace()).map_err(|err| match err {
            super::worktree::CheckedOutBranchesError::WorktreeListing(err) => delete::Error::WorktreeListing(err),
            super::worktree::CheckedOutBranchesError::OpenWorktreeRepo(err) => delete::Error::OpenWorktreeRepo(err),
            super::worktree::CheckedOutBranchesError::FollowSymref(err) => delete::Error::FollowSymref(err),
        })?;
        for name in &names {
            if let Some(worktree_dirs) = checked_out.get(name) {
                return Err(delete::Error::CheckedOut {
                    name: name.clone(),
                    worktree_dirs: worktree_dirs.clone(),
                });
            }
        }

        let edits: Vec<_> = branches
            .into_iter()
            .map(|(name, expected)| RefEdit::delete(name, expected))
            .collect();

        let config_path = self.common_dir().join("config");
        let mut config = self.config_file_mut(&config_path).map_err(delete::Error::ConfigFile)?;
        let removed_config = remove_branch_config(&mut config, &names, |_| true);

        let deleted: Vec<_> = self
            .edit_references(edits)?
            .into_iter()
            .filter_map(|edit| edit.change.previous_value().is_some().then_some(edit.name))
            .collect();

        if removed_config {
            config.commit().map_err(|err| delete::Error::Cleanup {
                references: names.clone(),
                deleted: deleted.clone(),
                source: delete::CleanupError::Config(err),
            })?;
            remove_branch_config(
                gix_features::threading::OwnShared::make_mut(&mut self.config.resolved),
                &names,
                |meta| {
                    meta.source == gix_config::Source::Local
                        && meta.level == 0
                        && meta.path.as_deref() == Some(config_path.as_path())
                },
            );
        }
        Ok(deleted)
    }
}

fn remove_branch_config(
    config: &mut gix_config::File,
    names: &[FullName],
    mut filter: impl FnMut(&gix_config::file::Metadata) -> bool,
) -> bool {
    let section_ids: Vec<_> = config
        .sections_and_ids_by_name("branch")
        .into_iter()
        .flatten()
        .filter_map(|(section, id)| {
            if !filter(section.meta()) {
                return None;
            }
            let subsection = section.header().subsection_name()?;
            names
                .iter()
                .any(|name| {
                    name.category_and_short_name()
                        .is_some_and(|(_, short)| short == subsection)
                })
                .then_some(id)
        })
        .collect();
    let removed = !section_ids.is_empty();
    for id in section_ids {
        config.remove_section_by_id(id);
    }
    removed
}
