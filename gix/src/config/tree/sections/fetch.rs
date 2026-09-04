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
    #[cfg(feature = "credentials")]
    impl crate::config::tree::sections::fetch::NegotiationAlgorithm {
        /// Derive the negotiation algorithm identified by `name`, case-sensitively.
        pub fn try_into_negotiation_algorithm(
            &'static self,
            name: impl gix_utils::AsBStr,
        ) -> Result<crate::remote::fetch::negotiate::Algorithm, crate::config::key::GenericErrorWithValue> {
            use crate::{bstr::ByteSlice, remote::fetch::negotiate::Algorithm};

            let name = name.as_bstr();
            Ok(match name.as_bstr().as_bytes() {
                b"noop" => Algorithm::Noop,
                b"consecutive" | b"default" => Algorithm::Consecutive,
                b"skipping" => Algorithm::Skipping,
                _ => {
                    return Err(crate::config::key::GenericErrorWithValue::from_value(self, name.into()));
                }
            })
        }
    }

    #[cfg(feature = "attributes")]
    impl crate::config::tree::sections::fetch::RecurseSubmodules {
        /// Obtain the way submodules should be updated.
        pub fn try_into_recurse_submodules(
            &'static self,
            value: Result<Option<bool>, gix_config::value::Error>,
        ) -> Result<Option<gix_submodule::config::FetchRecurse>, crate::config::key::GenericErrorWithValue> {
            gix_submodule::config::FetchRecurse::new(value)
                .map_err(|err| crate::config::key::GenericErrorWithValue::from_value(self, err))
        }
    }
}

mod validate {
    use crate::{bstr::BStr, config::tree::keys};
    #[cfg(any(feature = "credentials", feature = "attributes"))]
    use gix_error::ResultExt;

    #[derive(Clone, Copy)]
    pub struct NegotiationAlgorithm;
    impl keys::Validate for NegotiationAlgorithm {
        #[cfg_attr(not(feature = "credentials"), allow(unused_variables))]
        fn validate(&self, value: &BStr) -> Result<(), gix_error::Exn> {
            #[cfg(feature = "credentials")]
            crate::config::tree::Fetch::NEGOTIATION_ALGORITHM
                .try_into_negotiation_algorithm(value)
                .or_erased()?;
            Ok(())
        }
    }

    #[cfg(feature = "attributes")]
    #[derive(Clone, Copy)]
    pub struct RecurseSubmodules;
    #[cfg(feature = "attributes")]
    impl keys::Validate for RecurseSubmodules {
        fn validate(&self, value: &BStr) -> Result<(), gix_error::Exn> {
            {
                let boolean = gix_config::Boolean::try_from(value).map(|b| Some(b.0));
                crate::config::tree::Fetch::RECURSE_SUBMODULES
                    .try_into_recurse_submodules(boolean)
                    .or_erased()?;
            }
            Ok(())
        }
    }
}
