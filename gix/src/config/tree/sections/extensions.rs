use crate::{
    config,
    config::tree::{Extensions, Key, Section, keys},
};

impl Extensions {
    /// The `extensions.worktreeConfig` key.
    pub const WORKTREE_CONFIG: keys::Boolean = keys::Boolean::new_boolean("worktreeConfig", &config::Tree::EXTENSIONS);
    /// The `extensions.objectFormat` key.
    pub const OBJECT_FORMAT: ObjectFormat =
        ObjectFormat::new_with_validate("objectFormat", &config::Tree::EXTENSIONS, validate::ObjectFormat).with_note(
            "Support for SHA256 is prepared but not fully implemented yet. For now we abort when encountered",
        );
}

/// The `core.checkStat` key.
pub type ObjectFormat = keys::Any<validate::ObjectFormat>;

mod object_format {
    use std::borrow::Cow;

    use crate::{bstr::BStr, config, config::tree::sections::extensions::ObjectFormat};

    impl ObjectFormat {
        pub fn try_into_object_format(
            &'static self,
            value: Cow<'_, BStr>,
        ) -> Result<gix_hash::Kind, config::key::GenericErrorWithValue> {
            #[cfg(feature = "sha1")]
            if value.as_ref().eq_ignore_ascii_case(b"sha1") {
                return Ok(gix_hash::Kind::Sha1);
            }

            #[cfg(feature = "sha256")]
            if value.as_ref().eq_ignore_ascii_case(b"sha256") {
                return Ok(gix_hash::Kind::Sha256);
            }

            Err(config::key::GenericErrorWithValue::from_value(self, value.into_owned()))
        }
    }
}

impl Section for Extensions {
    fn name(&self) -> &str {
        "extensions"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::OBJECT_FORMAT, &Self::WORKTREE_CONFIG]
    }
}

mod validate {
    use crate::{bstr::BStr, config::tree::keys};

    #[derive(Clone, Copy)]
    pub struct ObjectFormat;

    impl keys::Validate for ObjectFormat {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            super::Extensions::OBJECT_FORMAT.try_into_object_format(value.into())?;
            Ok(())
        }
    }
}
