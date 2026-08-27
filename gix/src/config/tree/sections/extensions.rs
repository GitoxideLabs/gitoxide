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
    /// The `extensions.refStorage` key.
    pub const REF_STORAGE: RefStorage =
        RefStorage::new_with_validate("refStorage", &config::Tree::EXTENSIONS, validate::RefStorage);
}

/// The `core.checkStat` key.
pub type ObjectFormat = keys::Any<validate::ObjectFormat>;

/// The validated `extensions.refStorage` key.
pub type RefStorage = keys::Any<validate::RefStorage>;

mod object_format {
    use crate::{bstr::ByteSlice, config, config::tree::sections::extensions::ObjectFormat};

    impl ObjectFormat {
        pub fn try_into_object_format(
            &'static self,
            value: impl gix_utils::AsBStr,
        ) -> Result<gix_hash::Kind, config::key::GenericErrorWithValue> {
            let value = value.as_bstr();
            #[cfg(feature = "sha1")]
            if value.as_bstr().eq_ignore_ascii_case(b"sha1") {
                return Ok(gix_hash::Kind::Sha1);
            }

            #[cfg(feature = "sha256")]
            if value.as_bstr().eq_ignore_ascii_case(b"sha256") {
                return Ok(gix_hash::Kind::Sha256);
            }

            Err(config::key::GenericErrorWithValue::from_value(self, value.into()))
        }
    }
}

mod ref_storage {
    use crate::{config, config::tree::sections::extensions::RefStorage};

    impl RefStorage {
        /// Parse a Git reference-storage format name.
        pub fn try_into_reference_storage(
            &'static self,
            value: impl gix_utils::AsBStr,
        ) -> Result<crate::create::ReferenceStorage, config::key::GenericErrorWithValue> {
            let value = value.as_bstr();
            if value == b"files" {
                Ok(crate::create::ReferenceStorage::Files)
            } else if value == b"reftable" {
                Ok(crate::create::ReferenceStorage::Reftable)
            } else {
                Err(config::key::GenericErrorWithValue::from_value(self, value.into()))
            }
        }
    }
}

impl Section for Extensions {
    fn name(&self) -> &str {
        "extensions"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::OBJECT_FORMAT, &Self::REF_STORAGE, &Self::WORKTREE_CONFIG]
    }
}

mod validate {
    use crate::{bstr::BStr, config::tree::keys};

    #[derive(Clone, Copy)]
    pub struct ObjectFormat;

    impl keys::Validate for ObjectFormat {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            super::Extensions::OBJECT_FORMAT.try_into_object_format(value)?;
            Ok(())
        }
    }

    #[derive(Clone, Copy)]
    pub struct RefStorage;

    impl keys::Validate for RefStorage {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            super::Extensions::REF_STORAGE.try_into_reference_storage(value)?;
            Ok(())
        }
    }
}
