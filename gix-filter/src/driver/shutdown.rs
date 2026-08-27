use bstr::BString;

use crate::driver::State;

///
#[derive(Debug, Copy, Clone)]
pub enum Mode {
    /// Wait for long-running processes after signaling them to shut down by closing their input and output.
    WaitForProcesses,
    /// Do not do anything with long-running processes, which typically allows them to keep running or shut down on their own time.
    /// This is the fastest mode as no synchronization happens at all.
    Ignore,
}

/// Lifecycle
impl State {
    /// Handle long-running processes according to `mode`. If an error occurs, all remaining processes will be ignored automatically.
    /// Return a list of `(process, Option<status>)`
    pub fn shutdown(mut self, mode: Mode) -> Result<Vec<(BString, Option<std::process::ExitStatus>)>, std::io::Error> {
        let mut out = Vec::with_capacity(self.running.len());
        for (cmd, client) in self.running.drain() {
            match mode {
                Mode::WaitForProcesses => {
                    let mut child = client.into_child();
                    let status = child.wait()?;
                    out.push((cmd, Some(status)));
                }
                Mode::Ignore => {
                    out.push((cmd, None));
                }
            }
        }
        Ok(out)
    }
}
