use std::{ffi::OsString, path::PathBuf, process::Stdio};

use crate::{
    bstr::ByteSlice,
    config::tree::{Gpg, Key, User, gpg},
};
use gix_error::ResultExt;

use gix_object::signature::sign::is_literal_ssh_key;
pub use gix_object::signature::{Format, sign::Options};

/// Errors encountered when applying resolved signing options to a commit.
pub type Error = gix_error::Error;

/// Errors encountered when resolving commit-signing options.
pub mod options {
    /// The error returned by [`crate::Repository::commit_signing_options()`].
    pub type Error = gix_error::Error;
}

pub(crate) fn sign<'repo>(commit: &crate::Commit<'repo>) -> Result<crate::Commit<'repo>, Error> {
    let options = commit.repo.commit_signing_options()?;
    let signed = commit
        .decode()
        .or_raise(|| gix_error::message("Could not decode the commit before signing"))?
        .sign(options)
        .or_raise(|| gix_error::message("Could not sign the commit"))?;
    let id = commit.repo.write_object(&signed)?;
    commit.repo.find_commit(id)
}

pub(crate) fn signing_options(repo: &crate::Repository) -> Result<Options, options::Error> {
    use options::Error;

    let config = repo.config_snapshot();
    let format = config
        .string(Gpg::FORMAT)
        .unwrap_or_else(|| Gpg::FORMAT.default_value_or_panic().into());
    let format = parse_format(format.trim())
        .ok_or_else(|| Error::from_error(gix_error::message!("Unsupported value for gpg.format: {format:?}")))?;
    let program = match format {
        Format::OpenPgp => match config
            .trusted_path(gpg::OpenPgp::PROGRAM)
            .or_raise(|| gix_error::message("Could not interpolate the configured OpenPGP program path"))?
        {
            Some(program) => program.into_os_string(),
            None => config
                .trusted_path(Gpg::PROGRAM)
                .or_raise(|| gix_error::message("Could not interpolate the configured signing program path"))?
                .map_or_else(|| default_program(&gpg::OpenPgp::PROGRAM), PathBuf::into_os_string),
        },
        Format::X509 => config
            .trusted_path(gpg::X509::PROGRAM)
            .or_raise(|| gix_error::message("Could not interpolate the configured X.509 program path"))?
            .map_or_else(|| default_program(&gpg::X509::PROGRAM), PathBuf::into_os_string),
        Format::Ssh => config
            .trusted_path(gpg::Ssh::PROGRAM)
            .or_raise(|| gix_error::message("Could not interpolate the configured SSH signing program path"))?
            .map_or_else(|| default_program(&gpg::Ssh::PROGRAM), PathBuf::into_os_string),
    };
    let signing_key = match config
        .plumbing()
        .string_filter(User::SIGNING_KEY, &mut repo.filter_config_section())
    {
        Some(key) if !key.is_empty() && format == Format::Ssh && is_literal_ssh_key(&key).is_none() => config
            .trusted_path(User::SIGNING_KEY)
            .or_raise(|| gix_error::message("Could not interpolate the configured commit-signing key path"))?
            .map_or_else(|| gix_path::from_bstring(key).into_os_string(), PathBuf::into_os_string),
        Some(key) if !key.is_empty() => gix_path::from_bstring(key).into_os_string(),
        _ if format == Format::Ssh => default_ssh_key(&config)?.ok_or_else(|| {
            Error::from_error(gix_error::message(
                "user.signingKey or gpg.ssh.defaultKeyCommand must provide an SSH signing key",
            ))
        })?,
        _ => {
            let committer = repo
                .committer()
                .ok_or_else(|| {
                    Error::from_error(gix_error::message(
                        "Committer identity is not configured and user.signingKey is unset",
                    ))
                })?
                .or_raise(|| gix_error::message("Could not parse the committer time for commit signing"))?;
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
    let enabled = repo
        .config
        .may_sign_commits()
        .or_raise(|| gix_error::message("Could not determine whether commit signing is enabled"))?
        .then(|| signing_options(repo));
    enabled.transpose()
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
        .or_raise(|| gix_error::message!("Could not execute gpg.ssh.defaultKeyCommand {program:?}"))?
        .wait_with_output()
        .or_raise(|| gix_error::message!("Could not execute gpg.ssh.defaultKeyCommand {program:?}"))?;
    if !output.status.success() {
        return Err(Error::from_error(gix_error::message!(
            "gpg.ssh.defaultKeyCommand failed: {:?}",
            output.stderr.as_bstr()
        )));
    }
    let key = output.stdout.as_bstr().lines().next().unwrap_or_default().trim();
    if is_literal_ssh_key(key).is_none() {
        return Err(Error::from_error(gix_error::message!(
            "gpg.ssh.defaultKeyCommand returned an invalid key: {key:?}"
        )));
    }
    Ok(Some(gix_path::from_bstr(key.as_bstr()).into_owned().into_os_string()))
}
