use crate::{
    config,
    config::tree::{Author, Key, Section, keys},
};

impl Author {
    /// The `author.name` key, overridden by `GIT_AUTHOR_NAME` when resolving an author.
    pub const NAME: keys::Any =
        keys::Any::new("name", &config::Tree::AUTHOR).with_environment_override("GIT_AUTHOR_NAME");
    /// The `author.email` key, overridden by `GIT_AUTHOR_EMAIL` when resolving an author.
    pub const EMAIL: keys::Any =
        keys::Any::new("email", &config::Tree::AUTHOR).with_environment_override("GIT_AUTHOR_EMAIL");
}

impl Section for Author {
    fn name(&self) -> &str {
        "author"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::NAME, &Self::EMAIL]
    }
}
