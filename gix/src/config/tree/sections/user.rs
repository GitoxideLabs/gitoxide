use crate::{
    config,
    config::tree::{Key, Section, User, gitoxide, keys},
};

impl User {
    /// The `user.name` key, used as fallback for author and committer names.
    ///
    /// Unlike [`EMAIL`](Self::EMAIL), it has no generic environment fallback. Git's remaining name fallback is derived from
    /// the system account, which `gix` deliberately does not reproduce.
    pub const NAME: keys::Any = keys::Any::new("name", &config::Tree::USER);
    /// The `user.email` key, used as fallback for author and committer emails, with `EMAIL` as its own fallback.
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
