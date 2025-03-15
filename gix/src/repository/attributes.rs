//! exclude information
use crate::{config, AttributeStack, Repository};

/// The error returned by [`Repository::attributes()`].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    ConfigureAttributes(#[from] config::attribute_stack::Error),
    #[error(transparent)]
    ConfigureExcludes(#[from] config::exclude_stack::Error),
}

impl Repository {
    /// Configure a file-system cache for accessing git attributes *and* excludes on a per-path basis.
    ///
    /// Use `attribute_source` to specify where to read attributes from. Also note that exclude information will
    /// always try to read `.gitignore` files from disk before trying to read it from the `index`.
    ///
    /// Note that no worktree is required for this to work, even though access to in-tree `.gitattributes` and `.gitignore` files
    /// would require a non-empty `index` that represents a git tree.
    ///
    /// This takes into consideration all the usual repository configuration, namely:
    ///
    /// * `$XDG_CONFIG_HOME/…/ignore|attributes` if `core.excludesFile|attributesFile` is *not* set, otherwise use the configured file.
    /// * `$GIT_DIR/info/exclude|attributes` if present.
    #[cfg(feature = "attributes")]
    pub fn attributes(
        &self,
        index: &gix_index::State,
        attributes_source: gix_worktree::stack::state::attributes::Source,
        ignore_source: gix_worktree::stack::state::ignore::Source,
        exclude_overrides: Option<gix_ignore::Search>,
    ) -> Result<AttributeStack<'_>, Error> {
        let case = if self.config.ignore_case {
            gix_glob::pattern::Case::Fold
        } else {
            gix_glob::pattern::Case::Sensitive
        };
        let (attributes, mut buf) = self.config.assemble_attribute_globals(
            self.git_dir(),
            attributes_source,
            self.options.permissions.attributes,
        )?;
        let ignore =
            self.config
                .assemble_exclude_globals(self.git_dir(), exclude_overrides, ignore_source, &mut buf)?;
        let state = gix_worktree::stack::State::AttributesAndIgnoreStack { attributes, ignore };
        let attribute_list = state.id_mappings_from_index(index, index.path_backing(), case);
        Ok(AttributeStack::new(
            gix_worktree::Stack::new(
                // this is alright as we don't cause mutation of that directory, it's virtual.
                self.workdir().unwrap_or(self.git_dir()),
                state,
                case,
                buf,
                attribute_list,
            ),
            self,
        ))
    }

    /// Like [attributes()][Self::attributes()], but without access to exclude/ignore information.
    #[cfg(feature = "attributes")]
    pub fn attributes_only(
        &self,
        index: &gix_index::State,
        attributes_source: gix_worktree::stack::state::attributes::Source,
    ) -> Result<AttributeStack<'_>, config::attribute_stack::Error> {
        let case = if self.config.ignore_case {
            gix_glob::pattern::Case::Fold
        } else {
            gix_glob::pattern::Case::Sensitive
        };
        let (attributes, buf) = self.config.assemble_attribute_globals(
            self.git_dir(),
            attributes_source,
            self.options.permissions.attributes,
        )?;
        let state = gix_worktree::stack::State::AttributesStack(attributes);
        let attribute_list = state.id_mappings_from_index(index, index.path_backing(), case);
        Ok(AttributeStack::new(
            gix_worktree::Stack::new(
                // this is alright as we don't cause mutation of that directory, it's virtual.
                self.workdir().unwrap_or(self.git_dir()),
                state,
                case,
                buf,
                attribute_list,
            ),
            self,
        ))
    }

    /// Configure a file-system cache checking if files below the repository are excluded, reading `.gitignore` files from
    /// the specified `source`.
    ///
    /// Note that no worktree is required for this to work, even though access to in-tree `.gitignore` files would require
    /// a non-empty `index` that represents a tree with `.gitignore` files.
    ///
    /// This takes into consideration all the usual repository configuration, namely:
    ///
    /// * `$XDG_CONFIG_HOME/…/ignore` if `core.excludesFile` is *not* set, otherwise use the configured file.
    /// * `$GIT_DIR/info/exclude` if present.
    ///
    /// When only excludes are desired, this is the most efficient way to obtain them. Otherwise use
    /// [`Repository::attributes()`] for accessing both attributes and excludes.
    #[doc(alias = "is_path_ignored", alias = "git2")]
    #[cfg(feature = "excludes")]
    pub fn excludes(
        &self,
        index: &gix_index::State,
        overrides: Option<gix_ignore::Search>,
        source: gix_worktree::stack::state::ignore::Source,
    ) -> Result<AttributeStack<'_>, config::exclude_stack::Error> {
        let case = if self.config.ignore_case {
            gix_glob::pattern::Case::Fold
        } else {
            gix_glob::pattern::Case::Sensitive
        };
        let mut buf = Vec::with_capacity(512);
        let ignore = self
            .config
            .assemble_exclude_globals(self.git_dir(), overrides, source, &mut buf)?;
        let state = gix_worktree::stack::State::IgnoreStack(ignore);
        let attribute_list = state.id_mappings_from_index(index, index.path_backing(), case);
        Ok(AttributeStack::new(
            gix_worktree::Stack::new(
                // this is alright as we don't cause mutation of that directory, it's virtual.
                self.workdir().unwrap_or(self.git_dir()),
                state,
                case,
                buf,
                attribute_list,
            ),
            self,
        ))
    }
}
