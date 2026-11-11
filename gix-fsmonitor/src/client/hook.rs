use std::process::Stdio;

use bstr::BStr;
use gix_error::{Result, ResultExt, bail, message};

/// A filesystem monitor hook protocol version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    /// Timestamp-based requests with NUL-separated paths in the response.
    One,
    /// Opaque-token requests with a token followed by NUL-separated paths in the response.
    Two,
}

/// Invoke `command` with `previous` as its checkpoint, returning the successful protocol and raw response.
///
/// If `version` is unset, try protocol two first, falling back to one if the command fails.
/// A successful response is returned without validation. If the selected protocols all fail,
/// return `None`, or an error if the final protocol-one invocation cannot be launched.
/// The hook runs in the absolute `worktree_dir` required by `context`.
pub fn query(
    command: &BStr,
    previous: &BStr,
    version: Option<Version>,
    context: gix_command::Context,
) -> Result<Option<(Version, Vec<u8>)>> {
    let _span = gix_trace::coarse!("gix_fsmonitor::client::hook", requested_protocol = ?version);
    let Some(workdir) = context.worktree_dir.as_ref().filter(|path| path.is_absolute()) else {
        bail!(message(
            "Filesystem monitor hooks require an absolute worktree directory"
        ));
    };
    let run = |version: &str| -> Result<std::process::Output> {
        let _span = gix_trace::coarse!("gix_fsmonitor::client::hook::query", protocol = version);
        let command = gix_command::prepare(gix_path::from_bstr(command)?.into_owned())
            .command_may_be_shell_script()
            .with_context(context.clone())
            .arg(version)
            .arg(gix_path::from_bstr(previous)?.into_owned())
            .stdin(Stdio::null())
            .stdout(Stdio::piped());
        // Resolve relative scripts and their interpreters after entering the worktree on Windows.
        let mut command = std::process::Command::try_from(if cfg!(windows) { command.with_shell() } else { command })?;
        let out = command
            .current_dir(workdir)
            .output()
            .inspect_err(|_err| {
                gix_trace::debug!(error_kind = ?_err.kind(), "filesystem monitor hook could not be run");
            })
            .or_raise(|| message("Could not run the filesystem monitor hook"))?;
        gix_trace::debug!(success = out.status.success(), exit_code = ?out.status.code(), response_bytes = out.stdout.len(), "filesystem monitor hook finished");
        Ok(out)
    };
    if version != Some(Version::One) {
        if let Ok(out) = run("2")
            && out.status.success()
        {
            return Ok(Some((Version::Two, out.stdout)));
        }
        if version == Some(Version::Two) {
            return Ok(None);
        }
        gix_trace::debug!("retrying filesystem monitor hook with protocol v1");
    }
    let out = run("1")?;
    Ok(out.status.success().then_some((Version::One, out.stdout)))
}
