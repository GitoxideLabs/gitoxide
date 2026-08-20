use gix_ref::{
    Category, FullName,
    transaction::{Change, PreviousValue, RefEdit, RefLog},
};

/// Delete local branches.
pub mod delete {
    use std::path::PathBuf;

    use gix_ref::FullName;

    /// A configuration-cleanup failure after all requested references were made absent.
    #[derive(Debug, thiserror::Error)]
    pub enum CleanupError {
        /// The updated configuration could not be written to its lock file; the existing config file is unchanged.
        #[error("Could not write the updated local configuration")]
        Write(#[source] std::io::Error),
        /// The lock file containing the updated configuration could not replace the existing config file.
        #[error("Could not commit the updated local configuration")]
        Commit(#[source] std::io::Error),
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
        #[error("Could not acquire the local configuration lock")]
        ConfigLock(#[source] gix_error::Error),
        #[error("Could not read the local configuration")]
        ConfigRead(#[source] gix_config::file::init::from_paths::Error),
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
    /// any name belongs to another reference category or is checked out in any worktree. Missing branches are accepted so any
    /// associated local configuration is still removed. **It deliberately performs no merged-state check**.
    ///
    /// On success, every requested reference and its reflog is absent, and every matching `branch.<name>` section has been
    /// removed from the local configuration. A requested reference which was already missing is treated as successfully absent,
    /// and its configuration is still removed.
    ///
    /// Reference deletion and configuration cleanup cannot be one atomic transaction. Once reference deletion succeeds, a
    /// configuration write or commit failure is returned as [`delete::Error::Cleanup`]. Its `references` field contains
    /// every requested name—including names which were missing initially—and guarantees only that their references and reflogs
    /// are absent. See [`delete::CleanupError`] to determine whether the on-disk configuration was updated.
    pub fn delete_local_branches(&mut self, names: impl IntoIterator<Item = FullName>) -> Result<(), delete::Error> {
        let mut names: Vec<_> = names.into_iter().collect();
        names.sort();
        names.dedup();
        if names.is_empty() {
            return Ok(());
        }

        for name in &names {
            if name.category_and_short_name().map(|(category, _)| category) != Some(Category::LocalBranch) {
                return Err(delete::Error::NotLocal { name: name.clone() });
            }
        }

        let checked_out = self.checked_out_branches().map_err(|err| match err {
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

        let edits: Vec<_> = names
            .iter()
            .map(|name| RefEdit {
                change: Change::Delete {
                    expected: PreviousValue::Any,
                    log: RefLog::AndReference,
                },
                name: name.clone(),
                deref: false,
            })
            .collect();

        let config_path = self.common_dir().join("config");
        let mut config_lock =
            gix_lock::File::acquire_to_update_resource(&config_path, gix_lock::acquire::Fail::Immediately, None)
                .map_err(|err| delete::Error::ConfigLock(err.into_error()))?;
        let mut config = match gix_config::File::from_path_no_includes(config_path.clone(), gix_config::Source::Local) {
            Ok(config) => Some(config),
            // TODO(gix-error): this is what should just be `err.not_found()` in future, anywhere.
            Err(gix_config::file::init::from_paths::Error::Io { source, .. })
                if source.kind() == std::io::ErrorKind::NotFound =>
            {
                None
            }
            Err(err) => return Err(delete::Error::ConfigRead(err)),
        };
        let removed_config = config
            .as_mut()
            .is_some_and(|config| remove_branch_config(config, &names, |_| true));

        self.edit_references(edits)?;

        if removed_config {
            let config = config.expect("configuration was present when sections were removed");
            config
                .write_to(&mut config_lock)
                .map_err(|source| delete::Error::Cleanup {
                    references: names.clone(),
                    source: delete::CleanupError::Write(source),
                })?;
            config_lock.commit().map_err(|err| delete::Error::Cleanup {
                references: names.clone(),
                source: delete::CleanupError::Commit(err.error),
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
        Ok(())
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
