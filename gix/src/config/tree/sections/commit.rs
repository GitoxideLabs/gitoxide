use crate::{
    config,
    config::tree::{Commit, Key, Section, keys},
};

impl Commit {
    /// The `commit.gpgSign` key.
    pub const GPG_SIGN: keys::Boolean =
        keys::Boolean::new_boolean("gpgSign", &config::Tree::COMMIT).with_default(b"false");
}

impl Section for Commit {
    fn name(&self) -> &str {
        "commit"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::GPG_SIGN]
    }
}
