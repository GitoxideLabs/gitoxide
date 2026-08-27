use std::collections::HashMap;

use bstr::{BStr, BString, ByteSlice, ByteVec};

///
pub mod init;

///
pub mod apply;

///
pub mod shutdown;

///
pub mod delayed;

///
pub mod process;

/// A literal driver process.
pub enum Process<'a> {
    /// A spawned processes to handle a single file
    SingleFile {
        /// The child to use as handle for sending and receiving data.
        child: std::process::Child,
        /// The launched command that produced the `child` in the first place
        command: std::process::Command,
    },
    /// A multi-file process which is launched once to handle one or more files by using a custom IO protocol.
    MultiFile {
        /// A handle to interact with the long-running process.
        client: &'a mut process::Client,
        /// A way to refer to the `client` later if needed.
        key: Key,
    },
}

/// The kind of operation to apply using a driver
#[derive(Debug, Copy, Clone)]
pub enum Operation {
    /// Turn worktree content into content suitable for storage in `git`.
    Clean,
    /// Turn content stored in `git` to content suitable for the working tree.
    Smudge,
}

impl Operation {
    /// Return a string that identifies the operation. This happens to be the command-names used in long-running processes as well.
    pub fn as_str(&self) -> &'static str {
        match self {
            Operation::Clean => "clean",
            Operation::Smudge => "smudge",
        }
    }
}

/// State required to handle `process` filters, which are running until all their work is done.
///
/// These can be significantly faster on some platforms as they are launched only once, while supporting asynchronous processing.
///
/// ### Lifecycle
///
/// Long-running processes are terminated and waited for when this instance is dropped, so they never
/// linger in the process table of the operating system. Call
/// [`shutdown()`][State::shutdown()] or [`shutdown_mut()`][State::shutdown_mut()] explicitly to learn
/// how each of them exited, or to opt out of waiting with [`shutdown::Mode::Ignore`].
///
/// Note that [`clone()`][Clone::clone()] does *not* clone the running processes, so each clone owns and
/// terminates only the processes that it launched itself.
#[derive(Default)]
pub struct State {
    /// The list of currently running processes. These are preferred over simple clean-and-smudge programs.
    ///
    /// Note that these processes shut down once their stdin/stdout are dropped, but on Unix they keep
    /// occupying a slot in the process table until they are waited for, which is what dropping this
    /// instance does.
    running: HashMap<BString, process::Client>,

    /// The context to pass to spawned filter programs.
    pub context: gix_command::Context,
}

/// Initialization
impl State {
    /// Create a new instance using `context` to inform launched processes about their environment.
    pub fn new(context: gix_command::Context) -> Self {
        Self {
            running: Default::default(),
            context,
        }
    }
}

impl Clone for State {
    fn clone(&self) -> Self {
        State {
            running: Default::default(),
            context: self.context.clone(),
        }
    }
}

impl Drop for State {
    fn drop(&mut self) {
        // Waiting is expected to be brief: dropping a client closes the input and output of its process,
        // which is how it is told to exit. Errors are of no use here as there is nothing left to retry.
        let _ = self.shutdown_mut(shutdown::Mode::WaitForProcesses);
    }
}

/// A way to reference a running multi-file filter process for later acquisition of delayed output.
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Key(BString);

/// Substitute `path` as shell-save version into `cmd` which could be something like `cmd something %f`.
fn substitute_f_parameter(cmd: &BStr, path: &BStr) -> BString {
    let mut buf: BString = Vec::with_capacity(cmd.len()).into();

    let mut ofs = 0;
    while let Some(pos) = cmd[ofs..].find(b"%f") {
        buf.push_str(&cmd[ofs..ofs + pos]);
        buf.extend_from_slice(&gix_quote::single(path));
        ofs += pos + 2;
    }
    buf.push_str(&cmd[ofs..]);
    buf
}

#[cfg(test)]
mod tests {
    use super::substitute_f_parameter;

    #[test]
    fn each_f_parameter_is_replaced_with_the_quoted_path() {
        for (cmd, expected) in [
            ("cmd", "cmd"),
            ("cmd %f", "cmd 'path'"),
            ("cmd a %f b %f c", "cmd a 'path' b 'path' c"),
            ("%f %f %f", "'path' 'path' 'path'"),
        ] {
            assert_eq!(
                substitute_f_parameter(cmd.into(), "path".into()),
                expected,
                "only `%f` is replaced, the rest of the command is copied exactly once"
            );
        }
    }
}
