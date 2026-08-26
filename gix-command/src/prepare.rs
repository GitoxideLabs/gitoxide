use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use bstr::ByteSlice;

use crate::{Context, Prepare, extract_interpreter, is_bare_command, split_paths, win_path_lookup};

/// Builder
impl Prepare {
    /// If called, the command will be checked for characters that are typical for shell
    /// scripts, and if found will use `sh` to execute it or whatever is set as
    /// [`with_shell_program()`](Self::with_shell_program()).
    ///
    /// Commands are inspected as bytes, including non-UTF-8 commands on Unix. If the platform
    /// cannot represent a command as bytes, it is invoked directly.
    ///
    /// If a shell is used, then arguments given here with [arg()](Self::arg) or
    /// [args()](Self::args) will be substituted via `"$@"` if it's not already present in the
    /// command.
    ///
    ///
    /// The [`command_may_be_shell_script_allow_manual_argument_splitting()`](Self::command_may_be_shell_script_allow_manual_argument_splitting())
    /// and [`command_may_be_shell_script_disallow_manual_argument_splitting()`](Self::command_may_be_shell_script_disallow_manual_argument_splitting())
    /// methods also call this method.
    ///
    /// If neither this method nor [`with_shell()`](Self::with_shell()) is called, commands are
    /// always executed verbatim and directly, without the use of a shell.
    pub fn command_may_be_shell_script(mut self) -> Self {
        self.use_shell = gix_path::os_str_into_bstr(&self.command)
            .is_ok_and(|cmd| cmd.find_byteset(b"|&;<>()$`\\\"' \t\n*?[#~=%").is_some());
        self
    }

    /// If called, unconditionally use a shell to execute the command and its arguments.
    ///
    /// This uses `sh` to execute it, or whatever is set as
    /// [`with_shell_program()`](Self::with_shell_program()).
    ///
    /// Arguments given here with [arg()](Self::arg) or [args()](Self::args) will be
    /// substituted via `"$@"` if it's not already present in the command.
    ///
    /// If neither this method nor
    /// [`command_may_be_shell_script()`](Self::command_may_be_shell_script()) is called,
    /// commands are always executed verbatim and directly, without the use of a shell. (But
    /// see [`command_may_be_shell_script()`](Self::command_may_be_shell_script()) on other
    /// methods that call that method.)
    ///
    /// We also disallow manual argument splitting
    /// (see [`command_may_be_shell_script_disallow_manual_argument_splitting`](Self::command_may_be_shell_script_disallow_manual_argument_splitting()))
    /// to assure a shell is indeed used, no matter what.
    pub fn with_shell(mut self) -> Self {
        self.use_shell = true;
        self.allow_manual_arg_splitting = false;
        self
    }

    /// Quote the command if it is run in a shell, so its path is left intact.
    ///
    /// This is only meaningful if the command has been arranged to run in a shell, either
    /// unconditionally with [`with_shell()`](Self::with_shell()), or conditionally with
    /// [`command_may_be_shell_script()`](Self::command_may_be_shell_script()).
    ///
    /// Note that this should not be used if the command is a script - quoting is only the
    /// right choice if it's known to be a program path.
    ///
    /// Note also that this does not affect arguments passed with [arg()](Self::arg) or
    /// [args()](Self::args), which do not have to be quoted by the *caller* because they are
    /// passed as `"$@"` positional parameters (`"$1"`, `"$2"`, and so on).
    pub fn with_quoted_command(mut self) -> Self {
        self.quote_command = true;
        self
    }

    /// Set the name or path to the shell `program` to use if a shell is to be used, to avoid
    /// using the default shell which is `sh`.
    ///
    /// Note that shells that are not Bourne-style cannot be expected to work correctly,
    /// because POSIX shell syntax is assumed when searching for and conditionally adding
    /// `"$@"` to receive arguments, where applicable (and in the behaviour of
    /// [`with_quoted_command()`](Self::with_quoted_command()), if called).
    pub fn with_shell_program(mut self, program: impl Into<OsString>) -> Self {
        self.shell_program = Some(program.into());
        self
    }

    /// Unconditionally turn off using the shell when spawning the command.
    ///
    /// Note that not using the shell is the default. So an effective use of this method
    /// is some time after [`command_may_be_shell_script()`](Self::command_may_be_shell_script())
    /// or [`with_shell()`](Self::with_shell()) was called.
    pub fn without_shell(mut self) -> Self {
        self.use_shell = false;
        self
    }

    /// Set additional `ctx` to be used when spawning the process.
    ///
    /// Note that this is a must for most kind of commands that `git` usually spawns, as at
    /// least they need to know the correct Git repository to function.
    pub fn with_context(mut self, ctx: Context) -> Self {
        self.context = Some(ctx);
        self
    }

