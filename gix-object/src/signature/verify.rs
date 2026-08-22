use std::{
    // defensive, as we rely on English when parsing output.
    ffi::{OsStr, OsString},
    io::Write,
    path::PathBuf,
    process::Stdio,
};

use bstr::{BStr, BString, ByteSlice};
use gix_error::{CorruptionError, ErrorExt, OptionExt, ResultExt, ValidationError, message};

use super::SignedData;

use super::Format;

/// Fully resolved options for verifying a commit or annotated-tag signature.
#[derive(Clone, Debug)]
pub enum Options {
    /// Verify an OpenPGP signature.
    OpenPgp {
        /// The external verification program or command.
        program: OsString,
        /// Additional arguments passed before Git's fixed arguments.
        ///
        /// Useful for selecting an alternate key store or configuring a wrapper around the verifier.
        program_arguments: Vec<OsString>,
        /// Environment variables set only for the verifier.
        environment: Vec<(OsString, OsString)>,
        /// The minimum trust required for [`Outcome::is_valid()`] to return `true`.
        minimum_trust: TrustLevel,
    },
    /// Verify an X.509 signature.
    X509 {
        /// The external verification program or command.
        program: OsString,
        /// Additional arguments passed before Git's fixed arguments, useful for selecting an alternate key store or
        /// configuring a wrapper around the verifier.
        program_arguments: Vec<OsString>,
        /// Environment variables set only for the verifier.
        environment: Vec<(OsString, OsString)>,
        /// The minimum trust required for [`Outcome::is_valid()`] to return `true`.
        minimum_trust: TrustLevel,
    },
    /// Verify an SSH signature.
    Ssh {
        /// The external verification program or command.
        program: OsString,
        /// Additional arguments passed before Git's fixed arguments, useful for selecting an alternate key store or
        /// configuring a wrapper around the verifier.
        program_arguments: Vec<OsString>,
        /// Environment variables set only for the verifier.
        environment: Vec<(OsString, OsString)>,
        /// The allowed-signers file.
        allowed_signers: PathBuf,
        /// An optional revocation file.
        revocation_file: Option<PathBuf>,
        /// The signature creation time passed to `ssh-keygen` as `-Overify-time` when evaluating `valid-after` and
        /// `valid-before` constraints in the allowed-signers file.
        ///
        /// For commits this should be the committer timestamp, so key rotation or expiry does not invalidate a
        /// signature created while the key was authorized.
        verify_time: gix_date::Time,
        /// The minimum trust required for [`Outcome::is_valid()`] to return `true`.
        minimum_trust: TrustLevel,
    },
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
    /// The verifier considers the signing key's claimed identity marginally valid under its configured trust model.
    ///
    /// The signature may be cryptographically correct, but the verifier has less confidence in the association between
    /// the key and its claimed identity than at [`TrustLevel::Fully`].
    Marginal,
    /// The verifier considers the signing key's claimed identity fully valid under its configured trust model.
    ///
    /// This describes confidence in the key-to-identity association, not greater cryptographic strength. For SSH
    /// signatures, this implementation reports this level when the signature is valid for a principal found in the
    /// allowed-signers file.
    Fully,
    /// The verifier's highest trust level for the signing key's claimed identity.
    ///
    /// For OpenPGP this commonly identifies one's own key or a key explicitly granted ultimate trust. This
    /// implementation does not assign this level to SSH signatures.
    Ultimate,
}

/// The complete result of verifying a commit or annotated-tag signature.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Outcome {
    /// The detected signature format and therefore the verifier whose output populated the remaining fields.
    pub format: Format,
    /// The cryptographic status parsed from GPG/GPGSM status records or from `ssh-keygen`'s success output.
    pub status: Status,
    /// The trust reported by GPG/GPGSM. For SSH this is [`TrustLevel::Fully`] for a principal in the allowed-signers
    /// file and [`TrustLevel::Undefined`] for an otherwise valid signature from an unknown key.
    pub trust_level: TrustLevel,
    /// The GPG/GPGSM user ID or the SSH principal found in the allowed-signers file, if available.
    pub signer: Option<BString>,
    /// The GPG/GPGSM key ID. For SSH this is the same value as [`Outcome::fingerprint`].
    pub key: Option<BString>,
    /// The GPG/GPGSM signing-key fingerprint or the fingerprint printed by `ssh-keygen`, if available.
    pub fingerprint: Option<BString>,
    /// The GPG/GPGSM primary-key fingerprint, if reported; always `None` for SSH.
    pub primary_key_fingerprint: Option<BString>,
    /// Human-readable GPG/GPGSM stderr, or the available `ssh-keygen` stdout and stderr concatenated in that order.
    pub output: BString,
    /// GPG/GPGSM `--status-fd` output. SSH has no separate machine-readable channel, so this equals [`Outcome::output`].
    pub raw_output: BString,
    valid: bool,
}

