use bstr::BString;
use gix_error::ErrorExt;

use crate::driver::State;

/// The result of shutting down all running filter processes.
#[derive(Debug)]
pub struct Outcome {
    /// Each filter command and its exit status, or `None` if [`Mode::Zombify`] was used.
    pub processes: Vec<(BString, Option<std::process::ExitStatus>)>,
}

/// A filter process that exited unsuccessfully during shutdown.
pub type Error = gix_error::Exn<gix_error::Message>;

impl Outcome {
    /// Return this outcome if all observed processes exited successfully, or the first failure otherwise.
    ///
    /// This is stricter than Git, which ignores a long-running filter's exit status during shutdown after it has
    /// successfully converted all requested input. Callers that require Git-compatible behavior should inspect or
    /// discard the outcome instead.
    pub fn into_result(self) -> Result<Self, Error> {
        if let Some((command, status)) = self.processes.iter().find_map(|(command, status)| {
            status
                .as_ref()
                .filter(|status| !status.success())
                .map(|status| (command, status))
        }) {
            return Err(gix_error::message!("Filter process {command:?} failed with {status}").raise());
        }
        Ok(self)
    }
}

///
#[derive(Debug, Copy, Clone)]
pub enum Mode {
    /// Wait for long-running processes after signaling them to shut down by closing their input and output.
    WaitForProcesses,
    /// Close communication handles without waiting for the child processes.
    /// On Unix, exited children remain zombies until the parent exits or reaps them.
    Zombify,
}

/// Lifecycle
impl State {
    /// Handle long-running processes according to `mode` while leaving this state ready to launch new ones.
    /// If an error occurs, all remaining processes will be ignored automatically.
    /// Return the process outcomes for inspection or conversion into an error with [`Outcome::into_result()`].
    pub fn shutdown(&mut self, mode: Mode) -> Result<Outcome, std::io::Error> {
        let mut out = Vec::with_capacity(self.running.len());
        for (cmd, client) in self.running.drain() {
            match mode {
                Mode::WaitForProcesses => {
                    let mut child = client.into_child();
                    let status = child.wait()?;
                    out.push((cmd, Some(status)));
                }
                Mode::Zombify => {
                    out.push((cmd, None));
                }
            }
        }
        Ok(Outcome { processes: out })
    }
}
