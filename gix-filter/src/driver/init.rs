use std::process::Stdio;

use bstr::{BStr, BString};

use crate::{
    Driver, driver,
    driver::{Operation, Process, State, process, substitute_f_parameter},
};

/// The error returned by [State::maybe_launch_process()][super::State::maybe_launch_process()].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Failed to spawn driver: {command:?}")]
    SpawnCommand {
        source: std::io::Error,
        command: std::process::Command,
    },
    #[error("Process handshake with command {command:?} failed")]
    ProcessHandshake {
        source: process::client::handshake::Error,
        command: std::process::Command,
    },
}

/// Lifecycle
impl State {
    /// Obtain a process as defined in `driver` suitable for a given `operation. `rela_path` may be used to substitute the current
    /// file for use in the invoked `SingleFile` process.
    ///
    /// Note that if a long-running process is defined, the `operation` isn't relevant and capabilities are to be checked by the caller.
    pub fn maybe_launch_process(
        &mut self,
        driver: &Driver,
        operation: Operation,
        rela_path: &BStr,
    ) -> Result<Option<Process<'_>>, Error> {
        match driver.process.as_ref() {
            Some(process) => {
                let client = match self.running.remove(process) {
                    Some(c) => c,
                    None => {
                        let (child, cmd) = spawn_driver(process.clone(), &self.context)?;
                        process::Client::handshake(child, "git-filter", &[2], &["clean", "smudge", "delay"]).map_err(
                            |err| Error::ProcessHandshake {
                                source: err,
                                command: cmd,
                            },
                        )?
                    }
                };

                // TODO: find a way to not have to do this 'borrow-dance'.
                // this strangeness is to workaround the borrowchecker, who otherwise won't let us return a reader. Quite sad :/.
                // One would want to `get_mut()` or insert essentially, but it won't work.
                self.running.insert(process.clone(), client);
                let client = self.running.get_mut(process).expect("just inserted");

                Ok(Some(Process::MultiFile {
                    client,
                    key: driver::Key(process.to_owned()),
                }))
            }
            None => {
                let cmd = match operation {
                    Operation::Clean => driver
                        .clean
                        .as_ref()
                        .map(|cmd| substitute_f_parameter(cmd.as_ref(), rela_path)),

                    Operation::Smudge => driver
                        .smudge
                        .as_ref()
                        .map(|cmd| substitute_f_parameter(cmd.as_ref(), rela_path)),
                };

                let cmd = match cmd {
                    Some(cmd) => cmd,
                    None => return Ok(None),
                };

                let (child, command) = spawn_driver(cmd, &self.context)?;
                Ok(Some(Process::SingleFile { child, command }))
            }
        }
    }
}

fn spawn_driver(
    cmd: BString,
    context: &gix_command::Context,
) -> Result<(std::process::Child, std::process::Command), Error> {
    let mut cmd: std::process::Command = gix_command::prepare(gix_path::from_bstr(cmd).into_owned())
        .command_may_be_shell_script()
        .with_context(context.clone())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .into();
    gix_trace::debug!(cmd = ?cmd, "launching filter driver");
    let child = match cmd.spawn() {
        Ok(child) => child,
        Err(err) => {
            return Err(Error::SpawnCommand {
                source: err,
                command: cmd,
            });
        }
    };
    Ok((child, cmd))
}
