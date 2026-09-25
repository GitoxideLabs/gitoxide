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
    use gix_error::ResultExt;

    use crate::{Result, config::tree::checkout::Workers};

    impl Workers {
        /// Return the amount of threads to use for checkout, with `0` meaning all available ones, after decoding our integer value from `config`,
        /// or `None` if the value isn't set which is typically interpreted as "as many threads as available"
        pub fn try_from_workers(&'static self, value: Result<Option<i64>>) -> Result<Option<usize>> {
            match value.or_raise(|| crate::config::key::error(self, "Could not decode checkout workers"))? {
                Some(v) if v < 0 => Ok(Some(0)),
                Some(v) => Ok(Some(v.try_into().expect("positive i64 can always be usize on 64 bit"))),
                None => Ok(None),
            }
        }
    }
}

///
pub mod validate {
    use crate::{Result, bstr::BStr, config::tree::keys};
    use gix_error::{ErrorExt, ResultExt};

    pub struct Workers;
    impl keys::Validate for Workers {
        fn validate(&self, value: &BStr) -> Result {
            super::Checkout::WORKERS
                .try_from_workers(
                    gix_config::Integer::try_from(value)
                        .and_then(|i| {
                            i.to_decimal().ok_or_else(|| {
                                gix_error::validation("Integer overflow")
                                    .with("input", value.to_owned())
                                    .raise()
                            })
                        })
                        .map(Some)
                        .map_err(Into::into),
                )
                .or_erased()?;
            Ok(())
        }
    }
}