    /// Like [`command_may_be_shell_script()`](Self::command_may_be_shell_script()), but try to
    /// split arguments by hand if this can be safely done without a shell.
    ///
    /// This is useful on platforms where spawning processes is slow, or where many processes
    /// have to be spawned in a row which should be sped up. Manual argument splitting is
    /// enabled by default on Windows only.
    ///
    /// Note that this does *not* check for the use of possible shell builtins. Commands may
    /// fail or behave differently if they are available as shell builtins and no corresponding
    /// external command exists, or the external command behaves differently.
    /// Leading shell assignment words are applied to the environment when followed by a command.
    pub fn command_may_be_shell_script_allow_manual_argument_splitting(mut self) -> Self {
        self.allow_manual_arg_splitting = true;
        self.command_may_be_shell_script()
    }

    /// Like [`command_may_be_shell_script()`](Self::command_may_be_shell_script()), but don't
    /// allow to bypass the shell even if manual argument splitting can be performed safely.
    pub fn command_may_be_shell_script_disallow_manual_argument_splitting(mut self) -> Self {
        self.allow_manual_arg_splitting = false;
        self.command_may_be_shell_script()
    }

    /// Configure the process to use `stdio` for _stdin_.
    pub fn stdin(mut self, stdio: Stdio) -> Self {
        self.stdin = stdio;
        self
    }
    /// Configure the process to use `stdio` for _stdout_.
    pub fn stdout(mut self, stdio: Stdio) -> Self {
        self.stdout = stdio;
        self
    }
    /// Configure the process to use `stdio` for _stderr_.
    pub fn stderr(mut self, stdio: Stdio) -> Self {
        self.stderr = stdio;
        self
    }

    /// Add `arg` to the list of arguments to call the command with.
    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Add `args` to the list of arguments to call the command with.
    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<OsString>>) -> Self {
        self.args
            .append(&mut args.into_iter().map(Into::into).collect::<Vec<_>>());
        self
    }

    /// Add `key` with `value` to the environment of the spawned command.
    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }
}

/// Finalization
impl Prepare {
    /// Spawn the command as configured.
    pub fn spawn(self) -> std::io::Result<std::process::Child> {
        let mut cmd = Command::from(self);
        gix_trace::debug!(cmd = ?cmd);
        cmd.spawn()
    }
}