impl TrustLevel {
    /// Parse a Git trust-level name case-insensitively, or return `None` if it is unknown.
    pub fn from_bytes(value: &[u8]) -> Option<Self> {
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
}

impl Outcome {
    /// Return `true` if Git would accept the signature with the configured minimum trust level.
    pub fn is_valid(&self) -> bool {
        self.valid
    }
}

/// The error returned when verifying an object signature.
pub type Error = gix_error::Exn;

impl SignedData<'_> {
    /// Verify `signature` over these exact object bytes with fully resolved `options`.
    pub fn verify(&self, signature: &BStr, options: Options) -> Result<Outcome, Error> {
        let format = Format::from_signature(signature)
            .ok_or_raise_erased(|| CorruptionError::new("The signature format is unsupported"))?;
        match options {
            Options::OpenPgp {
                program,
                program_arguments,
                environment,
                minimum_trust,
            } if format == Format::OpenPgp => self.verify_gpg(
                signature,
                Format::OpenPgp,
                program,
                program_arguments,
                environment,
                minimum_trust,
            ),
            Options::X509 {
                program,
                program_arguments,
                environment,
                minimum_trust,
            } if format == Format::X509 => self.verify_gpg(
                signature,
                Format::X509,
                program,
                program_arguments,
                environment,
                minimum_trust,
            ),
            Options::Ssh {
                program,
                program_arguments,
                environment,
                allowed_signers,
                revocation_file,
                verify_time,
                minimum_trust,
            } if format == Format::Ssh => self.verify_ssh(
                signature,
                program,
                program_arguments,
                environment,
                allowed_signers,
                revocation_file,
                verify_time,
                minimum_trust,
            ),
            Options::OpenPgp { .. } => Err(ValidationError::new(format!(
                "The configured program format {:?} does not match signature format {format:?}",
                Format::OpenPgp
            ))
            .raise_erased()),
            Options::X509 { .. } => Err(ValidationError::new(format!(
                "The configured program format {:?} does not match signature format {format:?}",
                Format::X509
            ))
            .raise_erased()),
            Options::Ssh { .. } => Err(ValidationError::new(format!(
                "The configured program format {:?} does not match signature format {format:?}",
                Format::Ssh
            ))
            .raise_erased()),
        }
    }

