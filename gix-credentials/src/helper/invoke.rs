use std::io::Read;

use gix_error::{ErrorExt, ResultExt, RetryableError, message};

use crate::helper::{Action, Context, Error, NextAction, Outcome, Result};

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
/// Usually the first call is performed with [`Action::Get`] to obtain `Some` identity, which subsequently can be used if it is complete.
/// Note that it may also only contain the username _or_ password, and should start out with everything the helper needs.
/// On successful usage, use [`NextAction::store()`], otherwise [`NextAction::erase()`], which is when this function
/// returns `Ok(None)` as no outcome is expected.
pub fn invoke(helper: &mut crate::Program, action: &Action) -> Result {
    let options = action.context().map(|ctx| ctx.options).unwrap_or_default();
    match raw(helper, action)? {
        None => Ok(None),
        Some(stdout) => {
            let ctx = Context::from_bytes(stdout.as_slice(), options).or_erased()?;
            Ok(Some(Outcome {
                username: ctx.username,
                password: ctx.password,
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

pub(crate) fn raw(helper: &mut crate::Program, action: &Action) -> std::result::Result<Option<Vec<u8>>, Error> {
    let communication_error = || message("An IO error occurred while communicating to the credentials helper");
    let (mut stdin, stdout) = helper.start(action).or_raise_erased(communication_error)?;
    if let (Action::Get(_), None) = (&action, &stdout) {
        panic!("BUG: `Helper` impls must return an output handle to read output from if Action::Get is provided")
    }
    action.send(&mut stdin).or_raise_erased(communication_error)?;
    drop(stdin);
    let stdout = stdout
        .map(|mut stdout| {
            let mut buf = Vec::new();
            stdout.read_to_end(&mut buf).map(|_| buf)
        })
        .transpose()
        .map_err(|err| RetryableError::new(err).raise_erased())?;
    helper.finish().map_err(|err| {
        if err.kind() == std::io::ErrorKind::Other {
            RetryableError::new(err).raise_erased()
        } else {
            err.and_raise(communication_error()).erased()
        }
    })?;

    match matches!(action, Action::Get(_)).then(|| stdout).flatten() {
        None => Ok(None),
        Some(stdout) => Ok(Some(stdout)),
    }
}
