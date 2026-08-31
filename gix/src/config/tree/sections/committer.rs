use crate::{
    config,
    config::tree::{Committer, Key, Section, keys},
};

impl Committer {
    /// The `committer.name` key, overridden by `GIT_COMMITTER_NAME` when resolving a committer.
    pub const NAME: keys::Any =
        keys::Any::new("name", &config::Tree::COMMITTER).with_environment_override("GIT_COMMITTER_NAME");
    /// The `committer.email` key, overridden by `GIT_COMMITTER_EMAIL` when resolving a committer.
    pub const EMAIL: keys::Any =
        keys::Any::new("email", &config::Tree::COMMITTER).with_environment_override("GIT_COMMITTER_EMAIL");
}

impl Section for Committer {
    fn name(&self) -> &str {
        "committer"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::NAME, &Self::EMAIL]
    }
}
