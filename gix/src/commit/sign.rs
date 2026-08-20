use std::{ffi::OsString, path::PathBuf, process::Stdio};

use crate::{
    bstr::{BString, ByteSlice},
    config::tree::{Gpg, Key, User, gpg},
};

use gix_object::signature::sign::is_literal_ssh_key;
pub use gix_object::signature::{Format, sign::Options};

/// Errors encountered when applying resolved signing options to a commit.
#[derive(Debug, thiserror::Error)]
#[expect(missing_docs)]
pub enum Error {
    #[error(transparent)]
    Options(#[from] options::Error),
    #[error(transparent)]
    Decode(#[from] gix_object::decode::Error),
    #[error(transparent)]
    Sign(#[from] gix_error::Error),
    #[error(transparent)]
    WriteObject(#[from] crate::object::write::Error),
    #[error(transparent)]
    FindCommit(#[from] crate::object::find::existing::with_conversion::Error),
}

/// Errors encountered when resolving commit-signing options.
pub mod options {
    use std::ffi::OsString;

    use crate::bstr::BString;

    /// The error returned by [`crate::Repository::commit_signing_options()`].
    #[derive(Debug, thiserror::Error)]
    #[expect(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        ConfigBoolean(#[from] crate::config::boolean::Error),
        #[error(transparent)]
        ParseTime(#[from] crate::config::time::Error),
        #[error("Unsupported value for gpg.format: {0:?}")]
        UnsupportedFormat(BString),
        #[error("Committer identity is not configured and user.signingKey is unset")]
        MissingCommitter,
        #[error("user.signingKey or gpg.ssh.defaultKeyCommand must provide an SSH signing key")]
        MissingSshSigningKey,
        #[error("Could not interpolate a configured commit-signing path")]
        ConfiguredPath(#[from] gix_error::Error),
        #[error("Could not execute gpg.ssh.defaultKeyCommand {program:?}")]
        DefaultKeyCommand {
            program: OsString,
            #[source]
            source: std::io::Error,
        },
        #[error("gpg.ssh.defaultKeyCommand failed: {0:?}")]
        DefaultKeyCommandFailed(BString),
        #[error("gpg.ssh.defaultKeyCommand returned an invalid key: {0:?}")]
        InvalidDefaultKey(BString),
    }
}

pub(crate) fn sign<'repo>(commit: &crate::Commit<'repo>) -> Result<crate::Commit<'repo>, Error> {
    let options = commit.repo.commit_signing_options()?;
    let signed = commit.decode()?.sign(options).map_err(gix_error::Exn::into_error)?;
    let id = commit.repo.write_object(&signed)?;
    Ok(commit.repo.find_commit(id)?)
}

pub(crate) fn signing_options(repo: &crate::Repository) -> Result<Options, options::Error> {
    use options::Error;

    let config = repo.config_snapshot();
    let format = config
        .string(Gpg::FORMAT)
        .unwrap_or_else(|| Gpg::FORMAT.default_value_or_panic().into());
    let format = parse_format(format.trim()).ok_or(Error::UnsupportedFormat(format))?;
    let program = match format {
        Format::OpenPgp => match config.trusted_path(gpg::OpenPgp::PROGRAM)? {
            Some(program) => program.into_os_string(),
            None => config
                .trusted_path(Gpg::PROGRAM)?
                .map_or_else(|| default_program(&gpg::OpenPgp::PROGRAM), PathBuf::into_os_string),
        },
        Format::X509 => config
            .trusted_path(gpg::X509::PROGRAM)?
            .map_or_else(|| default_program(&gpg::X509::PROGRAM), PathBuf::into_os_string),
        Format::Ssh => config
            .trusted_path(gpg::Ssh::PROGRAM)?
            .map_or_else(|| default_program(&gpg::Ssh::PROGRAM), PathBuf::into_os_string),
    };
    let signing_key = match config
        .plumbing()
        .string_filter(User::SIGNING_KEY, &mut repo.filter_config_section())
    {
        Some(key) if !key.is_empty() && format == Format::Ssh && is_literal_ssh_key(&key).is_none() => config
            .trusted_path(User::SIGNING_KEY)?
            .map_or_else(|| gix_path::from_bstring(key).into_os_string(), PathBuf::into_os_string),
        Some(key) if !key.is_empty() => gix_path::from_bstring(key).into_os_string(),
        _ if format == Format::Ssh => default_ssh_key(&config)?.ok_or(Error::MissingSshSigningKey)?,
        _ => {
            let committer = repo.committer().ok_or(Error::MissingCommitter)??;
            let mut identity = committer.name.to_owned();
            identity.extend_from_slice(b" <");
            identity.extend_from_slice(committer.email);
            identity.extend_from_slice(b">");
            gix_path::from_bstring(identity).into_os_string()
        }
    };
    Ok(Options {
        format,
        program,
        program_arguments: Vec::new(),
        signing_key,
        environment: Vec::new(),
    })
}

pub(crate) fn signing_options_if_enabled(repo: &crate::Repository) -> Result<Option<Options>, options::Error> {
    repo.config
        .may_sign_commits()?
        .then(|| signing_options(repo))
        .transpose()
}

fn default_program(key: &crate::config::tree::keys::Program) -> OsString {
    gix_path::from_bstr(key.default_value_or_panic())
        .into_owned()
        .into_os_string()
}

fn parse_format(value: &[u8]) -> Option<Format> {
    if value.eq_ignore_ascii_case(b"openpgp") {
        Some(Format::OpenPgp)
    } else if value.eq_ignore_ascii_case(b"x509") {
        Some(Format::X509)
    } else if value.eq_ignore_ascii_case(b"ssh") {
        Some(Format::Ssh)
    } else {
        None
    }
}

fn default_ssh_key(config: &crate::config::Snapshot<'_>) -> Result<Option<OsString>, options::Error> {
    use options::Error;

    let Some(program) = config.trusted_program(gpg::Ssh::DEFAULT_KEY_COMMAND) else {
        return Ok(None);
    };
    let output = gix_command::prepare(&program)
        .command_may_be_shell_script()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| Error::DefaultKeyCommand {
            program: program.clone(),
            source,
        })?
        .wait_with_output()
        .map_err(|source| Error::DefaultKeyCommand {
            program: program.clone(),
            source,
        })?;
    if !output.status.success() {
        return Err(Error::DefaultKeyCommandFailed(output.stderr.into()));
    }
    let key = output.stdout.as_bstr().lines().next().unwrap_or_default().trim();
    if is_literal_ssh_key(key).is_none() {
        return Err(Error::InvalidDefaultKey(BString::from(key)));
    }
    Ok(Some(gix_path::from_bstr(key.as_bstr()).into_owned().into_os_string()))
}
