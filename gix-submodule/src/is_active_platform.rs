use bstr::BStr;

use crate::IsActivePlatform;

impl IsActivePlatform {
    /// Returns `true` if the submodule named `name` is active or `false` otherwise.
    /// `config` is the configuration that was passed to the originating [modules file](crate::File).
    /// `attributes(relative_path, case, is_dir, outcome)` provides a way to resolve the attributes mentioned
    /// in `submodule.active` pathspecs that are evaluated in the platforms git configuration.
    ///
    /// A submodule's active state is determined in the following order
    ///
    /// * it's `submodule.<name>.active` is set in `config`
    /// * it matches a `submodule.active` pathspec either positively or negatively via `:!<spec>`
    /// * it's active if it has any `url` set in `config`
    pub fn is_active(
        &mut self,
        config: &gix_config::File,
        name: &BStr,
        attributes: &mut dyn FnMut(
            &BStr,
            gix_pathspec::attributes::glob::pattern::Case,
            bool,
            &mut gix_pathspec::attributes::search::Outcome,
        ) -> bool,
    ) -> Result<bool, gix_error::Exn<gix_error::ValidationError>> {
        if let Some(val) = config.boolean(&format!("submodule.{name}.active"))? {
            return Ok(val);
        }
        if let Some(val) = self.search.as_mut().map(|search| {
            search
                .pattern_matching_relative_path(name, Some(true), attributes)
                .is_some_and(|m| !m.is_excluded())
        }) {
            return Ok(val);
        }
        Ok(config.string(&format!("submodule.{name}.url")).is_some())
    }
}
