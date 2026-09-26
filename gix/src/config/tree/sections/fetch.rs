use crate::{
    config,
    config::tree::{Fetch, Key, Section, keys},
};

impl Fetch {
    /// The `fetch.negotiationAlgorithm` key.
    pub const NEGOTIATION_ALGORITHM: NegotiationAlgorithm = NegotiationAlgorithm::new_with_validate(
        "negotiationAlgorithm",
        &config::Tree::FETCH,
        validate::NegotiationAlgorithm,
    );
    /// The `fetch.recurseSubmodules` key.
    #[cfg(feature = "attributes")]
    pub const RECURSE_SUBMODULES: RecurseSubmodules =
        RecurseSubmodules::new_with_validate("recurseSubmodules", &config::Tree::FETCH, validate::RecurseSubmodules);
}

impl Section for Fetch {
    fn name(&self) -> &str {
        "fetch"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[
            &Self::NEGOTIATION_ALGORITHM,
            #[cfg(feature = "attributes")]
            &Self::RECURSE_SUBMODULES,
        ]
    }
}

/// The `fetch.negotiationAlgorithm` key.
pub type NegotiationAlgorithm = keys::Any<validate::NegotiationAlgorithm>;

/// The `fetch.recurseSubmodules` key.
#[cfg(feature = "attributes")]
pub type RecurseSubmodules = keys::Any<validate::RecurseSubmodules>;

mod algorithm {
    #[cfg(any(feature = "credentials", feature = "attributes"))]
    use crate::Result;

    #[cfg(feature = "credentials")]
    impl crate::config::tree::sections::fetch::NegotiationAlgorithm {
        /// Derive the negotiation algorithm identified by `name`, case-sensitively.
        pub fn try_into_negotiation_algorithm(
            &'static self,
            name: impl gix_utils::AsBStr,
        ) -> Result<crate::remote::fetch::negotiate::Algorithm> {
            use crate::{Error, bstr::ByteSlice, remote::fetch::negotiate::Algorithm};

            let name = name.as_bstr();
            Ok(match name.as_bstr().as_bytes() {
                b"noop" => Algorithm::Noop,
                b"consecutive" | b"default" => Algorithm::Consecutive,
                b"skipping" => Algorithm::Skipping,
                _ => {
                    return Err(Error::from_error(crate::config::key::error_with_value(
                        self,
                        "Invalid configuration value",
                        name,
                    )));
                }
            })
        }
    }

    #[cfg(feature = "attributes")]
    impl crate::config::tree::sections::fetch::RecurseSubmodules {
        /// Obtain the way submodules should be updated from a boolean configuration lookup.
        pub fn try_into_recurse_submodules(
            &'static self,
            value: Result<Option<bool>>,
        ) -> Result<Option<gix_submodule::config::FetchRecurse>> {
            gix_submodule::config::FetchRecurse::new(value).map_err(|input| {
                crate::Error::from_error(crate::config::key::error_with_value(
                    self,
                    "Invalid configuration value",
                    input,
                ))
            })
        }
    }
}

mod validate {
    use crate::{Result, bstr::BStr, config::tree::keys};

    #[derive(Clone, Copy)]
    pub struct NegotiationAlgorithm;
    impl keys::Validate for NegotiationAlgorithm {
        #[cfg_attr(not(feature = "credentials"), allow(unused_variables))]
        fn validate(&self, value: &BStr) -> Result {
            #[cfg(feature = "credentials")]
            crate::config::tree::Fetch::NEGOTIATION_ALGORITHM.try_into_negotiation_algorithm(value)?;
            Ok(())
        }
    }

    #[cfg(feature = "attributes")]
    #[derive(Clone, Copy)]
    pub struct RecurseSubmodules;
    #[cfg(feature = "attributes")]
    impl keys::Validate for RecurseSubmodules {
        fn validate(&self, value: &BStr) -> Result {
            {
                let boolean = gix_config::Boolean::try_from(value).map(|b| Some(b.0));
                crate::config::tree::Fetch::RECURSE_SUBMODULES.try_into_recurse_submodules(boolean)?;
            }
            Ok(())
        }
    }
}
