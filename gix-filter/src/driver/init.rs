use gix_error::Result;
use std::process::Stdio;

use bstr::{BStr, BString};

use crate::{
    Driver, driver,
    driver::{Operation, Process, RunningProcess, State, process, substitute_f_parameter},
};

/// Lifecycle
impl State {
    /// Obtain a process as defined in `driver` suitable for a given `operation. `rela_path` may be used to substitute the current
    /// file for use in the invoked `SingleFile` process.
    ///
    /// Note that if a long-running process is defined, the `operation` isn't relevant and capabilities are to be checked by the caller.
    ///
    /// Spawn and handshake failures provide [metadata](gix_error::Error::metadata) with `program` (Path), the invoked
    /// program name or path, and `command` (String), the prepared command's debug representation, while preserving
    /// the underlying error as a cause.
    pub fn maybe_launch_process(
        &mut self,
        driver: &Driver,
        operation: Operation,
        rela_path: &BStr,
    ) -> Result<Option<Process<'_>>> {
        match driver.process.as_ref() {
            Some(process) => {
                let running = match self.running.remove(process) {
                    Some(running) => running,
                    None => {
                        let (child, command) = spawn_driver(process.clone(), &self.context)?;
                        use gix_error::{ResultExt, message};
                        let client =
                            process::Client::handshake(child, "git-filter", &[2], &["clean", "smudge", "delay"])
                                .or_raise(|| message("Process handshake failed").with_command(&command))?;
                        RunningProcess { client, command }
                    }
                };

                // TODO: find a way to not have to do this 'borrow-dance'.
                // this strangeness is to workaround the borrowchecker, who otherwise won't let us return a reader. Quite sad :/.
                // One would want to `get_mut()` or insert essentially, but it won't work.
                self.running.insert(process.clone(), running);
                let client = &mut self.running.get_mut(process).expect("just inserted").client;

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

fn spawn_driver(cmd: BString, context: &gix_command::Context) -> Result<(std::process::Child, std::process::Command)> {
    let mut cmd: std::process::Command = gix_command::prepare(gix_path::from_bstr(cmd)?.into_owned())
        .command_may_be_shell_script()
        .with_context(context.clone())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .try_into()?;
    if let Some(worktree_dir) = &context.worktree_dir {
        cmd.current_dir(worktree_dir);
    }
    gix_trace::debug!(cmd = ?cmd, "launching filter driver");
    use gix_error::{ResultExt, message};
    let child = cmd
        .spawn()
        .or_raise(|| message("Failed to spawn driver").with_command(&cmd))?;
    Ok((child, cmd))
}
