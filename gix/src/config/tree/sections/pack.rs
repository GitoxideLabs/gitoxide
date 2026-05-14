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
}

/// The `pack.indexVersion` key.
pub type IndexVersion = keys::Any<validate::IndexVersion>;

mod index_version {
    use crate::{config, config::tree::sections::pack::IndexVersion};

    impl IndexVersion {
        /// Try to interpret an integer value as index version.
        pub fn try_into_index_version(
            &'static self,
            value: Result<i64, gix_config::value::Error>,
        ) -> Result<gix_pack::index::Version, config::key::GenericError> {
            let value = value.map_err(|err| config::key::GenericError::from(self).with_source(err))?;
            Ok(match value {
                1 => gix_pack::index::Version::V1,
                2 => gix_pack::index::Version::V2,
                _ => return Err(config::key::GenericError::from(self)),
            })
        }
    }
}

impl Section for Pack {
    fn name(&self) -> &str {
        "pack"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::THREADS, &Self::INDEX_VERSION]
    }
}

mod validate {
    use crate::{bstr::BStr, config::tree::keys};

    #[derive(Clone, Copy)]
    pub struct IndexVersion;
    impl keys::Validate for IndexVersion {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            super::Pack::INDEX_VERSION.try_into_index_version(gix_config::Integer::try_from(value).and_then(
                |int| {
                    int.to_decimal()
                        .ok_or_else(|| gix_config::value::Error::new("integer out of range", value))
                },
            ))?;
            Ok(())
        }
    }
}
