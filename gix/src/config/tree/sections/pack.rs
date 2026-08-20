use crate::{
    config,
    config::tree::{Key, Pack, Section, keys},
};

impl Pack {
    /// The `pack.threads` key.
    pub const THREADS: keys::UnsignedInteger =
        keys::UnsignedInteger::new_unsigned_integer("threads", &config::Tree::PACK)
            .with_deviation("Leaving this key unspecified uses all available cores, instead of 1");

    /// The `pack.indexVersion` key.
    pub const INDEX_VERSION: IndexVersion =
        IndexVersion::new_with_validate("indexVersion", &config::Tree::PACK, validate::IndexVersion);

    /// The `pack.compression` key.
    pub const COMPRESSION: keys::Compression = keys::Compression::new_compression("compression", &config::Tree::PACK);
}

/// The `pack.indexVersion` key.
pub type IndexVersion = keys::Any<validate::IndexVersion>;

mod index_version {
    use crate::{config, config::tree::sections::pack::IndexVersion};

    impl IndexVersion {
        /// Try to interpret an integer value as index version.
        pub fn try_into_index_version(
            &'static self,
            value: Result<Option<i64>, gix_config::value::Error>,
        ) -> Result<Option<gix_pack::index::Version>, config::key::GenericError> {
            let Some(value) =
                value.map_err(|err| config::key::GenericError::from(self).with_source(err.into_error()))?
            else {
                return Ok(None);
            };
            Ok(Some(match value {
                1 => gix_pack::index::Version::V1,
                2 => gix_pack::index::Version::V2,
                _ => return Err(config::key::GenericError::from(self)),
            }))
        }
    }
}

impl Section for Pack {
    fn name(&self) -> &str {
        "pack"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::THREADS, &Self::INDEX_VERSION, &Self::COMPRESSION]
    }
}

mod validate {
    use crate::{bstr::BStr, config::tree::keys};
    use gix_error::{ErrorExt, ValidationError};

    #[derive(Clone, Copy)]
    pub struct IndexVersion;
    impl keys::Validate for IndexVersion {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            super::Pack::INDEX_VERSION.try_into_index_version(
                gix_config::Integer::try_from(value)
                    .and_then(|int| {
                        int.to_decimal()
                            .ok_or_else(|| ValidationError::new_with_input("integer out of range", value).raise())
                    })
                    .map(Some),
            )?;
            Ok(())
        }
    }
}
