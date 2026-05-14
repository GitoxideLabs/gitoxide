use gix_pathspec::MagicSignature;

use crate::{AttributeStack, Pathspec, Repository, bstr::BStr, config::cache::util::ApplyLeniencyDefault};

impl Repository {
    /// Create a new pathspec abstraction that allows to conduct searches using `patterns`.
    /// `inherit_ignore_case` should be `true` if `patterns` will match against files on disk, or `false` otherwise, for more natural matching
    /// (but also note that `git` does not do that).
    /// `index` may be needed to load attributes which is required only if `patterns` refer to attributes via `:(attr:…)` syntax.
    /// In the same vein, `attributes_source` affects where `.gitattributes` files are read from if pathspecs need to match against attributes.
    /// If `empty_patterns_match_prefix` is `true`, then even empty patterns will match only what's inside of the prefix. Otherwise
    /// they will match everything.
    ///
    /// It will be initialized exactly how it would, and attribute matching will be conducted by reading the worktree first if available.
    /// If that is not desirable, consider calling [`Pathspec::new()`] directly.
    #[doc(alias = "Pathspec", alias = "git2")]
    pub fn pathspec(
        &self,
        empty_patterns_match_prefix: bool,
        patterns: impl IntoIterator<Item = impl AsRef<BStr>>,
        inherit_ignore_case: bool,
        index: &gix_index::State,
        attributes_source: gix_worktree::stack::state::attributes::Source,
    ) -> Result<Pathspec<'_>, crate::pathspec::init::Error> {
        Pathspec::new(self, empty_patterns_match_prefix, patterns, inherit_ignore_case, || {
            self.attributes_only(index, attributes_source)
                .map(AttributeStack::detach)
                .map_err(Into::into)
        })
    }

    /// Return default settings that are required when [parsing pathspecs](gix_pathspec::parse()) by hand.
    ///
    /// These are stemming from environment variables which have been converted to [config settings](crate::config::tree::gitoxide::Pathspec),
    /// which now serve as authority for configuration.
    pub fn pathspec_defaults(&self) -> Result<gix_pathspec::Defaults, gix_pathspec::defaults::from_environment::Error> {
        self.config.pathspec_defaults()
    }

    /// Similar to [Self::pathspec_defaults()], but will automatically configure the returned defaults to match case-insensitively if the underlying
    /// filesystem is also configured to be case-insensitive according to `core.ignoreCase`, and `inherit_ignore_case` is `true`.
    pub fn pathspec_defaults_inherit_ignore_case(
        &self,
        inherit_ignore_case: bool,
    ) -> Result<gix_pathspec::Defaults, crate::repository::pathspec_defaults_ignore_case::Error> {
        let mut defaults = self.config.pathspec_defaults()?;
        if inherit_ignore_case
            && self
                .config
                .fs_capabilities()
                .with_lenient_default(self.config.lenient_config)?
                .ignore_case
        {
            defaults.signature |= MagicSignature::ICASE;
        }
        Ok(defaults)
    }
}
