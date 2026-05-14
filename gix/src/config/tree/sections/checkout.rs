use crate::{
    config,
    config::tree::{Checkout, Key, Section, keys},
};

impl Checkout {
    /// The `checkout.workers` key.
    pub const WORKERS: Workers = Workers::new_with_validate("workers", &config::Tree::CHECKOUT, validate::Workers)
        .with_deviation("if unset, uses all cores instead of just one");
}

/// The `checkout.workers` key.
pub type Workers = keys::Any<validate::Workers>;

impl Section for Checkout {
    fn name(&self) -> &str {
        "checkout"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::WORKERS]
    }
}

mod workers {
    use crate::config::tree::checkout::Workers;

    impl Workers {
        /// Return the amount of threads to use for checkout, with `0` meaning all available ones, after decoding our integer value from `config`,
        /// or `None` if the value isn't set which is typically interpreted as "as many threads as available"
        pub fn try_from_workers(
            &'static self,
            value: Result<i64, gix_config::value::Error>,
        ) -> Result<usize, crate::config::checkout::workers::Error> {
            match value {
                Ok(v) if v < 0 => Ok(0),
                Ok(v) => Ok(v.try_into().expect("positive i64 can always be usize on 64 bit")),
                Err(err) => Err(crate::config::key::Error::from(&super::Checkout::WORKERS).with_source(err)),
            }
        }
    }
}

///
pub mod validate {
    use crate::{bstr::BStr, config::tree::keys};

    pub struct Workers;
    impl keys::Validate for Workers {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            super::Checkout::WORKERS.try_from_workers(gix_config::Integer::try_from(value).and_then(|i| {
                i.to_decimal()
                    .ok_or_else(|| gix_config::value::Error::new("Integer overflow", value.to_owned()))
            }))?;
            Ok(())
        }
    }
}
