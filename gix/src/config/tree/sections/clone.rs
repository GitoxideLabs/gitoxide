use crate::{
    config,
    config::tree::{Clone, Key, Section, keys},
};

impl Clone {
    /// The `clone.defaultRemoteName` key.
    pub const DEFAULT_REMOTE_NAME: keys::RemoteName =
        keys::RemoteName::new_remote_name("defaultRemoteName", &config::Tree::CLONE);
    /// The `clone.rejectShallow` key.
    pub const REJECT_SHALLOW: keys::Boolean = keys::Boolean::new_boolean("rejectShallow", &config::Tree::CLONE);
}

impl Section for Clone {
    fn name(&self) -> &str {
        "clone"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::DEFAULT_REMOTE_NAME, &Self::REJECT_SHALLOW]
    }
}
