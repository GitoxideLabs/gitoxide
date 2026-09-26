use crate::{
    config,
    config::tree::{Key, Protocol, Section, keys},
};

impl Protocol {
    /// The `protocol.allow` key.
    pub const ALLOW: Allow = Allow::new_with_validate("allow", &config::Tree::PROTOCOL, validate::Allow);
    /// The `protocol.version` key.
    pub const VERSION: Version = Version::new_with_validate("version", &config::Tree::PROTOCOL, validate::Version);

    /// The `protocol.<name>` subsection
    pub const NAME_PARAMETER: NameParameter = NameParameter;
}

/// The `protocol.allow` key type.
pub type Allow = keys::Any<validate::Allow>;

/// The `protocol.version` key.
pub type Version = keys::Any<validate::Version>;

#[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
mod allow {
    use crate::{Result, bstr::ByteSlice, config::tree::protocol::Allow, remote::url::scheme_permission};
    use gix_error::ResultExt;

    impl Allow {
        /// Convert `value` into its respective `Allow` variant, possibly informing about the `scheme` we are looking at in the error.
        ///
        /// Invalid input is retained in the parser error's `input` metadata.
        pub fn try_into_allow(
            &'static self,
            value: impl gix_utils::AsBStr,
            scheme: Option<&str>,
        ) -> Result<scheme_permission::Allow> {
            let value = value.as_bstr();
            scheme_permission::Allow::try_from(value.as_bstr()).or_raise(|| {
                gix_error::validation(format!(
                    "The value {value:?} must be allow|deny|user in configuration key protocol{}.allow",
                    scheme.map(|scheme| format!(".{scheme}")).unwrap_or_default()
                ))
            })
        }
    }
}

/// The `protocol.<name>` parameter section.
pub struct NameParameter;

impl NameParameter {
    /// The `protocol.<name>.allow` key.
    pub const ALLOW: Allow = Allow::new_with_validate("allow", &Protocol::NAME_PARAMETER, validate::Allow);
}

impl Section for NameParameter {
    fn name(&self) -> &str {
        "<name>"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::ALLOW]
    }

    fn parent(&self) -> Option<&dyn Section> {
        Some(&config::Tree::PROTOCOL)
    }
}

impl Section for Protocol {
    fn name(&self) -> &str {
        "protocol"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::ALLOW, &Self::VERSION]
    }

    fn sub_sections(&self) -> &[&dyn Section] {
        &[&Self::NAME_PARAMETER]
    }
}

mod key_impls {
    #[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
    use crate::{Error, Result};
    impl super::Version {
        /// Convert `value` into the corresponding protocol version, possibly applying the correct default.
        #[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
        pub fn try_into_protocol_version(
            &'static self,
            value: Result<Option<i64>>,
        ) -> Result<gix_protocol::transport::Protocol> {
            use gix_error::ResultExt;

            let Some(value) = value.or_raise(|| crate::config::key::error(self, "Invalid protocol version"))? else {
                return Ok(gix_protocol::transport::Protocol::V2);
            };
            Ok(match value {
                0 => gix_protocol::transport::Protocol::V0,
                1 => gix_protocol::transport::Protocol::V1,
                2 => gix_protocol::transport::Protocol::V2,
                other => {
                    return Err(Error::from_error(crate::config::key::error_with_value(
                        self,
                        "Invalid protocol version",
                        other,
                    )));
                }
            })
        }
    }
}

mod validate {
    use crate::{Result, bstr::BStr, config::tree::keys};
    use gix_error::{ErrorExt, message};

    #[derive(Clone, Copy)]
    pub struct Allow;
    impl keys::Validate for Allow {
        fn validate(&self, _value: &BStr) -> Result {
            #[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
            super::Protocol::ALLOW.try_into_allow(_value, None)?;
            Ok(())
        }
    }

    #[derive(Clone, Copy)]
    pub struct Version;
    impl keys::Validate for Version {
        fn validate(&self, value: &BStr) -> Result {
            let value = gix_config::Integer::try_from(value)?
                .to_decimal()
                .ok_or_else(|| message!("integer {value} cannot be represented as integer").raise_erased())?;
            match value {
                0..=2 => Ok(()),
                _ => Err(message!("protocol version {value} is unknown").raise()),
            }
        }
    }
}
