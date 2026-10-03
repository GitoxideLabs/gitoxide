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
    pub const OBJECT_FORMAT: ObjectFormat =
        ObjectFormat::new_with_validate("objectFormat", &config::Tree::EXTENSIONS, validate::ObjectFormat);
}

/// The `extensions.objectFormat` key.
pub type ObjectFormat = keys::Any<validate::ObjectFormat>;

mod object_format {
    use gix_error::bail;

    use crate::{
        Result,
        bstr::ByteSlice,
        config::{key::error_with_value, tree::sections::extensions::ObjectFormat},
    };

    impl ObjectFormat {
        /// Parse an object format, distinguishing unknown names from known hashes disabled in this build.
        /// Disabled hashes are classified as [`gix_error::Class::Unsupported`], unknown names as validation failures.
        pub fn try_into_object_format(&'static self, value: impl gix_utils::AsBStr) -> Result<gix_hash::Kind> {
            let value = value.as_bstr();
            #[cfg(feature = "sha1")]
            if value.as_bstr().eq_ignore_ascii_case(b"sha1") {
                return Ok(gix_hash::Kind::Sha1);
            }

            #[cfg(feature = "sha256")]
            if value.as_bstr().eq_ignore_ascii_case(b"sha256") {
                return Ok(gix_hash::Kind::Sha256);
            }

            if value.eq_ignore_ascii_case(b"sha1") || value.eq_ignore_ascii_case(b"sha256") {
                bail!(error_with_value(self, "Object format is not enabled in this build", value).unsupported_error(),);
            }

            Err(error_with_value(self, "Invalid configuration value", value).validation_error())
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

mod validate {
    use crate::{Result, bstr::BStr, config::tree::keys};

    #[derive(Clone, Copy)]
    pub struct ObjectFormat;

    impl keys::Validate for ObjectFormat {
        fn validate(&self, value: &BStr) -> Result {
            super::Extensions::OBJECT_FORMAT.try_into_object_format(value)?;
            Ok(())
        }
    }
}
