use crate::{Repository, Result};

impl Repository {
    /// Return options that can be used to drive a low-level checkout operation.
    /// Use `attributes_source` to determine where `.gitattributes` files should be read from, which depends on
    /// the presence of a worktree to begin with.
    /// Here, typically this value would be [`gix_worktree::stack::state::attributes::Source::IdMapping`].
    ///
    /// Resolving these options may create and remove a temporary symlink in the worktree to probe symlink
    /// support; see [`Self::filesystem_options()`] for when this happens.
    pub fn checkout_options(
        &self,
        attributes_source: gix_worktree::stack::state::attributes::Source,
    ) -> Result<gix_worktree_state::checkout::Options> {
        self.config.checkout_options(self, attributes_source)
    }
}
