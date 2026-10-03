use std::{ffi::OsString, path::PathBuf, process::Stdio};

use crate::error::{ResultExt, bail, message};
use crate::{
    Error, Result,
    bstr::ByteSlice,
    config::tree::{Gpg, User, gpg},
};

use gix_object::signature::sign::is_literal_ssh_key;
pub use gix_object::signature::{Format, sign::Options};

pub(crate) fn sign<'repo>(commit: &crate::Commit<'repo>) -> Result<crate::Commit<'repo>> {
    let options = commit.repo.commit_signing_options()?;
    let signed = commit
        .decode()?
        .sign(options)
        .or_raise(|| message("Could not sign the commit"))?;
    let id = commit.repo.write_object(&signed)?;
    commit.repo.find_commit(id)
}

pub(crate) fn signing_options(repo: &crate::Repository) -> Result<Options> {
    let config = repo.config_snapshot();
    let value = config.string(Gpg::FORMAT);
    let format = Gpg::FORMAT.try_into_signature_format(value.as_ref().map(|value| value.as_bstr()))?;
    let program = super::signature_program(&config, format)
        .or_raise(|| message("Could not interpolate the configured signing program path"))?;
    let signing_key = match config
        .plumbing()
        .string_filter(User::SIGNING_KEY, &mut repo.filter_config_section())
    {
        Some(key) if !key.is_empty() && format == Format::Ssh && is_literal_ssh_key(&key).is_none() => config
            .trusted_path(User::SIGNING_KEY)
            .or_raise(|| message("Could not interpolate the configured commit-signing key path"))?
            .map_or_else(|| gix_path::from_bstring(key).into_os_string(), PathBuf::into_os_string),
        Some(key) if !key.is_empty() => gix_path::from_bstring(key).into_os_string(),
        _ if format == Format::Ssh => default_ssh_key(&config)?.ok_or_else(|| {
            Error::from_error(message(
                "user.signingKey or gpg.ssh.defaultKeyCommand must provide an SSH signing key",
            ))
        })?,
        _ => {
            let committer = repo
                .committer()
                .ok_or_else(|| {
                    Error::from_error(message(
                        "Committer identity is not configured and user.signingKey is unset",
                    ))
                })?
                .or_raise(|| message("Could not parse the committer time for commit signing"))?;
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

pub(crate) fn signing_options_if_enabled(repo: &crate::Repository) -> Result<Option<Options>> {
    let enabled = repo.config.may_sign_commits()?.then(|| signing_options(repo));
    enabled.transpose()
}

fn default_ssh_key(config: &crate::config::Snapshot<'_>) -> Result<Option<OsString>> {
    let Some(program) = config.trusted_program(gpg::Ssh::DEFAULT_KEY_COMMAND) else {
        return Ok(None);
    };
    let mut command: std::process::Command = gix_command::prepare(&program)
        .command_may_be_shell_script()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .into();
    let program_display = if program == command.get_program() {
        String::new()
    } else {
        format!(" {}", program.display())
    };
    let output = command
        .spawn()
        .or_raise(|| {
            message!("Could not execute gpg.ssh.defaultKeyCommand{program_display}").with_program(command.get_program())
        })?
        .wait_with_output()
        .or_raise(|| {
            message!("Could not execute gpg.ssh.defaultKeyCommand{program_display}").with_program(command.get_program())
        })?;
    if !output.status.success() {
        bail!(message("gpg.ssh.defaultKeyCommand failed").with_command_output(&command, output));
    }
    let key = output.stdout.as_bstr().lines().next().unwrap_or_default().trim();
    if is_literal_ssh_key(key).is_none() {
        bail!("gpg.ssh.defaultKeyCommand returned an invalid key: {key:?}");
    }
    Ok(Some(gix_path::from_bstr(key.as_bstr()).into_owned().into_os_string()))
}
