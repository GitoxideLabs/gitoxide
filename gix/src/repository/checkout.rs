use crate::{config, Repository};

impl Repository {
    /// Return options that can be used to drive a low-level checkout operation.
    /// Use `attributes_source` to determine where `.gitattributes` files should be read from, which depends on
    /// the presence of a worktree to begin with.
    /// Here, typically this value would be [`gix_worktree::stack::state::attributes::Source::IdMapping`]
    pub fn checkout_options(
        &self,
        attributes_source: gix_worktree::stack::state::attributes::Source,
    ) -> Result<gix_worktree_state::checkout::Options, config::checkout_options::Error> {
        self.config.checkout_options(self, attributes_source)
    }
}
