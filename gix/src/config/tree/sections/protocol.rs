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
    use std::borrow::Cow;

    use crate::{bstr::BStr, config, config::tree::protocol::Allow, remote::url::scheme_permission};

    impl Allow {
        /// Convert `value` into its respective `Allow` variant, possibly informing about the `scheme` we are looking at in the error.
        pub fn try_into_allow(
            &'static self,
            value: Cow<'_, BStr>,
            scheme: Option<&str>,
        ) -> Result<scheme_permission::Allow, config::protocol::allow::Error> {
            scheme_permission::Allow::try_from(value).map_err(|value| config::protocol::allow::Error {
                value,
                scheme: scheme.map(ToOwned::to_owned),
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
            value: Option<Result<i64, gix_config::value::Error>>,
        ) -> Result<gix_protocol::transport::Protocol, crate::config::key::GenericErrorWithValue> {
            let value = match value {
                None => return Ok(gix_protocol::transport::Protocol::V2),
                Some(v) => v,
            };
            Ok(match value {
                Ok(0) => gix_protocol::transport::Protocol::V0,
                Ok(1) => gix_protocol::transport::Protocol::V1,
                Ok(2) => gix_protocol::transport::Protocol::V2,
                Ok(other) => {
                    return Err(crate::config::key::GenericErrorWithValue::from_value(
                        self,
                        other.to_string().into(),
                    ));
                }
                Err(err) => {
                    return Err(
                        crate::config::key::GenericErrorWithValue::from_value(self, "unknown".into()).with_source(err),
                    );
                }
            })
        }
    }
}

mod validate {
    use crate::{bstr::BStr, config::tree::keys};

    #[derive(Clone, Copy)]
    pub struct Allow;
    impl keys::Validate for Allow {
        fn validate(&self, _value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            #[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
            super::Protocol::ALLOW.try_into_allow(std::borrow::Cow::Borrowed(_value), None)?;
            Ok(())
        }
    }

    #[derive(Clone, Copy)]
    pub struct Version;
    impl keys::Validate for Version {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            let value = gix_config::Integer::try_from(value)?
                .to_decimal()
                .ok_or_else(|| format!("integer {value} cannot be represented as integer"))?;
            match value {
                0..=2 => Ok(()),
                _ => Err(format!("protocol version {value} is unknown").into()),
            }
        }
    }
}
