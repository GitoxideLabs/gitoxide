//! Git style prompting with support for `GIT_ASKPASS` and `askpass` program configuration.
//!
//! ### Compatibility
//!
//! This is a unix-only crate which will return with an error when trying to obtain any prompt on other platforms.
//! On those platforms it is common to have helpers which perform this task so it shouldn't be a problem.
#![deny(missing_docs)]
#![forbid(unsafe_code)]

use gix_error::Result;
mod types;
pub use types::{Mode, Options};

///
pub mod unix;
#[cfg(unix)]
use unix::imp;

#[cfg(not(unix))]
mod imp {
    use crate::Options;
    use gix_error::{ErrorExt, Result, message};

    pub(crate) fn ask(_prompt: &str, _opts: &Options) -> Result<String> {
        Err(message("The current platform has no implementation for prompting in the terminal").raise())
    }
}

/// Ask the user given a `prompt`, returning the result.
pub fn ask(prompt: &str, opts: &Options) -> Result<String> {
    ask_with_environment(prompt, opts, &[])
}

/// Ask like [`ask()`], applying `environment` only to the askpass child process.
///
/// Repeated keys use their last value. The current process environment is never modified.
pub fn ask_with_environment(
    prompt: &str,
    opts: &Options,
    environment: &[(std::ffi::OsString, std::ffi::OsString)],
) -> Result<String> {
    if let Some(askpass) = opts.askpass.as_deref() {
        let mut command = gix_command::prepare(askpass).arg(prompt);
        command.env.extend_from_slice(environment);
        match command.spawn() {
            Ok(cmd) => {
                if let Some(mut stdout) = cmd
                    .wait_with_output()
                    .ok()
                    .filter(|out| out.status.success())
                    .and_then(|out| String::from_utf8(out.stdout).ok())
                {
                    if stdout.ends_with('\n') {
                        stdout.pop();
                    }
                    if stdout.ends_with('\r') {
                        stdout.pop();
                    }
                    return Ok(stdout);
                }
            }
            Err(err) => eprintln!(
                "Cannot run askpass program: \"{askpass}\" with error: {err}",
                askpass = askpass.display()
            ),
        }
    }
    imp::ask(prompt, opts)
}

/// Ask for information typed by the user into the terminal after showing the prompt, like `"Username: `.
///
/// Use [`ask()`] for more control.
pub fn openly(prompt: impl AsRef<str>) -> Result<String> {
    imp::ask(
        prompt.as_ref(),
        &Options {
            mode: Mode::Visible,
            askpass: None,
        },
    )
}

/// Ask for information _securely_ after showing the `prompt` (like `"password: "`) by not showing what's typed.
///
/// Use [`ask()`] for more control.
pub fn securely(prompt: impl AsRef<str>) -> Result<String> {
    imp::ask(
        prompt.as_ref(),
        &Options {
            mode: Mode::Hidden,
            askpass: None,
        },
    )
}
