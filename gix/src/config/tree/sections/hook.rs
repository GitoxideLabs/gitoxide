use crate::config::tree::{Hook, Key, Section, SubSectionRequirement, keys};

const NAME: Option<SubSectionRequirement> = Some(SubSectionRequirement::Parameter("name"));

impl Hook {
    /// The `hook.<name>.command` command or shell script.
    pub const COMMAND: keys::Program =
        keys::Program::new_program("command", &crate::config::Tree::HOOK).with_subsection_requirement(NAME);
    /// The `hook.<name>.event` registration, with empty values clearing earlier events.
    pub const EVENT: keys::Any = keys::Any::new("event", &crate::config::Tree::HOOK).with_subsection_requirement(NAME);
    /// The `hook.<name>.enabled` per-hook or event-wide switch.
    pub const ENABLED: keys::Boolean =
        keys::Boolean::new_boolean("enabled", &crate::config::Tree::HOOK).with_subsection_requirement(NAME);
    /// The `hook.<name>.parallel` flag, identifying a configured hook name even during forced serial execution.
    pub const PARALLEL: keys::Boolean =
        keys::Boolean::new_boolean("parallel", &crate::config::Tree::HOOK).with_subsection_requirement(NAME);
}

impl Section for Hook {
    fn name(&self) -> &str {
        "hook"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::COMMAND, &Self::EVENT, &Self::ENABLED, &Self::PARALLEL]
    }
}
