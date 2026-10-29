use crate::{
    config,
    config::tree::{CredentialStore, Key, Section, keys},
};

impl CredentialStore {
    /// The timeout in milliseconds when locking the plaintext credential store.
    pub const LOCK_TIMEOUT_MS: keys::LockTimeout =
        keys::LockTimeout::new_lock_timeout("lockTimeoutMS", &config::Tree::CREDENTIAL_STORE);
}

impl Section for CredentialStore {
    fn name(&self) -> &str {
        "credentialStore"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::LOCK_TIMEOUT_MS]
    }
}
