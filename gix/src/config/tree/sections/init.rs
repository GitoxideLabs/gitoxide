use crate::{
    config,
    config::tree::{Init, Key, Section, keys},
};

impl Init {
    /// The `init.defaultBranch` key.
    // TODO: review its usage for cases where this key is not set - sometimes it's 'master', sometimes it's 'main'.
    pub const DEFAULT_BRANCH: keys::Any = keys::Any::new("defaultBranch", &config::Tree::INIT)
        .with_deviation("If not set, we use `main` instead of `master`");
}

impl Section for Init {
    fn name(&self) -> &str {
        "init"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::DEFAULT_BRANCH]
    }
}
