use std::io::Read;

use bstr::ByteSlice;
use gix_error::{ErrorExt, Result, ResultExt, message};

use crate::helper::{Action, Context, NextAction, Outcome};

impl Action {
    /// Send ourselves to the given `write` which is expected to be credentials-helper compatible
    pub fn send(&self, write: &mut dyn std::io::Write) -> std::io::Result<()> {
        match self {
            Action::Get(ctx) => ctx.write_to(write),
            Action::Store(last) | Action::Erase(last) => {
                write.write_all(last).ok();
                write.write_all(b"\n").ok();
                Ok(())
            }
        }
    }
}

/// Invoke the given `helper` with `action` in `context`.
///
/// Usually the first call is performed with [`Action::Get`] to obtain `Some` credentials, which subsequently can be used if they are complete.
/// Note that these may also only contain the username _or_ password, and should start out with everything the helper needs.
/// On successful usage, use [`NextAction::store()`], otherwise [`NextAction::erase()`], which is when this function
/// returns `Ok(None)` as no outcome is expected.
pub fn invoke(helper: &mut crate::Program, action: &Action) -> Result<Option<Outcome>> {
    let options = action.context().map(|ctx| ctx.options).unwrap_or_default();
    match raw(helper, action)? {
        None => Ok(None),
        Some(mut stdout) => {
            let mut ctx = Context::from_bytes(stdout.as_slice(), options)?;
            let caller_supports_authtype = action.context().is_some_and(Context::supports_authtype);
            if !caller_supports_authtype {
                ctx.authtype = None;
                ctx.credential = None;
            }
            let mut authtype_negotiated = false;
            stdout = stdout
                .lines()
                .zip(stdout.lines_with_terminator())
                .take_while(|(line, _)| !line.is_empty())
                .filter(|(line, _)| {
                    if *line == b"capability[]=authtype" {
                        authtype_negotiated = caller_supports_authtype;
                        return authtype_negotiated;
                    }
                    if line.starts_with(b"capability[]=")
                        || line.starts_with(b"state[]=")
                        || line.starts_with(b"continue=")
                    {
                        return false;
                    }
                    authtype_negotiated
                        || (!line.starts_with(b"authtype=")
                            && !line.starts_with(b"credential=")
                            && !line.starts_with(b"ephemeral="))
                })
                .flat_map(|(_, line)| line)
                .copied()
                .collect();
            Ok(Some(Outcome {
                username: ctx.username,
                password: ctx.password,
                authtype: ctx.authtype,
                credential: ctx.credential,
                oauth_refresh_token: ctx.oauth_refresh_token,
                quit: ctx.quit.unwrap_or(false),
                next: NextAction {
                    previous_output: stdout.into(),
                    options,
                },
            }))
        }
    }
}

/// A helper failure that permits trying the next helper, without promising that retrying this one will help.
#[derive(Debug)]
pub(super) struct HelperFailure(std::io::Error);

impl std::fmt::Display for HelperFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Credentials helper failed")
    }
}

impl std::error::Error for HelperFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

pub(crate) fn raw(helper: &mut crate::Program, action: &Action) -> Result<Option<Vec<u8>>> {
    let communication_error = || message("An IO error occurred while communicating to the credentials helper");
    let (mut stdin, stdout) = helper.start(action).or_raise(communication_error)?;
    if let (Action::Get(_), None) = (&action, &stdout) {
        panic!("BUG: `Helper` impls must return an output handle to read output from if Action::Get is provided")
    }
    action.send(&mut stdin).or_raise(communication_error)?;
    drop(stdin);
    let stdout = stdout
        .map(|mut stdout| {
            let mut buf = Vec::new();
            stdout.read_to_end(&mut buf).map(|_| buf)
        })
        .transpose()
        .map_err(|err| HelperFailure(err).raise())?;
    helper.finish().map_err(|err| {
        if err.kind() == std::io::ErrorKind::Other {
            HelperFailure(err).raise()
        } else {
            err.and_raise(communication_error())
        }
    })?;

    match matches!(action, Action::Get(_)).then(|| stdout).flatten() {
        None => Ok(None),
        Some(stdout) => Ok(Some(stdout)),
    }
}
