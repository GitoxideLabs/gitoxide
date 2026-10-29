use std::{ffi::OsString, process::Stdio};

use bstr::{BString, ByteSlice, ByteVec};
use gix_error::{Result, ResultExt};

use crate::{Program, helper};

mod builtin;
pub(crate) mod store;

/// Git-compatible in-memory credential caching over Unix sockets.
#[cfg(unix)]
pub mod cache;

/// Prefix a helper name while preserving the shell syntax of its optional arguments.
fn external_name_command(name_and_args: &bstr::BStr, action: &helper::Action) -> Result<gix_command::Prepare> {
    let name = name_and_args
        .split(|byte| matches!(*byte, b' ' | b'\t' | b'\n'))
        .next()
        .unwrap_or_default();
    let name = if cfg!(windows) {
        name.strip_suffix(b".exe").unwrap_or(name)
    } else {
        name
    };
    let builtin = matches!(name, b"store" | b"cache" | b"cache--daemon");
    // Keep builtin precedence until these helper implementations move into this crate.
    let mut command = if builtin {
        let program = gix_path::to_unix_separators_on_windows(gix_path::into_bstr(gix_path::env::exe_invocation())?);
        let mut command = gix_quote::single(program.as_ref());
        command.push_str(" credential-");
        command
    } else {
        BString::from(if cfg!(windows) {
            "git-credential-"
        } else {
            "command git-credential-"
        })
    };
    command.push_str(name_and_args);
    command.push(b' ');
    command.push_str(action.as_arg(true));
    let command = gix_command::prepare(gix_path::from_bstring(command)?);
    Ok(if builtin || cfg!(windows) {
        command.command_may_be_shell_script_allow_manual_argument_splitting()
    } else {
        // Preserve Git's ENOEXEC fallback for executable scripts without a shebang.
        command.with_shell()
    })
}

/// The kind of helper program to use.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Kind {
    /// The built-in `git credential` helper program, part of any `git` distribution.
    Builtin,
    /// A custom credentials helper, as identified just by the name with optional arguments
    ExternalName {
        /// The name like `foo` along with optional args, like `foo --arg --bar="a b"`, with arguments using `sh` shell quoting rules.
        /// The program executed will be `git-credential-foo [args]` if `name_and_args` starts with `foo [args]`.
        /// Named helpers use Git's exec-path followed by `PATH`, without a Git dispatcher.
        /// [`helper::invoke()`] runs static `store` definitions natively, and `cache` on Unix.
        /// Native cache startup needs an explicitly configured `Program::cache_daemon` on Unix.
        /// [`Program::to_command()`] always returns a subprocess command, including for native helpers.
        name_and_args: BString,
    },
    /// A custom credentials helper, as identified just by the absolute path to the program and optional arguments. The program is executed through a shell.
    ExternalPath {
        /// The absolute path to the executable, like `/path/to/exe` along with optional args, like `/path/to/exe --arg --bar="a b"`, with arguments using `sh`
        /// shell quoting rules.
        path_and_args: BString,
    },
    /// A script to execute with `sh`.
    ExternalShellScript(BString),
}

/// Initialization
impl Program {
    /// Create a new program of the given `kind`.
    pub fn from_kind(kind: Kind) -> Self {
        Program {
            kind,
            child: None,
            current_dir: None,
            stderr: true,
            store_lock_timeout: std::time::Duration::from_secs(1).into(),
            #[cfg(unix)]
            cache_daemon: None,
        }
    }

    /// Parse the given input as per the custom helper definition, supporting `!<script>`, `name` and `/absolute/name`, the latter two
    /// also support arguments which are ignored here.
    ///
    /// Returns an error if the helper name or path cannot be represented by the platform.
    pub fn from_custom_definition(input: impl Into<BString>) -> Result<Self> {
        fn from_custom_definition_inner(mut input: BString) -> Result<Program> {
            let kind = if input.starts_with(b"!") {
                input.remove(0);
                Kind::ExternalShellScript(input)
            } else {
                let path = gix_path::from_bstr(
                    input
                        .find_byte(b' ')
                        .map_or(input.as_slice(), |pos| &input[..pos])
                        .as_bstr(),
                )?;
                if gix_path::is_absolute(path) {
                    Kind::ExternalPath { path_and_args: input }
                } else {
                    Kind::ExternalName { name_and_args: input }
                }
            };
            Ok(Program {
                kind,
                child: None,
                current_dir: None,
                stderr: true,
                store_lock_timeout: std::time::Duration::from_secs(1).into(),
                #[cfg(unix)]
                cache_daemon: None,
            })
        }
        from_custom_definition_inner(input.into())
    }

