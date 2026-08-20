use std::path::PathBuf;

use crate::config::tree::{Gpg, gpg};

pub use gix_object::signature::{
    Format,
    verify::{Outcome, Status, TrustLevel},
};

/// The error returned by [`crate::Commit::verify_signature()`].
#[derive(Debug, thiserror::Error)]
#[expect(missing_docs)]
pub enum Error {
    #[error(transparent)]
    Decode(#[from] gix_object::decode::Error),
    #[error(transparent)]
    Commit(#[from] crate::object::commit::Error),
    #[error(transparent)]
    InvalidTrustLevel(#[from] crate::config::key::GenericErrorWithValue),
    #[error("Could not interpolate a configured signature-verification path")]
    ConfiguredPath(#[from] gix_error::Error),
    #[error("gpg.ssh.allowedSignersFile must be configured for SSH signature verification")]
    MissingAllowedSigners,
    #[error(transparent)]
    Verify(#[from] gix_object::signature::verify::Error),
}

pub(crate) fn verify(commit: &crate::Commit<'_>) -> Result<Option<Outcome>, Error> {
    let Some((signature, signed_data)) = commit.signature()? else {
        return Ok(None);
    };
    let config = commit.repo.config_snapshot();
    let minimum_trust = config
        .string(Gpg::MIN_TRUST_LEVEL)
        .map(|value| Gpg::MIN_TRUST_LEVEL.try_into_trust_level(value))
        .transpose()?
        .unwrap_or_default();
    let format = Format::from_signature(&signature).ok_or(gix_object::signature::verify::Error::UnsupportedFormat)?;
    let program = super::signature_program(&config, format)?;
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
                .trusted_path(gpg::Ssh::ALLOWED_SIGNERS_FILE)?
                .ok_or(Error::MissingAllowedSigners)?;
            let revocation_file = config
                .trusted_path(gpg::Ssh::REVOCATION_FILE)?
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
    signed_data.verify(&signature, options).map(Some).map_err(Into::into)
}

fn resolve_relative_to_repository(repo: &crate::Repository, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        repo.workdir().unwrap_or_else(|| repo.git_dir()).join(path)
    }
}
