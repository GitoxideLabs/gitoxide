use gix_error::{Result, bail, message};

use crate::driver::State;

/// The result of shutting down all running filter processes.
#[derive(Debug)]
pub struct Outcome {
    /// Each prepared filter command, including its original launch context, and its exit status,
    /// or `None` if [`Mode::Zombify`] was used.
    pub processes: Vec<(std::process::Command, Option<std::process::ExitStatus>)>,
}

impl Outcome {
    /// Return this outcome if all observed processes exited successfully, or the first failure otherwise.
    ///
    /// This is stricter than Git, which ignores a long-running filter's exit status during shutdown after it has
    /// successfully converted all requested input. Callers that require Git-compatible behavior should inspect or
    /// discard the outcome instead.
    ///
    /// Failures provide [metadata](gix_error::Error::metadata) with `program` (Path), the invoked program name or
    /// path, `command` (String), the prepared command's debug representation, `exit_status` (String), and
    /// `exit_code` (I64) when available.
    pub fn into_result(self) -> Result<Self> {
        if let Some((command, status)) = self.processes.iter().find_map(|(command, status)| {
            status
                .as_ref()
                .filter(|status| !status.success())
                .map(|status| (command, status))
        }) {
            bail!(
                message("Filter process failed")
                    .with_command(command)
                    .with_exit_status(*status)
            );
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
    pub fn shutdown(&mut self, mode: Mode) -> std::result::Result<Outcome, std::io::Error> {
        let mut out = Vec::with_capacity(self.running.len());
        for (_, process) in self.running.drain() {
            match mode {
                Mode::WaitForProcesses => {
                    let mut child = process.client.into_child();
                    let status = child.wait()?;
                    out.push((process.command, Some(status)));
                }
                Mode::Zombify => {
                    out.push((process.command, None));
                }
            }
        }
        Ok(Outcome { processes: out })
    }
}