    fn verify_gpg(
        &self,
        signature: &BStr,
        format: Format,
        program: OsString,
        program_arguments: Vec<OsString>,
        environment: Vec<(OsString, OsString)>,
        minimum_trust: TrustLevel,
    ) -> Result<Outcome, Error> {
        let mut signature_file = signature_file(signature)?;
        let path = signature_path(&mut signature_file)?;
        let mut command = prepare(&program, program_arguments, &environment);
        if format == Format::OpenPgp {
            command = command.arg("--keyid-format=long");
        }
        let command = command.args(["--status-fd=1", "--verify"]).arg(path);
        let output = if format == Format::X509 {
            let mut signed_file = temporary_file(self.segments())?;
            let signed_path = signature_path(&mut signed_file)?;
            run_without_input(
                command
                    .arg(signed_path)
                    .stdin(Stdio::null())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped()),
                &program,
            )?
        } else {
            self.run(
                command
                    .arg("-")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped()),
                &program,
            )?
        };
        let mut outcome = parse_gpg_output(format, output.stderr.into(), output.stdout.into());
        outcome.valid =
            output.status.success() && outcome.status == Status::Good && outcome.trust_level >= minimum_trust;
        Ok(outcome)
    }

    #[expect(clippy::too_many_arguments)]
    fn verify_ssh(
        &self,
        signature: &BStr,
        program: OsString,
        program_arguments: Vec<OsString>,
        mut environment: Vec<(OsString, OsString)>,
        allowed_signers: PathBuf,
        revocation_file: Option<PathBuf>,
        verify_time: gix_date::Time,
        minimum_trust: TrustLevel,
    ) -> Result<Outcome, Error> {
        let verify_time = verify_time
            .format(gix_date::time::CustomFormat::new("%Y%m%d%H%M%S"))
            .or_raise_erased(|| message("Signature time could not be formatted for SSH verification"))?;
        let verify_time = format!("-Overify-time={verify_time}");
        let mut signature_file = signature_file(signature)?;
        let signature_path = signature_path(&mut signature_file)?;
        // defensive, as we rely on English when parsing output.
        environment.extend([("LANG".into(), "C".into()), ("LC_ALL".into(), "C".into())]);
        let common = (
            program.as_os_str(),
            program_arguments.as_slice(),
            environment.as_slice(),
        );
        let principals = run_prepared(
            common,
            [
                "-Y".into(),
                "find-principals".into(),
                "-f".into(),
                allowed_signers.as_os_str().into(),
                "-s".into(),
                signature_path.as_os_str().into(),
                verify_time.as_str().into(),
            ],
            &[],
        )?;
        let mut final_output = None;
        let mut signer = None;
        if principals.status.success() {
            for principal in principals.stdout.lines().filter(|line| !line.trim().is_empty()) {
                let principal = OsString::from(String::from_utf8_lossy(principal.trim()).as_ref());
                let mut args = vec![
                    "-Y".into(),
                    "verify".into(),
                    "-n".into(),
                    "git".into(),
                    "-f".into(),
                    allowed_signers.as_os_str().into(),
                    "-I".into(),
                    principal.clone(),
                    "-s".into(),
                    signature_path.as_os_str().into(),
                    verify_time.as_str().into(),
                ];
                if let Some(revocation_file) = &revocation_file {
                    args.extend(["-r".into(), revocation_file.as_os_str().into()]);
                }
                let output = self.run_prepared(common, args)?;
                if output.status.success() && output.stdout.starts_with(b"Good") {
                    signer = Some(principal.to_string_lossy().as_bytes().into());
                    final_output = Some(output);
                    break;
                }
                final_output = Some(output);
            }
        }
        let (output, trust_level, command_success) = match final_output {
            Some(output) => {
                let command_success = output.status.success();
                (output, TrustLevel::Fully, command_success)
            }
            None => (
                self.run_prepared(
                    common,
                    [
                        "-Y".into(),
                        "check-novalidate".into(),
                        "-n".into(),
                        "git".into(),
                        "-s".into(),
                        signature_path.as_os_str().into(),
                        verify_time.as_str().into(),
                    ],
                )?,
                TrustLevel::Undefined,
                false,
            ),
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

    fn run(&self, command: gix_command::Prepare, program: &OsStr) -> Result<std::process::Output, Error> {
        let mut child = command
            .spawn()
            .or_raise_erased(|| message!("Could not execute signature verifier {program:?}"))?;
        let mut stdin = child.stdin.take().expect("configured as piped");
        let [before, after] = self.segments();
        if let Err(source) = stdin.write_all(before).and_then(|_| stdin.write_all(after)) {
            // A verifier may reject the invocation and exit without consuming all input. Its status and output are
            // still authoritative, whereas other write failures indicate an actual communication problem.
            if source.kind() != std::io::ErrorKind::BrokenPipe {
                return Err(source
                    .and_raise(message!("Could not communicate with signature verifier {program:?}"))
                    .erased());
            }
        }
        drop(stdin);
        child
            .wait_with_output()
            .or_raise_erased(|| message!("Could not communicate with signature verifier {program:?}"))
    }

    fn run_prepared(
        &self,
        common: (&OsStr, &[OsString], &[(OsString, OsString)]),
        args: impl IntoIterator<Item = OsString>,
    ) -> Result<std::process::Output, Error> {
        let (program, program_arguments, environment) = common;
        self.run(
            prepare(program, program_arguments.iter().cloned(), environment)
                .args(args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped()),
            program,
        )
    }
}

fn prepare(
    program: &OsStr,
    arguments: impl IntoIterator<Item = OsString>,
    environment: &[(OsString, OsString)],
) -> gix_command::Prepare {
    environment.iter().fold(
        gix_command::prepare(program)
            .command_may_be_shell_script()
            .args(arguments),
        |command, (key, value)| command.env(key, value),
    )
}

fn run_prepared(
    common: (&OsStr, &[OsString], &[(OsString, OsString)]),
    args: impl IntoIterator<Item = OsString>,
    input: &[u8],
) -> Result<std::process::Output, Error> {
    let (program, program_arguments, environment) = common;
    let command = prepare(program, program_arguments.iter().cloned(), environment)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .or_raise_erased(|| message!("Could not execute signature verifier {program:?}"))?;
    child
        .stdin
        .take()
        .expect("configured as piped")
        .write_all(input)
        .or_raise_erased(|| message!("Could not communicate with signature verifier {program:?}"))?;
    child
        .wait_with_output()
        .or_raise_erased(|| message!("Could not communicate with signature verifier {program:?}"))
}

fn signature_file(signature: &BStr) -> Result<gix_tempfile::Handle<gix_tempfile::handle::Writable>, Error> {
    temporary_file([signature.as_ref()])
}

fn temporary_file<'a>(
    data: impl IntoIterator<Item = &'a [u8]>,
) -> Result<gix_tempfile::Handle<gix_tempfile::handle::Writable>, Error> {
    let mut file = gix_tempfile::new(
        std::env::temp_dir(),
        gix_tempfile::ContainingDirectory::Exists,
        gix_tempfile::AutoRemove::Tempfile,
    )
    .or_raise_erased(|| message("Could not create or write the temporary signature file"))?;
    file.with_mut(|file| {
        for data in data {
            file.write_all(data)?;
        }
        Ok::<_, std::io::Error>(())
    })
    .or_raise_erased(|| message("Could not create or write the temporary signature file"))?
    .or_raise_erased(|| message("Could not create or write the temporary signature file"))?;
    Ok(file)
}

