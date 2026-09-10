use crate::{
    config,
    config::tree::{Key, Section, Worktree, keys},
};

impl Worktree {
    /// The `worktree.useRelativePaths` key, defaulting to false.
    pub const USE_RELATIVE_PATHS: keys::Boolean =
        keys::Boolean::new_boolean("useRelativePaths", &config::Tree::WORKTREE);
}

impl Section for Worktree {
    fn name(&self) -> &str {
        "worktree"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::USE_RELATIVE_PATHS]
    }
}