impl From<Prepare> for Command {
    fn from(mut prep: Prepare) -> Command {
        let mut inline_env = Vec::new();
        let mut cmd = if prep.use_shell {
            let split_args = prep
                .allow_manual_arg_splitting
                .then(|| {
                    let command = gix_path::os_str_into_bstr(&prep.command).ok()?;
                    if command.find_byteset(b"\\|&;<>()$`\n*?[#~%").is_none() {
                        crate::parse::command_line(command).ok()
                    } else {
                        None
                    }
                })
                .flatten();
            match split_args {
                Some(parsed) => {
                    let mut cmd = if cfg!(windows) {
                        windows_command(parsed.command, &prep.env, &parsed.env)
                    } else {
                        Command::new(parsed.command)
                    };
                    cmd.args(parsed.args);
                    inline_env = parsed.env;
                    cmd
                }
                None => {
                    let mut cmd = match prep.shell_program {
                        Some(shell) => Command::new(shell),
                        None => gix_path::env::shell_command(),
                    };
                    // Passed as `command_name` after `-c <script>`; the shell uses it
                    // as `$0`, which prefixes its own diagnostic messages. If the
                    // shell path has no extractable basename — reachable only via
                    // degenerate input like `""` or `/` — fall back to `_`, the
                    // conventional placeholder for an unused `$0`, rather than
                    // making a false claim about which shell is running.
                    let arg0 = std::path::Path::new(cmd.get_program())
                        .file_name()
                        .unwrap_or(std::ffi::OsStr::new("_"))
                        .to_os_string();
                    cmd.arg("-c");
                    if !prep.args.is_empty() {
                        if !gix_path::os_str_into_bstr(&prep.command).is_ok_and(|cmd| cmd.contains_str("$@")) {
                            if prep.quote_command {
                                if let Ok(command) = gix_path::os_str_into_bstr(&prep.command) {
                                    prep.command = gix_path::from_bstring(gix_quote::single(command)).into();
                                }
                            }
                            prep.command.push(r#" "$@""#);
                        } else {
                            gix_trace::debug!(
                                r#"Will not add '"$@"' to '{:?}' as it seems to contain '$@' already"#,
                                prep.command
                            );
                        }
                    }
                    cmd.arg(prep.command);
                    cmd.arg(arg0);
                    cmd
                }
            }
        } else if cfg!(windows) {
            windows_command(prep.command, &prep.env, &[])
        } else {
            Command::new(prep.command)
        };
        // We never want to have terminals pop-up on Windows if this runs from a GUI application.
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        cmd.stdin(prep.stdin)
            .stdout(prep.stdout)
            .stderr(prep.stderr)
            .envs(prep.env)
            .args(prep.args);
        if let Some(ctx) = prep.context {
            if let Some(git_dir) = ctx.git_dir {
                cmd.env("GIT_DIR", &git_dir);
            }
            if let Some(worktree_dir) = ctx.worktree_dir {
                cmd.env("GIT_WORK_TREE", worktree_dir);
            }
            if let Some(value) = ctx.no_replace_objects {
                cmd.env("GIT_NO_REPLACE_OBJECTS", usize::from(value).to_string());
            }
            if let Some(namespace) = ctx.ref_namespace {
                cmd.env("GIT_NAMESPACE", gix_path::from_bstring(namespace));
            }
            if let Some(value) = ctx.literal_pathspecs {
                cmd.env("GIT_LITERAL_PATHSPECS", usize::from(value).to_string());
            }
            if let Some(value) = ctx.glob_pathspecs {
                cmd.env(
                    if value {
                        "GIT_GLOB_PATHSPECS"
                    } else {
                        "GIT_NOGLOB_PATHSPECS"
                    },
                    "1",
                );
            }
            if let Some(value) = ctx.icase_pathspecs {
                cmd.env("GIT_ICASE_PATHSPECS", usize::from(value).to_string());
            }
            if let Some(stderr) = ctx.stderr {
                cmd.stderr(if stderr { Stdio::inherit() } else { Stdio::null() });
            }
        }
        cmd.envs(inline_env);
        cmd
    }
}

/// Create a Windows command using Git-compatible `PATH` lookup and shebang dispatch.
///
/// The last `PATH` in `inline_env` overrides the last one in `env`, which overrides the inherited value (of this process).
/// The selected `PATH` is searched in order. A resolved shebang script is launched through its interpreter, ignoring shebang
/// arguments. If an explicit `PATH` does not resolve a bare command, the missing program remains anchored in its first entry
/// so Rust's broader Windows lookup cannot find it elsewhere.
fn windows_command(command: OsString, env: &[(OsString, OsString)], inline_env: &[(String, OsString)]) -> Command {
    let explicit_joined_paths = inline_env
        .iter()
        .rev()
        .find(|(name, _)| name.eq_ignore_ascii_case("PATH"))
        .map(|(_, value)| value.as_os_str())
        .or_else(|| {
            env.iter()
                .rev()
                .find(|(name, _)| name.to_str().is_some_and(|name| name.eq_ignore_ascii_case("PATH")))
                .map(|(_, value)| value.as_os_str())
        });
    let joined_paths = explicit_joined_paths
        .map(Cow::Borrowed)
        .or_else(|| std::env::var_os("PATH").map(Cow::Owned));
    let looked_up = joined_paths
        .as_deref()
        .and_then(|joined_paths| win_path_lookup(command.as_ref(), joined_paths));
    let program: Cow<'_, Path> = match (looked_up, explicit_joined_paths) {
        // Use the manually resolved path.
        (Some(program), _) => Cow::Owned(program),
        // An explicit `PATH` miss must not fall back to `std::process::Command` broader Windows search.
        (None, Some(explicit_joined_paths)) if is_bare_command(Path::new(&command)) => {
            Cow::Owned(prevent_further_path_lookup(command.as_ref(), explicit_joined_paths))
        }
        // Preserve non-bare commands and let `std::process::Command` resolve bare commands without an explicit `PATH`.
        (None, _) => Cow::Borrowed(command.as_ref()),
    };
    if let Some(shebang) = extract_interpreter(program.as_ref()) {
        let mut cmd = Command::new(shebang.interpreter);
        // Git for Windows ignores shebang arguments and passes only the script path.
        cmd.arg(program.as_ref());
        cmd
    } else {
        match program {
            // Process lookup happens before the child's environment is installed, so an explicitly
            // configured PATH must be handled here for ordinary executables as well.
            Cow::Owned(program) if explicit_joined_paths.is_some() => Command::new(program),
            _ => Command::new(command),
        }
    }
}

/// Represent the failed lookup of `command` in an explicitly assigned `PATH` without permitting another search.
///
/// `joined_paths` is the complete value of the explicit `PATH`, not one of its entries. The first non-empty entry is
/// joined with `command`, producing a path that Rust's Windows resolver will not look up elsewhere. If there is no such
/// entry, a trailing separator makes `command` invalid instead.
fn prevent_further_path_lookup(command: &Path, joined_paths: &OsStr) -> PathBuf {
    if let Some(mut root) = split_paths(joined_paths).next() {
        root.push(command);
        root
    } else {
        let mut missing = command.to_owned();
        // A trailing separator makes the program invalid without allowing Rust to search ambient locations.
        missing.push("");
        missing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_path_lookup_failure_stays_within_that_path() -> gix_testtools::Result {
        let joined_paths = std::env::join_paths(["", "not/a/real/path", "also/not/real"])?;
        let cmd = windows_command("missing.exe".into(), &[], &[("PATH".into(), joined_paths)]);
        assert_eq!(
            cmd.get_program(),
            std::path::Path::new("not/a/real/path/missing.exe"),
            "an inline PATH miss must not leave a bare program for Rust to resolve elsewhere"
        );
        Ok(())
    }
}
