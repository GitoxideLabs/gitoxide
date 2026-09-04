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
    use crate::{bstr::ByteSlice, config, config::tree::protocol::Allow, remote::url::scheme_permission};

    impl Allow {
        /// Convert `value` into its respective `Allow` variant, possibly informing about the `scheme` we are looking at in the error.
        pub fn try_into_allow(
            &'static self,
            value: impl gix_utils::AsBStr,
            scheme: Option<&str>,
        ) -> Result<scheme_permission::Allow, config::protocol::allow::Error> {
            let value = value.as_bstr();
            scheme_permission::Allow::try_from(value.as_bstr()).map_err(|value| {
                gix_error::Error::from_error(gix_error::ValidationError::new(format!(
                    "The value {value:?} must be allow|deny|user in configuration key protocol{}.allow",
                    scheme.map(|scheme| format!(".{scheme}")).unwrap_or_default()
                )))
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
    impl super::Version {
        /// Convert `value` into the corresponding protocol version, possibly applying the correct default.
        #[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
        pub fn try_into_protocol_version(
            &'static self,
            value: Result<Option<i64>, gix_config::value::Error>,
        ) -> Result<gix_protocol::transport::Protocol, crate::config::key::GenericErrorWithValue> {
            let value = match value {
                Ok(None) => return Ok(gix_protocol::transport::Protocol::V2),
                Ok(Some(value)) => value,
                Err(err) => {
                    return Err(
                        crate::config::key::GenericErrorWithValue::from_value(self, "unknown".into())
                            .with_source(err.into_error()),
                    );
                }
            };
            Ok(match value {
                0 => gix_protocol::transport::Protocol::V0,
                1 => gix_protocol::transport::Protocol::V1,
                2 => gix_protocol::transport::Protocol::V2,
                other => {
                    return Err(crate::config::key::GenericErrorWithValue::from_value(
                        self,
                        other.to_string().into(),
                    ));
                }
            })
        }
    }
}

mod validate {
    use crate::{bstr::BStr, config::tree::keys};
    use gix_error::{ErrorExt, ResultExt, message};

    #[derive(Clone, Copy)]
    pub struct Allow;
    impl keys::Validate for Allow {
        fn validate(&self, _value: &BStr) -> Result<(), gix_error::Exn> {
            #[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
            super::Protocol::ALLOW.try_into_allow(_value, None).or_erased()?;
            Ok(())
        }
    }

    #[derive(Clone, Copy)]
    pub struct Version;
    impl keys::Validate for Version {
        fn validate(&self, value: &BStr) -> Result<(), gix_error::Exn> {
            let value = gix_config::Integer::try_from(value)
                .or_erased()?
                .to_decimal()
                .ok_or_else(|| message!("integer {value} cannot be represented as integer").raise_erased())?;
            match value {
                0..=2 => Ok(()),
                _ => Err(message!("protocol version {value} is unknown").raise_erased()),
            }
        }
    }
}
