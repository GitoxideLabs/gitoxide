use crate::{
    bstr::{BStr, BString, ByteVec},
    config::tree::key::validate_assignment,
};

/// Provide information about a configuration section.
pub trait Section {
    /// The section name, like `remote` in `remote.origin.url`.
    fn name(&self) -> &str;
    /// The keys directly underneath it for carrying configuration values.
    fn keys(&self) -> &[&dyn Key];
    /// The list of sub-section names, which may be empty if there are no statically known sub-sections.
    fn sub_sections(&self) -> &[&dyn Section] {
        &[]
    }
    /// The parent section if this is a statically known sub-section.
    fn parent(&self) -> Option<&dyn Section> {
        None
    }
}

/// Determine how subsections may be used with a given key, suitable for obtaining the full name for use in assignments.
#[derive(Debug, Copy, Clone)]
pub enum SubSectionRequirement {
    /// Subsections must not be used, this key can only be below a section.
    Never,
    /// The sub-section is used as parameter with the given name.
    Parameter(&'static str),
}

/// A way to link a key with other resources.
#[derive(Debug, Copy, Clone)]
pub enum Link {
    /// The environment variable of the given name will override the value of this key.
    EnvironmentOverride(&'static str),
    /// This config key is used as fallback if this key isn't set.
    FallbackKey(&'static dyn Key),
}

/// A note attached to a key.
#[derive(Debug, Copy, Clone)]
pub enum Note {
    /// A piece of information related to a key to help the user.
    Informative(&'static str),
    /// This key works differently than is described by git, explaining the deviation further.
    Deviation(&'static str),
}

/// A leaf-level entry in the git configuration, like `url` in `remote.origin.url`.
pub trait Key: std::fmt::Debug {
    /// The key's name, like `url` in `remote.origin.url`.
    fn name(&self) -> &str;
    /// See if `value` is allowed as value of this key, or return a descriptive error if it is not.
    fn validate(&self, value: &BStr) -> Result<(), crate::config::tree::key::validate::Error>;
    /// The section containing this key. Git configuration has no free-standing keys, they are always underneath a section.
    fn section(&self) -> &dyn Section;
    /// The return value encodes three possible states to indicate subsection requirements
    /// * `None` = subsections may or may not be used, the most flexible setting.
    /// * `Some([Requirement][SubSectionRequirement])` = subsections must or must not be used, depending on the value
    fn subsection_requirement(&self) -> Option<&SubSectionRequirement> {
        Some(&SubSectionRequirement::Never)
    }
    /// Return the link to other resources, if available.
    fn link(&self) -> Option<&Link> {
        None
    }
    /// Return a note about this key, if available.
    fn note(&self) -> Option<&Note> {
        None
    }

    /// Return the name of an environment variable that would override this value (after following links until one is found).
    fn environment_override(&self) -> Option<&str> {
        let mut cursor = self.link()?;
        loop {
            match cursor {
                Link::EnvironmentOverride(name) => return Some(name),
                Link::FallbackKey(next) => {
                    cursor = next.link()?;
                }
            }
        }
    }

    /// Return the environment override that must be set on this key.
    /// # Panics
    /// If no environment variable is set
    fn the_environment_override(&self) -> &str {
        self.environment_override()
            .expect("BUG: environment override must be set")
    }
    /// Produce a name that describes how the name is composed. This is `core.bare` for statically known keys, or `branch.<name>.key`
    /// for complex ones.
    fn logical_name(&self) -> String {
        let section = self.section();
        let mut buf = String::new();
        let parameter = if let Some(parent) = section.parent() {
            buf.push_str(parent.name());
            buf.push('.');
            None
        } else {
            self.subsection_requirement().and_then(|requirement| match requirement {
                SubSectionRequirement::Parameter(name) => Some(name),
                SubSectionRequirement::Never => None,
            })
        };
        buf.push_str(section.name());
        buf.push('.');
        if let Some(parameter) = parameter {
            buf.push('<');
            buf.push_str(parameter);
            buf.push('>');
            buf.push('.');
        }
        buf.push_str(self.name());
        buf
    }

    /// The full name of the key for use in configuration overrides, like `core.bare`, or `remote.<subsection>.url` if `subsection` is
    /// not `None`.
    /// May fail if this key needs a subsection, or may not have a subsection.
    fn full_name(&self, subsection: Option<&BStr>) -> Result<BString, String> {
        let section = self.section();
        let mut buf = BString::default();
        let subsection = match self.subsection_requirement() {
            None => subsection,
            Some(requirement) => match (requirement, subsection) {
                (SubSectionRequirement::Never, Some(_)) => {
                    return Err(format!(
                        "The key named '{}' cannot be used with non-static subsections.",
                        self.logical_name()
                    ));
                }
                (SubSectionRequirement::Parameter(_), None) => {
                    return Err(format!(
                        "The key named '{}' cannot be used without subsections.",
                        self.logical_name()
                    ));
                }
                _ => subsection,
            },
        };

        if let Some(parent) = section.parent() {
            buf.push_str(parent.name());
            buf.push(b'.');
        }
        buf.push_str(section.name());
        buf.push(b'.');
        if let Some(subsection) = subsection {
            debug_assert!(
                section.parent().is_none(),
                "BUG: sections with parameterized sub-sections must be top-level sections"
            );
            buf.push_str(subsection);
            buf.push(b'.');
        }
        buf.push_str(self.name());
        Ok(buf)
    }

    /// Return an assignment with the keys full name to `value`, suitable for [configuration overrides][crate::open::Options::config_overrides()].
    /// Note that this will fail if the key requires a subsection name.
    fn validated_assignment(&self, value: &BStr) -> Result<BString, validate_assignment::Error> {
        self.validate(value)?;
        let mut key = self
            .full_name(None)
            .map_err(|message| validate_assignment::Error::Name { message })?;
        key.push(b'=');
        key.push_str(value);
        Ok(key)
    }

    /// Return an assignment with the keys full name to `value`, suitable for [configuration overrides][crate::open::Options::config_overrides()].
    /// Note that this will fail if the key requires a subsection name.
    fn validated_assignment_fmt(
        &self,
        value: &dyn std::fmt::Display,
    ) -> Result<BString, crate::config::tree::key::validate_assignment::Error> {
        let value = value.to_string();
        self.validated_assignment(value.as_str().into())
    }

    /// Return an assignment to `value` with the keys full name within `subsection`, suitable for [configuration overrides][crate::open::Options::config_overrides()].
    /// Note that this is only valid if this key supports parameterized sub-sections, or else an error is returned.
    fn validated_assignment_with_subsection(
        &self,
        value: &BStr,
        subsection: &BStr,
    ) -> Result<BString, crate::config::tree::key::validate_assignment::Error> {
        self.validate(value)?;
        let mut key = self
            .full_name(Some(subsection))
            .map_err(|message| validate_assignment::Error::Name { message })?;
        key.push(b'=');
        key.push_str(value);
        Ok(key)
    }
}

impl gix_config::AsKey for &dyn Key {
    fn as_key(&self) -> gix_config::KeyRef<'_> {
        self.try_as_key().expect("infallible")
    }

    fn try_as_key(&self) -> Option<gix_config::KeyRef<'_>> {
        let section_name = self
            .section()
            .parent()
            .map_or_else(|| self.section().name(), Section::name);
        let subsection_name = if self.section().parent().is_some() {
            Some(self.section().name().into())
        } else {
            None
        };
        let value_name = self.name();
        gix_config::KeyRef {
            section_name,
            subsection_name,
            value_name,
        }
        .into()
    }
}
