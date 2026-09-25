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
    use gix_error::ResultExt;

    use crate::{Error, Result, config, config::tree::sections::pack::IndexVersion};

    impl IndexVersion {
        /// Try to interpret an integer value as index version.
        pub fn try_into_index_version(
            &'static self,
            value: Result<Option<i64>>,
        ) -> Result<Option<gix_pack::index::Version>> {
            let Some(value) = value.or_raise(|| config::key::error(self, "Invalid pack index version"))? else {
                return Ok(None);
            };
            Ok(Some(match value {
                1 => gix_pack::index::Version::V1,
                2 => gix_pack::index::Version::V2,
                _ => {
                    return Err(Error::from_error(config::key::error_with_value(
                        self,
                        "Invalid pack index version",
                        value,
                    )));
                }
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
    use crate::{Result, bstr::BStr, config::tree::keys};
    use gix_error::{ErrorExt, ResultExt};

    #[derive(Clone, Copy)]
    pub struct IndexVersion;
    impl keys::Validate for IndexVersion {
        fn validate(&self, value: &BStr) -> Result {
            super::Pack::INDEX_VERSION
                .try_into_index_version(
                    gix_config::Integer::try_from(value)
                        .and_then(|int| {
                            (int.to_decimal().ok_or_else(|| {
                                gix_error::validation("integer out of range")
                                    .with("input", value)
                                    .raise()
                            }))
                            .map_err(Into::into)
                        })
                        .map(Some),
                )
                .or_erased()?;
            Ok(())
        }
    }
}
