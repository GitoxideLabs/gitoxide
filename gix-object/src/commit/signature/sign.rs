use std::{
    ffi::{OsStr, OsString},
    io::Write,
    path::PathBuf,
    process::Stdio,
};

use bstr::{BString, ByteSlice};

use crate::{Commit, CommitRef, WriteTo};

use super::Format;

/// Options for signing a commit.
#[derive(Clone, Debug)]
pub struct Options {
    /// The signature format.
    pub format: Format,
    /// The external signing program or command.
    pub program: OsString,
    /// Additional arguments passed to the signing program before Git's fixed arguments.
    pub program_arguments: Vec<OsString>,
    /// The key, identity, or key path passed to the signing program.
    ///
    /// SSH key paths must already be resolved; this plumbing layer does not perform Git-style path interpolation.
    pub signing_key: OsString,
    /// Environment variables set only for the signing program.
    pub environment: Vec<(OsString, OsString)>,
}

/// The error returned when signing a commit.
#[derive(Debug, thiserror::Error)]
#[expect(missing_docs)]
pub enum Error {
    #[error(transparent)]
    Decode(#[from] crate::decode::Error),
    #[error(transparent)]
    Encode(#[from] std::io::Error),
    #[error("A signing key is required")]
    MissingSigningKey,
    #[error("Could not create or write a temporary signing file")]
    TemporaryFile(#[source] std::io::Error),
    #[error("Could not execute signing program {program:?}")]
    Spawn {
        program: OsString,
        #[source]
        source: std::io::Error,
    },
    #[error("Could not communicate with signing program {program:?}")]
    Communicate {
        program: OsString,
        #[source]
        source: std::io::Error,
    },
    #[error("Signing program {program:?} failed: {output}")]
    Failed { program: OsString, output: BString },
    #[error("The OpenPGP/X.509 signer did not report SIG_CREATED")]
    MissingSignatureConfirmation,
    #[error("The SSH signer produced no signature")]
    MissingSshSignature(#[source] std::io::Error),
}

impl CommitRef<'_> {
    /// Return an owned copy of this commit with its active signature replaced by a newly created one.
    pub fn sign(self, options: Options) -> Result<Commit, Error> {
        self.into_owned()?.sign(options)
    }
}

impl Commit {
    /// Return this commit with its active signature replaced by a newly created one.
    pub fn sign(mut self, options: Options) -> Result<Commit, Error> {
        let signature_field = crate::commit::signature_field_name(self.tree.kind());
        self.extra_headers.retain(|(name, _)| name != signature_field);
        let mut payload = Vec::new();
        self.write_to(&mut payload)?;
        let signature = match options.format {
            Format::OpenPgp | Format::X509 => sign_gpg(&payload, &options)?,
            Format::Ssh => sign_ssh(&payload, &options)?,
        };
        self.extra_headers.push((signature_field.into(), signature));
        Ok(self)
    }
}

fn command(options: &Options) -> gix_command::Prepare {
    options.environment.iter().fold(
        gix_command::prepare(&options.program)
            .command_may_be_shell_script()
            .args(&options.program_arguments),
        |command, (key, value)| command.env(key, value),
    )
}

fn sign_gpg(payload: &[u8], options: &Options) -> Result<BString, Error> {
    if options.signing_key.is_empty() {
        return Err(Error::MissingSigningKey);
    }
    let output = run(
        command(options)
            .args([OsStr::new("--status-fd=2"), OsStr::new("-bsau")])
            .arg(&options.signing_key)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()),
        &options.program,
        payload,
    )?;
    if !output.status.success() {
        return Err(Error::Failed {
            program: options.program.clone(),
            output: output.stderr.into(),
        });
    }
    if !output
        .stderr
        .lines()
        .any(|line| line.starts_with(b"[GNUPG:] SIG_CREATED "))
    {
        return Err(Error::MissingSignatureConfirmation);
    }
    Ok(strip_cr_before_lf(output.stdout).into())
}

fn sign_ssh(payload: &[u8], options: &Options) -> Result<BString, Error> {
    if options.signing_key.is_empty() {
        return Err(Error::MissingSigningKey);
    }
    let mut literal_key_file = None;
    let literal_key = options
        .signing_key
        .to_str()
        .and_then(|key| is_literal_ssh_key(key.as_bytes()));
    let (key, literal) = match literal_key {
        Some(key) => {
            let mut file = secure_temporary_file()?;
            write_temporary(&mut file, key)?;
            let path = temporary_path(&mut file)?;
            literal_key_file = Some(file);
            (path.into_os_string(), true)
        }
        // Unlike literal keys, resolved key paths can be passed directly to `ssh-keygen -f`.
        None => (options.signing_key.clone(), false),
    };
    let mut payload_file = secure_temporary_file()?;
    write_temporary(&mut payload_file, payload)?;
    let payload_path = temporary_path(&mut payload_file)?;
    let mut signature_path = payload_path.as_os_str().to_owned();
    signature_path.push(".sig");
    let signature_path = PathBuf::from(signature_path);
    let mut command = command(options).args(["-Y", "sign", "-n", "git", "-f"]).arg(key);
    if literal {
        command = command.arg("-U");
    }
    let output = command
        .arg(&payload_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| Error::Spawn {
            program: options.program.clone(),
            source,
        })?
        .wait_with_output()
        .map_err(|source| Error::Communicate {
            program: options.program.clone(),
            source,
        })?;
    drop(literal_key_file);
    if !output.status.success() {
        return Err(Error::Failed {
            program: options.program.clone(),
            output: output.stderr.into(),
        });
    }
    let signature = std::fs::read(&signature_path).map_err(Error::MissingSshSignature);
    let _ = std::fs::remove_file(signature_path);
    Ok(strip_cr_before_lf(signature?).into())
}

/// Return the SSH public key encoded by Git's literal-key syntax.
///
/// A literal key either starts with `key::`, in which case the prefix is removed, or directly with `ssh-`, in which
/// case it is returned unchanged. All other values are considered paths.
pub fn is_literal_ssh_key(key: &[u8]) -> Option<&[u8]> {
    key.strip_prefix(b"key::")
        .or_else(|| key.starts_with(b"ssh-").then_some(key))
}

/// On Unix, creates a file with 0o600 just like Git.
fn secure_temporary_file() -> Result<gix_tempfile::Handle<gix_tempfile::handle::Writable>, Error> {
    gix_tempfile::new(
        std::env::temp_dir(),
        gix_tempfile::ContainingDirectory::Exists,
        gix_tempfile::AutoRemove::Tempfile,
    )
    .map_err(Error::TemporaryFile)
}

fn write_temporary(file: &mut gix_tempfile::Handle<gix_tempfile::handle::Writable>, data: &[u8]) -> Result<(), Error> {
    file.with_mut(|file| file.write_all(data))
        .map_err(Error::TemporaryFile)?
        .map_err(Error::TemporaryFile)
}

fn temporary_path(file: &mut gix_tempfile::Handle<gix_tempfile::handle::Writable>) -> Result<PathBuf, Error> {
    file.with_mut(|file| file.path().to_owned())
        .map_err(Error::TemporaryFile)
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

/// Normalize signer-produced CRLF line endings to LF before embedding the signature in an object.
///
/// This matches Git and keeps signed object bytes independent of the platform on which the signer ran.
fn strip_cr_before_lf(input: Vec<u8>) -> Vec<u8> {
    let mut output = Vec::with_capacity(input.len());
    let mut bytes = input.into_iter().peekable();
    while let Some(byte) = bytes.next() {
        if byte != b'\r' || bytes.peek() != Some(&b'\n') {
            output.push(byte);
        }
    }
    output
}
