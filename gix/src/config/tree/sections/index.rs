use crate::{
    config,
    config::tree::{Index, Key, Section, keys},
};

impl Index {
    /// The `index.threads` key.
    pub const THREADS: IndexThreads =
        IndexThreads::new_with_validate("threads", &config::Tree::INDEX, validate::IndexThreads);
    /// The `index.skipHash` key.
    pub const SKIP_HASH: keys::Boolean = keys::Boolean::new_boolean("skipHash", &config::Tree::INDEX)
        .with_deviation("also used to skip the hash when reading, even if a hash exists in the index file");
}

/// The `index.threads` key.
pub type IndexThreads = keys::Any<validate::IndexThreads>;

mod index_threads {
    use crate::{
        bstr::ByteSlice,
        config,
        config::{key::GenericErrorWithValue, tree::index::IndexThreads},
    };

    impl IndexThreads {
        /// Parse `value` into the amount of threads to use, with `1` being single-threaded, or `0` indicating
        /// to select the amount of threads, with any other number being the specific amount of threads to use.
        pub fn try_into_index_threads(
            &'static self,
            value: impl crate::config::tree::keys::IntoBString,
        ) -> Result<usize, config::key::GenericErrorWithValue> {
            let value = value.into_bstring();
            gix_config::Integer::try_from(value.as_bstr())
                .ok()
                .and_then(|i| i.to_decimal().and_then(|i| i.try_into().ok()))
                .or_else(|| {
                    gix_config::Boolean::try_from(value.as_bstr())
                        .ok()
                        .map(|b| if b.0 { 0 } else { 1 })
                })
                .ok_or_else(|| GenericErrorWithValue::from_value(self, value))
        }
    }
}

impl Section for Index {
    fn name(&self) -> &str {
        "index"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::THREADS, &Self::SKIP_HASH]
    }
}

mod validate {
    use crate::{bstr::BStr, config::tree::keys};

    #[derive(Clone, Copy)]
    pub struct IndexThreads;
    impl keys::Validate for IndexThreads {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            super::Index::THREADS.try_into_index_threads(value)?;
            Ok(())
        }
    }
}