    /// Convert the program into the respective command, suitable to invoke `action`.
    ///
    /// Returns an error if helper or executable paths cannot be represented in the required encoding.
    pub fn to_command(&self, action: &helper::Action) -> Result<std::process::Command> {
        self.to_command_with_environment(action, &[])
    }

    /// Prepare a command like [`Self::to_command()`], applying `environment` only to the child.
    ///
    /// Repeated keys use their last value. The current process environment is never modified.
    pub fn to_command_with_environment(
        &self,
        action: &helper::Action,
        environment: &[(std::ffi::OsString, std::ffi::OsString)],
    ) -> Result<std::process::Command> {
        let mut prepare = match &self.kind {
            Kind::Builtin => gix_command::prepare(gix_path::env::exe_invocation())
                .arg("credential")
                .arg(action.as_arg(false)),
            Kind::ExternalName { name_and_args } => external_name_command(name_and_args.as_bstr(), action)?,
            Kind::ExternalShellScript(for_shell)
            | Kind::ExternalPath {
                path_and_args: for_shell,
            } => {
                // Git appends the action to the script itself, leaving shell positional parameters empty.
                let mut for_shell = for_shell.clone();
                for_shell.push(b' ');
                for_shell.push_str(action.as_arg(true));
                gix_command::prepare(gix_path::from_bstring(for_shell)?).command_may_be_shell_script()
            }
        };
        prepare.env.extend_from_slice(environment);
        prepare.current_dir.clone_from(&self.current_dir);
        if matches!(self.kind, Kind::ExternalName { .. }) {
            let effective_env = |name: &str| {
                environment
                    .iter()
                    .rev()
                    .find(|(key, _)| key == name)
                    .map(|(_, value)| value.clone())
                    .or_else(|| std::env::var_os(name))
            };
            if let Ok(cwd) = self
                .current_dir
                .as_deref()
                .map_or_else(std::env::current_dir, std::path::absolute)
                && let Some(core) = gix_path::env::core_dir_from_environment(effective_env, &cwd)
            {
                let path = effective_env("PATH").unwrap_or_default();
                let path =
                    std::env::join_paths(std::iter::once(core).chain(std::env::split_paths(&path))).or_error()?;
                prepare.env.push((OsString::from("PATH"), path));
            }
        }
        let mut cmd: std::process::Command = prepare.try_into()?;
        cmd.stdin(Stdio::piped())
            .stdout(if action.expects_output() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stderr(if self.stderr { Stdio::inherit() } else { Stdio::null() });
        Ok(cmd)
    }
}

/// Builder
impl Program {
    /// By default `stderr` of programs is inherited and typically displayed in the terminal.
    pub fn suppress_stderr(mut self) -> Self {
        self.stderr = false;
        self
    }
}

impl Program {
    pub(crate) fn start(
        &mut self,
        action: &helper::Action,
        environment: &[(std::ffi::OsString, std::ffi::OsString)],
    ) -> std::io::Result<(std::process::ChildStdin, Option<std::process::ChildStdout>)> {
        assert!(self.child.is_none(), "BUG: must not call `start()` twice");
        let mut cmd = self
            .to_command_with_environment(action, environment)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
        gix_trace::debug!(cmd = ?cmd, "launching credential helper");
        let mut child = cmd.spawn()?;
        let stdin = child.stdin.take().expect("stdin to be configured");
        let stdout = child.stdout.take();

        self.child = child.into();
        Ok((stdin, stdout))
    }

    pub(crate) fn finish(&mut self) -> std::io::Result<()> {
        let mut child = self.child.take().expect("Call `start()` before calling finish()");
        let status = child.wait()?;
        if status.success() {
            Ok(())
        } else {
            let failure = gix_error::message("Credentials helper program failed").with_exit_status(status);
            Err(std::io::Error::other(failure))
        }
    }
}

///
pub mod main;
pub use main::function::main;

#[cfg(test)]
mod tests {
    #[test]
    fn named_helpers_preserve_shell_expansion() -> gix_testtools::TestResult {
        let cmd: std::process::Command = super::external_name_command(
            "manager --config=~/credentials".into(),
            &crate::helper::Action::Get(Default::default()),
        )?
        .try_into()?;
        let script = cmd
            .get_args()
            .skip_while(|arg| *arg != std::ffi::OsStr::new("-c"))
            .nth(1)
            .expect("a shell invocation has its script after the '-c' argument");
        assert_eq!(
            script,
            if cfg!(windows) {
                "git-credential-manager --config=~/credentials get"
            } else {
                "command git-credential-manager --config=~/credentials get"
            },
            "the fixed action is appended to the script, without shell positional parameters"
        );
        Ok(())
    }
}
