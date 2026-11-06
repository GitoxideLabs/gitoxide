use crate::{
    config,
    config::tree::{Extensions, Key, Section, keys},
};

impl Extensions {
    /// The `extensions.worktreeConfig` key.
    pub const WORKTREE_CONFIG: keys::Boolean = keys::Boolean::new_boolean("worktreeConfig", &config::Tree::EXTENSIONS);
    /// The `extensions.relativeWorktrees` key, indicating that worktrees may use relative links.
    pub const RELATIVE_WORKTREES: keys::Boolean =
        keys::Boolean::new_boolean("relativeWorktrees", &config::Tree::EXTENSIONS);
    /// The `extensions.objectFormat` key.
    pub const OBJECT_FORMAT: ObjectFormat = ObjectFormat::new_object_format("objectFormat", &config::Tree::EXTENSIONS);
}

/// The `extensions.objectFormat` key.
pub type ObjectFormat = keys::Any<validate::ObjectFormat>;

mod object_format {
    use gix_error::{ResultExt, message};

    use crate::{
        Result,
        config::tree::{Key, Section, keys, sections::extensions::ObjectFormat},
    };

    impl ObjectFormat {
        /// Create a new instance.
        pub const fn new_object_format(name: &'static str, section: &'static dyn Section) -> Self {
            keys::Any::new_with_validate(name, section, super::validate::ObjectFormat)
        }

        /// Parse an object format, distinguishing unknown names from known hashes disabled in this build.
        /// Disabled hashes are classified as [`gix_error::Class::Unsupported`], unknown names as validation failures.
        pub fn try_into_object_format(&'static self, value: impl gix_utils::AsBStr) -> Result<gix_hash::Kind> {
            super::validate::ObjectFormat::parse(value.as_bstr()).or_raise(|| {
                let mut context = message("Invalid configuration value").with("key", self.logical_name());
                if let Some(environment) = self.environment_override() {
                    context = context.with("environment_override", environment);
                }
                context
            })
        }
    }
}

impl Section for Extensions {
    fn name(&self) -> &str {
        "extensions"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::OBJECT_FORMAT, &Self::WORKTREE_CONFIG, &Self::RELATIVE_WORKTREES]
    }
}

/// Validation for `extensions` configuration keys.
pub mod validate {
    use gix_error::message;

    use crate::{Result, bstr::BStr, config::tree::keys};

    /// Validate values for [`super::ObjectFormat`].
    #[derive(Clone, Copy)]
    pub struct ObjectFormat;

    impl keys::Validate for ObjectFormat {
        fn validate(&self, value: &BStr) -> Result {
            Self::parse(value).map(|_| ())
        }
    }

    impl ObjectFormat {
        pub(super) fn parse(value: &BStr) -> Result<gix_hash::Kind> {
            #[cfg(feature = "sha1")]
            if value.eq_ignore_ascii_case(b"sha1") {
                return Ok(gix_hash::Kind::Sha1);
            }

            #[cfg(feature = "sha256")]
            if value.eq_ignore_ascii_case(b"sha256") {
                return Ok(gix_hash::Kind::Sha256);
            }

            if value.eq_ignore_ascii_case(b"sha1") || value.eq_ignore_ascii_case(b"sha256") {
                return Err(message("Object format is not enabled in this build")
                    .with_input(value)
                    .unsupported_error());
            }

            Err(message("Invalid configuration value")
                .with_input(value)
                .validation_error())
        }
    }
}
