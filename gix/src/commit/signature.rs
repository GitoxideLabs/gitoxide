use std::{
    ffi::{OsStr, OsString},
    io::Write,
    path::PathBuf,
    process::Stdio,
};

use crate::bstr::{BStr, BString, ByteSlice};

/// The format of a cryptographic signature.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Format {
    /// An OpenPGP signature.
    OpenPgp,
    /// An X.509 signature.
    X509,
    /// An SSH signature.
    Ssh,
}

/// The result reported by the signature verifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    /// The signature is cryptographically valid.
    Good,
    /// The signature is invalid.
    Bad,
    /// The verifier could not check the signature.
    Error,
    /// The signature has expired.
    Expired,
    /// The signing key has expired.
    ExpiredKey,
    /// The signing key was revoked.
    RevokedKey,
    /// The verifier returned no recognized result.
    Unknown,
}

/// The trust level reported by the signature verifier.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub enum TrustLevel {
    /// No trust information is available.
    #[default]
    Undefined,
    /// The key must never be trusted.
    Never,
    /// The key is marginally trusted.
    Marginal,
    /// The key is fully trusted.
    Fully,
    /// The key is ultimately trusted.
    Ultimate,
}

/// The complete result of verifying a commit signature.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Outcome {
    /// The signature format.
    pub format: Format,
    /// The cryptographic status.
    pub status: Status,
    /// The verifier's trust assessment.
    pub trust_level: TrustLevel,
    /// The signer identity, if reported.
    pub signer: Option<BString>,
    /// The key identifier, if reported.
    pub key: Option<BString>,
    /// The key fingerprint, if reported.
    pub fingerprint: Option<BString>,
    /// The primary-key fingerprint, if reported.
    pub primary_key_fingerprint: Option<BString>,
    /// Human-readable verifier output.
    pub output: BString,
    /// Machine-readable verifier output.
    pub raw_output: BString,
    valid: bool,
}

impl Outcome {
    /// Return `true` if Git would accept the signature with the configured minimum trust level.
    pub fn is_valid(&self) -> bool {
        self.valid
    }
}