fn signature_path(file: &mut gix_tempfile::Handle<gix_tempfile::handle::Writable>) -> Result<PathBuf, Error> {
    file.with_mut(|file| file.path().to_owned())
        .or_raise_erased(|| message("Could not create or write the temporary signature file"))
}

fn run_without_input(command: gix_command::Prepare, program: &OsStr) -> Result<std::process::Output, Error> {
    command
        .spawn()
        .or_raise_erased(|| message!("Could not execute signature verifier {program:?}"))?
        .wait_with_output()
        .or_raise_erased(|| message!("Could not communicate with signature verifier {program:?}"))
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
            outcome.trust_level = TrustLevel::from_bytes(value.split(|byte| *byte == b' ').next().unwrap_or_default())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gpg_status() {
        let Outcome {
            format: _,
            status,
            trust_level,
            signer,
            key: _,
            fingerprint,
            primary_key_fingerprint,
            output: _,
            raw_output: _,
            valid: _,
        } = parse_gpg_output(
            Format::OpenPgp,
            "Good signature".into(),
            "[GNUPG:] GOODSIG 0123456789ABCDEF Fixture Signer\n\
             [GNUPG:] VALIDSIG FINGERPRINT 0 0 0 0 0 0 0 0 PRIMARY\n\
             [GNUPG:] TRUST_FULLY 0 pgp\n"
                .into(),
        );
        assert_eq!(status, Status::Good, "the signature is good");
        assert_eq!(trust_level, TrustLevel::Fully, "the key is fully trusted");
        assert_eq!(
            signer.as_ref().map(|value| value.as_slice()),
            Some(b"Fixture Signer".as_slice()),
            "the signer identity is parsed"
        );
        assert_eq!(
            fingerprint.as_ref().map(|value| value.as_slice()),
            Some(b"FINGERPRINT".as_slice()),
            "the signing-key fingerprint is parsed"
        );
        assert_eq!(
            primary_key_fingerprint.as_ref().map(|value| value.as_slice()),
            Some(b"PRIMARY".as_slice()),
            "the primary-key fingerprint is parsed"
        );
    }

    #[test]
    fn parses_ssh_output() {
        let Outcome {
            format,
            status,
            trust_level,
            signer,
            key,
            fingerprint,
            primary_key_fingerprint,
            output,
            raw_output,
            valid,
        } = parse_ssh_output(
            "Good \"git\" signature for Fixture Signer with ED25519 key SHA256:fixture\n".into(),
            Some("Fixture Signer".into()),
            TrustLevel::Fully,
        );
        assert_eq!(format, Format::Ssh, "the signature format is SSH");
        assert_eq!(status, Status::Good, "Git's success output is recognized");
        assert_eq!(trust_level, TrustLevel::Fully, "the supplied trust level is retained");
        assert_eq!(signer, Some("Fixture Signer".into()), "the supplied signer is retained");
        assert_eq!(key, Some("SHA256:fixture".into()), "the key fingerprint is parsed");
        assert_eq!(fingerprint, key, "the key and fingerprint are identical for SSH");
        assert_eq!(primary_key_fingerprint, None, "SSH has no primary-key fingerprint");
        assert_eq!(output, raw_output, "SSH has no separate machine-readable output");
        assert!(!valid, "parsing alone does not establish validity");

        assert_eq!(
            parse_ssh_output("invalid signature".into(), None, TrustLevel::Undefined).status,
            Status::Bad,
            "all output not starting with Git's success marker is bad"
        );
    }

    #[test]
    fn rejects_good_ssh_output_from_a_failed_verifier() -> Result<(), Error> {
        let signed = SignedData::new(b"payloadsignature", 7..16);
        let outcome = signed.verify(
            BStr::new(b"-----BEGIN SSH SIGNATURE-----\n"),
            Options::Ssh {
                program: r#"if [ "$2" = find-principals ]; then printf 'fixture\n'; else printf 'Good "git" signature for fixture with ED25519 key SHA256:fixture\n'; exit 1; fi # "$@""#.into(),
                program_arguments: Vec::new(),
                environment: Vec::new(),
                allowed_signers: "unused".into(),
                revocation_file: None,
                verify_time: gix_date::Time::default(),
                minimum_trust: TrustLevel::Undefined,
            },
        )?;
        assert_eq!(outcome.status, Status::Good, "the verifier's text is retained");
        assert!(!outcome.is_valid(), "a failed verifier cannot produce a valid outcome");
        Ok(())
    }

    #[test]
    fn parses_signature_formats_and_trust_levels() {
        for (signature, expected) in [
            (b"-----BEGIN PGP SIGNATURE-----".as_slice(), Format::OpenPgp),
            (b"-----BEGIN PGP MESSAGE-----".as_slice(), Format::OpenPgp),
            (b"-----BEGIN SIGNED MESSAGE-----".as_slice(), Format::X509),
            (b"-----BEGIN SSH SIGNATURE-----".as_slice(), Format::Ssh),
        ] {
            assert_eq!(Format::from_signature(signature), Some(expected));
        }
        assert_eq!(Format::from_signature(b"not a signature"), None);

        for (name, expected) in [
            (b"undefined".as_slice(), TrustLevel::Undefined),
            (b"NEVER".as_slice(), TrustLevel::Never),
            (b"Marginal".as_slice(), TrustLevel::Marginal),
            (b"fully".as_slice(), TrustLevel::Fully),
            (b"ultimate".as_slice(), TrustLevel::Ultimate),
        ] {
            assert_eq!(TrustLevel::from_bytes(name), Some(expected));
        }
        assert_eq!(TrustLevel::from_bytes(b"unknown"), None);
    }

    #[test]
    fn rejects_unsupported_and_mismatched_formats_before_running_a_program() {
        let signed = SignedData::new(b"payloadsignature", 7..16);
        let options = Options::X509 {
            program: "must-not-run".into(),
            program_arguments: Vec::new(),
            environment: Vec::new(),
            minimum_trust: TrustLevel::Undefined,
        };
        let unsupported = signed
            .verify(BStr::new(b"not a signature"), options.clone())
            .expect_err("the unsupported signature is rejected");
        assert!(
            unsupported.downcast_any_ref::<CorruptionError>().is_some(),
            "an unrecognized object signature is corrupt"
        );
        let mismatch = signed
            .verify(BStr::new(b"-----BEGIN SSH SIGNATURE-----\n"), options)
            .expect_err("the mismatched signature is rejected");
        assert!(
            mismatch.downcast_any_ref::<ValidationError>().is_some(),
            "a configured verifier which cannot handle the signature is invalid"
        );
        let mismatch = mismatch.to_string();
        assert!(
            mismatch.contains("X509") && mismatch.contains("Ssh"),
            "the mismatch identifies both the configured program and detected signature formats"
        );
    }

    #[test]
    fn commit_time_keeps_the_formatting_error() {
        let signed = SignedData::new(b"payloadsignature", 7..16);
        let err = signed
            .verify(
                BStr::new(b"-----BEGIN SSH SIGNATURE-----\n"),
                Options::Ssh {
                    program: "must-not-run".into(),
                    program_arguments: Vec::new(),
                    environment: Vec::new(),
                    allowed_signers: "unused".into(),
                    revocation_file: None,
                    verify_time: gix_date::Time {
                        seconds: i64::MAX,
                        offset: 0,
                    },
                    minimum_trust: TrustLevel::Undefined,
                },
            )
            .expect_err("the timestamp is outside jiff's supported range");
        assert_eq!(
            err.to_string(),
            "Signature time could not be formatted for SSH verification"
        );
        assert!(err.iter().count() > 1, "the concrete formatting error is retained");
    }
}
