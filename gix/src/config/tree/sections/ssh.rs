use crate::{
    config,
    config::tree::{Key, Section, Ssh, keys},
};

impl Ssh {
    /// The `ssh.variant` key
    pub const VARIANT: Variant = Variant::new_with_validate("variant", &config::Tree::SSH, validate::Variant)
        .with_environment_override("GIT_SSH_VARIANT")
        .with_deviation("We error if a variant is chosen that we don't know, as opposed to defaulting to 'ssh'");
}

/// The `ssh.variant` key.
pub type Variant = keys::Any<validate::Variant>;

#[cfg(feature = "blocking-network-client")]
mod variant {
    use std::borrow::Cow;

    use crate::{bstr::BStr, config, config::tree::ssh::Variant};

    impl Variant {
        pub fn try_into_variant(
            &'static self,
            value: Cow<'_, BStr>,
        ) -> Result<
            Option<gix_protocol::transport::client::blocking_io::ssh::ProgramKind>,
            config::key::GenericErrorWithValue,
        > {
            use gix_protocol::transport::client::blocking_io::ssh::ProgramKind;

            use crate::bstr::ByteSlice;
            Ok(Some(match value.as_ref().as_bytes() {
                b"auto" => return Ok(None),
                b"ssh" => ProgramKind::Ssh,
                b"plink" => ProgramKind::Plink,
                b"putty" => ProgramKind::Putty,
                b"tortoiseplink" => ProgramKind::TortoisePlink,
                b"simple" => ProgramKind::Simple,
                _ => return Err(config::key::GenericErrorWithValue::from_value(self, value.into_owned())),
            }))
        }
    }
}

impl Section for Ssh {
    fn name(&self) -> &str {
        "ssh"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::VARIANT]
    }
}

mod validate {
    use crate::{bstr::BStr, config::tree::keys};

    pub struct Variant;
    impl keys::Validate for Variant {
        fn validate(&self, _value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            #[cfg(feature = "blocking-network-client")]
            super::Ssh::VARIANT.try_into_variant(_value.into())?;
            Ok(())
        }
    }
}
