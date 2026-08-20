use gix_error::{ErrorExt, ResultExt};
use gix_ref::{
    Category, FullName,
    transaction::{PreviousValue, RefEdit},
};

/// Delete local branches.
pub mod delete {
    use gix_ref::FullName;

    /// A configuration-cleanup failure after all requested references were made absent.
    #[derive(Debug)]
    pub struct CleanupError {
        /// Every requested reference name, including names which were already missing before the call.
        ///
        /// All of these references and their reflogs are guaranteed to be absent. Their `branch.<name>` configuration
        /// sections may remain.
        pub references: Vec<FullName>,
    }

    impl std::fmt::Display for CleanupError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "References {:?} are absent, but local branch configuration cleanup failed",
                self.references
            )
        }
    }

    impl std::error::Error for CleanupError {}

    /// The error returned by [`Repository::delete_local_branches()`][crate::Repository::delete_local_branches()].
    pub type Error = gix_error::Error;
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
    /// configuration write or commit failure contains [`delete::CleanupError`] in its error chain. Its `references` field
    /// contains every requested name—including names which were missing initially—and guarantees only that their references and
    /// reflogs are absent. The remaining error chain identifies the failed cleanup phase.
    pub fn delete_local_branches(&mut self, names: impl IntoIterator<Item = FullName>) -> Result<(), delete::Error> {
        let mut names: Vec<_> = names.into_iter().collect();
        names.sort();
        names.dedup();
        if names.is_empty() {
            return Ok(());
        }

        for name in &names {
            if name.category_and_short_name().map(|(category, _)| category) != Some(Category::LocalBranch) {
                return Err(gix_error::message!("{name:?} is not a local branch").raise().into());
            }
        }

        let checked_out = self.checked_out_branches()?;
        for name in &names {
            if let Some(worktree_dirs) = checked_out.get(name) {
                return Err(
                    gix_error::message!("The local branch {name:?} is checked out in {worktree_dirs:?}")
                        .raise()
                        .into(),
                );
            }
        }

        let edits: Vec<_> = names
            .iter()
            .map(|name| RefEdit::delete(name.clone(), PreviousValue::Any))
            .collect();

        let config_path = self.common_dir().join("config");
        let mut config_lock =
            gix_lock::File::acquire_to_update_resource(&config_path, gix_lock::acquire::Fail::Immediately, None)
                .or_raise(|| gix_error::message("Could not acquire the local configuration lock"))?;
        let mut config = match gix_config::File::from_path_no_includes(config_path.clone(), gix_config::Source::Local) {
            Ok(config) => Some(config),
            Err(err)
                if err
                    .downcast_any_ref::<std::io::Error>()
                    .is_some_and(|source| source.kind() == std::io::ErrorKind::NotFound) =>
            {
                None
            }
            Err(err) => {
                return Err(err
                    .raise(gix_error::message("Could not read the local configuration"))
                    .into());
            }
        };
        let removed_config = config
            .as_mut()
            .is_some_and(|config| remove_branch_config(config, &names, |_| true));

        self.edit_references(edits)
            .or_raise(|| gix_error::message("Could not delete local branches"))?;

        if removed_config {
            let config = config.expect("configuration was present when sections were removed");
            config
                .write_to(&mut config_lock)
                .or_raise(|| gix_error::message("Could not write the updated local configuration"))
                .or_raise(|| delete::CleanupError {
                    references: names.clone(),
                })?;
            config_lock
                .commit()
                .or_raise(|| gix_error::message("Could not commit the updated local configuration"))
                .or_raise(|| delete::CleanupError {
                    references: names.clone(),
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