/// The error returned by [`crate::Commit::verify_signature()`].
#[derive(Debug, thiserror::Error)]
#[expect(missing_docs)]
pub enum Error {
    #[error(transparent)]
    Decode(#[from] gix_object::decode::Error),
    #[error(transparent)]
    Commit(#[from] crate::object::commit::Error),
    #[error("The signature format is unsupported")]
    UnsupportedFormat,
    #[error("Invalid value for gpg.minTrustLevel: {0:?}")]
    InvalidTrustLevel(BString),
    #[error("Could not interpolate a configured signature-verification path")]
    ConfiguredPath(#[from] gix_config::path::interpolate::Error),
    #[error("gpg.ssh.allowedSignersFile must be configured for SSH signature verification")]
    MissingAllowedSigners,
    #[error("Could not create or write the temporary signature file")]
    TemporaryFile(#[source] std::io::Error),
    #[error("Could not execute signature verifier {program:?}")]
    Spawn {
        program: OsString,
        #[source]
        source: std::io::Error,
    },
    #[error("Could not communicate with signature verifier {program:?}")]
    Communicate {
        program: OsString,
        #[source]
        source: std::io::Error,
    },
    #[error("Commit time could not be formatted for SSH verification")]
    CommitTime(#[from] jiff::Error),
}

pub(crate) fn verify(commit: &crate::Commit<'_>) -> Result<Option<Outcome>, Error> {
    let Some((signature, signed_data)) = commit.signature()? else {
        return Ok(None);
    };
    let format = signature_format(&signature)?;
    let minimum_trust = minimum_trust(commit.repo)?;
    match format {
        Format::OpenPgp | Format::X509 => verify_gpg(
            commit.repo,
            format,
            &signature,
            &signed_data.to_bstring(),
            minimum_trust,
        ),
        Format::Ssh => verify_ssh(commit, &signature, &signed_data.to_bstring(), minimum_trust),
    }
    .map(Some)
}

fn signature_format(signature: &BStr) -> Result<Format, Error> {
    if signature.starts_with_str("-----BEGIN PGP SIGNATURE-----")
        || signature.starts_with_str("-----BEGIN PGP MESSAGE-----")
    {
        Ok(Format::OpenPgp)
    } else if signature.starts_with_str("-----BEGIN SIGNED MESSAGE-----") {
        Ok(Format::X509)
    } else if signature.starts_with_str("-----BEGIN SSH SIGNATURE-----") {
        Ok(Format::Ssh)
    } else {
        Err(Error::UnsupportedFormat)
    }
}

fn minimum_trust(repo: &crate::Repository) -> Result<TrustLevel, Error> {
    let Some(value) = repo.config_snapshot().string("gpg.minTrustLevel") else {
        return Ok(TrustLevel::Undefined);
    };
    parse_trust(value.trim()).ok_or(Error::InvalidTrustLevel(value))
}

fn parse_trust(value: &[u8]) -> Option<TrustLevel> {
    if value.eq_ignore_ascii_case(b"undefined") {
        Some(TrustLevel::Undefined)
    } else if value.eq_ignore_ascii_case(b"never") {
        Some(TrustLevel::Never)
    } else if value.eq_ignore_ascii_case(b"marginal") {
        Some(TrustLevel::Marginal)
    } else if value.eq_ignore_ascii_case(b"fully") {
        Some(TrustLevel::Fully)
    } else if value.eq_ignore_ascii_case(b"ultimate") {
        Some(TrustLevel::Ultimate)
    } else {
        None
    }
}

fn verify_gpg(
    repo: &crate::Repository,
    format: Format,
    signature: &BStr,
    signed_data: &[u8],
    minimum_trust: TrustLevel,
) -> Result<Outcome, Error> {
    let config = repo.config_snapshot();
    let program = match format {
        Format::OpenPgp => config
            .trusted_program("gpg.openpgp.program")
            .or_else(|| config.trusted_program("gpg.program"))
            .unwrap_or_else(|| OsString::from("gpg")),
        Format::X509 => config
            .trusted_program("gpg.x509.program")
            .unwrap_or_else(|| OsString::from("gpgsm")),
        Format::Ssh => unreachable!("SSH uses its own verifier"),
    };
    let mut signature_file = signature_file(signature)?;
    let path = signature_file
        .with_mut(|file| file.path().to_owned())
        .map_err(Error::TemporaryFile)?;
    let command = crate::command::prepare(&program);
    let command = if format == Format::OpenPgp {
        command.arg("--keyid-format=long")
    } else {
        command
    };
    let command = command
        .args(["--status-fd=1", "--verify"])
        .arg(path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = run(command, &program, signed_data)?;
    let mut outcome = parse_gpg_output(format, output.stderr.into(), output.stdout.into());
    outcome.valid = output.status.success() && outcome.status == Status::Good && outcome.trust_level >= minimum_trust;
    Ok(outcome)
}

fn parse_gpg_output(format: Format, output: BString, raw_output: BString) -> Outcome {
    let mut outcome = Outcome {
        format,
        status: Status::Unknown,
        trust_level: TrustLevel::Undefined,
        signer: None,
        key: None,
        fingerprint: None,
        primary_key_fingerprint: None,
        output,
        raw_output,
        valid: false,
    };
    let mut exclusive = false;
    for line in outcome.raw_output.lines() {
        let Some(line) = line.strip_prefix(b"[GNUPG:] ") else {
            continue;
        };
        for (prefix, status) in [
            (b"GOODSIG ".as_slice(), Status::Good),
            (b"BADSIG ".as_slice(), Status::Bad),
            (b"ERRSIG ".as_slice(), Status::Error),
            (b"EXPSIG ".as_slice(), Status::Expired),
            (b"EXPKEYSIG ".as_slice(), Status::ExpiredKey),
            (b"REVKEYSIG ".as_slice(), Status::RevokedKey),
        ] {
            if let Some(value) = line.strip_prefix(prefix) {
                if exclusive {
                    outcome.status = Status::Error;
                    outcome.signer = None;
                    outcome.key = None;
                    break;
                }
                exclusive = true;
                outcome.status = status;
                let mut fields = value.splitn(2, |byte| *byte == b' ');
                outcome.key = fields.next().filter(|value| !value.is_empty()).map(BString::from);
                outcome.signer = fields.next().filter(|value| !value.is_empty()).map(BString::from);
                break;
            }
        }
        if let Some(value) = line.strip_prefix(b"TRUST_") {
            outcome.trust_level = parse_trust(value.split(|byte| *byte == b' ').next().unwrap_or_default())
                .unwrap_or(TrustLevel::Undefined);
        } else if let Some(value) = line.strip_prefix(b"VALIDSIG ") {
            let fields: Vec<_> = value.split(|byte| *byte == b' ').collect();
            outcome.fingerprint = fields
                .first()
                .filter(|value| !value.is_empty())
                .map(|value| BString::from(*value));
            outcome.primary_key_fingerprint = fields
                .get(9)
                .filter(|value| !value.is_empty())
                .map(|value| BString::from(*value));
        }
    }
    outcome
}

fn verify_ssh(
    commit: &crate::Commit<'_>,
    signature: &BStr,
    signed_data: &[u8],
    minimum_trust: TrustLevel,
) -> Result<Outcome, Error> {
    let config = commit.repo.config_snapshot();
    let program = config
        .trusted_program("gpg.ssh.program")
        .unwrap_or_else(|| OsString::from("ssh-keygen"));
    let allowed = config
        .trusted_path("gpg.ssh.allowedSignersFile")?
        .ok_or(Error::MissingAllowedSigners)?;
    let allowed = repository_relative(commit.repo, allowed);
    let revocation = config
        .trusted_path("gpg.ssh.revocationFile")?
        .filter(|path| path.exists())
        .map(|path| repository_relative(commit.repo, path));
    let verify_time = commit
        .time()?
        .format(gix_date::time::CustomFormat::new("%Y%m%d%H%M%S"))?;
    let verify_time = format!("-Overify-time={verify_time}");
    let mut signature_file = signature_file(signature)?;
    let path = signature_file
        .with_mut(|file| file.path().to_owned())
        .map_err(Error::TemporaryFile)?;

    let principals = run_prepared(
        &program,
        [
            OsString::from("-Y"),
            OsString::from("find-principals"),
            OsString::from("-f"),
            allowed.as_os_str().to_owned(),
            OsString::from("-s"),
            path.as_os_str().to_owned(),
            OsString::from(&verify_time),
        ],
        &[],
    )?;
    let mut final_output = None;
    let mut signer = None;
    if principals.status.success() {
        for principal in principals.stdout.lines().filter(|line| !line.trim().is_empty()) {
            let principal = gix_path::from_bstr(principal.trim().as_bstr()).into_owned();
            let mut args = vec![
                OsString::from("-Y"),
                OsString::from("verify"),
                OsString::from("-n"),
                OsString::from("git"),
                OsString::from("-f"),
                allowed.as_os_str().to_owned(),
                OsString::from("-I"),
                principal.clone().into_os_string(),
                OsString::from("-s"),
                path.as_os_str().to_owned(),
                OsString::from(&verify_time),
            ];
            if let Some(revocation) = &revocation {
                args.extend([OsString::from("-r"), revocation.as_os_str().to_owned()]);
            }
            let output = run_prepared(&program, args, signed_data)?;
            if output.status.success() && output.stdout.starts_with(b"Good") {
                signer = Some(BString::from(
                    gix_path::os_str_into_bstr(principal.as_os_str()).unwrap_or_default(),
                ));
                final_output = Some(output);
                break;
            }
            final_output = Some(output);
        }
    }
    let (output, trust_level, command_success) = match final_output {
        Some(output) => (output, TrustLevel::Fully, true),
        None => {
            let output = run_prepared(
                &program,
                [
                    OsString::from("-Y"),
                    OsString::from("check-novalidate"),
                    OsString::from("-n"),
                    OsString::from("git"),
                    OsString::from("-s"),
                    path.as_os_str().to_owned(),
                    OsString::from(&verify_time),
                ],
                signed_data,
            )?;
            (output, TrustLevel::Undefined, false)
        }
    };
    let human = if output.stdout.is_empty() {
        output.stderr
    } else if output.stderr.is_empty() {
        output.stdout
    } else {
        [output.stdout, output.stderr].concat()
    };
    let mut outcome = parse_ssh_output(human.into(), signer, trust_level);
    outcome.valid = command_success && outcome.status == Status::Good && trust_level >= minimum_trust;
    Ok(outcome)
}

fn parse_ssh_output(output: BString, signer: Option<BString>, trust_level: TrustLevel) -> Outcome {
    let status = if output.starts_with(b"Good \"git\" signature") {
        Status::Good
    } else {
        Status::Bad
    };
    let fingerprint = output
        .lines()
        .next()
        .and_then(|line| line.rsplit_once_str(" key "))
        .map(|(_, value)| value.into());
    Outcome {
        format: Format::Ssh,
        status,
        trust_level,
        signer,
        key: fingerprint.clone(),
        fingerprint,
        primary_key_fingerprint: None,
        raw_output: output.clone(),
        output,
        valid: false,
    }
}

fn signature_file(signature: &BStr) -> Result<gix_tempfile::Handle<gix_tempfile::handle::Writable>, Error> {
    let mut file = gix_tempfile::new(
        std::env::temp_dir(),
        gix_tempfile::ContainingDirectory::Exists,
        gix_tempfile::AutoRemove::Tempfile,
    )
    .map_err(Error::TemporaryFile)?;
    file.with_mut(|file| file.write_all(signature))
        .map_err(Error::TemporaryFile)?
        .map_err(Error::TemporaryFile)?;
    Ok(file)
}

fn repository_relative(repo: &crate::Repository, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        repo.workdir().unwrap_or_else(|| repo.git_dir()).join(path)
    }
}

fn run(command: gix_command::Prepare, program: &OsStr, input: &[u8]) -> Result<std::process::Output, Error> {
    let mut child = command.spawn().map_err(|source| Error::Spawn {
        program: program.to_owned(),
        source,
    })?;
    child
        .stdin
        .take()
        .expect("configured as piped")
        .write_all(input)
        .map_err(|source| Error::Communicate {
            program: program.to_owned(),
            source,
        })?;
    child.wait_with_output().map_err(|source| Error::Communicate {
        program: program.to_owned(),
        source,
    })
}

fn run_prepared(
    program: &OsStr,
    args: impl IntoIterator<Item = OsString>,
    input: &[u8],
) -> Result<std::process::Output, Error> {
    let command = crate::command::prepare(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    run(command, program, input)
}
