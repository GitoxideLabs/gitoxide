use crate::{
    config,
    config::tree::{Credential, Key, Section, keys},
};

impl Credential {
    /// The `credential.helper` key.
    pub const HELPER: keys::Program = keys::Program::new_program("helper", &config::Tree::CREDENTIAL);
    /// The `credential.username` key.
    pub const USERNAME: keys::Any = keys::Any::new("username", &config::Tree::CREDENTIAL);
    /// The `credential.useHttpPath` key.
    pub const USE_HTTP_PATH: keys::Boolean = keys::Boolean::new_boolean("useHttpPath", &config::Tree::CREDENTIAL);

    /// The `credential.<url>` subsection
    pub const URL_PARAMETER: UrlParameter = UrlParameter;
}

/// The `credential.<url>` parameter section.
pub struct UrlParameter;

impl UrlParameter {
    /// The `credential.<url>.helper` key.
    pub const HELPER: keys::Program = keys::Program::new_program("helper", &Credential::URL_PARAMETER);
    /// The `credential.<url>.username` key.
    pub const USERNAME: keys::Any = keys::Any::new("username", &Credential::URL_PARAMETER);
    /// The `credential.<url>.useHttpPath` key.
    pub const USE_HTTP_PATH: keys::Boolean = keys::Boolean::new_boolean("useHttpPath", &Credential::URL_PARAMETER);
}

impl Section for UrlParameter {
    fn name(&self) -> &str {
        "<url>"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::HELPER, &Self::USERNAME, &Self::USE_HTTP_PATH]
    }

    fn parent(&self) -> Option<&dyn Section> {
        Some(&config::Tree::CREDENTIAL)
    }
}

impl Section for Credential {
    fn name(&self) -> &str {
        "credential"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::HELPER, &Self::USERNAME, &Self::USE_HTTP_PATH]
    }

    fn sub_sections(&self) -> &[&dyn Section] {
        &[&Self::URL_PARAMETER]
    }
}
