use crate::Result;
use crate::config::{
    self,
    tree::{Key, Section, keys},
};

impl super::Gpg {
    /// The `gpg.format` key.
    pub const FORMAT: Format =
        Format::new_with_validate("format", &config::Tree::GPG, validate::Format).with_default(b"openpgp");
    /// The legacy `gpg.program` key used as an OpenPGP program fallback.
    pub const PROGRAM: keys::Program = keys::Program::new_program("program", &config::Tree::GPG).with_default(b"gpg");
    /// The `gpg.minTrustLevel` key.
    pub const MIN_TRUST_LEVEL: MinTrustLevel =
        MinTrustLevel::new_with_validate("minTrustLevel", &config::Tree::GPG, validate::MinTrustLevel);

    /// The `gpg.openpgp` subsection.
    pub const OPENPGP: OpenPgp = OpenPgp;
    /// The `gpg.x509` subsection.
    pub const X509: X509 = X509;
    /// The `gpg.ssh` subsection.
    pub const SSH: Ssh = Ssh;
}

/// The `gpg.format` key type.
pub type Format = keys::Any<validate::Format>;

impl Format {
    /// Parse `value` as a signature format, defaulting to OpenPGP if it is absent.
    /// Unknown format names are unsupported; malformed names are validation failures.
    pub fn try_into_signature_format(
        &'static self,
        value: Option<&crate::bstr::BStr>,
    ) -> Result<gix_object::signature::Format> {
        use gix_error::ResultExt;

        let value = value.unwrap_or_else(|| self.default_value_or_panic());
        gix_object::signature::Format::parse(value)
            .or_raise(|| gix_error::message("Could not parse signature format").with("key", self.logical_name()))
    }
}

/// The `gpg.minTrustLevel` key type.
pub type MinTrustLevel = keys::Any<validate::MinTrustLevel>;

#[cfg(feature = "command")]
impl MinTrustLevel {
    /// Parse `value` as one of Git's supported signature trust levels.
    pub fn try_into_trust_level(
        &'static self,
        value: impl gix_utils::AsBStr,
    ) -> Result<gix_object::signature::verify::TrustLevel> {
        use crate::bstr::ByteSlice;

        let value = value.as_bstr();
        gix_object::signature::verify::TrustLevel::from_bytes(value.trim()).ok_or_else(|| {
            config::key::error_with_value(self, "Invalid signature trust level", value).validation_error()
        })
    }
}

impl Section for super::Gpg {
    fn name(&self) -> &str {
        "gpg"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::FORMAT, &Self::PROGRAM, &Self::MIN_TRUST_LEVEL]
    }

    fn sub_sections(&self) -> &[&dyn Section] {
        &[&Self::OPENPGP, &Self::X509, &Self::SSH]
    }
}

/// The `gpg.openpgp` subsection.
#[derive(Copy, Clone, Default)]
pub struct OpenPgp;

impl OpenPgp {
    /// The `gpg.openpgp.program` key.
    pub const PROGRAM: keys::Program = keys::Program::new_program("program", &super::Gpg::OPENPGP).with_default(b"gpg");
}

impl Section for OpenPgp {
    fn name(&self) -> &str {
        "openpgp"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::PROGRAM]
    }

    fn parent(&self) -> Option<&dyn Section> {
        Some(&config::Tree::GPG)
    }
}

/// The `gpg.x509` subsection.
#[derive(Copy, Clone, Default)]
pub struct X509;

impl X509 {
    /// The `gpg.x509.program` key.
    pub const PROGRAM: keys::Program = keys::Program::new_program("program", &super::Gpg::X509).with_default(b"gpgsm");
}

impl Section for X509 {
    fn name(&self) -> &str {
        "x509"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::PROGRAM]
    }

    fn parent(&self) -> Option<&dyn Section> {
        Some(&config::Tree::GPG)
    }
}

/// The `gpg.ssh` subsection.
#[derive(Copy, Clone, Default)]
pub struct Ssh;

impl Ssh {
    /// The `gpg.ssh.program` key.
    pub const PROGRAM: keys::Program =
        keys::Program::new_program("program", &super::Gpg::SSH).with_default(b"ssh-keygen");
    /// The `gpg.ssh.defaultKeyCommand` key.
    pub const DEFAULT_KEY_COMMAND: keys::Program = keys::Program::new_program("defaultKeyCommand", &super::Gpg::SSH);
    /// The `gpg.ssh.allowedSignersFile` key.
    pub const ALLOWED_SIGNERS_FILE: keys::Path = keys::Path::new_path("allowedSignersFile", &super::Gpg::SSH);
    /// The `gpg.ssh.revocationFile` key.
    pub const REVOCATION_FILE: keys::Path = keys::Path::new_path("revocationFile", &super::Gpg::SSH);
}

impl Section for Ssh {
    fn name(&self) -> &str {
        "ssh"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[
            &Self::PROGRAM,
            &Self::DEFAULT_KEY_COMMAND,
            &Self::ALLOWED_SIGNERS_FILE,
            &Self::REVOCATION_FILE,
        ]
    }

    fn parent(&self) -> Option<&dyn Section> {
        Some(&config::Tree::GPG)
    }
}

mod validate {
    use crate::{
        Result,
        bstr::BStr,
        config::tree::{Gpg, keys},
    };

    #[derive(Copy, Clone)]
    pub struct Format;

    impl keys::Validate for Format {
        fn validate(&self, value: &BStr) -> Result {
            Gpg::FORMAT.try_into_signature_format(Some(value))?;
            Ok(())
        }
    }

    #[derive(Copy, Clone)]
    pub struct MinTrustLevel;

    impl keys::Validate for MinTrustLevel {
        fn validate(&self, value: &BStr) -> Result {
            #[cfg(feature = "command")]
            {
                Gpg::MIN_TRUST_LEVEL.try_into_trust_level(value)?;
                Ok(())
            }
            #[cfg(not(feature = "command"))]
            {
                let err =
                    crate::config::key::error_with_value(&Gpg::MIN_TRUST_LEVEL, "Invalid signature trust level", value);
                Err(err.validation_error())
            }
        }
    }
}
