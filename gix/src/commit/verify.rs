use std::path::PathBuf;

use crate::config::tree::{Gpg, gpg};
use gix_error::ResultExt;

pub use gix_object::signature::{
    Format,
    verify::{Outcome, Status, TrustLevel},
};

/// The error returned by [`crate::Commit::verify_signature()`].
pub type Error = gix_error::Error;

pub(crate) fn verify(commit: &crate::Commit<'_>) -> Result<Option<Outcome>, Error> {
    let Some((signature, signed_data)) = commit
        .signature()
        .or_raise(|| gix_error::message("Could not decode the commit signature"))?
    else {
        return Ok(None);
    };
    let config = commit.repo.config_snapshot();
    let minimum_trust = config
        .string(Gpg::MIN_TRUST_LEVEL)
        .map(|value| Gpg::MIN_TRUST_LEVEL.try_into_trust_level(value))
        .transpose()
        .or_raise(|| gix_error::message("The configured minimum signature trust level is invalid"))?
        .unwrap_or_default();
    let format = Format::from_signature(&signature)
        .ok_or_else(|| Error::from_error(gix_error::CorruptionError::new("The signature format is unsupported")))?;
    let program = super::signature_program(&config, format)
        .or_raise(|| gix_error::message("Could not interpolate the configured signature-verification program path"))?;
    let options = match format {
        Format::OpenPgp => gix_object::signature::verify::Options::OpenPgp {
            program,
            program_arguments: Vec::new(),
            environment: Vec::new(),
            minimum_trust,
        },
        Format::X509 => gix_object::signature::verify::Options::X509 {
            program,
            program_arguments: Vec::new(),
            environment: Vec::new(),
            minimum_trust,
        },
        Format::Ssh => {
            let allowed_signers = config
                .trusted_path(gpg::Ssh::ALLOWED_SIGNERS_FILE)
                .or_raise(|| gix_error::message("Could not interpolate a configured signature-verification path"))?
                .ok_or_else(|| {
                    Error::from_error(gix_error::message(
                        "gpg.ssh.allowedSignersFile must be configured for SSH signature verification",
                    ))
                })?;
            let revocation_file = config
                .trusted_path(gpg::Ssh::REVOCATION_FILE)
                .or_raise(|| gix_error::message("Could not interpolate a configured signature-verification path"))?
                .filter(|path| path.exists());
            gix_object::signature::verify::Options::Ssh {
                program,
                program_arguments: Vec::new(),
                environment: Vec::new(),
                allowed_signers: resolve_relative_to_repository(commit.repo, allowed_signers),
                revocation_file: revocation_file.map(|path| resolve_relative_to_repository(commit.repo, path)),
                verify_time: commit.time()?,
                minimum_trust,
            }
        }
    };
    let outcome = signed_data
        .verify(&signature, options)
        .or_raise(|| gix_error::message("Could not verify the commit signature"))?;
    Ok(Some(outcome))
}

fn resolve_relative_to_repository(repo: &crate::Repository, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        repo.workdir().unwrap_or_else(|| repo.git_dir()).join(path)
    }
}
