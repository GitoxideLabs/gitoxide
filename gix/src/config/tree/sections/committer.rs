use crate::{
    config,
    config::tree::{Committer, Key, Section, gitoxide, keys},
};

impl Committer {
    /// The `committer.name` key.
    pub const NAME: keys::Any =
        keys::Any::new("name", &config::Tree::COMMITTER).with_fallback(&gitoxide::Committer::NAME_FALLBACK);
    /// The `committer.email` key.
    pub const EMAIL: keys::Any =
        keys::Any::new("email", &config::Tree::COMMITTER).with_fallback(&gitoxide::Committer::EMAIL_FALLBACK);
}

impl Section for Committer {
    fn name(&self) -> &str {
        "committer"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::NAME, &Self::EMAIL]
    }
}
