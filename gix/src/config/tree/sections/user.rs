use crate::{
    config,
    config::tree::{Key, Section, User, gitoxide, keys},
};

impl User {
    /// The `user.name` key
    pub const NAME: keys::Any = keys::Any::new("name", &config::Tree::USER);
    /// The `user.email` key
    pub const EMAIL: keys::Any =
        keys::Any::new("email", &config::Tree::USER).with_fallback(&gitoxide::User::EMAIL_FALLBACK);
    /// The `user.signingKey` key.
    pub const SIGNING_KEY: keys::Any = keys::Any::new("signingKey", &config::Tree::USER);
}

impl Section for User {
    fn name(&self) -> &str {
        "user"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::NAME, &Self::EMAIL, &Self::SIGNING_KEY]
    }
}
